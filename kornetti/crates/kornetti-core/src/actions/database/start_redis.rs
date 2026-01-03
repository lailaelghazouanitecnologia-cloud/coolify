//! Start Redis Action
//!
//! Starts a Redis database container with optional authentication and persistence.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tracing::{info, instrument};

use crate::models::{Server, StandaloneDatabase};
use super::{DatabaseConfig, DatabaseType, ResourceLimits, generate_compose, database_configuration_dir};
use crate::actions::{Action, ActionError, ActionResult, CommandBuilder};
use kornetti_ssh::SshClient;

/// Redis specific configuration
#[derive(Debug, Clone)]
pub struct RedisConfig {
    /// Redis password (optional)
    pub redis_password: Option<String>,
    /// Custom redis.conf content
    pub redis_conf: Option<String>,
    /// Enable AOF persistence
    pub aof_enabled: bool,
    /// Max memory setting (e.g., "256mb")
    pub maxmemory: Option<String>,
    /// Eviction policy
    pub maxmemory_policy: Option<String>,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            redis_password: None,
            redis_conf: None,
            aof_enabled: true,
            maxmemory: None,
            maxmemory_policy: Some("volatile-lru".to_string()),
        }
    }
}

/// Input for starting Redis
pub struct StartRedisInput<'a> {
    pub server: &'a Server,
    pub database: &'a StandaloneDatabase,
    pub config: RedisConfig,
}

/// Action to start a Redis database
pub struct StartRedis {
    ssh: Arc<SshClient>,
}

impl StartRedis {
    pub fn new(ssh: Arc<SshClient>) -> Self {
        Self { ssh }
    }

    fn build_config(database: &StandaloneDatabase, redis_config: &RedisConfig) -> DatabaseConfig {
        let container_name = database.uuid.to_string();
        let config_dir = database_configuration_dir(&container_name);

        let mut environment = HashMap::new();

        if let Some(ref password) = redis_config.redis_password {
            environment.insert("REDIS_PASSWORD".to_string(), password.clone());
        }

        let volumes = vec![
            (format!("{}_data", container_name), "/data".to_string()),
        ];

        DatabaseConfig {
            container_name,
            image: database.image.clone().unwrap_or_else(|| "redis:7-alpine".to_string()),
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

    /// Generate Redis command arguments
    fn generate_redis_command(config: &RedisConfig) -> Vec<String> {
        let mut args = vec!["redis-server".to_string()];

        if let Some(ref password) = config.redis_password {
            args.push("--requirepass".to_string());
            args.push(password.clone());
        }

        if config.aof_enabled {
            args.push("--appendonly".to_string());
            args.push("yes".to_string());
        }

        if let Some(ref maxmem) = config.maxmemory {
            args.push("--maxmemory".to_string());
            args.push(maxmem.clone());
        }

        if let Some(ref policy) = config.maxmemory_policy {
            args.push("--maxmemory-policy".to_string());
            args.push(policy.clone());
        }

        args
    }
}

#[async_trait]
impl Action for StartRedis {
    type Input = StartRedisInput<'static>;
    type Output = ActionResult;

    fn name(&self) -> &'static str {
        "start_redis"
    }

    #[instrument(skip(self, input), fields(database_id = %input.database.id))]
    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        let server = input.server;
        let database = input.database;
        let redis_config = &input.config;
        let start = std::time::Instant::now();

        info!("Starting Redis database {}", database.id);

        let config = Self::build_config(database, redis_config);
        let container_name = &config.container_name;
        let config_dir = database_configuration_dir(container_name);

        let session = self.ssh.connect(server)
            .await
            .map_err(|e| ActionError::ssh_connection_failed(e.to_string()))?;

        let mut builder = CommandBuilder::new();

        builder.echo("Starting Redis database...");
        builder.mkdir(&config_dir);

        // Generate docker-compose with custom command
        let compose_content = generate_compose(DatabaseType::Redis, &config);
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
        builder.echo("Redis started successfully!");

        let commands = builder.build();

        for cmd in &commands {
            let result = session.execute(cmd)
                .await
                .map_err(|e| ActionError::command_failed(cmd, e.to_string()))?;

            if result.exit_code != 0 && !cmd.contains("|| true") {
                return Err(ActionError::command_failed(cmd, result.stderr));
            }
        }

        Ok(ActionResult::success("Redis started successfully")
            .with_duration(start.elapsed())
            .with_commands(commands))
    }
}
