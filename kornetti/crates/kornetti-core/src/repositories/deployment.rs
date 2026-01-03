//! Deployment repository

use sqlx::{PgPool, FromRow};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::{Result, Error};
use crate::models::{Deployment, DeploymentStatus, DeploymentType, DeploymentLog, LogLevel};

#[derive(FromRow)]
struct DeploymentRow {
    id: Uuid,
    application_id: Uuid,
    server_id: Uuid,
    deployment_uuid: String,
    status: String,
    deployment_type: String,
    commit_sha: Option<String>,
    commit_message: Option<String>,
    triggered_by: Option<Uuid>,
    force_rebuild: bool,
    pull_request_id: Option<i32>,
    logs: Option<String>,
    queued_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl From<DeploymentRow> for Deployment {
    fn from(row: DeploymentRow) -> Self {
        Deployment {
            id: row.id,
            application_id: row.application_id,
            server_id: row.server_id,
            status: parse_deployment_status(&row.status),
            deployment_type: parse_deployment_type(&row.deployment_type),
            commit_sha: row.commit_sha,
            commit_message: row.commit_message,
            triggered_by: row.triggered_by,
            logs: vec![], // Logs are parsed separately
            started_at: row.started_at.unwrap_or(row.queued_at),
            finished_at: row.finished_at,
            created_at: row.created_at,
        }
    }
}

fn parse_deployment_status(s: &str) -> DeploymentStatus {
    match s {
        "queued" => DeploymentStatus::Queued,
        "in_progress" => DeploymentStatus::InProgress,
        "finished" => DeploymentStatus::Finished,
        "failed" => DeploymentStatus::Failed,
        "cancelled" => DeploymentStatus::Cancelled,
        _ => DeploymentStatus::Failed,
    }
}

fn parse_deployment_type(s: &str) -> DeploymentType {
    match s {
        "deploy" => DeploymentType::Deploy,
        "redeploy" => DeploymentType::Redeploy,
        "rollback" => DeploymentType::Rollback,
        "pull_request" => DeploymentType::PullRequest,
        _ => DeploymentType::Deploy,
    }
}

pub struct DeploymentRepository {
    pool: PgPool,
}

impl DeploymentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new deployment
    pub async fn create(&self, deployment: &Deployment) -> Result<Deployment> {
        let status = format!("{:?}", deployment.status).to_lowercase();
        let dtype = format!("{:?}", deployment.deployment_type).to_lowercase();
        let deployment_uuid = Uuid::new_v4().to_string();

        let row = sqlx::query_as::<_, DeploymentRow>(
            r#"
            INSERT INTO application_deployments (
                id, application_id, server_id, deployment_uuid, status, deployment_type,
                commit_sha, commit_message, triggered_by, force_rebuild
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#
        )
        .bind(deployment.id)
        .bind(deployment.application_id)
        .bind(deployment.server_id)
        .bind(&deployment_uuid)
        .bind(&status)
        .bind(&dtype)
        .bind(&deployment.commit_sha)
        .bind(&deployment.commit_message)
        .bind(deployment.triggered_by)
        .bind(false)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(Deployment::from(row))
    }

    /// Find deployment by ID
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Deployment>> {
        let row = sqlx::query_as::<_, DeploymentRow>(
            "SELECT * FROM application_deployments WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.map(Deployment::from))
    }

    /// Find deployments by application
    pub async fn find_by_application(&self, app_id: Uuid, limit: i64) -> Result<Vec<Deployment>> {
        let rows = sqlx::query_as::<_, DeploymentRow>(
            r#"
            SELECT * FROM application_deployments
            WHERE application_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#
        )
        .bind(app_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(Deployment::from).collect())
    }

    /// Find queued deployments
    pub async fn find_queued(&self, limit: i64) -> Result<Vec<Deployment>> {
        let rows = sqlx::query_as::<_, DeploymentRow>(
            r#"
            SELECT * FROM application_deployments
            WHERE status = 'queued'
            ORDER BY queued_at ASC
            LIMIT $1
            "#
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(Deployment::from).collect())
    }

    /// Find in-progress deployments for a server
    pub async fn find_in_progress_by_server(&self, server_id: Uuid) -> Result<Vec<Deployment>> {
        let rows = sqlx::query_as::<_, DeploymentRow>(
            r#"
            SELECT * FROM application_deployments
            WHERE server_id = $1 AND status = 'in_progress'
            ORDER BY started_at ASC
            "#
        )
        .bind(server_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(Deployment::from).collect())
    }

    /// Update deployment status
    pub async fn update_status(&self, id: Uuid, status: DeploymentStatus) -> Result<()> {
        let status_str = format!("{:?}", status).to_lowercase();
        let now = Utc::now();

        let query = match status {
            DeploymentStatus::InProgress => {
                sqlx::query(
                    "UPDATE application_deployments SET status = $2, started_at = $3 WHERE id = $1"
                )
                .bind(id)
                .bind(&status_str)
                .bind(now)
            }
            DeploymentStatus::Finished | DeploymentStatus::Failed | DeploymentStatus::Cancelled => {
                sqlx::query(
                    "UPDATE application_deployments SET status = $2, finished_at = $3 WHERE id = $1"
                )
                .bind(id)
                .bind(&status_str)
                .bind(now)
            }
            _ => {
                sqlx::query("UPDATE application_deployments SET status = $2 WHERE id = $1")
                    .bind(id)
                    .bind(&status_str)
            }
        };

        query.execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    /// Append log to deployment
    pub async fn append_log(&self, id: Uuid, log_line: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE application_deployments
            SET logs = COALESCE(logs, '') || $2 || E'\n'
            WHERE id = $1
            "#
        )
        .bind(id)
        .bind(log_line)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    /// Get deployment logs
    pub async fn get_logs(&self, id: Uuid) -> Result<String> {
        let row = sqlx::query_as::<_, (Option<String>,)>(
            "SELECT logs FROM application_deployments WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.and_then(|r| r.0).unwrap_or_default())
    }

    /// Cancel queued deployments for an application
    pub async fn cancel_queued(&self, app_id: Uuid) -> Result<i64> {
        let result = sqlx::query(
            r#"
            UPDATE application_deployments
            SET status = 'cancelled', finished_at = NOW()
            WHERE application_id = $1 AND status = 'queued'
            "#
        )
        .bind(app_id)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(result.rows_affected() as i64)
    }

    /// Get latest successful deployment for rollback
    pub async fn find_latest_successful(&self, app_id: Uuid) -> Result<Option<Deployment>> {
        let row = sqlx::query_as::<_, DeploymentRow>(
            r#"
            SELECT * FROM application_deployments
            WHERE application_id = $1 AND status = 'finished' AND deployment_type != 'rollback'
            ORDER BY finished_at DESC
            LIMIT 1
            "#
        )
        .bind(app_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.map(Deployment::from))
    }

    /// Count deployments by status for an application
    pub async fn count_by_status(&self, app_id: Uuid) -> Result<Vec<(String, i64)>> {
        let rows = sqlx::query_as::<_, (String, i64)>(
            r#"
            SELECT status, COUNT(*) as count
            FROM application_deployments
            WHERE application_id = $1
            GROUP BY status
            "#
        )
        .bind(app_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows)
    }

    /// Cleanup old deployments (keep last N)
    pub async fn cleanup_old(&self, app_id: Uuid, keep: i64) -> Result<i64> {
        let result = sqlx::query(
            r#"
            DELETE FROM application_deployments
            WHERE application_id = $1
            AND id NOT IN (
                SELECT id FROM application_deployments
                WHERE application_id = $1
                ORDER BY created_at DESC
                LIMIT $2
            )
            "#
        )
        .bind(app_id)
        .bind(keep)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(result.rows_affected() as i64)
    }
}
