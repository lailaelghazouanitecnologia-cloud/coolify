//! Scheduled task models for cron jobs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Scheduled task (cron job) for applications/services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: Uuid,
    /// The application or service this task belongs to
    pub resource_id: Uuid,
    pub resource_type: ScheduledTaskResourceType,
    pub name: String,
    /// Command to execute
    pub command: String,
    /// Cron expression
    pub frequency: String,
    /// Container name to execute in (optional, uses main container if empty)
    pub container: Option<String>,
    /// Whether task is enabled
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScheduledTaskResourceType {
    Application,
    Service,
}

impl ScheduledTask {
    pub fn new(
        resource_id: Uuid,
        resource_type: ScheduledTaskResourceType,
        name: String,
        command: String,
        frequency: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            resource_id,
            resource_type,
            name,
            command,
            frequency,
            container: None,
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Common cron frequencies
    pub fn every_minute(resource_id: Uuid, resource_type: ScheduledTaskResourceType, name: String, command: String) -> Self {
        Self::new(resource_id, resource_type, name, command, "* * * * *".to_string())
    }

    pub fn every_hour(resource_id: Uuid, resource_type: ScheduledTaskResourceType, name: String, command: String) -> Self {
        Self::new(resource_id, resource_type, name, command, "0 * * * *".to_string())
    }

    pub fn daily_at_midnight(resource_id: Uuid, resource_type: ScheduledTaskResourceType, name: String, command: String) -> Self {
        Self::new(resource_id, resource_type, name, command, "0 0 * * *".to_string())
    }

    /// Human-readable frequency description
    pub fn frequency_description(&self) -> String {
        match self.frequency.as_str() {
            "* * * * *" => "Every minute".to_string(),
            "*/5 * * * *" => "Every 5 minutes".to_string(),
            "*/15 * * * *" => "Every 15 minutes".to_string(),
            "*/30 * * * *" => "Every 30 minutes".to_string(),
            "0 * * * *" => "Every hour".to_string(),
            "0 */2 * * *" => "Every 2 hours".to_string(),
            "0 */6 * * *" => "Every 6 hours".to_string(),
            "0 */12 * * *" => "Every 12 hours".to_string(),
            "0 0 * * *" => "Daily at midnight".to_string(),
            "0 0 * * 0" => "Weekly on Sunday".to_string(),
            "0 0 1 * *" => "Monthly on the 1st".to_string(),
            _ => format!("Custom: {}", self.frequency),
        }
    }
}

/// Execution record for a scheduled task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTaskExecution {
    pub id: Uuid,
    pub scheduled_task_id: Uuid,
    pub status: TaskExecutionStatus,
    /// Exit code of the command
    pub exit_code: Option<i32>,
    /// Command output (stdout + stderr)
    pub output: Option<String>,
    /// Error message if failed
    pub message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskExecutionStatus {
    Running,
    Success,
    Failed,
}

impl ScheduledTaskExecution {
    pub fn start(scheduled_task_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            scheduled_task_id,
            status: TaskExecutionStatus::Running,
            exit_code: None,
            output: None,
            message: None,
            created_at: Utc::now(),
            finished_at: None,
        }
    }

    pub fn complete(mut self, exit_code: i32, output: String) -> Self {
        self.status = if exit_code == 0 {
            TaskExecutionStatus::Success
        } else {
            TaskExecutionStatus::Failed
        };
        self.exit_code = Some(exit_code);
        self.output = Some(output);
        self.finished_at = Some(Utc::now());
        self
    }

    pub fn fail(mut self, message: String) -> Self {
        self.status = TaskExecutionStatus::Failed;
        self.message = Some(message);
        self.finished_at = Some(Utc::now());
        self
    }

    pub fn duration_seconds(&self) -> Option<i64> {
        self.finished_at.map(|f| (f - self.created_at).num_seconds())
    }
}
