//! Service Database Model
//!
//! Databases deployed as part of a Docker Compose service.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::service_application::ResourceLimits;

/// Database deployed within a service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDatabase {
    pub id: Uuid,
    /// UUID for external references
    pub uuid: String,
    /// Database name (from compose)
    pub name: String,
    /// Human-readable description
    pub description: Option<String>,
    /// Parent service ID
    pub service_id: Uuid,
    /// Docker image
    pub image: String,
    /// Database type
    pub database_type: ServiceDatabaseType,
    /// Internal database name
    pub database_name: Option<String>,
    /// Database user
    pub database_user: Option<String>,
    /// Database password (encrypted)
    pub database_password: Option<String>,
    /// Root/admin password (encrypted)
    pub database_root_password: Option<String>,
    /// Internal port
    pub internal_port: u16,
    /// Public port (if exposed)
    pub public_port: Option<u16>,
    /// Whether to expose publicly
    pub is_public: bool,
    /// Custom configuration
    pub configuration: Option<String>,
    /// Volume mount path
    pub data_path: Option<String>,
    /// Resource limits
    pub limits: Option<ResourceLimits>,
    /// Current status
    pub status: ServiceDatabaseStatus,
    /// Exclude from status checks
    pub exclude_from_status: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ServiceDatabaseType {
    Postgresql,
    Mysql,
    Mariadb,
    Mongodb,
    Redis,
    Keydb,
    Dragonfly,
    Clickhouse,
    Meilisearch,
    Elasticsearch,
    Other,
}

impl ServiceDatabaseType {
    pub fn from_image(image: &str) -> Self {
        let lower = image.to_lowercase();
        if lower.contains("postgres") {
            ServiceDatabaseType::Postgresql
        } else if lower.contains("mysql") {
            ServiceDatabaseType::Mysql
        } else if lower.contains("mariadb") {
            ServiceDatabaseType::Mariadb
        } else if lower.contains("mongo") {
            ServiceDatabaseType::Mongodb
        } else if lower.contains("redis") {
            ServiceDatabaseType::Redis
        } else if lower.contains("keydb") {
            ServiceDatabaseType::Keydb
        } else if lower.contains("dragonfly") {
            ServiceDatabaseType::Dragonfly
        } else if lower.contains("clickhouse") {
            ServiceDatabaseType::Clickhouse
        } else if lower.contains("meilisearch") {
            ServiceDatabaseType::Meilisearch
        } else if lower.contains("elasticsearch") {
            ServiceDatabaseType::Elasticsearch
        } else {
            ServiceDatabaseType::Other
        }
    }

    pub fn default_port(&self) -> u16 {
        match self {
            ServiceDatabaseType::Postgresql => 5432,
            ServiceDatabaseType::Mysql | ServiceDatabaseType::Mariadb => 3306,
            ServiceDatabaseType::Mongodb => 27017,
            ServiceDatabaseType::Redis | ServiceDatabaseType::Keydb | ServiceDatabaseType::Dragonfly => 6379,
            ServiceDatabaseType::Clickhouse => 9000,
            ServiceDatabaseType::Meilisearch => 7700,
            ServiceDatabaseType::Elasticsearch => 9200,
            ServiceDatabaseType::Other => 0,
        }
    }

