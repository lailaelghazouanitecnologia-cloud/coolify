//! Server repository

use sqlx::{PgPool, FromRow};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::{Result, Error};
use crate::models::{Server, ServerStatus, ServerSettings, CloudProvider, ProxyType};

#[derive(FromRow)]
struct ServerRow {
    id: Uuid,
    team_id: Uuid,
    private_key_id: Option<Uuid>,
    name: String,
    description: Option<String>,
    ip: String,
    port: i32,
    user: String,
    status: String,
    provider: Option<String>,
    provider_id: Option<String>,
    region: Option<String>,
    settings: serde_json::Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<ServerRow> for Server {
    fn from(row: ServerRow) -> Self {
        let settings: ServerSettings = serde_json::from_value(row.settings)
            .unwrap_or_default();

        Server {
            id: row.id,
            team_id: row.team_id,
            name: row.name,
            description: row.description,
            ip: row.ip,
            port: row.port as u16,
            user: row.user,
            private_key_id: row.private_key_id.unwrap_or(Uuid::nil()),
            status: parse_server_status(&row.status),
            provider: row.provider.map(|p| parse_cloud_provider(&p)),
            provider_id: row.provider_id,
            region: row.region,
            settings,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

fn parse_server_status(s: &str) -> ServerStatus {
    match s {
        "new" => ServerStatus::New,
        "validating" => ServerStatus::Validating,
        "reachable" => ServerStatus::Reachable,
        "unreachable" => ServerStatus::Unreachable,
        "installing" => ServerStatus::Installing,
        "ready" => ServerStatus::Ready,
        _ => ServerStatus::Error,
    }
}

fn parse_cloud_provider(s: &str) -> CloudProvider {
    match s {
        "vultr" => CloudProvider::Vultr,
        "hetzner" => CloudProvider::Hetzner,
        "digitalocean" => CloudProvider::DigitalOcean,
        "aws" => CloudProvider::Aws,
        "linode" => CloudProvider::Linode,
        "gcp" => CloudProvider::Gcp,
        "azure" => CloudProvider::Azure,
        _ => CloudProvider::Custom,
    }
}

pub struct ServerRepository {
    pool: PgPool,
}

impl ServerRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Find all servers for a team
    pub async fn find_by_team(&self, team_id: Uuid) -> Result<Vec<Server>> {
        let rows = sqlx::query_as::<_, ServerRow>(
            r#"
            SELECT id, team_id, private_key_id, name, description, ip, port, "user",
                   status, provider, provider_id, region, settings, created_at, updated_at
            FROM servers
            WHERE team_id = $1
            ORDER BY created_at DESC
            "#
        )
        .bind(team_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(Server::from).collect())
    }

    /// Find server by ID
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Server>> {
        let row = sqlx::query_as::<_, ServerRow>(
            r#"
            SELECT id, team_id, private_key_id, name, description, ip, port, "user",
                   status, provider, provider_id, region, settings, created_at, updated_at
            FROM servers
            WHERE id = $1
            "#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.map(Server::from))
    }

    /// Find server by ID and team (for authorization)
    pub async fn find_by_id_and_team(&self, id: Uuid, team_id: Uuid) -> Result<Option<Server>> {
        let row = sqlx::query_as::<_, ServerRow>(
            r#"
            SELECT id, team_id, private_key_id, name, description, ip, port, "user",
                   status, provider, provider_id, region, settings, created_at, updated_at
            FROM servers
            WHERE id = $1 AND team_id = $2
            "#
        )
        .bind(id)
        .bind(team_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.map(Server::from))
    }

    /// Create a new server
    pub async fn create(&self, server: &Server) -> Result<Server> {
        let settings = serde_json::to_value(&server.settings)
            .map_err(|e| Error::Internal(e.to_string()))?;

        let status = format!("{:?}", server.status).to_lowercase();
        let provider = server.provider.as_ref().map(|p| format!("{:?}", p).to_lowercase());

        let row = sqlx::query_as::<_, ServerRow>(
            r#"
            INSERT INTO servers (id, team_id, private_key_id, name, description, ip, port, "user",
                                 status, provider, provider_id, region, settings)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING id, team_id, private_key_id, name, description, ip, port, "user",
                      status, provider, provider_id, region, settings, created_at, updated_at
            "#
        )
        .bind(server.id)
        .bind(server.team_id)
        .bind(if server.private_key_id.is_nil() { None } else { Some(server.private_key_id) })
        .bind(&server.name)
        .bind(&server.description)
        .bind(&server.ip)
        .bind(server.port as i32)
        .bind(&server.user)
        .bind(&status)
        .bind(&provider)
        .bind(&server.provider_id)
        .bind(&server.region)
        .bind(&settings)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(Server::from(row))
    }

    /// Update server
    pub async fn update(&self, server: &Server) -> Result<Server> {
        let settings = serde_json::to_value(&server.settings)
            .map_err(|e| Error::Internal(e.to_string()))?;

        let status = format!("{:?}", server.status).to_lowercase();

        let row = sqlx::query_as::<_, ServerRow>(
            r#"
            UPDATE servers
            SET name = $2, description = $3, ip = $4, port = $5, "user" = $6,
                status = $7, settings = $8
            WHERE id = $1
            RETURNING id, team_id, private_key_id, name, description, ip, port, "user",
                      status, provider, provider_id, region, settings, created_at, updated_at
            "#
        )
        .bind(server.id)
        .bind(&server.name)
        .bind(&server.description)
        .bind(&server.ip)
        .bind(server.port as i32)
        .bind(&server.user)
        .bind(&status)
        .bind(&settings)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(Server::from(row))
    }

    /// Update server status
    pub async fn update_status(&self, id: Uuid, status: ServerStatus) -> Result<()> {
        let status_str = format!("{:?}", status).to_lowercase();

        sqlx::query("UPDATE servers SET status = $2 WHERE id = $1")
            .bind(id)
            .bind(&status_str)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    /// Delete server
    pub async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM servers WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    /// Find servers that need health check
    pub async fn find_for_health_check(&self) -> Result<Vec<Server>> {
        let rows = sqlx::query_as::<_, ServerRow>(
            r#"
            SELECT id, team_id, private_key_id, name, description, ip, port, "user",
                   status, provider, provider_id, region, settings, created_at, updated_at
            FROM servers
            WHERE status IN ('ready', 'reachable', 'unreachable')
            ORDER BY updated_at ASC
            LIMIT 100
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(Server::from).collect())
    }

    /// Count servers by status for a team
    pub async fn count_by_status(&self, team_id: Uuid) -> Result<Vec<(String, i64)>> {
        let rows = sqlx::query_as::<_, (String, i64)>(
            r#"
            SELECT status, COUNT(*) as count
            FROM servers
            WHERE team_id = $1
            GROUP BY status
            "#
        )
        .bind(team_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows)
    }
}
