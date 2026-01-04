//! Standalone Database Models
//!
//! Models for all standalone database types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Database type enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
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

impl DatabaseType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DatabaseType::Postgresql => "postgresql",
            DatabaseType::Mysql => "mysql",
            DatabaseType::Mariadb => "mariadb",
            DatabaseType::Mongodb => "mongodb",
            DatabaseType::Redis => "redis",
            DatabaseType::Keydb => "keydb",
            DatabaseType::Dragonfly => "dragonfly",
            DatabaseType::Clickhouse => "clickhouse",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "postgresql" | "postgres" => Some(DatabaseType::Postgresql),
            "mysql" => Some(DatabaseType::Mysql),
            "mariadb" => Some(DatabaseType::Mariadb),
            "mongodb" | "mongo" => Some(DatabaseType::Mongodb),
            "redis" => Some(DatabaseType::Redis),
            "keydb" => Some(DatabaseType::Keydb),
            "dragonfly" => Some(DatabaseType::Dragonfly),
            "clickhouse" => Some(DatabaseType::Clickhouse),
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            DatabaseType::Postgresql => "PostgreSQL",
            DatabaseType::Mysql => "MySQL",
            DatabaseType::Mariadb => "MariaDB",
            DatabaseType::Mongodb => "MongoDB",
            DatabaseType::Redis => "Redis",
            DatabaseType::Keydb => "KeyDB",
            DatabaseType::Dragonfly => "Dragonfly",
            DatabaseType::Clickhouse => "ClickHouse",
        }
    }

    pub fn default_image(&self) -> &'static str {
        match self {
            DatabaseType::Postgresql => "postgres:16-alpine",
            DatabaseType::Mysql => "mysql:8.0",
            DatabaseType::Mariadb => "mariadb:11",
            DatabaseType::Mongodb => "mongo:7.0",
            DatabaseType::Redis => "redis:7-alpine",
            DatabaseType::Keydb => "eqalpha/keydb:latest",
            DatabaseType::Dragonfly => "docker.dragonflydb.io/dragonflydb/dragonfly",
            DatabaseType::Clickhouse => "clickhouse/clickhouse-server:latest",
        }
    }

    pub fn default_port(&self) -> u16 {
        match self {
            DatabaseType::Postgresql => 5432,
            DatabaseType::Mysql => 3306,
            DatabaseType::Mariadb => 3306,
            DatabaseType::Mongodb => 27017,
            DatabaseType::Redis => 6379,
            DatabaseType::Keydb => 6379,
            DatabaseType::Dragonfly => 6379,
            DatabaseType::Clickhouse => 9000,
        }
    }

    pub fn has_password(&self) -> bool {
        match self {
            DatabaseType::Redis | DatabaseType::Keydb | DatabaseType::Dragonfly => false,
            _ => true,
        }
    }

    pub fn has_database_name(&self) -> bool {
        match self {
            DatabaseType::Redis | DatabaseType::Keydb | DatabaseType::Dragonfly => false,
            _ => true,
        }
    }
}

/// Base standalone database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandaloneDatabase {
    pub id: Uuid,
    pub uuid: String,
    /// Database type
    pub database_type: DatabaseType,
    /// Human-readable name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Docker image
    pub image: String,
    /// Environment ID
    pub environment_id: Uuid,
    /// Destination (Docker network) ID
    pub destination_id: Uuid,
    /// Server ID
    pub server_id: Uuid,

    // Connection settings
    /// Database name (for SQL databases)
    pub database_name: Option<String>,
    /// Username
    pub username: Option<String>,
    /// Password (encrypted)
    pub password: Option<String>,
    /// Root password (for MySQL/MariaDB)
    pub root_password: Option<String>,
    /// Internal host (container name)
    pub internal_host: String,
    /// Internal port
    pub internal_port: u16,
    /// Public port (if exposed)
    pub public_port: Option<u16>,
    /// Whether to expose publicly
    pub is_public: bool,

    // Configuration
    /// Custom configuration (database-specific)
    pub config: Option<String>,
    /// Initialization SQL/scripts
    pub init_script: Option<String>,
    /// Custom start command
    pub custom_start_command: Option<String>,
    /// Custom docker run options
    pub custom_docker_run_options: Option<String>,

    // Resources
    /// Memory limit
    pub limits_memory: Option<String>,
    /// Memory reservation
    pub limits_memory_reservation: Option<String>,
    /// Memory swap limit
    pub limits_memory_swap: Option<String>,
    /// Memory swappiness
    pub limits_memory_swappiness: Option<i32>,
    /// CPU limit
    pub limits_cpus: Option<String>,
    /// CPU shares
    pub limits_cpu_shares: Option<i32>,

    // Status
    pub status: DatabaseStatus,
    pub started_at: Option<DateTime<Utc>>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Restarting,
    Error,
    Unknown,
}

