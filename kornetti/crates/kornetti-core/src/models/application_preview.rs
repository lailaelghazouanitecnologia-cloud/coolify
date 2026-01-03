//! Application Preview model
//!
//! Tracks pull request preview deployments.
//! Mirrors Coolify's ApplicationPreview model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::application_deployment_queue::DeploymentStatus;

/// Application preview for a pull request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationPreview {
    pub id: Uuid,
    pub application_id: Uuid,
    /// Pull request number
    pub pull_request_id: i64,
    /// PR title/description
    pub pull_request_title: Option<String>,
    /// Source branch name
    pub pull_request_head: Option<String>,
    /// Target branch name
    pub pull_request_base: Option<String>,
    /// PR author
    pub pull_request_author: Option<String>,
    /// Latest commit in the PR
    pub commit: Option<String>,
    /// FQDN for the preview (e.g., pr-123.app.example.com)
    pub fqdn: Option<String>,
    /// Current deployment status
    pub status: DeploymentStatus,
    /// Whether preview is soft deleted
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ApplicationPreview {
    pub fn new(application_id: Uuid, pull_request_id: i64) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            application_id,
            pull_request_id,
            pull_request_title: None,
            pull_request_head: None,
            pull_request_base: None,
            pull_request_author: None,
            commit: None,
            fqdn: None,
            status: DeploymentStatus::Queued,
            deleted_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create from PR webhook payload
    pub fn from_pull_request(
        application_id: Uuid,
        pr_id: i64,
        title: String,
        head: String,
        base: String,
        author: String,
        commit: String,
    ) -> Self {
        let mut preview = Self::new(application_id, pr_id);
        preview.pull_request_title = Some(title);
        preview.pull_request_head = Some(head);
        preview.pull_request_base = Some(base);
        preview.pull_request_author = Some(author);
        preview.commit = Some(commit);
        preview
    }

    /// Generate container name for this preview
    pub fn container_name(&self, app_uuid: &str) -> String {
        format!("{}-pr-{}", app_uuid, self.pull_request_id)
    }

    /// Check if preview is active (not deleted)
    pub fn is_active(&self) -> bool {
        self.deleted_at.is_none()
    }

    /// Soft delete the preview
    pub fn soft_delete(&mut self) {
        self.deleted_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Update commit hash
    pub fn update_commit(&mut self, commit: String) {
        self.commit = Some(commit);
        self.updated_at = Utc::now();
    }

    /// Set FQDN for the preview
    pub fn set_fqdn(&mut self, fqdn: String) {
        self.fqdn = Some(fqdn);
        self.updated_at = Utc::now();
    }
}
