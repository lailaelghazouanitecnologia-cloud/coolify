//! Start MariaDB Action
//!
//! Starts a MariaDB database container (MySQL-compatible).

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tracing::{info, instrument};

use crate::models::{Server, StandaloneDatabase};
use super::{DatabaseConfig, DatabaseType, ResourceLimits, InitScript, generate_compose, database_configuration_dir};
use crate::actions::{Action, ActionError, ActionResult, CommandBuilder};
use crate::ssh_stub::SshClient;

/// MariaDB specific configuration (similar to MySQL)
#[derive(Debug, Clone)]
pub struct MariadbConfig {
    pub mariadb_root_password: String,
    pub mariadb_database: String,
    pub mariadb_user: Option<String>,
    pub mariadb_password: Option<String>,
    pub mariadb_conf: Option<String>,
    pub init_scripts: Vec<InitScript>,
}

impl Default for MariadbConfig {
    fn default() -> Self {
        Self {
            mariadb_root_password: String::new(),
            mariadb_database: "mariadb".to_string(),
            mariadb_user: None,
            mariadb_password: None,
            mariadb_conf: None,
            init_scripts: Vec::new(),
        }
    }
}

pub struct StartMariadbInput<'a> {
    pub server: &'a Server,
    pub database: &'a StandaloneDatabase,
    pub config: MariadbConfig,
}

pub struct StartMariadb {
    ssh: Arc<dyn SshClient>,
}

impl StartMariadb {
    pub fn new(ssh: Arc<dyn SshClient>) -> Self {
        Self { ssh }
    }

    fn build_config(database: &StandaloneDatabase, config: &MariadbConfig) -> DatabaseConfig {
        let container_name = database.uuid.to_string();
        let config_dir = database_configuration_dir(&container_name);

        let mut environment = HashMap::new();
        environment.insert("MARIADB_ROOT_PASSWORD".to_string(), config.mariadb_root_password.clone());
        environment.insert("MARIADB_DATABASE".to_string(), config.mariadb_database.clone());

        if let Some(ref user) = config.mariadb_user {
            environment.insert("MARIADB_USER".to_string(), user.clone());
        }
        if let Some(ref password) = config.mariadb_password {
            environment.insert("MARIADB_PASSWORD".to_string(), password.clone());
        }

        let mut volumes = vec![
            (format!("{}_data", container_name), "/var/lib/mysql".to_string()),
        ];

        if !config.init_scripts.is_empty() {
            volumes.push((
                format!("{}/docker-entrypoint-initdb.d", config_dir),
                "/docker-entrypoint-initdb.d".to_string(),
            ));
        }

        DatabaseConfig {
            container_name,
            image: database.image.clone().unwrap_or_else(|| "mariadb:11".to_string()),
            network: "coolify".to_string(),
            ports: database.port_mappings.clone(),
            environment,
            volumes,
            limits: ResourceLimits::default(),
            custom_docker_options: database.custom_docker_run_options.clone(),
            init_scripts: config.init_scripts.clone(),
            enable_ssl: false,
        }
    }
}

#[async_trait]
impl Action for StartMariadb {
    type Input = StartMariadbInput<'static>;
    type Output = ActionResult;

    fn name(&self) -> &'static str {
        "start_mariadb"
    }

    #[instrument(skip(self, input), fields(database_id = %input.database.id))]
    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        let server = input.server;
        let database = input.database;
        let mariadb_config = &input.config;
        let start = std::time::Instant::now();

        info!("Starting MariaDB database {}", database.id);

        let config = Self::build_config(database, mariadb_config);
        let container_name = &config.container_name;
        let config_dir = database_configuration_dir(container_name);

        let session = self.ssh.connect(server)
            .await
            .map_err(|e| ActionError::ssh_connection_failed(e.to_string()))?;

        let mut builder = CommandBuilder::new();

        builder.echo("Starting MariaDB database...");
        builder.mkdir(&config_dir);
        builder.mkdir(&format!("{}/docker-entrypoint-initdb.d", config_dir));

        let compose_content = generate_compose(DatabaseType::Mariadb, &config);
        let compose_base64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &compose_content
        );
        builder.add(format!(
            "echo '{}' | base64 -d > {}/docker-compose.yml",
            compose_base64, config_dir
        ));

        builder.add(format!("docker compose -f {}/docker-compose.yml pull", config_dir));
        builder.add(format!("docker stop -t 10 {} 2>/dev/null || true", container_name));
        builder.add(format!("docker rm -f {} 2>/dev/null || true", container_name));
        builder.add(format!("docker compose -f {}/docker-compose.yml up -d", config_dir));
        builder.echo("MariaDB started successfully!");

        let commands = builder.build();

        for cmd in &commands {
            let result = session.execute(cmd)
                .await
                .map_err(|e| ActionError::command_failed(cmd, e.to_string()))?;

            if result.exit_code != 0 && !cmd.contains("|| true") {
                return Err(ActionError::command_failed(cmd, result.stderr));
            }
        }

        Ok(ActionResult::success("MariaDB started successfully")
            .with_duration(start.elapsed())
            .with_commands(commands))
    }
}
