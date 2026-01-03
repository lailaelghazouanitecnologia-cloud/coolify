//! Start MySQL Action
//!
//! Starts a MySQL database container with proper configuration.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tracing::{info, instrument};

use crate::models::{Server, StandaloneDatabase};
use super::{DatabaseConfig, DatabaseType, ResourceLimits, InitScript, generate_compose, database_configuration_dir};
use crate::actions::{Action, ActionError, ActionResult, CommandBuilder};
use kornetti_ssh::SshClient;

/// MySQL specific configuration
#[derive(Debug, Clone)]
pub struct MysqlConfig {
    /// Root password
    pub mysql_root_password: String,
    /// Default database name
    pub mysql_database: String,
    /// Optional user
    pub mysql_user: Option<String>,
    /// User password
    pub mysql_password: Option<String>,
    /// Custom my.cnf content
    pub mysql_conf: Option<String>,
    /// Init scripts
    pub init_scripts: Vec<InitScript>,
}

impl Default for MysqlConfig {
    fn default() -> Self {
        Self {
            mysql_root_password: String::new(),
            mysql_database: "mysql".to_string(),
            mysql_user: None,
            mysql_password: None,
            mysql_conf: None,
            init_scripts: Vec::new(),
        }
    }
}

/// Input for starting MySQL
pub struct StartMysqlInput<'a> {
    pub server: &'a Server,
    pub database: &'a StandaloneDatabase,
    pub config: MysqlConfig,
}

/// Action to start a MySQL database
pub struct StartMysql {
    ssh: Arc<SshClient>,
}

impl StartMysql {
    pub fn new(ssh: Arc<SshClient>) -> Self {
        Self { ssh }
    }

    fn build_config(database: &StandaloneDatabase, mysql_config: &MysqlConfig) -> DatabaseConfig {
        let container_name = database.uuid.to_string();
        let config_dir = database_configuration_dir(&container_name);

        let mut environment = HashMap::new();
        environment.insert("MYSQL_ROOT_PASSWORD".to_string(), mysql_config.mysql_root_password.clone());
        environment.insert("MYSQL_DATABASE".to_string(), mysql_config.mysql_database.clone());

        if let Some(ref user) = mysql_config.mysql_user {
            environment.insert("MYSQL_USER".to_string(), user.clone());
        }
        if let Some(ref password) = mysql_config.mysql_password {
            environment.insert("MYSQL_PASSWORD".to_string(), password.clone());
        }

        let mut volumes = vec![
            (format!("{}_data", container_name), "/var/lib/mysql".to_string()),
        ];

        if !mysql_config.init_scripts.is_empty() {
            volumes.push((
                format!("{}/docker-entrypoint-initdb.d", config_dir),
                "/docker-entrypoint-initdb.d".to_string(),
            ));
        }

        if mysql_config.mysql_conf.is_some() {
            volumes.push((
                format!("{}/my.cnf", config_dir),
                "/etc/mysql/conf.d/custom.cnf".to_string(),
            ));
        }

        DatabaseConfig {
            container_name,
            image: database.image.clone().unwrap_or_else(|| "mysql:8.0".to_string()),
            network: "coolify".to_string(),
            ports: database.port_mappings.clone(),
            environment,
            volumes,
            limits: ResourceLimits {
                memory: database.limits_memory.clone(),
                memory_swap: database.limits_memory_swap.clone(),
                memory_reservation: database.limits_memory_reservation.clone(),
                cpus: database.limits_cpus,
                cpu_shares: database.limits_cpu_shares,
            },
            custom_docker_options: database.custom_docker_run_options.clone(),
            init_scripts: mysql_config.init_scripts.clone(),
            enable_ssl: false,
        }
    }
}

#[async_trait]
impl Action for StartMysql {
    type Input = StartMysqlInput<'static>;
    type Output = ActionResult;

    fn name(&self) -> &'static str {
        "start_mysql"
    }

    #[instrument(skip(self, input), fields(database_id = %input.database.id))]
    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        let server = input.server;
        let database = input.database;
        let mysql_config = &input.config;
        let start = std::time::Instant::now();

        info!("Starting MySQL database {}", database.id);

        let config = Self::build_config(database, mysql_config);
        let container_name = &config.container_name;
        let config_dir = database_configuration_dir(container_name);

        let session = self.ssh.connect(server)
            .await
            .map_err(|e| ActionError::ssh_connection_failed(e.to_string()))?;

        let mut builder = CommandBuilder::new();

        builder.echo("Starting MySQL database...");
        builder.mkdir(&config_dir);
        builder.mkdir(&format!("{}/docker-entrypoint-initdb.d", config_dir));

        // Clean up and add init scripts
        builder.add(format!("rm -rf {}/docker-entrypoint-initdb.d/*", config_dir));

        for script in &mysql_config.init_scripts {
            let content_base64 = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                &script.content
            );
            builder.add(format!(
                "echo '{}' | base64 -d > {}/docker-entrypoint-initdb.d/{}",
                content_base64, config_dir, script.filename
            ));
        }

        // Custom config
        if let Some(ref conf) = mysql_config.mysql_conf {
            let content_base64 = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                conf
            );
            builder.add(format!(
                "echo '{}' | base64 -d > {}/my.cnf",
                content_base64, config_dir
            ));
        }

        // Generate and write docker-compose
        let compose_content = generate_compose(DatabaseType::Mysql, &config);
        let compose_base64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &compose_content
        );
        builder.add(format!(
            "echo '{}' | base64 -d > {}/docker-compose.yml",
            compose_base64, config_dir
        ));

        builder.echo(&format!("Pulling {} image...", config.image));
        builder.add(format!("docker compose -f {}/docker-compose.yml pull", config_dir));

        builder.add(format!("docker stop -t 10 {} 2>/dev/null || true", container_name));
        builder.add(format!("docker rm -f {} 2>/dev/null || true", container_name));

        builder.echo("Starting container...");
        builder.add(format!("docker compose -f {}/docker-compose.yml up -d", config_dir));
        builder.echo("MySQL started successfully!");

        let commands = builder.build();

        for cmd in &commands {
            let result = session.execute(cmd)
                .await
                .map_err(|e| ActionError::command_failed(cmd, e.to_string()))?;

            if result.exit_code != 0 && !cmd.contains("|| true") {
                return Err(ActionError::command_failed(cmd, result.stderr));
            }
        }

        Ok(ActionResult::success("MySQL started successfully")
            .with_duration(start.elapsed())
            .with_commands(commands))
    }
}
