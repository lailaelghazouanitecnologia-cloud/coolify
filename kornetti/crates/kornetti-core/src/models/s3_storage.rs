//! S3 storage configuration for backups

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// S3-compatible storage configuration for database backups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Storage {
    pub id: Uuid,
    pub team_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    /// S3 endpoint (e.g., s3.amazonaws.com, minio.example.com)
    pub endpoint: String,
    /// S3 bucket name
    pub bucket: String,
    /// S3 region
    pub region: String,
    /// Access key ID
    pub access_key: String,
    /// Secret access key (encrypted at rest)
    pub secret_key: String,
    /// Whether to use path-style URLs (for MinIO, etc.)
    pub use_path_style: bool,
    /// Whether this storage is verified/working
    pub is_usable: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl S3Storage {
    pub fn new(team_id: Uuid, name: String, endpoint: String, bucket: String, region: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            team_id,
            name,
            description: None,
            endpoint,
            bucket,
            region,
            access_key: String::new(),
            secret_key: String::new(),
            use_path_style: false,
            is_usable: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create AWS S3 storage
    pub fn aws(team_id: Uuid, name: String, bucket: String, region: String, access_key: String, secret_key: String) -> Self {
        let mut storage = Self::new(team_id, name, format!("s3.{}.amazonaws.com", region), bucket, region);
        storage.access_key = access_key;
        storage.secret_key = secret_key;
        storage
    }

    /// Create MinIO storage
    pub fn minio(team_id: Uuid, name: String, endpoint: String, bucket: String, access_key: String, secret_key: String) -> Self {
        let mut storage = Self::new(team_id, name, endpoint, bucket, "us-east-1".to_string());
        storage.access_key = access_key;
        storage.secret_key = secret_key;
        storage.use_path_style = true;
        storage
    }

    /// Get the full endpoint URL
    pub fn endpoint_url(&self) -> String {
        if self.endpoint.starts_with("http") {
            self.endpoint.clone()
        } else {
            format!("https://{}", self.endpoint)
        }
    }

    /// Get the bucket URL for path-style or virtual-hosted style
    pub fn bucket_url(&self) -> String {
        if self.use_path_style {
            format!("{}/{}", self.endpoint_url(), self.bucket)
        } else {
            format!("https://{}.{}", self.bucket, self.endpoint)
        }
    }
}

/// Scheduled database backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledDatabaseBackup {
    pub id: Uuid,
    pub database_id: Uuid,
    pub database_type: String,
    pub s3_storage_id: Option<Uuid>,
    /// Cron expression for scheduling
    pub frequency: String,
    /// Whether backup is enabled
    pub enabled: bool,
    /// Whether to save to local disk
    pub save_to_disk: bool,
    /// Number of backups to keep
    pub keep_locally: i32,
    /// Number of days to keep in S3
    pub keep_s3: i32,
    /// Databases to backup (comma-separated, empty = all)
    pub databases_to_backup: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ScheduledDatabaseBackup {
    pub fn new(database_id: Uuid, database_type: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            database_id,
            database_type,
            s3_storage_id: None,
            frequency: "0 0 * * *".to_string(), // Daily at midnight
            enabled: true,
            save_to_disk: true,
            keep_locally: 7,
            keep_s3: 30,
            databases_to_backup: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Parse cron expression to human-readable format
    pub fn frequency_description(&self) -> String {
        match self.frequency.as_str() {
            "0 0 * * *" => "Daily at midnight".to_string(),
            "0 * * * *" => "Every hour".to_string(),
            "*/15 * * * *" => "Every 15 minutes".to_string(),
            "0 0 * * 0" => "Weekly on Sunday".to_string(),
            "0 0 1 * *" => "Monthly on the 1st".to_string(),
            _ => format!("Custom: {}", self.frequency),
        }
    }
}

/// Backup execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledDatabaseBackupExecution {
    pub id: Uuid,
    pub scheduled_backup_id: Uuid,
    pub status: BackupStatus,
    pub message: Option<String>,
    /// Filename of the backup
    pub filename: Option<String>,
    /// Size in bytes
    pub size: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackupStatus {
    Running,
    Success,
    Failed,
}

impl ScheduledDatabaseBackupExecution {
    pub fn start(scheduled_backup_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            scheduled_backup_id,
            status: BackupStatus::Running,
            message: None,
            filename: None,
            size: None,
            created_at: Utc::now(),
            finished_at: None,
        }
    }

    pub fn success(mut self, filename: String, size: i64) -> Self {
        self.status = BackupStatus::Success;
        self.filename = Some(filename);
        self.size = Some(size);
        self.finished_at = Some(Utc::now());
        self
    }

    pub fn failed(mut self, message: String) -> Self {
        self.status = BackupStatus::Failed;
        self.message = Some(message);
        self.finished_at = Some(Utc::now());
        self
    }

    pub fn duration_seconds(&self) -> Option<i64> {
        self.finished_at.map(|f| (f - self.created_at).num_seconds())
    }
}
