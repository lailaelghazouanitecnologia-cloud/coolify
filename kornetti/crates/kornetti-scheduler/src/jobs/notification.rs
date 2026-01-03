//! Notification Jobs
//!
//! Send notifications via Discord, Telegram, Slack, and Email.

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::{error, info, instrument};
use uuid::Uuid;

use kornetti_core::{
    Result, Error,
    models::{
        NotificationMessage, NotificationLevel, NotificationType,
        DiscordNotificationSettings, TelegramNotificationSettings,
        SlackNotificationSettings, EmailNotificationSettings,
    },
};

/// Discord notification job
pub struct SendDiscordNotificationJob {
    settings: DiscordNotificationSettings,
    message: NotificationMessage,
}

impl SendDiscordNotificationJob {
    pub fn new(settings: DiscordNotificationSettings, message: NotificationMessage) -> Self {
        Self { settings, message }
    }

    /// Send notification to Discord
    #[instrument(skip(self))]
    pub async fn run(&self) -> Result<()> {
        if !self.settings.enabled {
            return Ok(());
        }

        let webhook_url = self.settings.webhook_url.as_ref()
            .ok_or_else(|| Error::Validation("Discord webhook URL not configured".to_string()))?;

        // Check if this notification type is enabled
        if !self.is_notification_enabled() {
            return Ok(());
        }

        let color = self.get_color();
        let payload = serde_json::json!({
            "embeds": [{
                "title": self.message.title,
                "description": self.message.message,
                "color": color,
                "url": self.message.url,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "footer": {
                    "text": "Kornetti"
                }
            }]
        });

        let client = reqwest::Client::new();
        let response = client
            .post(webhook_url)
            .json(&payload)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| Error::Internal(format!("Discord request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Internal(format!("Discord API error: {} - {}", status, body)));
        }

        info!("Discord notification sent successfully");
        Ok(())
    }

    fn is_notification_enabled(&self) -> bool {
        match self.message.notification_type {
            NotificationType::Deployment => self.settings.notify_deployments,
            NotificationType::DeploymentFailed => self.settings.notify_deployments_failed,
            NotificationType::StatusChange => self.settings.notify_status_changes,
            NotificationType::Backup => self.settings.notify_backups,
            NotificationType::BackupFailed => self.settings.notify_backups_failed,
            NotificationType::ScheduledTask => self.settings.notify_scheduled_tasks,
            NotificationType::ScheduledTaskFailed => self.settings.notify_scheduled_tasks_failed,
            NotificationType::DiskUsage => self.settings.notify_disk_usage,
            NotificationType::Test => self.settings.notify_test,
        }
    }

    fn get_color(&self) -> u32 {
        match self.message.level {
            NotificationLevel::Success => 0x00FF00, // Green
            NotificationLevel::Info => 0x0099FF,    // Blue
            NotificationLevel::Warning => 0xFFAA00, // Orange
            NotificationLevel::Error => 0xFF0000,   // Red
        }
    }
}

/// Telegram notification job
pub struct SendTelegramNotificationJob {
    settings: TelegramNotificationSettings,
    message: NotificationMessage,
}

impl SendTelegramNotificationJob {
    pub fn new(settings: TelegramNotificationSettings, message: NotificationMessage) -> Self {
        Self { settings, message }
    }

    /// Send notification to Telegram
    #[instrument(skip(self))]
    pub async fn run(&self) -> Result<()> {
        if !self.settings.enabled {
            return Ok(());
        }

        let bot_token = self.settings.bot_token.as_ref()
            .ok_or_else(|| Error::Validation("Telegram bot token not configured".to_string()))?;

        let chat_id = self.settings.chat_id.as_ref()
            .ok_or_else(|| Error::Validation("Telegram chat ID not configured".to_string()))?;

        // Check if this notification type is enabled
        if !self.is_notification_enabled() {
            return Ok(());
        }

        let emoji = self.get_emoji();
        let text = format!(
            "{} *{}*\n\n{}{}",
            emoji,
            escape_markdown(&self.message.title),
            escape_markdown(&self.message.message),
            self.message.url.as_ref()
                .map(|u| format!("\n\n[View Details]({})", u))
                .unwrap_or_default()
        );

        let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);
        let payload = serde_json::json!({
            "chat_id": chat_id,
            "text": text,
            "parse_mode": "MarkdownV2",
            "disable_web_page_preview": false
        });

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .json(&payload)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| Error::Internal(format!("Telegram request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Internal(format!("Telegram API error: {} - {}", status, body)));
        }

        info!("Telegram notification sent successfully");
        Ok(())
    }

