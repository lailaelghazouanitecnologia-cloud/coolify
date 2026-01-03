//! Start PostgreSQL Action
//!
//! Starts a PostgreSQL database container with proper configuration,
//! volumes, SSL support, and init scripts.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tracing::{info, instrument};

use crate::models::{Server, StandaloneDatabase};
use super::{DatabaseConfig, DatabaseType, ResourceLimits, InitScript, generate_compose, database_configuration_dir};
use crate::actions::{Action, ActionError, ActionResult, CommandBuilder};
use kornetti_ssh::SshClient;

/// PostgreSQL specific configuration
#[derive(Debug, Clone)]
pub struct PostgresqlConfig {
    /// PostgreSQL user
    pub postgres_user: String,
    /// PostgreSQL password
    pub postgres_password: String,
    /// Default database name
    pub postgres_db: String,
    /// Custom postgresql.conf content
    pub postgres_conf: Option<String>,
    /// Init scripts
    pub init_scripts: Vec<InitScript>,
    /// Enable SSL
    pub enable_ssl: bool,
}

impl Default for PostgresqlConfig {
    fn default() -> Self {
        Self {
            postgres_user: "postgres".to_string(),
            postgres_password: String::new(),
            postgres_db: "postgres".to_string(),
            postgres_conf: None,
            init_scripts: Vec::new(),
            enable_ssl: false,
        }
    }
}

/// Input for starting PostgreSQL
pub struct StartPostgresqlInput<'a> {
    pub server: &'a Server,
    pub database: &'a StandaloneDatabase,
    pub config: PostgresqlConfig,
}

/// Action to start a PostgreSQL database
pub struct StartPostgresql {
    ssh: Arc<SshClient>,
}

impl StartPostgresql {
    pub fn new(ssh: Arc<SshClient>) -> Self {
        Self { ssh }
    }

    /// Build database configuration
    fn build_config(database: &StandaloneDatabase, pg_config: &PostgresqlConfig) -> DatabaseConfig {
        let container_name = database.uuid.to_string();
        let config_dir = database_configuration_dir(&container_name);

        let mut environment = HashMap::new();
        environment.insert("POSTGRES_USER".to_string(), pg_config.postgres_user.clone());
        environment.insert("POSTGRES_PASSWORD".to_string(), pg_config.postgres_password.clone());
        environment.insert("POSTGRES_DB".to_string(), pg_config.postgres_db.clone());
        environment.insert("PGUSER".to_string(), pg_config.postgres_user.clone());

        let mut volumes = vec![
            (format!("{}_data", container_name), "/var/lib/postgresql/data".to_string()),
        ];

        // Add init scripts directory
        if !pg_config.init_scripts.is_empty() {
            volumes.push((
                format!("{}/docker-entrypoint-initdb.d", config_dir),
                "/docker-entrypoint-initdb.d".to_string(),
            ));
        }

        // Add custom config
        if pg_config.postgres_conf.is_some() {
            volumes.push((
                format!("{}/custom-postgres.conf", config_dir),
                "/etc/postgresql/postgresql.conf".to_string(),
            ));
        }

        // Add SSL certificates
        if pg_config.enable_ssl {
            volumes.push((
                format!("{}/ssl", config_dir),
                "/var/lib/postgresql/certs".to_string(),
            ));
        }

        DatabaseConfig {
            container_name,
            image: database.image.clone().unwrap_or_else(|| "postgres:16-alpine".to_string()),
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
            init_scripts: pg_config.init_scripts.clone(),
            enable_ssl: pg_config.enable_ssl,
        }
    }
}

#[async_trait]
impl Action for StartPostgresql {
    type Input = StartPostgresqlInput<'static>;
    type Output = ActionResult;

