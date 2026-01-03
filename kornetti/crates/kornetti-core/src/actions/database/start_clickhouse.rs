//! Start Clickhouse Action
//!
//! Starts a Clickhouse analytics database container.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tracing::{info, instrument};

use crate::models::{Server, StandaloneDatabase};
use super::{DatabaseConfig, DatabaseType, ResourceLimits, generate_compose, database_configuration_dir};
use crate::actions::{Action, ActionError, ActionResult, CommandBuilder};
use kornetti_ssh::SshClient;

/// Clickhouse specific configuration
#[derive(Debug, Clone)]
pub struct ClickhouseConfig {
    pub clickhouse_user: String,
    pub clickhouse_password: String,
    pub clickhouse_default_database: Option<String>,
}

impl Default for ClickhouseConfig {
    fn default() -> Self {
        Self {
            clickhouse_user: "default".to_string(),
            clickhouse_password: String::new(),
            clickhouse_default_database: None,
        }
    }
}

pub struct StartClickhouseInput<'a> {
    pub server: &'a Server,
    pub database: &'a StandaloneDatabase,
    pub config: ClickhouseConfig,
}

pub struct StartClickhouse {
    ssh: Arc<SshClient>,
}

impl StartClickhouse {
    pub fn new(ssh: Arc<SshClient>) -> Self {
        Self { ssh }
    }

    fn build_config(database: &StandaloneDatabase, ch_config: &ClickhouseConfig) -> DatabaseConfig {
        let container_name = database.uuid.to_string();

        let mut environment = HashMap::new();
        environment.insert("CLICKHOUSE_USER".to_string(), ch_config.clickhouse_user.clone());
        environment.insert("CLICKHOUSE_PASSWORD".to_string(), ch_config.clickhouse_password.clone());

        if let Some(ref db) = ch_config.clickhouse_default_database {
            environment.insert("CLICKHOUSE_DEFAULT_ACCESS_MANAGEMENT".to_string(), "1".to_string());
            environment.insert("CLICKHOUSE_DB".to_string(), db.clone());
        }

        let volumes = vec![
            (format!("{}_data", container_name), "/var/lib/clickhouse".to_string()),
            (format!("{}_logs", container_name), "/var/log/clickhouse-server".to_string()),
        ];

        DatabaseConfig {
            container_name,
            image: database.image.clone().unwrap_or_else(|| "clickhouse/clickhouse-server:latest".to_string()),
            network: "coolify".to_string(),
            ports: database.port_mappings.clone(),
            environment,
            volumes,
            limits: ResourceLimits::default(),
            custom_docker_options: database.custom_docker_run_options.clone(),
            init_scripts: Vec::new(),
            enable_ssl: false,
        }
    }
}

#[async_trait]
impl Action for StartClickhouse {
    type Input = StartClickhouseInput<'static>;
    type Output = ActionResult;

    fn name(&self) -> &'static str {
        "start_clickhouse"
    }

    #[instrument(skip(self, input), fields(database_id = %input.database.id))]
    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        let server = input.server;
        let database = input.database;
        let ch_config = &input.config;
        let start = std::time::Instant::now();

        info!("Starting Clickhouse database {}", database.id);

        let config = Self::build_config(database, ch_config);
        let container_name = &config.container_name;
        let config_dir = database_configuration_dir(container_name);

        let session = self.ssh.connect(server)
            .await
            .map_err(|e| ActionError::ssh_connection_failed(e.to_string()))?;

        let mut builder = CommandBuilder::new();

        builder.echo("Starting Clickhouse database...");
        builder.mkdir(&config_dir);

        let compose_content = generate_compose(DatabaseType::Clickhouse, &config);
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
        builder.echo("Clickhouse started successfully!");

        let commands = builder.build();

        for cmd in &commands {
            let result = session.execute(cmd)
                .await
                .map_err(|e| ActionError::command_failed(cmd, e.to_string()))?;

            if result.exit_code != 0 && !cmd.contains("|| true") {
                return Err(ActionError::command_failed(cmd, result.stderr));
            }
        }

        Ok(ActionResult::success("Clickhouse started successfully")
            .with_duration(start.elapsed())
            .with_commands(commands))
    }
}