    fn is_notification_enabled(&self) -> bool {
        match self.message.notification_type {
            NotificationType::Deployment => self.settings.notify_deployments,
            NotificationType::DeploymentFailed => self.settings.notify_deployments_failed,
            NotificationType::StatusChange => self.settings.notify_status_changes,
            NotificationType::Backup => self.settings.notify_backups,
            NotificationType::BackupFailed => self.settings.notify_backups_failed,
            NotificationType::ScheduledTask => self.settings.notify_scheduled_tasks,
            NotificationType::ScheduledTaskFailed => self.settings.notify_scheduled_tasks_failed,
            NotificationType::DiskUsage => self.settings.notify_disk_usage,
            NotificationType::Test => self.settings.notify_test,
        }
    }

    fn get_emoji(&self) -> &str {
        match self.message.level {
            NotificationLevel::Success => "✅",
            NotificationLevel::Info => "ℹ️",
            NotificationLevel::Warning => "⚠️",
            NotificationLevel::Error => "❌",
        }
    }
}

/// Slack notification job
pub struct SendSlackNotificationJob {
    settings: SlackNotificationSettings,
    message: NotificationMessage,
}

impl SendSlackNotificationJob {
    pub fn new(settings: SlackNotificationSettings, message: NotificationMessage) -> Self {
        Self { settings, message }
    }

    /// Send notification to Slack
    #[instrument(skip(self))]
    pub async fn run(&self) -> Result<()> {
        if !self.settings.enabled {
            return Ok(());
        }

        let webhook_url = self.settings.webhook_url.as_ref()
            .ok_or_else(|| Error::Validation("Slack webhook URL not configured".to_string()))?;

        // Check if this notification type is enabled
        if !self.is_notification_enabled() {
            return Ok(());
        }

        let color = self.get_color();
        let payload = serde_json::json!({
            "attachments": [{
                "color": color,
                "title": self.message.title,
                "text": self.message.message,
                "title_link": self.message.url,
                "footer": "Kornetti",
                "ts": chrono::Utc::now().timestamp()
            }]
        });

        let client = reqwest::Client::new();
        let response = client
            .post(webhook_url)
            .json(&payload)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| Error::Internal(format!("Slack request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Internal(format!("Slack API error: {} - {}", status, body)));
        }

        info!("Slack notification sent successfully");
        Ok(())
    }

    fn is_notification_enabled(&self) -> bool {
        match self.message.notification_type {
            NotificationType::Deployment => self.settings.notify_deployments,
            NotificationType::DeploymentFailed => self.settings.notify_deployments_failed,
            NotificationType::StatusChange => self.settings.notify_status_changes,
            NotificationType::Backup => self.settings.notify_backups,
            NotificationType::BackupFailed => self.settings.notify_backups_failed,
            NotificationType::ScheduledTask => self.settings.notify_scheduled_tasks,
            NotificationType::ScheduledTaskFailed => self.settings.notify_scheduled_tasks_failed,
            NotificationType::DiskUsage => self.settings.notify_disk_usage,
            NotificationType::Test => self.settings.notify_test,
        }
    }

    fn get_color(&self) -> &str {
        match self.message.level {
            NotificationLevel::Success => "good",
            NotificationLevel::Info => "#0099FF",
            NotificationLevel::Warning => "warning",
            NotificationLevel::Error => "danger",
        }
    }
}

/// Escape special characters for Telegram MarkdownV2
fn escape_markdown(text: &str) -> String {
    text.chars()
        .map(|c| {
            if "_*[]()~`>#+-=|{}.!".contains(c) {
                format!("\\{}", c)
            } else {
                c.to_string()
            }
        })
        .collect()
}

/// Dispatcher for sending notifications to all configured channels
pub struct NotificationDispatcher {
    pub discord: Option<DiscordNotificationSettings>,
    pub telegram: Option<TelegramNotificationSettings>,
    pub slack: Option<SlackNotificationSettings>,
    pub email: Option<EmailNotificationSettings>,
}

impl NotificationDispatcher {
    pub fn new() -> Self {
        Self {
            discord: None,
            telegram: None,
            slack: None,
            email: None,
        }
    }

    /// Send notification to all configured channels
    pub async fn dispatch(&self, message: NotificationMessage) -> Vec<Result<()>> {
        let mut results = Vec::new();

        if let Some(ref settings) = self.discord {
            let job = SendDiscordNotificationJob::new(settings.clone(), message.clone());
            results.push(job.run().await);
        }

        if let Some(ref settings) = self.telegram {
            let job = SendTelegramNotificationJob::new(settings.clone(), message.clone());
            results.push(job.run().await);
        }

        if let Some(ref settings) = self.slack {
            let job = SendSlackNotificationJob::new(settings.clone(), message.clone());
            results.push(job.run().await);
        }

        // TODO: Add email notification

        results
    }
}
