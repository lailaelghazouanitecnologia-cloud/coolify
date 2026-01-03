//! Deployment models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deployment {
    pub id: Uuid,
    pub application_id: Uuid,
    pub server_id: Uuid,
    pub status: DeploymentStatus,
    pub deployment_type: DeploymentType,
    pub commit_sha: Option<String>,
    pub commit_message: Option<String>,
    pub triggered_by: Option<Uuid>,
    pub logs: Vec<DeploymentLog>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentStatus {
    Queued,
    InProgress,
    Finished,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentType {
    Deploy,
    Redeploy,
    Rollback,
    PullRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentLog {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub message: String,
    pub step: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

impl Deployment {
    pub fn new(
        application_id: Uuid,
        server_id: Uuid,
        deployment_type: DeploymentType,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            application_id,
            server_id,
            status: DeploymentStatus::Queued,
            deployment_type,
            commit_sha: None,
            commit_message: None,
            triggered_by: None,
            logs: vec![],
            started_at: now,
            finished_at: None,
            created_at: now,
        }
    }

    pub fn add_log(&mut self, level: LogLevel, message: String, step: Option<String>) {
        self.logs.push(DeploymentLog {
            timestamp: Utc::now(),
            level,
            message,
            step,
        });
    }

    pub fn is_finished(&self) -> bool {
        matches!(
            self.status,
            DeploymentStatus::Finished | DeploymentStatus::Failed | DeploymentStatus::Cancelled
        )
    }

    pub fn duration_seconds(&self) -> Option<i64> {
        self.finished_at.map(|f| (f - self.started_at).num_seconds())
    }
}
