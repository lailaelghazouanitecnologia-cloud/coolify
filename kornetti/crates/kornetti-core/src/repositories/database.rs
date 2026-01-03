//! Standalone Database repository

use sqlx::{PgPool, FromRow};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::{Result, Error};
use crate::models::{StandaloneDatabase, DatabaseType, DatabaseStatus, DatabaseConfig};

#[derive(FromRow)]
struct DatabaseRow {
    id: Uuid,
    environment_id: Uuid,
    server_id: Uuid,
    name: String,
    description: Option<String>,
    database_type: String,
    image: String,
    version: Option<String>,
    status: String,
    internal_db_url: Option<String>,
    external_db_url: Option<String>,
    public_port: Option<i32>,
    database_name: Option<String>,
    database_user: Option<String>,
    database_password: Option<String>,
    root_password: Option<String>,
    configuration: serde_json::Value,
    limits_memory: Option<String>,
    limits_cpus: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<DatabaseRow> for StandaloneDatabase {
    fn from(row: DatabaseRow) -> Self {
        StandaloneDatabase {
            id: row.id,
            environment_id: row.environment_id,
            server_id: row.server_id,
            name: row.name,
            description: row.description,
            database_type: parse_database_type(&row.database_type),
            version: row.version.unwrap_or_else(|| "latest".to_string()),
            status: parse_database_status(&row.status),
            config: DatabaseConfig {
                public_port: row.public_port.map(|p| p as u16),
                internal_db_url: row.internal_db_url.unwrap_or_default(),
                external_db_url: row.external_db_url,
                root_password: row.root_password.unwrap_or_default(),
                database_name: row.database_name.unwrap_or_default(),
                database_user: row.database_user.unwrap_or_default(),
                database_password: row.database_password.unwrap_or_default(),
            },
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

fn parse_database_type(s: &str) -> DatabaseType {
    match s {
        "postgresql" => DatabaseType::Postgresql,
        "mysql" => DatabaseType::Mysql,
        "mariadb" => DatabaseType::Mariadb,
        "mongodb" => DatabaseType::Mongodb,
        "redis" => DatabaseType::Redis,
        "keydb" => DatabaseType::Keydb,
        "dragonfly" => DatabaseType::Dragonfly,
        "clickhouse" => DatabaseType::Clickhouse,
        _ => DatabaseType::Postgresql,
    }
}

fn parse_database_status(s: &str) -> DatabaseStatus {
    match s {
        "stopped" => DatabaseStatus::Stopped,
        "starting" => DatabaseStatus::Starting,
        "running" => DatabaseStatus::Running,
        "stopping" => DatabaseStatus::Stopping,
        _ => DatabaseStatus::Error,
    }
}

pub struct DatabaseRepository {
    pool: PgPool,
}

impl DatabaseRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Find databases by team
    pub async fn find_by_team(&self, team_id: Uuid) -> Result<Vec<StandaloneDatabase>> {
        let rows = sqlx::query_as::<_, DatabaseRow>(
            r#"
            SELECT d.*
            FROM standalone_databases d
            JOIN environments e ON d.environment_id = e.id
            JOIN projects p ON e.project_id = p.id
            WHERE p.team_id = $1
            ORDER BY d.name
            "#
        )
        .bind(team_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(StandaloneDatabase::from).collect())
    }

    /// Find database by ID
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<StandaloneDatabase>> {
        let row = sqlx::query_as::<_, DatabaseRow>(
            "SELECT * FROM standalone_databases WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.map(StandaloneDatabase::from))
    }

    /// Find databases by server
    pub async fn find_by_server(&self, server_id: Uuid) -> Result<Vec<StandaloneDatabase>> {
        let rows = sqlx::query_as::<_, DatabaseRow>(
            "SELECT * FROM standalone_databases WHERE server_id = $1 ORDER BY name"
        )
        .bind(server_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(StandaloneDatabase::from).collect())
    }

    /// Create database
    pub async fn create(&self, db: &StandaloneDatabase) -> Result<StandaloneDatabase> {
        let db_type = format!("{:?}", db.database_type).to_lowercase();
        let status = format!("{:?}", db.status).to_lowercase();
        let image = db.database_type.default_image();

        let row = sqlx::query_as::<_, DatabaseRow>(
            r#"
            INSERT INTO standalone_databases (
                id, environment_id, server_id, name, description,
                database_type, image, version, status,
                internal_db_url, external_db_url, public_port,
                database_name, database_user, database_password, root_password
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            RETURNING *
            "#
        )
        .bind(db.id)
        .bind(db.environment_id)
        .bind(db.server_id)
        .bind(&db.name)
        .bind(&db.description)
        .bind(&db_type)
        .bind(image)
        .bind(&db.version)
        .bind(&status)
        .bind(&db.config.internal_db_url)
        .bind(&db.config.external_db_url)
        .bind(db.config.public_port.map(|p| p as i32))
        .bind(&db.config.database_name)
        .bind(&db.config.database_user)
        .bind(&db.config.database_password)
        .bind(&db.config.root_password)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(StandaloneDatabase::from(row))
    }

    /// Update database status
    pub async fn update_status(&self, id: Uuid, status: DatabaseStatus) -> Result<()> {
        let status_str = format!("{:?}", status).to_lowercase();

        sqlx::query("UPDATE standalone_databases SET status = $2 WHERE id = $1")
            .bind(id)
            .bind(&status_str)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    /// Delete database
    pub async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM standalone_databases WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    /// Update public port
    pub async fn update_public_port(&self, id: Uuid, port: Option<u16>) -> Result<()> {
        sqlx::query("UPDATE standalone_databases SET public_port = $2 WHERE id = $1")
            .bind(id)
            .bind(port.map(|p| p as i32))
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }
}
