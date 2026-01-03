//! Application Deployment Queue model
//!
//! Tracks deployment requests and their status.
//! Mirrors Coolify's ApplicationDeploymentQueue model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Re-use DeploymentStatus from deployment module to avoid duplication
pub use super::deployment::DeploymentStatus;

/// Git source type for the deployment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitType {
    Github,
    Gitlab,
    Bitbucket,
    Gitea,
    Manual,
}

/// A single log entry in the deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentLogEntry {
    /// Log message
    pub output: String,
    /// Log type (stdout, stderr)
    pub log_type: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Whether this entry should be hidden in UI
    pub hidden: bool,
    /// Order in the log sequence
    pub order: i32,
    /// Batch number for grouping
    pub batch: i32,
}

/// Application deployment queue entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationDeploymentQueue {
    pub id: Uuid,
    pub application_id: Uuid,
    /// Unique deployment identifier
    pub deployment_uuid: String,
    /// Pull request ID (0 for main branch)
    pub pull_request_id: i64,
    /// Whether to force rebuild
    pub force_rebuild: bool,
    /// Git commit hash
    pub commit: Option<String>,
    /// Commit message
    pub commit_message: Option<String>,
    /// Current deployment status
    pub status: DeploymentStatus,
    /// Whether triggered by webhook
    pub is_webhook: bool,
    /// Whether triggered by API
    pub is_api: bool,
    /// Whether this is a restart only (no rebuild)
    pub restart_only: bool,
    /// Whether this is a rollback
    pub rollback: bool,
    /// Git source type
    pub git_type: Option<GitType>,
    /// Server ID where deployment runs
    pub server_id: Uuid,
    /// Destination ID (Docker host)
    pub destination_id: Option<Uuid>,
    /// Deploy only to this specific server
    pub only_this_server: bool,
    /// Horizon job ID for tracking
    pub horizon_job_id: Option<String>,
    /// Current process ID
    pub current_process_id: Option<String>,
    /// JSON-encoded deployment logs
    pub logs: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ApplicationDeploymentQueue {
    pub fn new(application_id: Uuid, server_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            application_id,
            deployment_uuid: Uuid::new_v4().to_string(),
            pull_request_id: 0,
            force_rebuild: false,
            commit: None,
            commit_message: None,
            status: DeploymentStatus::Queued,
            is_webhook: false,
            is_api: false,
            restart_only: false,
            rollback: false,
            git_type: None,
            server_id,
            destination_id: None,
            only_this_server: false,
            horizon_job_id: None,
            current_process_id: None,
            logs: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a webhook-triggered deployment
    pub fn from_webhook(application_id: Uuid, server_id: Uuid, commit: String, git_type: GitType) -> Self {
        let mut deployment = Self::new(application_id, server_id);
        deployment.is_webhook = true;
        deployment.commit = Some(commit);
        deployment.git_type = Some(git_type);
        deployment
    }

    /// Create a PR preview deployment
    pub fn for_pull_request(application_id: Uuid, server_id: Uuid, pr_id: i64, commit: String) -> Self {
        let mut deployment = Self::new(application_id, server_id);
        deployment.pull_request_id = pr_id;
        deployment.commit = Some(commit);
        deployment
    }

    /// Add a log entry
    pub fn add_log_entry(&mut self, message: &str, log_type: &str, hidden: bool) {
        let entry = DeploymentLogEntry {
            output: message.to_string(),
            log_type: log_type.to_string(),
            timestamp: Utc::now(),
            hidden,
            order: self.get_log_count() + 1,
            batch: 1,
        };

        let mut logs: Vec<DeploymentLogEntry> = self
            .logs
            .as_ref()
            .and_then(|l| serde_json::from_str(l).ok())
            .unwrap_or_default();

        logs.push(entry);
        self.logs = serde_json::to_string(&logs).ok();
        self.updated_at = Utc::now();
    }

    /// Get the number of log entries
    pub fn get_log_count(&self) -> i32 {
        self.logs
            .as_ref()
            .and_then(|l| serde_json::from_str::<Vec<DeploymentLogEntry>>(l).ok())
            .map(|v| v.len() as i32)
            .unwrap_or(0)
    }

    /// Get parsed log entries
    pub fn get_logs(&self) -> Vec<DeploymentLogEntry> {
        self.logs
            .as_ref()
            .and_then(|l| serde_json::from_str(l).ok())
            .unwrap_or_default()
    }

    /// Update deployment status
    pub fn set_status(&mut self, status: DeploymentStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }

    /// Check if deployment is for a PR
    pub fn is_pull_request(&self) -> bool {
        self.pull_request_id != 0
    }

    /// Get deployment duration in seconds
    pub fn duration_seconds(&self) -> Option<i64> {
        if self.status.is_terminal() {
            Some((self.updated_at - self.created_at).num_seconds())
        } else {
            None
        }
    }
}

/// Summary view of a deployment (for lists)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentSummary {
    pub id: Uuid,
    pub deployment_uuid: String,
    pub status: DeploymentStatus,
    pub commit: Option<String>,
    pub pull_request_id: i64,
    pub is_webhook: bool,
    pub created_at: DateTime<Utc>,
    pub duration_seconds: Option<i64>,
}

impl From<&ApplicationDeploymentQueue> for DeploymentSummary {
    fn from(deployment: &ApplicationDeploymentQueue) -> Self {
        Self {
            id: deployment.id,
            deployment_uuid: deployment.deployment_uuid.clone(),
            status: deployment.status,
            commit: deployment.commit.clone(),
            pull_request_id: deployment.pull_request_id,
            is_webhook: deployment.is_webhook,
            created_at: deployment.created_at,
            duration_seconds: deployment.duration_seconds(),
        }
    }
}
