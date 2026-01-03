//! Environment variable repository

use sqlx::{PgPool, FromRow};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::{Result, Error};
use crate::models::environment_variable::{EnvironmentVariable, ResourceType};

#[derive(FromRow)]
struct EnvVarRow {
    id: Uuid,
    resource_type: String,
    resource_id: Uuid,
    key: String,
    value: String,
    is_secret: bool,
    is_build: bool,
    is_preview: bool,
    is_multiline: Option<bool>,
    is_shown_once: Option<bool>,
    #[sqlx(rename = "sort_order")]
    order: Option<i32>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<EnvVarRow> for EnvironmentVariable {
    fn from(row: EnvVarRow) -> Self {
        EnvironmentVariable {
            id: row.id,
            resource_type: parse_resource_type(&row.resource_type),
            resource_id: row.resource_id,
            key: row.key,
            value: row.value,
            is_secret: row.is_secret,
            is_build_time: row.is_build,
            is_preview: row.is_preview,
            is_multiline: row.is_multiline.unwrap_or(false),
            is_shown_once: row.is_shown_once.unwrap_or(false),
            order: row.order.unwrap_or(0),
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

fn parse_resource_type(s: &str) -> ResourceType {
    match s {
        "application" => ResourceType::Application,
        "database" => ResourceType::Database,
        "service" => ResourceType::Service,
        "shared_variable" => ResourceType::SharedVariable,
        _ => ResourceType::Application,
    }
}

fn resource_type_to_string(rt: &ResourceType) -> &'static str {
    match rt {
        ResourceType::Application => "application",
        ResourceType::Database => "database",
        ResourceType::Service => "service",
        ResourceType::SharedVariable => "shared_variable",
    }
}

pub struct EnvironmentVariableRepository {
    pool: PgPool,
}

impl EnvironmentVariableRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Find all environment variables for a resource
    pub async fn find_by_resource(
        &self,
        resource_type: ResourceType,
        resource_id: Uuid,
    ) -> Result<Vec<EnvironmentVariable>> {
        let rt = resource_type_to_string(&resource_type);

        let rows = sqlx::query_as::<_, EnvVarRow>(
            r#"
            SELECT id, resource_type, resource_id, key, value, is_secret,
                   is_build, is_preview, created_at, updated_at
            FROM environment_variables
            WHERE resource_type = $1 AND resource_id = $2
            ORDER BY key ASC
            "#
        )
        .bind(rt)
        .bind(resource_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(EnvironmentVariable::from).collect())
    }

    /// Find environment variable by ID
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<EnvironmentVariable>> {
        let row = sqlx::query_as::<_, EnvVarRow>(
            r#"
            SELECT id, resource_type, resource_id, key, value, is_secret,
                   is_build, is_preview, created_at, updated_at
            FROM environment_variables
            WHERE id = $1
            "#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.map(EnvironmentVariable::from))
    }

    /// Create a new environment variable
    pub async fn create(&self, env_var: &EnvironmentVariable) -> Result<EnvironmentVariable> {
        let rt = resource_type_to_string(&env_var.resource_type);

        let row = sqlx::query_as::<_, EnvVarRow>(
            r#"
            INSERT INTO environment_variables (id, resource_type, resource_id, key, value,
                                               is_secret, is_build, is_preview)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, resource_type, resource_id, key, value, is_secret,
                      is_build, is_preview, created_at, updated_at
            "#
        )
        .bind(env_var.id)
        .bind(rt)
        .bind(env_var.resource_id)
        .bind(&env_var.key)
        .bind(&env_var.value)
        .bind(env_var.is_secret)
        .bind(env_var.is_build_time)
        .bind(env_var.is_preview)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(EnvironmentVariable::from(row))
    }

    /// Bulk create environment variables
    pub async fn create_many(&self, env_vars: &[EnvironmentVariable]) -> Result<Vec<EnvironmentVariable>> {
        let mut results = Vec::with_capacity(env_vars.len());

        // Use a transaction for atomicity
        let mut tx = self.pool.begin().await
            .map_err(|e| Error::Database(e.to_string()))?;

        for env_var in env_vars {
            let rt = resource_type_to_string(&env_var.resource_type);

            let row = sqlx::query_as::<_, EnvVarRow>(
                r#"
                INSERT INTO environment_variables (id, resource_type, resource_id, key, value,
                                                   is_secret, is_build, is_preview)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                RETURNING id, resource_type, resource_id, key, value, is_secret,
                          is_build, is_preview, created_at, updated_at
                "#
            )
            .bind(env_var.id)
            .bind(rt)
            .bind(env_var.resource_id)
            .bind(&env_var.key)
            .bind(&env_var.value)
            .bind(env_var.is_secret)
            .bind(env_var.is_build_time)
            .bind(env_var.is_preview)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

            results.push(EnvironmentVariable::from(row));
        }

        tx.commit().await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(results)
    }

    /// Update an environment variable
    pub async fn update(&self, env_var: &EnvironmentVariable) -> Result<EnvironmentVariable> {
        let row = sqlx::query_as::<_, EnvVarRow>(
            r#"
            UPDATE environment_variables
            SET key = $2, value = $3, is_secret = $4, is_build = $5, is_preview = $6
            WHERE id = $1
            RETURNING id, resource_type, resource_id, key, value, is_secret,
                      is_build, is_preview, created_at, updated_at
            "#
        )
        .bind(env_var.id)
        .bind(&env_var.key)
        .bind(&env_var.value)
        .bind(env_var.is_secret)
        .bind(env_var.is_build_time)
        .bind(env_var.is_preview)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(EnvironmentVariable::from(row))
    }

    /// Delete an environment variable
    pub async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM environment_variables WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    /// Delete all environment variables for a resource
    pub async fn delete_by_resource(
        &self,
        resource_type: ResourceType,
        resource_id: Uuid,
    ) -> Result<u64> {
        let rt = resource_type_to_string(&resource_type);

        let result = sqlx::query(
            "DELETE FROM environment_variables WHERE resource_type = $1 AND resource_id = $2"
        )
        .bind(rt)
        .bind(resource_id)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(result.rows_affected())
    }

    /// Get build-time environment variables for a resource
    pub async fn find_build_vars(
        &self,
        resource_type: ResourceType,
        resource_id: Uuid,
    ) -> Result<Vec<EnvironmentVariable>> {
        let rt = resource_type_to_string(&resource_type);

        let rows = sqlx::query_as::<_, EnvVarRow>(
            r#"
            SELECT id, resource_type, resource_id, key, value, is_secret,
                   is_build, is_preview, created_at, updated_at
            FROM environment_variables
            WHERE resource_type = $1 AND resource_id = $2 AND is_build = true
            ORDER BY key ASC
            "#
        )
        .bind(rt)
        .bind(resource_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(EnvironmentVariable::from).collect())
    }

    /// Get runtime environment variables for a resource (non-build-time)
    pub async fn find_runtime_vars(
        &self,
        resource_type: ResourceType,
        resource_id: Uuid,
    ) -> Result<Vec<EnvironmentVariable>> {
        let rt = resource_type_to_string(&resource_type);

        let rows = sqlx::query_as::<_, EnvVarRow>(
            r#"
            SELECT id, resource_type, resource_id, key, value, is_secret,
                   is_build, is_preview, created_at, updated_at
            FROM environment_variables
            WHERE resource_type = $1 AND resource_id = $2 AND is_build = false
            ORDER BY key ASC
            "#
        )
        .bind(rt)
        .bind(resource_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(EnvironmentVariable::from).collect())
    }
}