    fn name(&self) -> &'static str {
        "start_postgresql"
    }

    #[instrument(skip(self, input), fields(database_id = %input.database.id))]
    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        let server = input.server;
        let database = input.database;
        let pg_config = &input.config;
        let start = std::time::Instant::now();

        info!("Starting PostgreSQL database {}", database.id);

        let config = Self::build_config(database, pg_config);
        let container_name = &config.container_name;
        let config_dir = database_configuration_dir(container_name);

        let session = self.ssh.connect(server)
            .await
            .map_err(|e| ActionError::ssh_connection_failed(e.to_string()))?;

        let mut builder = CommandBuilder::new();

        // Create directories
        builder.echo("Starting PostgreSQL database...");
        builder.echo("Creating directories...");
        builder.mkdir(&config_dir);
        builder.mkdir(&format!("{}/docker-entrypoint-initdb.d", config_dir));

        // Handle SSL setup if enabled
        if pg_config.enable_ssl {
            builder.echo("Setting up SSL...");
            builder.mkdir(&format!("{}/ssl", config_dir));
            // Note: In production, you'd generate/copy SSL certificates here
        } else {
            builder.add(format!("rm -rf {}/ssl", config_dir));
        }

        // Clean up old init scripts and add new ones
        builder.add(format!("rm -rf {}/docker-entrypoint-initdb.d/*", config_dir));

        for script in &pg_config.init_scripts {
            let content_base64 = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                &script.content
            );
            builder.add(format!(
                "echo '{}' | base64 -d > {}/docker-entrypoint-initdb.d/{}",
                content_base64, config_dir, script.filename
            ));
        }

        // Handle custom postgres.conf
        if let Some(ref conf) = pg_config.postgres_conf {
            let mut conf_content = conf.clone();
            if !conf_content.contains("listen_addresses") {
                conf_content.push_str("\nlisten_addresses = '*'");
            }
            let content_base64 = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                &conf_content
            );
            builder.add(format!(
                "echo '{}' | base64 -d > {}/custom-postgres.conf",
                content_base64, config_dir
            ));
        } else {
            builder.add(format!("rm -f {}/custom-postgres.conf", config_dir));
        }

        // Generate docker-compose.yml
        let compose_content = generate_compose(DatabaseType::Postgresql, &config);
        let compose_base64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &compose_content
        );
        builder.add(format!(
            "echo '{}' | base64 -d > {}/docker-compose.yml",
            compose_base64, config_dir
        ));

        // Pull image
        builder.echo(&format!("Pulling {} image...", config.image));
        builder.add(format!("docker compose -f {}/docker-compose.yml pull", config_dir));

        // Stop and remove existing container
        builder.add(format!("docker stop -t 10 {} 2>/dev/null || true", container_name));
        builder.add(format!("docker rm -f {} 2>/dev/null || true", container_name));

        // Start new container
        builder.echo("Starting container...");
        builder.add(format!("docker compose -f {}/docker-compose.yml up -d", config_dir));

        // Fix SSL permissions if enabled
        if pg_config.enable_ssl {
            builder.add(format!(
                "docker exec {} chown postgres:postgres /var/lib/postgresql/certs/*.key /var/lib/postgresql/certs/*.crt 2>/dev/null || true",
                container_name
            ));
        }

        builder.echo("PostgreSQL started successfully!");

        let commands = builder.build();

        // Execute commands
        for cmd in &commands {
            let result = session.execute(cmd)
                .await
                .map_err(|e| ActionError::command_failed(cmd, e.to_string()))?;

            if result.exit_code != 0 && !cmd.contains("|| true") && !cmd.contains("2>/dev/null") {
                return Err(ActionError::command_failed(cmd, result.stderr));
            }
        }

        // Verify container is running
        let status = session.execute(&format!("docker inspect -f '{{{{.State.Running}}}}' {}", container_name))
            .await
            .map_err(|e| ActionError::command_failed("check container", e.to_string()))?;

        if status.stdout.trim() != "true" {
            return Err(ActionError::new(
                "CONTAINER_NOT_RUNNING",
                "PostgreSQL container failed to start"
            ));
        }

        Ok(ActionResult::success("PostgreSQL started successfully")
            .with_duration(start.elapsed())
            .with_commands(commands)
            .with_output(serde_json::json!({
                "container_name": container_name,
                "image": config.image,
                "config_dir": config_dir,
            })))
    }
}
