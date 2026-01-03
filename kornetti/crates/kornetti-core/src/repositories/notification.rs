//! Notification settings repository

use sqlx::{PgPool, FromRow};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::{Result, Error};
use crate::models::notification::{
    DiscordNotificationSettings, TelegramNotificationSettings,
    SlackNotificationSettings, EmailNotificationSettings,
};

/// Notification type enum for database storage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationType {
    Discord,
    Telegram,
    Slack,
    Email,
}

impl NotificationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            NotificationType::Discord => "discord",
            NotificationType::Telegram => "telegram",
            NotificationType::Slack => "slack",
            NotificationType::Email => "email",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "discord" => Some(NotificationType::Discord),
            "telegram" => Some(NotificationType::Telegram),
            "slack" => Some(NotificationType::Slack),
            "email" => Some(NotificationType::Email),
            _ => None,
        }
    }
}

/// Base notification settings row
#[derive(FromRow)]
struct NotificationSettingsRow {
    id: Uuid,
    team_id: Uuid,
    name: String,
    notification_type: String,
    enabled: bool,
    notify_on_deployment_success: bool,
    notify_on_deployment_failure: bool,
    notify_on_status_change: bool,
    notify_on_backup_success: bool,
    notify_on_backup_failure: bool,
    notify_on_scheduled_task: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Combined notification settings with type-specific data
#[derive(Debug, Clone)]
pub struct NotificationSettings {
    pub id: Uuid,
    pub team_id: Uuid,
    pub name: String,
    pub notification_type: NotificationType,
    pub enabled: bool,
    pub notify_on_deployment_success: bool,
    pub notify_on_deployment_failure: bool,
    pub notify_on_status_change: bool,
    pub notify_on_backup_success: bool,
    pub notify_on_backup_failure: bool,
    pub notify_on_scheduled_task: bool,
    pub type_settings: NotificationTypeSettings,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum NotificationTypeSettings {
    Discord { webhook_url: String },
    Telegram { bot_token: String, chat_id: String },
    Slack { webhook_url: String, channel: Option<String> },
    Email { recipients: Vec<String>, use_instance_smtp: bool },
}

pub struct NotificationRepository {
    pool: PgPool,
}

impl NotificationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Find all notification settings for a team
    pub async fn find_by_team(&self, team_id: Uuid) -> Result<Vec<NotificationSettings>> {
        let rows = sqlx::query_as::<_, NotificationSettingsRow>(
            r#"
            SELECT id, team_id, name, notification_type, enabled,
                   notify_on_deployment_success, notify_on_deployment_failure,
                   notify_on_status_change, notify_on_backup_success,
                   notify_on_backup_failure, notify_on_scheduled_task,
                   created_at, updated_at
            FROM notification_settings
            WHERE team_id = $1
            ORDER BY created_at DESC
            "#
        )
        .bind(team_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            if let Some(settings) = self.load_type_settings(&row).await? {
                results.push(settings);
            }
        }

        Ok(results)
    }

    /// Find notification settings by ID
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<NotificationSettings>> {
        let row = sqlx::query_as::<_, NotificationSettingsRow>(
            r#"
            SELECT id, team_id, name, notification_type, enabled,
                   notify_on_deployment_success, notify_on_deployment_failure,
                   notify_on_status_change, notify_on_backup_success,
                   notify_on_backup_failure, notify_on_scheduled_task,
                   created_at, updated_at
            FROM notification_settings
            WHERE id = $1
            "#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        match row {
            Some(r) => self.load_type_settings(&r).await,
            None => Ok(None),
        }
    }

