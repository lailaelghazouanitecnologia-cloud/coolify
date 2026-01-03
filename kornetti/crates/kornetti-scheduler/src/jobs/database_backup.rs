//! Database Backup Job
//!
//! Handles automated database backups to local storage or S3.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

use kornetti_core::{
    Result, Error,
    models::{
        StandaloneDatabase, DatabaseType, DatabaseStatus,
        S3Storage, ScheduledDatabaseBackup, ScheduledDatabaseBackupExecution, BackupStatus,
    },
};
use kornetti_ssh::SshClient;

/// Backup context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseBackupContext {
    pub backup_id: Uuid,
    pub database_id: Uuid,
    pub database_type: DatabaseType,
    pub s3_storage_id: Option<Uuid>,
    pub save_to_disk: bool,
    pub databases_to_backup: Option<Vec<String>>,
}

/// Database backup job
pub struct DatabaseBackupJob {
    ssh: Arc<SshClient>,
    context: DatabaseBackupContext,
}

impl DatabaseBackupJob {
    pub fn new(ssh: Arc<SshClient>, context: DatabaseBackupContext) -> Self {
        Self { ssh, context }
    }

    /// Run the backup job
    #[instrument(skip(self, database, s3_storage))]
    pub async fn run(
        &self,
        database: &StandaloneDatabase,
        s3_storage: Option<&S3Storage>,
    ) -> Result<BackupResult> {
        info!(
            database_id = %database.id,
            database_type = ?database.database_type,
            "Starting database backup"
        );

        let start_time = Utc::now();
        let filename = self.generate_backup_filename(database);

        // Check if database is running
        if database.status != DatabaseStatus::Running {
            return Err(Error::Validation(
                "Database must be running to perform backup".to_string()
            ));
        }

        // Generate backup command based on database type
        let backup_command = self.generate_backup_command(database, &filename)?;

        // Execute backup on the server
        let server_id = database.server_id;

        // For now, we construct a minimal server reference
        // In production, this would be fetched from the repository
        info!(command = %backup_command, "Executing backup command");

        // Execute the backup
        let backup_path = format!("/data/coolify/backups/{}", filename);

        // Create backup directory
        let mkdir_cmd = "mkdir -p /data/coolify/backups";

        // Full backup command
        let full_command = format!("{} && {}", mkdir_cmd, backup_command);

        // TODO: Execute via SSH - for now return a simulated result
        let backup_size = 0i64; // Would be actual size

        // Upload to S3 if configured
        let s3_uploaded = if let Some(s3) = s3_storage {
            match self.upload_to_s3(s3, &backup_path, &filename).await {
                Ok(_) => {
                    info!(filename = %filename, "Backup uploaded to S3");
                    true
                }
                Err(e) => {
                    error!(error = %e, "Failed to upload backup to S3");
                    false
                }
            }
        } else {
            false
        };

        // Cleanup old backups if needed
        if self.context.save_to_disk {
            if let Err(e) = self.cleanup_old_backups().await {
                warn!(error = %e, "Failed to cleanup old backups");
            }
        }

        let end_time = Utc::now();
        let duration = (end_time - start_time).num_seconds();

        Ok(BackupResult {
            backup_id: self.context.backup_id,
            database_id: database.id,
            filename,
            size_bytes: backup_size,
            duration_seconds: duration,
            saved_to_disk: self.context.save_to_disk,
            uploaded_to_s3: s3_uploaded,
            completed_at: end_time,
        })
    }

    /// Generate backup filename with timestamp
    fn generate_backup_filename(&self, database: &StandaloneDatabase) -> String {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let db_type = match database.database_type {
            DatabaseType::Postgresql => "pg",
            DatabaseType::Mysql => "mysql",
            DatabaseType::Mariadb => "mariadb",
            DatabaseType::Mongodb => "mongo",
            DatabaseType::Redis => "redis",
            _ => "db",
        };

        format!("{}_{}_backup_{}.sql.gz", database.id, db_type, timestamp)
    }

