//! Start MongoDB Action
//!
//! Starts a MongoDB database container with authentication and replica set support.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tracing::{info, instrument};

use crate::models::{Server, StandaloneDatabase};
use super::{DatabaseConfig, DatabaseType, ResourceLimits, generate_compose, database_configuration_dir};
use crate::actions::{Action, ActionError, ActionResult, CommandBuilder};
use kornetti_ssh::SshClient;

/// MongoDB specific configuration
#[derive(Debug, Clone)]
pub struct MongodbConfig {
    /// Root username
    pub mongo_initdb_root_username: String,
    /// Root password
    pub mongo_initdb_root_password: String,
    /// Default database
    pub mongo_initdb_database: Option<String>,
}

impl Default for MongodbConfig {
    fn default() -> Self {
        Self {
            mongo_initdb_root_username: "root".to_string(),
            mongo_initdb_root_password: String::new(),
            mongo_initdb_database: None,
        }
    }
}

/// Input for starting MongoDB
pub struct StartMongodbInput<'a> {
    pub server: &'a Server,
    pub database: &'a StandaloneDatabase,
    pub config: MongodbConfig,
}

/// Action to start a MongoDB database
pub struct StartMongodb {
    ssh: Arc<SshClient>,
}

impl StartMongodb {
    pub fn new(ssh: Arc<SshClient>) -> Self {
        Self { ssh }
    }

    fn build_config(database: &StandaloneDatabase, mongo_config: &MongodbConfig) -> DatabaseConfig {
        let container_name = database.uuid.to_string();

        let mut environment = HashMap::new();
        environment.insert("MONGO_INITDB_ROOT_USERNAME".to_string(), mongo_config.mongo_initdb_root_username.clone());
        environment.insert("MONGO_INITDB_ROOT_PASSWORD".to_string(), mongo_config.mongo_initdb_root_password.clone());

        if let Some(ref db) = mongo_config.mongo_initdb_database {
            environment.insert("MONGO_INITDB_DATABASE".to_string(), db.clone());
        }

        let volumes = vec![
            (format!("{}_data", container_name), "/data/db".to_string()),
            (format!("{}_configdb", container_name), "/data/configdb".to_string()),
        ];

        DatabaseConfig {
            container_name,
            image: database.image.clone().unwrap_or_else(|| "mongo:7".to_string()),
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
            init_scripts: Vec::new(),
            enable_ssl: false,
        }
    }
}

#[async_trait]
impl Action for StartMongodb {
    type Input = StartMongodbInput<'static>;
    type Output = ActionResult;

    fn name(&self) -> &'static str {
        "start_mongodb"
    }

    #[instrument(skip(self, input), fields(database_id = %input.database.id))]
    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        let server = input.server;
        let database = input.database;
        let mongo_config = &input.config;
        let start = std::time::Instant::now();

        info!("Starting MongoDB database {}", database.id);

        let config = Self::build_config(database, mongo_config);
        let container_name = &config.container_name;
        let config_dir = database_configuration_dir(container_name);

        let session = self.ssh.connect(server)
            .await
            .map_err(|e| ActionError::ssh_connection_failed(e.to_string()))?;

        let mut builder = CommandBuilder::new();

        builder.echo("Starting MongoDB database...");
        builder.mkdir(&config_dir);

        let compose_content = generate_compose(DatabaseType::Mongodb, &config);
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
        builder.echo("MongoDB started successfully!");

        let commands = builder.build();

        for cmd in &commands {
            let result = session.execute(cmd)
                .await
                .map_err(|e| ActionError::command_failed(cmd, e.to_string()))?;

            if result.exit_code != 0 && !cmd.contains("|| true") {
                return Err(ActionError::command_failed(cmd, result.stderr));
            }
        }

        Ok(ActionResult::success("MongoDB started successfully")
            .with_duration(start.elapsed())
            .with_commands(commands))
    }
}