impl Default for DatabaseStatus {
    fn default() -> Self {
        DatabaseStatus::Stopped
    }
}

impl StandaloneDatabase {
    pub fn new(
        database_type: DatabaseType,
        name: String,
        environment_id: Uuid,
        destination_id: Uuid,
        server_id: Uuid,
    ) -> Self {
        let now = Utc::now();
        let uuid = Uuid::new_v4().to_string();
        let container_name = format!("{}-{}", name.to_lowercase().replace(' ', "-"), &uuid[..8]);

        Self {
            id: Uuid::new_v4(),
            uuid,
            database_type,
            name,
            description: None,
            image: database_type.default_image().to_string(),
            environment_id,
            destination_id,
            server_id,
            database_name: if database_type.has_database_name() {
                Some("default".to_string())
            } else {
                None
            },
            username: if database_type.has_password() {
                Some("admin".to_string())
            } else {
                None
            },
            password: None,
            root_password: None,
            internal_host: container_name,
            internal_port: database_type.default_port(),
            public_port: None,
            is_public: false,
            config: None,
            init_script: None,
            custom_start_command: None,
            custom_docker_run_options: None,
            limits_memory: None,
            limits_memory_reservation: None,
            limits_memory_swap: None,
            limits_memory_swappiness: None,
            limits_cpus: None,
            limits_cpu_shares: None,
            status: DatabaseStatus::Stopped,
            started_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Get the connection string for this database
    pub fn connection_string(&self, use_public: bool) -> Option<String> {
        let host = if use_public {
            // Would need the server's public IP
            return None;
        } else {
            &self.internal_host
        };
        let port = if use_public {
            self.public_port?
        } else {
            self.internal_port
        };

        match self.database_type {
            DatabaseType::Postgresql => {
                let user = self.username.as_deref().unwrap_or("postgres");
                let db = self.database_name.as_deref().unwrap_or("postgres");
                Some(format!("postgresql://{}@{}:{}/{}", user, host, port, db))
            }
            DatabaseType::Mysql | DatabaseType::Mariadb => {
                let user = self.username.as_deref().unwrap_or("root");
                let db = self.database_name.as_deref().unwrap_or("mysql");
                Some(format!("mysql://{}@{}:{}/{}", user, host, port, db))
            }
            DatabaseType::Mongodb => {
                let user = self.username.as_deref();
                let db = self.database_name.as_deref().unwrap_or("admin");
                if let Some(u) = user {
                    Some(format!("mongodb://{}@{}:{}/{}", u, host, port, db))
                } else {
                    Some(format!("mongodb://{}:{}/{}", host, port, db))
                }
            }
            DatabaseType::Redis | DatabaseType::Keydb | DatabaseType::Dragonfly => {
                Some(format!("redis://{}:{}", host, port))
            }
            DatabaseType::Clickhouse => {
                let user = self.username.as_deref().unwrap_or("default");
                let db = self.database_name.as_deref().unwrap_or("default");
                Some(format!("clickhouse://{}@{}:{}/{}", user, host, port, db))
            }
        }
    }

    /// Get environment variables for this database container
    pub fn env_vars(&self) -> Vec<(String, String)> {
        let mut vars = Vec::new();

        match self.database_type {
            DatabaseType::Postgresql => {
                if let Some(db) = &self.database_name {
                    vars.push(("POSTGRES_DB".to_string(), db.clone()));
                }
                if let Some(user) = &self.username {
                    vars.push(("POSTGRES_USER".to_string(), user.clone()));
                }
                if let Some(pass) = &self.password {
                    vars.push(("POSTGRES_PASSWORD".to_string(), pass.clone()));
                }
            }
            DatabaseType::Mysql => {
                if let Some(db) = &self.database_name {
                    vars.push(("MYSQL_DATABASE".to_string(), db.clone()));
                }
                if let Some(user) = &self.username {
                    vars.push(("MYSQL_USER".to_string(), user.clone()));
                }
                if let Some(pass) = &self.password {
                    vars.push(("MYSQL_PASSWORD".to_string(), pass.clone()));
                }
                if let Some(root_pass) = &self.root_password {
                    vars.push(("MYSQL_ROOT_PASSWORD".to_string(), root_pass.clone()));
                }
            }
            DatabaseType::Mariadb => {
                if let Some(db) = &self.database_name {
                    vars.push(("MARIADB_DATABASE".to_string(), db.clone()));
                }
                if let Some(user) = &self.username {
                    vars.push(("MARIADB_USER".to_string(), user.clone()));
                }
                if let Some(pass) = &self.password {
                    vars.push(("MARIADB_PASSWORD".to_string(), pass.clone()));
                }
                if let Some(root_pass) = &self.root_password {
                    vars.push(("MARIADB_ROOT_PASSWORD".to_string(), root_pass.clone()));
                }
            }
            DatabaseType::Mongodb => {
                if let Some(user) = &self.username {
                    vars.push(("MONGO_INITDB_ROOT_USERNAME".to_string(), user.clone()));
                }
                if let Some(pass) = &self.password {
                    vars.push(("MONGO_INITDB_ROOT_PASSWORD".to_string(), pass.clone()));
                }
                if let Some(db) = &self.database_name {
                    vars.push(("MONGO_INITDB_DATABASE".to_string(), db.clone()));
                }
            }
            DatabaseType::Redis | DatabaseType::Keydb | DatabaseType::Dragonfly => {
                // These typically don't use env vars for auth
            }
            DatabaseType::Clickhouse => {
                if let Some(db) = &self.database_name {
                    vars.push(("CLICKHOUSE_DB".to_string(), db.clone()));
                }
                if let Some(user) = &self.username {
                    vars.push(("CLICKHOUSE_USER".to_string(), user.clone()));
                }
                if let Some(pass) = &self.password {
                    vars.push(("CLICKHOUSE_PASSWORD".to_string(), pass.clone()));
                }
            }
        }

        vars
    }

    /// Get the data volume path for this database
    pub fn data_volume_path(&self) -> &'static str {
        match self.database_type {
            DatabaseType::Postgresql => "/var/lib/postgresql/data",
            DatabaseType::Mysql => "/var/lib/mysql",
            DatabaseType::Mariadb => "/var/lib/mysql",
            DatabaseType::Mongodb => "/data/db",
            DatabaseType::Redis | DatabaseType::Keydb => "/data",
            DatabaseType::Dragonfly => "/data",
            DatabaseType::Clickhouse => "/var/lib/clickhouse",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_type_from_str() {
        assert_eq!(DatabaseType::from_str("postgresql"), Some(DatabaseType::Postgresql));
        assert_eq!(DatabaseType::from_str("postgres"), Some(DatabaseType::Postgresql));
        assert_eq!(DatabaseType::from_str("mongo"), Some(DatabaseType::Mongodb));
        assert_eq!(DatabaseType::from_str("invalid"), None);
    }

    #[test]
    fn test_standalone_database_new() {
        let env_id = Uuid::new_v4();
        let dest_id = Uuid::new_v4();
        let server_id = Uuid::new_v4();

        let db = StandaloneDatabase::new(
            DatabaseType::Postgresql,
            "My Database".to_string(),
            env_id,
            dest_id,
            server_id,
        );

        assert_eq!(db.database_type, DatabaseType::Postgresql);
        assert_eq!(db.name, "My Database");
        assert_eq!(db.internal_port, 5432);
        assert!(db.database_name.is_some());
    }

    #[test]
    fn test_redis_no_password() {
        assert!(!DatabaseType::Redis.has_password());
        assert!(!DatabaseType::Redis.has_database_name());
    }

    #[test]
    fn test_connection_string() {
        let env_id = Uuid::new_v4();
        let dest_id = Uuid::new_v4();
        let server_id = Uuid::new_v4();

        let mut db = StandaloneDatabase::new(
            DatabaseType::Postgresql,
            "test".to_string(),
            env_id,
            dest_id,
            server_id,
        );
        db.username = Some("user".to_string());
        db.database_name = Some("mydb".to_string());

        let conn = db.connection_string(false).unwrap();
        assert!(conn.starts_with("postgresql://"));
        assert!(conn.contains("mydb"));
    }

    #[test]
    fn test_env_vars() {
        let env_id = Uuid::new_v4();
        let dest_id = Uuid::new_v4();
        let server_id = Uuid::new_v4();

        let mut db = StandaloneDatabase::new(
            DatabaseType::Postgresql,
            "test".to_string(),
            env_id,
            dest_id,
            server_id,
        );
        db.password = Some("secret".to_string());

        let vars = db.env_vars();
        assert!(vars.iter().any(|(k, _)| k == "POSTGRES_PASSWORD"));
    }
}