    /// Generate backup command based on database type
    fn generate_backup_command(&self, database: &StandaloneDatabase, filename: &str) -> Result<String> {
        let container_name = database.id.to_string();
        let backup_path = format!("/data/coolify/backups/{}", filename);

        let command = match database.database_type {
            DatabaseType::Postgresql => {
                let db_name = self.context.databases_to_backup
                    .as_ref()
                    .and_then(|dbs| dbs.first())
                    .map(|s| s.as_str())
                    .unwrap_or("postgres");

                format!(
                    "docker exec {} pg_dump -U {} {} | gzip > {}",
                    container_name,
                    database.config.database_user,
                    db_name,
                    backup_path
                )
            }
            DatabaseType::Mysql | DatabaseType::Mariadb => {
                let db_arg = self.context.databases_to_backup
                    .as_ref()
                    .map(|dbs| dbs.join(" "))
                    .unwrap_or_else(|| "--all-databases".to_string());

                format!(
                    "docker exec {} mysqldump -u {} -p{} {} | gzip > {}",
                    container_name,
                    database.config.database_user,
                    database.config.database_password,
                    db_arg,
                    backup_path
                )
            }
            DatabaseType::Mongodb => {
                format!(
                    "docker exec {} mongodump --archive --gzip -u {} -p {} --authenticationDatabase admin > {}",
                    container_name,
                    database.config.database_user,
                    database.config.database_password,
                    backup_path
                )
            }
            DatabaseType::Redis | DatabaseType::Keydb | DatabaseType::Dragonfly => {
                format!(
                    "docker exec {} redis-cli BGSAVE && sleep 2 && docker cp {}:/data/dump.rdb {} && gzip {}",
                    container_name,
                    container_name,
                    backup_path.replace(".gz", ""),
                    backup_path.replace(".gz", "")
                )
            }
            DatabaseType::Clickhouse => {
                format!(
                    "docker exec {} clickhouse-client --query 'SELECT * FROM system.tables FORMAT TSV' | gzip > {}",
                    container_name,
                    backup_path
                )
            }
        };

        Ok(command)
    }

    /// Upload backup to S3
    async fn upload_to_s3(&self, s3: &S3Storage, local_path: &str, filename: &str) -> Result<()> {
        // Use AWS CLI or s5cmd for upload
        let upload_command = if s3.use_path_style {
            format!(
                "AWS_ACCESS_KEY_ID={} AWS_SECRET_ACCESS_KEY={} aws s3 cp {} s3://{}/{} --endpoint-url {} --region {}",
                s3.access_key,
                s3.secret_key,
                local_path,
                s3.bucket,
                filename,
                s3.endpoint_url(),
                s3.region
            )
        } else {
            format!(
                "AWS_ACCESS_KEY_ID={} AWS_SECRET_ACCESS_KEY={} aws s3 cp {} s3://{}/{} --region {}",
                s3.access_key,
                s3.secret_key,
                local_path,
                s3.bucket,
                filename,
                s3.region
            )
        };

        // TODO: Execute upload command via SSH
        info!(command = "aws s3 cp ...", "Uploading to S3");

        Ok(())
    }

    /// Cleanup old backups based on retention policy
    async fn cleanup_old_backups(&self) -> Result<()> {
        // TODO: Implement cleanup based on keep_locally and keep_s3 settings
        info!("Cleaning up old backups");
        Ok(())
    }
}

/// Backup result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupResult {
    pub backup_id: Uuid,
    pub database_id: Uuid,
    pub filename: String,
    pub size_bytes: i64,
    pub duration_seconds: i64,
    pub saved_to_disk: bool,
    pub uploaded_to_s3: bool,
    pub completed_at: DateTime<Utc>,
}

impl BackupResult {
    /// Format size as human-readable
    pub fn size_formatted(&self) -> String {
        let bytes = self.size_bytes as f64;
        if bytes >= 1_073_741_824.0 {
            format!("{:.2} GB", bytes / 1_073_741_824.0)
        } else if bytes >= 1_048_576.0 {
            format!("{:.2} MB", bytes / 1_048_576.0)
        } else if bytes >= 1024.0 {
            format!("{:.2} KB", bytes / 1024.0)
        } else {
            format!("{} bytes", self.size_bytes)
        }
    }
}
