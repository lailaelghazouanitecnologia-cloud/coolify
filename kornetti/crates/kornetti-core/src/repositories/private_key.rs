//! Private key repository

use sqlx::{PgPool, FromRow};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::{Result, Error};
use crate::models::private_key::PrivateKey;

#[derive(FromRow)]
struct PrivateKeyRow {
    id: Uuid,
    team_id: Uuid,
    name: String,
    description: Option<String>,
    private_key: String,
    public_key: Option<String>,
    fingerprint: Option<String>,
    is_git_related: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<PrivateKeyRow> for PrivateKey {
    fn from(row: PrivateKeyRow) -> Self {
        PrivateKey {
            id: row.id,
            team_id: row.team_id,
            name: row.name,
            description: row.description,
            private_key: row.private_key,
            public_key: row.public_key,
            fingerprint: row.fingerprint,
            is_git_related: row.is_git_related,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

pub struct PrivateKeyRepository {
    pool: PgPool,
}

impl PrivateKeyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Find all private keys for a team
    pub async fn find_by_team(&self, team_id: Uuid) -> Result<Vec<PrivateKey>> {
        let rows = sqlx::query_as::<_, PrivateKeyRow>(
            r#"
            SELECT id, team_id, name, description, private_key, public_key,
                   fingerprint, is_git_related, created_at, updated_at
            FROM private_keys
            WHERE team_id = $1
            ORDER BY created_at DESC
            "#
        )
        .bind(team_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(PrivateKey::from).collect())
    }

    /// Find private key by ID
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<PrivateKey>> {
        let row = sqlx::query_as::<_, PrivateKeyRow>(
            r#"
            SELECT id, team_id, name, description, private_key, public_key,
                   fingerprint, is_git_related, created_at, updated_at
            FROM private_keys
            WHERE id = $1
            "#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.map(PrivateKey::from))
    }

    /// Find private key by ID and team (for authorization)
    pub async fn find_by_id_and_team(&self, id: Uuid, team_id: Uuid) -> Result<Option<PrivateKey>> {
        let row = sqlx::query_as::<_, PrivateKeyRow>(
            r#"
            SELECT id, team_id, name, description, private_key, public_key,
                   fingerprint, is_git_related, created_at, updated_at
            FROM private_keys
            WHERE id = $1 AND team_id = $2
            "#
        )
        .bind(id)
        .bind(team_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.map(PrivateKey::from))
    }

    /// Create a new private key
    pub async fn create(&self, key: &PrivateKey) -> Result<PrivateKey> {
        let row = sqlx::query_as::<_, PrivateKeyRow>(
            r#"
            INSERT INTO private_keys (id, team_id, name, description, private_key,
                                      public_key, fingerprint, is_git_related)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, team_id, name, description, private_key, public_key,
                      fingerprint, is_git_related, created_at, updated_at
            "#
        )
        .bind(key.id)
        .bind(key.team_id)
        .bind(&key.name)
        .bind(&key.description)
        .bind(&key.private_key)
        .bind(&key.public_key)
        .bind(&key.fingerprint)
        .bind(key.is_git_related)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(PrivateKey::from(row))
    }

    /// Update a private key
    pub async fn update(&self, key: &PrivateKey) -> Result<PrivateKey> {
        let row = sqlx::query_as::<_, PrivateKeyRow>(
            r#"
            UPDATE private_keys
            SET name = $2, description = $3, private_key = $4, public_key = $5,
                fingerprint = $6, is_git_related = $7
            WHERE id = $1
            RETURNING id, team_id, name, description, private_key, public_key,
                      fingerprint, is_git_related, created_at, updated_at
            "#
        )
        .bind(key.id)
        .bind(&key.name)
        .bind(&key.description)
        .bind(&key.private_key)
        .bind(&key.public_key)
        .bind(&key.fingerprint)
        .bind(key.is_git_related)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(PrivateKey::from(row))
    }

    /// Delete a private key
    pub async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM private_keys WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    /// Check if a private key is in use by servers
    pub async fn is_in_use(&self, id: Uuid) -> Result<bool> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM servers WHERE private_key_id = $1"
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(count.0 > 0)
    }
}
