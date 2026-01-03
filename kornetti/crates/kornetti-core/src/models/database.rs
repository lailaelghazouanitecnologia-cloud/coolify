//! Standalone Database models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandaloneDatabase {
    pub id: Uuid,
    pub environment_id: Uuid,
    pub server_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub database_type: DatabaseType,
    pub version: String,
    pub status: DatabaseStatus,
    pub config: DatabaseConfig,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseType {
    Postgresql,
    Mysql,
    Mariadb,
    Mongodb,
    Redis,
    Keydb,
    Dragonfly,
    Clickhouse,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub public_port: Option<u16>,
    pub internal_db_url: String,
    pub external_db_url: Option<String>,
    pub root_password: String,
    pub database_name: String,
    pub database_user: String,
    pub database_password: String,
}

impl DatabaseType {
    pub fn default_port(&self) -> u16 {
        match self {
            DatabaseType::Postgresql => 5432,
            DatabaseType::Mysql | DatabaseType::Mariadb => 3306,
            DatabaseType::Mongodb => 27017,
            DatabaseType::Redis | DatabaseType::Keydb | DatabaseType::Dragonfly => 6379,
            DatabaseType::Clickhouse => 8123,
        }
    }

    pub fn default_image(&self) -> &'static str {
        match self {
            DatabaseType::Postgresql => "postgres",
            DatabaseType::Mysql => "mysql",
            DatabaseType::Mariadb => "mariadb",
            DatabaseType::Mongodb => "mongo",
            DatabaseType::Redis => "redis",
            DatabaseType::Keydb => "eqalpha/keydb",
            DatabaseType::Dragonfly => "docker.dragonflydb.io/dragonflydb/dragonfly",
            DatabaseType::Clickhouse => "clickhouse/clickhouse-server",
        }
    }
}
