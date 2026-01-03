//! Notification settings and messages

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Discord notification settings for a team
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordNotificationSettings {
    pub id: Uuid,
    pub team_id: Uuid,
    pub enabled: bool,
    pub webhook_url: Option<String>,
    pub notify_deployments: bool,
    pub notify_deployments_failed: bool,
    pub notify_status_changes: bool,
    pub notify_backups: bool,
    pub notify_backups_failed: bool,
    pub notify_scheduled_tasks: bool,
    pub notify_scheduled_tasks_failed: bool,
    pub notify_disk_usage: bool,
    pub notify_test: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for DiscordNotificationSettings {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            team_id: Uuid::nil(),
            enabled: false,
            webhook_url: None,
            notify_deployments: true,
            notify_deployments_failed: true,
            notify_status_changes: true,
            notify_backups: false,
            notify_backups_failed: true,
            notify_scheduled_tasks: false,
            notify_scheduled_tasks_failed: true,
            notify_disk_usage: true,
            notify_test: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

/// Telegram notification settings for a team
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramNotificationSettings {
    pub id: Uuid,
    pub team_id: Uuid,
    pub enabled: bool,
    pub bot_token: Option<String>,
    pub chat_id: Option<String>,
    pub notify_deployments: bool,
    pub notify_deployments_failed: bool,
    pub notify_status_changes: bool,
    pub notify_backups: bool,
    pub notify_backups_failed: bool,
    pub notify_scheduled_tasks: bool,
    pub notify_scheduled_tasks_failed: bool,
    pub notify_disk_usage: bool,
    pub notify_test: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for TelegramNotificationSettings {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            team_id: Uuid::nil(),
            enabled: false,
            bot_token: None,
            chat_id: None,
            notify_deployments: true,
            notify_deployments_failed: true,
            notify_status_changes: true,
            notify_backups: false,
            notify_backups_failed: true,
            notify_scheduled_tasks: false,
            notify_scheduled_tasks_failed: true,
            notify_disk_usage: true,
            notify_test: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

/// Slack notification settings for a team
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackNotificationSettings {
    pub id: Uuid,
    pub team_id: Uuid,
    pub enabled: bool,
    pub webhook_url: Option<String>,
    pub notify_deployments: bool,
    pub notify_deployments_failed: bool,
    pub notify_status_changes: bool,
    pub notify_backups: bool,
    pub notify_backups_failed: bool,
    pub notify_scheduled_tasks: bool,
    pub notify_scheduled_tasks_failed: bool,
    pub notify_disk_usage: bool,
    pub notify_test: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for SlackNotificationSettings {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            team_id: Uuid::nil(),
            enabled: false,
            webhook_url: None,
            notify_deployments: true,
            notify_deployments_failed: true,
            notify_status_changes: true,
            notify_backups: false,
            notify_backups_failed: true,
            notify_scheduled_tasks: false,
            notify_scheduled_tasks_failed: true,
            notify_disk_usage: true,
            notify_test: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

/// Email notification settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailNotificationSettings {
    pub id: Uuid,
    pub team_id: Uuid,
    pub enabled: bool,
    /// Use instance SMTP settings
    pub use_instance_settings: bool,
    /// SMTP host
    pub smtp_host: Option<String>,
    pub smtp_port: Option<u16>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_encryption: Option<SmtpEncryption>,
    pub smtp_from_address: Option<String>,
    pub smtp_from_name: Option<String>,
    /// Recipients (comma-separated)
    pub recipients: Option<String>,
    pub notify_deployments: bool,
    pub notify_deployments_failed: bool,
    pub notify_status_changes: bool,
    pub notify_backups: bool,
    pub notify_backups_failed: bool,
    pub notify_scheduled_tasks: bool,
    pub notify_scheduled_tasks_failed: bool,
    pub notify_disk_usage: bool,
    pub notify_test: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SmtpEncryption {
    None,
    Tls,
    Starttls,
}

impl Default for EmailNotificationSettings {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            team_id: Uuid::nil(),
            enabled: false,
            use_instance_settings: true,
            smtp_host: None,
            smtp_port: Some(587),
            smtp_username: None,
            smtp_password: None,
            smtp_encryption: Some(SmtpEncryption::Tls),
            smtp_from_address: None,
            smtp_from_name: None,
            recipients: None,
            notify_deployments: true,
            notify_deployments_failed: true,
            notify_status_changes: true,
            notify_backups: false,
            notify_backups_failed: true,
            notify_scheduled_tasks: false,
            notify_scheduled_tasks_failed: true,
            notify_disk_usage: true,
            notify_test: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

/// Notification message to send
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationMessage {
    pub notification_type: NotificationType,
    pub title: String,
    pub message: String,
    pub url: Option<String>,
    pub level: NotificationLevel,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    Deployment,
    DeploymentFailed,
    StatusChange,
    Backup,
    BackupFailed,
    ScheduledTask,
    ScheduledTaskFailed,
    DiskUsage,
    Test,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NotificationLevel {
    Info,
    Success,
    Warning,
    Error,
}

impl NotificationMessage {
    pub fn deployment_success(app_name: &str, url: Option<String>) -> Self {
        Self {
            notification_type: NotificationType::Deployment,
            title: format!("✅ Deployment Successful: {}", app_name),
            message: format!("Application {} has been deployed successfully.", app_name),
            url,
            level: NotificationLevel::Success,
            metadata: None,
        }
    }

    pub fn deployment_failed(app_name: &str, error: &str, url: Option<String>) -> Self {
        Self {
            notification_type: NotificationType::DeploymentFailed,
            title: format!("❌ Deployment Failed: {}", app_name),
            message: format!("Deployment of {} failed: {}", app_name, error),
            url,
            level: NotificationLevel::Error,
            metadata: None,
        }
    }

    pub fn backup_success(db_name: &str, size: &str) -> Self {
        Self {
            notification_type: NotificationType::Backup,
            title: format!("✅ Backup Completed: {}", db_name),
            message: format!("Database {} backup completed. Size: {}", db_name, size),
            url: None,
            level: NotificationLevel::Success,
            metadata: None,
        }
    }

    pub fn backup_failed(db_name: &str, error: &str) -> Self {
        Self {
            notification_type: NotificationType::BackupFailed,
            title: format!("❌ Backup Failed: {}", db_name),
            message: format!("Database {} backup failed: {}", db_name, error),
            url: None,
            level: NotificationLevel::Error,
            metadata: None,
        }
    }

    pub fn disk_usage_warning(server_name: &str, usage_percent: u8) -> Self {
        Self {
            notification_type: NotificationType::DiskUsage,
            title: format!("⚠️ High Disk Usage: {}", server_name),
            message: format!("Server {} disk usage is at {}%", server_name, usage_percent),
            url: None,
            level: NotificationLevel::Warning,
            metadata: None,
        }
    }

    pub fn test() -> Self {
        Self {
            notification_type: NotificationType::Test,
            title: "🧪 Test Notification".to_string(),
            message: "This is a test notification from Kornetti.".to_string(),
            url: None,
            level: NotificationLevel::Info,
            metadata: None,
        }
    }
}