    pub fn data_path(&self) -> &'static str {
        match self {
            ServiceDatabaseType::Postgresql => "/var/lib/postgresql/data",
            ServiceDatabaseType::Mysql => "/var/lib/mysql",
            ServiceDatabaseType::Mariadb => "/var/lib/mysql",
            ServiceDatabaseType::Mongodb => "/data/db",
            ServiceDatabaseType::Redis | ServiceDatabaseType::Keydb | ServiceDatabaseType::Dragonfly => "/data",
            ServiceDatabaseType::Clickhouse => "/var/lib/clickhouse",
            ServiceDatabaseType::Meilisearch => "/meili_data",
            ServiceDatabaseType::Elasticsearch => "/usr/share/elasticsearch/data",
            ServiceDatabaseType::Other => "/data",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ServiceDatabaseStatus {
    Running,
    Stopped,
    Starting,
    Stopping,
    Restarting,
    Error,
    Unknown,
}

impl Default for ServiceDatabaseStatus {
    fn default() -> Self {
        ServiceDatabaseStatus::Unknown
    }
}

impl ServiceDatabase {
    pub fn new(name: String, service_id: Uuid, image: String) -> Self {
        let db_type = ServiceDatabaseType::from_image(&image);
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            uuid: Uuid::new_v4().to_string(),
            name,
            description: None,
            service_id,
            image,
            database_type: db_type,
            database_name: None,
            database_user: None,
            database_password: None,
            database_root_password: None,
            internal_port: db_type.default_port(),
            public_port: None,
            is_public: false,
            configuration: None,
            data_path: Some(db_type.data_path().to_string()),
            limits: None,
            status: ServiceDatabaseStatus::Unknown,
            exclude_from_status: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Get container name for this database
    pub fn container_name(&self, service_uuid: &str) -> String {
        format!("{}-{}", service_uuid, self.name)
    }

    /// Get environment variables for this database
    pub fn env_vars(&self) -> Vec<(String, String)> {
        let mut vars = Vec::new();

        match self.database_type {
            ServiceDatabaseType::Postgresql => {
                if let Some(ref db) = self.database_name {
                    vars.push(("POSTGRES_DB".to_string(), db.clone()));
                }
                if let Some(ref user) = self.database_user {
                    vars.push(("POSTGRES_USER".to_string(), user.clone()));
                }
                if let Some(ref pass) = self.database_password {
                    vars.push(("POSTGRES_PASSWORD".to_string(), pass.clone()));
                }
            }
            ServiceDatabaseType::Mysql => {
                if let Some(ref db) = self.database_name {
                    vars.push(("MYSQL_DATABASE".to_string(), db.clone()));
                }
                if let Some(ref user) = self.database_user {
                    vars.push(("MYSQL_USER".to_string(), user.clone()));
                }
                if let Some(ref pass) = self.database_password {
                    vars.push(("MYSQL_PASSWORD".to_string(), pass.clone()));
                }
                if let Some(ref root) = self.database_root_password {
                    vars.push(("MYSQL_ROOT_PASSWORD".to_string(), root.clone()));
                }
            }
            ServiceDatabaseType::Mariadb => {
                if let Some(ref db) = self.database_name {
                    vars.push(("MARIADB_DATABASE".to_string(), db.clone()));
                }
                if let Some(ref user) = self.database_user {
                    vars.push(("MARIADB_USER".to_string(), user.clone()));
                }
                if let Some(ref pass) = self.database_password {
                    vars.push(("MARIADB_PASSWORD".to_string(), pass.clone()));
                }
                if let Some(ref root) = self.database_root_password {
                    vars.push(("MARIADB_ROOT_PASSWORD".to_string(), root.clone()));
                }
            }
            ServiceDatabaseType::Mongodb => {
                if let Some(ref user) = self.database_user {
                    vars.push(("MONGO_INITDB_ROOT_USERNAME".to_string(), user.clone()));
                }
                if let Some(ref pass) = self.database_password {
                    vars.push(("MONGO_INITDB_ROOT_PASSWORD".to_string(), pass.clone()));
                }
            }
            _ => {}
        }

        vars
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_database_new() {
        let service_id = Uuid::new_v4();
        let db = ServiceDatabase::new(
            "db".to_string(),
            service_id,
            "postgres:16".to_string(),
        );

        assert_eq!(db.name, "db");
        assert_eq!(db.database_type, ServiceDatabaseType::Postgresql);
        assert_eq!(db.internal_port, 5432);
    }

    #[test]
    fn test_database_type_from_image() {
        assert_eq!(
            ServiceDatabaseType::from_image("postgres:16-alpine"),
            ServiceDatabaseType::Postgresql
        );
        assert_eq!(
            ServiceDatabaseType::from_image("redis:7"),
            ServiceDatabaseType::Redis
        );
        assert_eq!(
            ServiceDatabaseType::from_image("mongo:latest"),
            ServiceDatabaseType::Mongodb
        );
    }

    #[test]
    fn test_env_vars() {
        let service_id = Uuid::new_v4();
        let mut db = ServiceDatabase::new(
            "db".to_string(),
            service_id,
            "postgres:16".to_string(),
        );
        db.database_name = Some("mydb".to_string());
        db.database_user = Some("user".to_string());
        db.database_password = Some("pass".to_string());

        let vars = db.env_vars();
        assert!(vars.iter().any(|(k, _)| k == "POSTGRES_DB"));
        assert!(vars.iter().any(|(k, _)| k == "POSTGRES_USER"));
        assert!(vars.iter().any(|(k, _)| k == "POSTGRES_PASSWORD"));
    }
}
