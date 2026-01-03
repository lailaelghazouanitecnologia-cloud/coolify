//! Database Actions
//!
//! Actions for managing standalone databases including PostgreSQL, MySQL,
//! Redis, MongoDB, MariaDB, Clickhouse, and more.

mod start_postgresql;
mod start_mysql;
mod start_redis;
mod start_mongodb;
mod start_mariadb;
mod start_clickhouse;
mod stop_database;
mod restart_database;

pub use start_postgresql::StartPostgresql;
pub use start_mysql::StartMysql;
pub use start_redis::StartRedis;
pub use start_mongodb::StartMongodb;
pub use start_mariadb::StartMariadb;
pub use start_clickhouse::StartClickhouse;
pub use stop_database::StopDatabase;
pub use restart_database::RestartDatabase;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Database type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DatabaseType {
    Postgresql,
    Mysql,
    Mariadb,
    Redis,
    Mongodb,
    Clickhouse,
    Keydb,
    Dragonfly,
}

impl DatabaseType {
    /// Get default port for this database type
    pub fn default_port(&self) -> u16 {
        match self {
            DatabaseType::Postgresql => 5432,
            DatabaseType::Mysql | DatabaseType::Mariadb => 3306,
            DatabaseType::Redis | DatabaseType::Keydb | DatabaseType::Dragonfly => 6379,
            DatabaseType::Mongodb => 27017,
            DatabaseType::Clickhouse => 8123,
        }
    }

    /// Get default Docker image for this database type
    pub fn default_image(&self) -> &'static str {
        match self {
            DatabaseType::Postgresql => "postgres:16-alpine",
            DatabaseType::Mysql => "mysql:8.0",
            DatabaseType::Mariadb => "mariadb:11",
            DatabaseType::Redis => "redis:7-alpine",
            DatabaseType::Mongodb => "mongo:7",
            DatabaseType::Clickhouse => "clickhouse/clickhouse-server:latest",
            DatabaseType::Keydb => "eqalpha/keydb:latest",
            DatabaseType::Dragonfly => "docker.dragonflydb.io/dragonflydb/dragonfly:latest",
        }
    }

    /// Get health check command for this database type
    pub fn health_check(&self) -> Vec<String> {
        match self {
            DatabaseType::Postgresql => vec![
                "CMD-SHELL".to_string(),
                "pg_isready -U $POSTGRES_USER -d $POSTGRES_DB || exit 1".to_string(),
            ],
            DatabaseType::Mysql | DatabaseType::Mariadb => vec![
                "CMD-SHELL".to_string(),
                "mysqladmin ping -h localhost -u root -p$MYSQL_ROOT_PASSWORD || exit 1".to_string(),
            ],
            DatabaseType::Redis | DatabaseType::Keydb | DatabaseType::Dragonfly => vec![
                "CMD-SHELL".to_string(),
                "redis-cli ping | grep PONG || exit 1".to_string(),
            ],
            DatabaseType::Mongodb => vec![
                "CMD-SHELL".to_string(),
                "mongosh --eval 'db.runCommand({ping:1})' --quiet || exit 1".to_string(),
            ],
            DatabaseType::Clickhouse => vec![
                "CMD-SHELL".to_string(),
                "clickhouse-client --query 'SELECT 1' || exit 1".to_string(),
            ],
        }
    }
}

/// Common database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Container name (usually UUID)
    pub container_name: String,
    /// Docker image to use
    pub image: String,
    /// Docker network to attach to
    pub network: String,
    /// Port mappings (host:container)
    pub ports: Vec<(u16, u16)>,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Volume mounts (host_path:container_path)
    pub volumes: Vec<(String, String)>,
    /// Resource limits
    pub limits: ResourceLimits,
    /// Custom Docker run options
    pub custom_docker_options: Option<String>,
    /// Init scripts to run on first start
    pub init_scripts: Vec<InitScript>,
    /// Enable SSL
    pub enable_ssl: bool,
}

/// Resource limits for containers
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub memory: Option<String>,         // e.g., "512m", "1g"
    pub memory_swap: Option<String>,
    pub memory_reservation: Option<String>,
    pub cpus: Option<f64>,              // e.g., 0.5, 2.0
    pub cpu_shares: Option<u32>,
}

/// Database init script
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitScript {
    pub filename: String,
    pub content: String,
}