    /// Load type-specific settings for a notification
    async fn load_type_settings(&self, row: &NotificationSettingsRow) -> Result<Option<NotificationSettings>> {
        let notification_type = match NotificationType::from_str(&row.notification_type) {
            Some(t) => t,
            None => return Ok(None),
        };

        let type_settings = match notification_type {
            NotificationType::Discord => {
                let discord = sqlx::query_as::<_, (String,)>(
                    "SELECT webhook_url FROM discord_notifications WHERE notification_setting_id = $1"
                )
                .bind(row.id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| Error::Database(e.to_string()))?;

                match discord {
                    Some((webhook_url,)) => NotificationTypeSettings::Discord { webhook_url },
                    None => return Ok(None),
                }
            }
            NotificationType::Telegram => {
                let telegram = sqlx::query_as::<_, (String, String)>(
                    "SELECT bot_token, chat_id FROM telegram_notifications WHERE notification_setting_id = $1"
                )
                .bind(row.id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| Error::Database(e.to_string()))?;

                match telegram {
                    Some((bot_token, chat_id)) => NotificationTypeSettings::Telegram { bot_token, chat_id },
                    None => return Ok(None),
                }
            }
            NotificationType::Slack => {
                let slack = sqlx::query_as::<_, (String, Option<String>)>(
                    "SELECT webhook_url, channel FROM slack_notifications WHERE notification_setting_id = $1"
                )
                .bind(row.id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| Error::Database(e.to_string()))?;

                match slack {
                    Some((webhook_url, channel)) => NotificationTypeSettings::Slack { webhook_url, channel },
                    None => return Ok(None),
                }
            }
            NotificationType::Email => {
                let email = sqlx::query_as::<_, (Vec<String>, bool)>(
                    "SELECT recipients, use_instance_smtp FROM email_notifications WHERE notification_setting_id = $1"
                )
                .bind(row.id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| Error::Database(e.to_string()))?;

                match email {
                    Some((recipients, use_instance_smtp)) => NotificationTypeSettings::Email { recipients, use_instance_smtp },
                    None => return Ok(None),
                }
            }
        };

        Ok(Some(NotificationSettings {
            id: row.id,
            team_id: row.team_id,
            name: row.name.clone(),
            notification_type,
            enabled: row.enabled,
            notify_on_deployment_success: row.notify_on_deployment_success,
            notify_on_deployment_failure: row.notify_on_deployment_failure,
            notify_on_status_change: row.notify_on_status_change,
            notify_on_backup_success: row.notify_on_backup_success,
            notify_on_backup_failure: row.notify_on_backup_failure,
            notify_on_scheduled_task: row.notify_on_scheduled_task,
            type_settings,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    /// Create notification settings
    pub async fn create(&self, settings: &NotificationSettings) -> Result<NotificationSettings> {
        let mut tx = self.pool.begin().await
            .map_err(|e| Error::Database(e.to_string()))?;

        // Insert base settings
        sqlx::query(
            r#"
            INSERT INTO notification_settings (id, team_id, name, notification_type, enabled,
                notify_on_deployment_success, notify_on_deployment_failure,
                notify_on_status_change, notify_on_backup_success,
                notify_on_backup_failure, notify_on_scheduled_task)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#
        )
        .bind(settings.id)
        .bind(settings.team_id)
        .bind(&settings.name)
        .bind(settings.notification_type.as_str())
        .bind(settings.enabled)
        .bind(settings.notify_on_deployment_success)
        .bind(settings.notify_on_deployment_failure)
        .bind(settings.notify_on_status_change)
        .bind(settings.notify_on_backup_success)
        .bind(settings.notify_on_backup_failure)
        .bind(settings.notify_on_scheduled_task)
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        // Insert type-specific settings
        match &settings.type_settings {
            NotificationTypeSettings::Discord { webhook_url } => {
                sqlx::query(
                    "INSERT INTO discord_notifications (id, notification_setting_id, webhook_url) VALUES ($1, $2, $3)"
                )
                .bind(Uuid::new_v4())
                .bind(settings.id)
                .bind(webhook_url)
                .execute(&mut *tx)
                .await
                .map_err(|e| Error::Database(e.to_string()))?;
            }
            NotificationTypeSettings::Telegram { bot_token, chat_id } => {
                sqlx::query(
                    "INSERT INTO telegram_notifications (id, notification_setting_id, bot_token, chat_id) VALUES ($1, $2, $3, $4)"
                )
                .bind(Uuid::new_v4())
                .bind(settings.id)
                .bind(bot_token)
                .bind(chat_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| Error::Database(e.to_string()))?;
            }
            NotificationTypeSettings::Slack { webhook_url, channel } => {
                sqlx::query(
                    "INSERT INTO slack_notifications (id, notification_setting_id, webhook_url, channel) VALUES ($1, $2, $3, $4)"
                )
                .bind(Uuid::new_v4())
                .bind(settings.id)
                .bind(webhook_url)
                .bind(channel)
                .execute(&mut *tx)
                .await
                .map_err(|e| Error::Database(e.to_string()))?;
            }
            NotificationTypeSettings::Email { recipients, use_instance_smtp } => {
                sqlx::query(
                    "INSERT INTO email_notifications (id, notification_setting_id, recipients, use_instance_smtp) VALUES ($1, $2, $3, $4)"
                )
                .bind(Uuid::new_v4())
                .bind(settings.id)
                .bind(recipients)
                .bind(use_instance_smtp)
                .execute(&mut *tx)
                .await
                .map_err(|e| Error::Database(e.to_string()))?;
            }
        }

        tx.commit().await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(settings.clone())
    }

    /// Update enabled status
    pub async fn set_enabled(&self, id: Uuid, enabled: bool) -> Result<()> {
        sqlx::query("UPDATE notification_settings SET enabled = $2 WHERE id = $1")
            .bind(id)
            .bind(enabled)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    /// Delete notification settings
    pub async fn delete(&self, id: Uuid) -> Result<()> {
        // Type-specific tables have ON DELETE CASCADE
        sqlx::query("DELETE FROM notification_settings WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    /// Find enabled notifications for a team by event type
    pub async fn find_enabled_for_event(
        &self,
        team_id: Uuid,
        event: NotificationEvent,
    ) -> Result<Vec<NotificationSettings>> {
        let column = match event {
            NotificationEvent::DeploymentSuccess => "notify_on_deployment_success",
            NotificationEvent::DeploymentFailure => "notify_on_deployment_failure",
            NotificationEvent::StatusChange => "notify_on_status_change",
            NotificationEvent::BackupSuccess => "notify_on_backup_success",
            NotificationEvent::BackupFailure => "notify_on_backup_failure",
            NotificationEvent::ScheduledTask => "notify_on_scheduled_task",
        };

        let query = format!(
            r#"
            SELECT id, team_id, name, notification_type, enabled,
                   notify_on_deployment_success, notify_on_deployment_failure,
                   notify_on_status_change, notify_on_backup_success,
                   notify_on_backup_failure, notify_on_scheduled_task,
                   created_at, updated_at
            FROM notification_settings
            WHERE team_id = $1 AND enabled = true AND {} = true
            "#,
            column
        );

        let rows = sqlx::query_as::<_, NotificationSettingsRow>(&query)
            .bind(team_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            if let Some(settings) = self.load_type_settings(&row).await? {
                results.push(settings);
            }
        }

        Ok(results)
    }
}

/// Notification event types
#[derive(Debug, Clone, Copy)]
pub enum NotificationEvent {
    DeploymentSuccess,
    DeploymentFailure,
    StatusChange,
    BackupSuccess,
    BackupFailure,
    ScheduledTask,
}
