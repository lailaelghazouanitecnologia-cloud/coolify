//! Job definitions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A job to be executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: Uuid,
    pub job_type: JobType,
    pub payload: serde_json::Value,
    pub status: JobStatus,
    pub attempts: u32,
    pub max_attempts: u32,
    pub scheduled_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobType {
    ApplicationDeployment,
    DatabaseBackup,
    ServerCheck,
    DockerCleanup,
    SslRenewal,
    ScheduledTask,
    Notification,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Scheduled,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl Job {
    pub fn new(job_type: JobType, payload: serde_json::Value) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            job_type,
            payload,
            status: JobStatus::Pending,
            attempts: 0,
            max_attempts: 3,
            scheduled_at: now,
            started_at: None,
            completed_at: None,
            error: None,
            created_at: now,
        }
    }

    pub fn scheduled(job_type: JobType, payload: serde_json::Value, scheduled_at: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::new_v4(),
            job_type,
            payload,
            status: JobStatus::Scheduled,
            attempts: 0,
            max_attempts: 3,
            scheduled_at,
            started_at: None,
            completed_at: None,
            error: None,
            created_at: Utc::now(),
        }
    }

    pub fn is_ready(&self) -> bool {
        matches!(self.status, JobStatus::Pending | JobStatus::Scheduled)
            && self.scheduled_at <= Utc::now()
    }

    pub fn can_retry(&self) -> bool {
        self.attempts < self.max_attempts
    }

    pub fn mark_started(&mut self) {
        self.status = JobStatus::Running;
        self.started_at = Some(Utc::now());
        self.attempts += 1;
    }

    pub fn mark_completed(&mut self) {
        self.status = JobStatus::Completed;
        self.completed_at = Some(Utc::now());
    }

    pub fn mark_failed(&mut self, error: String) {
        self.status = JobStatus::Failed;
        self.completed_at = Some(Utc::now());
        self.error = Some(error);
    }
}
