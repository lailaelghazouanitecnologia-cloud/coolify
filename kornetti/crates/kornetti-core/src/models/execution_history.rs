//! Execution History Models
//!
//! Models for tracking execution of scheduled tasks, backups, and cleanup operations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// Scheduled Task Execution
// ============================================================================

/// Record of a scheduled task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTaskExecution {
    pub id: Uuid,
    /// Parent scheduled task ID
    pub scheduled_task_id: Uuid,
    /// Execution status
    pub status: ExecutionStatus,
    /// Command that was executed
    pub command: String,
    /// Output from the command
    pub output: Option<String>,
    /// Error message if failed
    pub error: Option<String>,
    /// Exit code
    pub exit_code: Option<i32>,
    /// Server where executed
    pub server_id: Uuid,
    /// Duration in seconds
    pub duration_seconds: Option<i32>,
    /// When execution started
    pub started_at: DateTime<Utc>,
    /// When execution finished
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl ScheduledTaskExecution {
    pub fn new(scheduled_task_id: Uuid, server_id: Uuid, command: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            scheduled_task_id,
            status: ExecutionStatus::Running,
            command,
            output: None,
            error: None,
            exit_code: None,
            server_id,
            duration_seconds: None,
            started_at: now,
            finished_at: None,
            created_at: now,
        }
    }

    pub fn mark_success(&mut self, output: Option<String>, exit_code: i32) {
        self.status = ExecutionStatus::Success;
        self.output = output;
        self.exit_code = Some(exit_code);
        self.finished_at = Some(Utc::now());
        if let Some(finished) = self.finished_at {
            self.duration_seconds = Some((finished - self.started_at).num_seconds() as i32);
        }
    }

    pub fn mark_failed(&mut self, error: String, exit_code: Option<i32>) {
        self.status = ExecutionStatus::Failed;
        self.error = Some(error);
        self.exit_code = exit_code;
        self.finished_at = Some(Utc::now());
        if let Some(finished) = self.finished_at {
            self.duration_seconds = Some((finished - self.started_at).num_seconds() as i32);
        }
    }

    pub fn mark_cancelled(&mut self) {
        self.status = ExecutionStatus::Cancelled;
        self.finished_at = Some(Utc::now());
    }
}

/// Execution status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    Pending,
    Running,
    Success,
    Failed,
    Cancelled,
    Timeout,
}

impl Default for ExecutionStatus {
    fn default() -> Self {
        ExecutionStatus::Pending
    }
}

// ============================================================================
// Docker Cleanup Execution
// ============================================================================

/// Record of a Docker cleanup operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerCleanupExecution {
    pub id: Uuid,
    /// Server where cleanup was performed
    pub server_id: Uuid,
    /// Cleanup type
    pub cleanup_type: CleanupType,
    /// Execution status
    pub status: ExecutionStatus,
    /// Space reclaimed in bytes
    pub space_reclaimed_bytes: Option<i64>,
    /// Items removed (images, containers, volumes, etc.)
    pub items_removed: Option<i32>,
    /// Detailed output
    pub output: Option<String>,
    /// Error message if failed
    pub error: Option<String>,
    /// Duration in seconds
    pub duration_seconds: Option<i32>,
    /// When cleanup started
    pub started_at: DateTime<Utc>,
    /// When cleanup finished
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CleanupType {
    /// docker system prune
    SystemPrune,
    /// docker image prune
    ImagePrune,
    /// docker container prune
    ContainerPrune,
    /// docker volume prune
    VolumePrune,
    /// docker network prune
    NetworkPrune,
    /// docker builder prune
    BuilderPrune,
    /// Full cleanup (all of the above)
    Full,
}

