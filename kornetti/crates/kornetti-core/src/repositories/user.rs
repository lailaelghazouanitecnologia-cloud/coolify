//! User repository

use sqlx::{PgPool, FromRow};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::{Result, Error};

#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub password_hash: String,
    pub is_admin: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Find user by ID
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<User>> {
        let row = sqlx::query_as::<_, User>(
            "SELECT id, name, email, email_verified_at, password_hash, is_admin, created_at, updated_at FROM users WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row)
    }

    /// Find user by email
    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
        let row = sqlx::query_as::<_, User>(
            "SELECT id, name, email, email_verified_at, password_hash, is_admin, created_at, updated_at FROM users WHERE email = $1"
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row)
    }

    /// Create user
    pub async fn create(&self, name: &str, email: &str, password_hash: &str) -> Result<User> {
        let id = Uuid::new_v4();

        let row = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (id, name, email, password_hash)
            VALUES ($1, $2, $3, $4)
            RETURNING id, name, email, email_verified_at, password_hash, is_admin, created_at, updated_at
            "#
        )
        .bind(id)
        .bind(name)
        .bind(email)
        .bind(password_hash)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row)
    }

    /// Update user
    pub async fn update(&self, user: &User) -> Result<User> {
        let row = sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET name = $2, email = $3
            WHERE id = $1
            RETURNING id, name, email, email_verified_at, password_hash, is_admin, created_at, updated_at
            "#
        )
        .bind(user.id)
        .bind(&user.name)
        .bind(&user.email)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row)
    }

    /// Update password
    pub async fn update_password(&self, id: Uuid, password_hash: &str) -> Result<()> {
        sqlx::query("UPDATE users SET password_hash = $2 WHERE id = $1")
            .bind(id)
            .bind(password_hash)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    /// Verify email
    pub async fn verify_email(&self, id: Uuid) -> Result<()> {
        sqlx::query("UPDATE users SET email_verified_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    /// Delete user
    pub async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    /// Check if email exists
    pub async fn email_exists(&self, email: &str) -> Result<bool> {
        let row = sqlx::query_as::<_, (i64,)>(
            "SELECT COUNT(*) FROM users WHERE email = $1"
        )
        .bind(email)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.0 > 0)
    }

    /// Count users (for first user detection)
    pub async fn count(&self) -> Result<i64> {
        let row = sqlx::query_as::<_, (i64,)>(
            "SELECT COUNT(*) FROM users"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.0)
    }
}
