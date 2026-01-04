//! Scheduled Database Backup Model
//!
//! Configuration for automated database backups.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Backup frequency type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BackupFrequency {
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Custom,
}

impl BackupFrequency {
    pub fn to_cron(&self) -> &'static str {
        match self {
            BackupFrequency::Hourly => "0 * * * *",
            BackupFrequency::Daily => "0 0 * * *",
            BackupFrequency::Weekly => "0 0 * * 0",
            BackupFrequency::Monthly => "0 0 1 * *",
            BackupFrequency::Custom => "",
        }
    }
}

/// Scheduled database backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledDatabaseBackup {
    pub id: Uuid,
    /// UUID for external references
    pub uuid: String,
    /// Backup enabled
    pub enabled: bool,
    /// Database ID (can be any database type)
    pub database_id: Uuid,
    /// Database type
    pub database_type: String,
    /// S3 Storage ID (for remote backups)
    pub s3_storage_id: Option<Uuid>,
    /// Backup frequency
    pub frequency: BackupFrequency,
    /// Custom cron expression (if frequency is Custom)
    pub cron_expression: Option<String>,
    /// Number of backups to keep
    pub keep_backups: i32,
    /// Whether to keep all backups (ignore keep_backups)
    pub keep_all_backups: bool,
    /// Databases to backup (comma-separated, or empty for all)
    pub databases_to_backup: Option<String>,
    /// Team ID (for ownership)
    pub team_id: Uuid,
    /// Last run time
    pub last_run_at: Option<DateTime<Utc>>,
    /// Next scheduled run
    pub next_run_at: Option<DateTime<Utc>>,
    /// Last backup status
    pub last_status: Option<BackupStatus>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BackupStatus {
    Success,
    Failed,
    Running,
    Cancelled,
}

impl ScheduledDatabaseBackup {
    pub fn new(database_id: Uuid, database_type: String, team_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            uuid: Uuid::new_v4().to_string(),
            enabled: true,
            database_id,
            database_type,
            s3_storage_id: None,
            frequency: BackupFrequency::Daily,
            cron_expression: None,
            keep_backups: 7,
            keep_all_backups: false,
            databases_to_backup: None,
            team_id,
            last_run_at: None,
            next_run_at: None,
            last_status: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Get the effective cron expression
    pub fn get_cron(&self) -> &str {
        match self.frequency {
            BackupFrequency::Custom => self.cron_expression.as_deref().unwrap_or("0 0 * * *"),
            _ => self.frequency.to_cron(),
        }
    }

    /// Get list of databases to backup
    pub fn get_databases(&self) -> Vec<&str> {
        self.databases_to_backup
            .as_deref()
            .map(|s| s.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect())
            .unwrap_or_default()
    }

    /// Check if backup is to S3
    pub fn is_s3_backup(&self) -> bool {
        self.s3_storage_id.is_some()
    }

    /// Get backup filename prefix
    pub fn backup_filename_prefix(&self) -> String {
        format!("{}-{}", self.database_type, &self.uuid[..8])
    }

    /// Record a backup run
    pub fn record_run(&mut self, status: BackupStatus) {
        self.last_run_at = Some(Utc::now());
        self.last_status = Some(status);
        self.updated_at = Utc::now();
    }
}

/// Backup execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledDatabaseBackupExecution {
    pub id: Uuid,
    /// Backup schedule ID
    pub scheduled_backup_id: Uuid,
    /// Database that was backed up
    pub database_name: String,
    /// Backup status
    pub status: BackupStatus,
    /// Backup size in bytes
    pub size: Option<i64>,
    /// Backup filename
    pub filename: String,
    /// S3 path (if uploaded)
    pub s3_path: Option<String>,
    /// Error message (if failed)
    pub error_message: Option<String>,
    /// Backup duration in seconds
    pub duration_seconds: Option<i32>,
    pub created_at: DateTime<Utc>,
}

impl ScheduledDatabaseBackupExecution {
    pub fn new(scheduled_backup_id: Uuid, database_name: String, filename: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            scheduled_backup_id,
            database_name,
            status: BackupStatus::Running,
            size: None,
            filename,
            s3_path: None,
            error_message: None,
            duration_seconds: None,
            created_at: Utc::now(),
        }
    }

    pub fn mark_success(&mut self, size: i64, duration: i32) {
        self.status = BackupStatus::Success;
        self.size = Some(size);
        self.duration_seconds = Some(duration);
    }

    pub fn mark_failed(&mut self, error: String) {
        self.status = BackupStatus::Failed;
        self.error_message = Some(error);
    }

    /// Get human-readable size
    pub fn size_human(&self) -> Option<String> {
        self.size.map(|bytes| {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_frequency_cron() {
        assert_eq!(BackupFrequency::Daily.to_cron(), "0 0 * * *");
        assert_eq!(BackupFrequency::Weekly.to_cron(), "0 0 * * 0");
    }

    #[test]
    fn test_scheduled_backup_new() {
        let db_id = Uuid::new_v4();
        let team_id = Uuid::new_v4();

        let backup = ScheduledDatabaseBackup::new(
            db_id,
            "postgresql".to_string(),
            team_id,
        );

        assert!(backup.enabled);
        assert_eq!(backup.frequency, BackupFrequency::Daily);
        assert_eq!(backup.keep_backups, 7);
    }

    #[test]
    fn test_get_cron() {
        let db_id = Uuid::new_v4();
        let team_id = Uuid::new_v4();

        let mut backup = ScheduledDatabaseBackup::new(db_id, "postgresql".to_string(), team_id);

        assert_eq!(backup.get_cron(), "0 0 * * *");

        backup.frequency = BackupFrequency::Custom;
        backup.cron_expression = Some("*/15 * * * *".to_string());
        assert_eq!(backup.get_cron(), "*/15 * * * *");
    }

    #[test]
    fn test_execution_size_human() {
        let mut exec = ScheduledDatabaseBackupExecution::new(
            Uuid::new_v4(),
            "mydb".to_string(),
            "backup.sql.gz".to_string(),
        );

        exec.size = Some(1024 * 1024 * 50);
        assert!(exec.size_human().unwrap().contains("MB"));
    }
}