impl DockerCleanupExecution {
    pub fn new(server_id: Uuid, cleanup_type: CleanupType) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            server_id,
            cleanup_type,
            status: ExecutionStatus::Running,
            space_reclaimed_bytes: None,
            items_removed: None,
            output: None,
            error: None,
            duration_seconds: None,
            started_at: now,
            finished_at: None,
            created_at: now,
        }
    }

    pub fn mark_success(&mut self, space_reclaimed: i64, items_removed: i32, output: Option<String>) {
        self.status = ExecutionStatus::Success;
        self.space_reclaimed_bytes = Some(space_reclaimed);
        self.items_removed = Some(items_removed);
        self.output = output;
        self.finished_at = Some(Utc::now());
        if let Some(finished) = self.finished_at {
            self.duration_seconds = Some((finished - self.started_at).num_seconds() as i32);
        }
    }

    pub fn mark_failed(&mut self, error: String) {
        self.status = ExecutionStatus::Failed;
        self.error = Some(error);
        self.finished_at = Some(Utc::now());
    }

    /// Get human-readable space reclaimed
    pub fn space_reclaimed_human(&self) -> Option<String> {
        self.space_reclaimed_bytes.map(|bytes| {
            if bytes >= 1024 * 1024 * 1024 {
                format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
            } else if bytes >= 1024 * 1024 {
                format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
            } else if bytes >= 1024 {
                format!("{:.2} KB", bytes as f64 / 1024.0)
            } else {
                format!("{} B", bytes)
            }
        })
    }
}

// ============================================================================
// Personal Access Token
// ============================================================================

/// API access token for a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalAccessToken {
    pub id: Uuid,
    /// Token name/label
    pub name: String,
    /// User who owns this token
    pub user_id: Uuid,
    /// Hashed token value
    pub token_hash: String,
    /// Scopes/abilities granted
    pub abilities: Vec<String>,
    /// Last time token was used
    pub last_used_at: Option<DateTime<Utc>>,
    /// IP address of last use
    pub last_used_ip: Option<String>,
    /// Expiration time (None = never expires)
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PersonalAccessToken {
    pub fn new(name: String, user_id: Uuid, token_hash: String, abilities: Vec<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            user_id,
            token_hash,
            abilities,
            last_used_at: None,
            last_used_ip: None,
            expires_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(exp) => exp < Utc::now(),
            None => false,
        }
    }

    /// Check if token has a specific ability
    pub fn has_ability(&self, ability: &str) -> bool {
        self.abilities.iter().any(|a| a == "*" || a == ability)
    }

    /// Record token usage
    pub fn record_usage(&mut self, ip: Option<String>) {
        self.last_used_at = Some(Utc::now());
        self.last_used_ip = ip;
        self.updated_at = Utc::now();
    }
}

// ============================================================================
// User Changelog Read
// ============================================================================

/// Track which changelog entries a user has read
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserChangelogRead {
    pub id: Uuid,
    pub user_id: Uuid,
    /// Last changelog version/date read
    pub last_read_version: String,
    /// Timestamp of last read
    pub read_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UserChangelogRead {
    pub fn new(user_id: Uuid, version: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            last_read_version: version,
            read_at: now,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn mark_read(&mut self, version: String) {
        self.last_read_version = version;
        self.read_at = Utc::now();
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduled_task_execution() {
        let task_id = Uuid::new_v4();
        let server_id = Uuid::new_v4();
        let mut exec = ScheduledTaskExecution::new(
            task_id,
            server_id,
            "echo 'hello'".to_string(),
        );

        assert_eq!(exec.status, ExecutionStatus::Running);

        exec.mark_success(Some("hello".to_string()), 0);

        assert_eq!(exec.status, ExecutionStatus::Success);
        assert_eq!(exec.exit_code, Some(0));
        assert!(exec.finished_at.is_some());
    }

    #[test]
    fn test_docker_cleanup_execution() {
        let server_id = Uuid::new_v4();
        let mut exec = DockerCleanupExecution::new(server_id, CleanupType::SystemPrune);

        exec.mark_success(1024 * 1024 * 500, 10, None);

        assert_eq!(exec.status, ExecutionStatus::Success);
        assert_eq!(exec.items_removed, Some(10));
        assert!(exec.space_reclaimed_human().unwrap().contains("MB"));
    }

    #[test]
    fn test_personal_access_token() {
        let user_id = Uuid::new_v4();
        let token = PersonalAccessToken::new(
            "My Token".to_string(),
            user_id,
            "hashed".to_string(),
            vec!["read".to_string(), "write".to_string()],
        );

        assert!(!token.is_expired());
        assert!(token.has_ability("read"));
        assert!(token.has_ability("write"));
        assert!(!token.has_ability("admin"));
    }

    #[test]
    fn test_token_wildcard_ability() {
        let user_id = Uuid::new_v4();
        let token = PersonalAccessToken::new(
            "Admin Token".to_string(),
            user_id,
            "hashed".to_string(),
            vec!["*".to_string()],
        );

        assert!(token.has_ability("anything"));
        assert!(token.has_ability("admin"));
    }
}