/// Generate Docker Compose YAML for a database
pub fn generate_compose(
    db_type: DatabaseType,
    config: &DatabaseConfig,
) -> String {
    let mut compose = serde_yaml::Mapping::new();

    // Services section
    let mut services = serde_yaml::Mapping::new();
    let mut service = serde_yaml::Mapping::new();

    service.insert(
        serde_yaml::Value::String("image".to_string()),
        serde_yaml::Value::String(config.image.clone()),
    );

    service.insert(
        serde_yaml::Value::String("container_name".to_string()),
        serde_yaml::Value::String(config.container_name.clone()),
    );

    service.insert(
        serde_yaml::Value::String("restart".to_string()),
        serde_yaml::Value::String("unless-stopped".to_string()),
    );

    // Environment
    let env: Vec<serde_yaml::Value> = config.environment
        .iter()
        .map(|(k, v)| serde_yaml::Value::String(format!("{}={}", k, v)))
        .collect();

    if !env.is_empty() {
        service.insert(
            serde_yaml::Value::String("environment".to_string()),
            serde_yaml::Value::Sequence(env),
        );
    }

    // Networks
    service.insert(
        serde_yaml::Value::String("networks".to_string()),
        serde_yaml::Value::Sequence(vec![
            serde_yaml::Value::String(config.network.clone())
        ]),
    );

    // Ports
    if !config.ports.is_empty() {
        let ports: Vec<serde_yaml::Value> = config.ports
            .iter()
            .map(|(h, c)| serde_yaml::Value::String(format!("{}:{}", h, c)))
            .collect();

        service.insert(
            serde_yaml::Value::String("ports".to_string()),
            serde_yaml::Value::Sequence(ports),
        );
    }

    // Volumes
    if !config.volumes.is_empty() {
        let volumes: Vec<serde_yaml::Value> = config.volumes
            .iter()
            .map(|(h, c)| serde_yaml::Value::String(format!("{}:{}", h, c)))
            .collect();

        service.insert(
            serde_yaml::Value::String("volumes".to_string()),
            serde_yaml::Value::Sequence(volumes),
        );
    }

    // Health check
    let health_check = db_type.health_check();
    let mut hc = serde_yaml::Mapping::new();
    hc.insert(
        serde_yaml::Value::String("test".to_string()),
        serde_yaml::Value::Sequence(
            health_check.into_iter().map(serde_yaml::Value::String).collect()
        ),
    );
    hc.insert(
        serde_yaml::Value::String("interval".to_string()),
        serde_yaml::Value::String("5s".to_string()),
    );
    hc.insert(
        serde_yaml::Value::String("timeout".to_string()),
        serde_yaml::Value::String("5s".to_string()),
    );
    hc.insert(
        serde_yaml::Value::String("retries".to_string()),
        serde_yaml::Value::Number(10.into()),
    );

    service.insert(
        serde_yaml::Value::String("healthcheck".to_string()),
        serde_yaml::Value::Mapping(hc),
    );

    // Resource limits
    if let Some(ref mem) = config.limits.memory {
        service.insert(
            serde_yaml::Value::String("mem_limit".to_string()),
            serde_yaml::Value::String(mem.clone()),
        );
    }

    if let Some(cpus) = config.limits.cpus {
        service.insert(
            serde_yaml::Value::String("cpus".to_string()),
            serde_yaml::Value::Number(serde_yaml::Number::from(cpus)),
        );
    }

    // Labels
    let labels = vec![
        serde_yaml::Value::String("coolify.managed=true".to_string()),
        serde_yaml::Value::String(format!("coolify.type=database")),
        serde_yaml::Value::String(format!("coolify.database.type={:?}", db_type).to_lowercase()),
    ];

    service.insert(
        serde_yaml::Value::String("labels".to_string()),
        serde_yaml::Value::Sequence(labels),
    );

    services.insert(
        serde_yaml::Value::String(config.container_name.clone()),
        serde_yaml::Value::Mapping(service),
    );

    compose.insert(
        serde_yaml::Value::String("services".to_string()),
        serde_yaml::Value::Mapping(services),
    );

    // Networks section
    let mut networks = serde_yaml::Mapping::new();
    let mut network_config = serde_yaml::Mapping::new();
    network_config.insert(
        serde_yaml::Value::String("external".to_string()),
        serde_yaml::Value::Bool(true),
    );
    network_config.insert(
        serde_yaml::Value::String("name".to_string()),
        serde_yaml::Value::String(config.network.clone()),
    );

    networks.insert(
        serde_yaml::Value::String(config.network.clone()),
        serde_yaml::Value::Mapping(network_config),
    );

    compose.insert(
        serde_yaml::Value::String("networks".to_string()),
        serde_yaml::Value::Mapping(networks),
    );

    serde_yaml::to_string(&serde_yaml::Value::Mapping(compose))
        .unwrap_or_default()
}

/// Get configuration directory for a database
pub fn database_configuration_dir(container_name: &str) -> String {
    format!("/data/coolify/databases/{}", container_name)
}
