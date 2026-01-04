//! Notification Settings Models
//!
//! Configuration for various notification channels.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Base notification settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettingsBase {
    pub id: Uuid,
    /// Team that owns these settings
    pub team_id: Uuid,
    /// Whether this channel is enabled
    pub enabled: bool,
    /// Notify on deployment success
    pub notify_on_deployment_success: bool,
    /// Notify on deployment failure
    pub notify_on_deployment_failure: bool,
    /// Notify on backup success
    pub notify_on_backup_success: bool,
    /// Notify on backup failure
    pub notify_on_backup_failure: bool,
    /// Notify on server health issues
    pub notify_on_server_health: bool,
    /// Notify on scheduled task results
    pub notify_on_scheduled_task: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for NotificationSettingsBase {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            team_id: Uuid::nil(),
            enabled: false,
            notify_on_deployment_success: true,
            notify_on_deployment_failure: true,
            notify_on_backup_success: false,
            notify_on_backup_failure: true,
            notify_on_server_health: true,
            notify_on_scheduled_task: false,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Discord notification settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordNotificationSettings {
    #[serde(flatten)]
    pub base: NotificationSettingsBase,
    /// Discord webhook URL
    pub webhook_url: String,
}

impl DiscordNotificationSettings {
    pub fn new(team_id: Uuid, webhook_url: String) -> Self {
        let mut base = NotificationSettingsBase::default();
        base.team_id = team_id;
        Self { base, webhook_url }
    }

    /// Validate the webhook URL format
    pub fn is_valid_webhook(&self) -> bool {
        self.webhook_url.starts_with("https://discord.com/api/webhooks/")
            || self.webhook_url.starts_with("https://discordapp.com/api/webhooks/")
    }
}

/// Slack notification settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackNotificationSettings {
    #[serde(flatten)]
    pub base: NotificationSettingsBase,
    /// Slack webhook URL
    pub webhook_url: String,
}

impl SlackNotificationSettings {
    pub fn new(team_id: Uuid, webhook_url: String) -> Self {
        let mut base = NotificationSettingsBase::default();
        base.team_id = team_id;
        Self { base, webhook_url }
    }

    /// Validate the webhook URL format
    pub fn is_valid_webhook(&self) -> bool {
        self.webhook_url.starts_with("https://hooks.slack.com/services/")
    }
}

/// Email notification settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailNotificationSettings {
    #[serde(flatten)]
    pub base: NotificationSettingsBase,
    /// SMTP host
    pub smtp_host: String,
    /// SMTP port
    pub smtp_port: u16,
    /// SMTP username
    pub smtp_username: Option<String>,
    /// SMTP password (encrypted)
    pub smtp_password: Option<String>,
    /// Use STARTTLS
    pub smtp_starttls: bool,
    /// Use SSL/TLS
    pub smtp_ssl: bool,
    /// From email address
    pub from_email: String,
    /// From name
    pub from_name: Option<String>,
    /// Recipients (comma-separated)
    pub recipients: String,
}

impl EmailNotificationSettings {
    pub fn new(team_id: Uuid, smtp_host: String, from_email: String, recipients: String) -> Self {
        let mut base = NotificationSettingsBase::default();
        base.team_id = team_id;
        Self {
            base,
            smtp_host,
            smtp_port: 587,
            smtp_username: None,
            smtp_password: None,
            smtp_starttls: true,
            smtp_ssl: false,
            from_email,
            from_name: None,
            recipients,
        }
    }

    /// Get recipients as a list
    pub fn recipient_list(&self) -> Vec<&str> {
        self.recipients
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

/// Pushover notification settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushoverNotificationSettings {
    #[serde(flatten)]
    pub base: NotificationSettingsBase,
    /// Pushover API token
    pub api_token: String,
    /// Pushover user key
    pub user_key: String,
    /// Sound for notifications
    pub sound: Option<String>,
    /// Priority (-2 to 2)
    pub priority: i8,
}

impl PushoverNotificationSettings {
    pub fn new(team_id: Uuid, api_token: String, user_key: String) -> Self {
        let mut base = NotificationSettingsBase::default();
        base.team_id = team_id;
        Self {
            base,
            api_token,
            user_key,
            sound: None,
            priority: 0,
        }
    }
}

/// Telegram notification settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramNotificationSettings {
    #[serde(flatten)]
    pub base: NotificationSettingsBase,
    /// Telegram bot token
    pub bot_token: String,
    /// Telegram chat ID
    pub chat_id: String,
    /// Parse mode (HTML, Markdown, MarkdownV2)
    pub parse_mode: Option<String>,
    /// Disable notification sound
    pub disable_notification: bool,
}

impl TelegramNotificationSettings {
    pub fn new(team_id: Uuid, bot_token: String, chat_id: String) -> Self {
        let mut base = NotificationSettingsBase::default();
        base.team_id = team_id;
        Self {
            base,
            bot_token,
            chat_id,
            parse_mode: Some("HTML".to_string()),
            disable_notification: false,
        }
    }

    /// Get the Telegram API URL for sending messages
    pub fn send_message_url(&self) -> String {
        format!("https://api.telegram.org/bot{}/sendMessage", self.bot_token)
    }
}

/// Unified notification channel enum
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum NotificationChannel {
    Discord(DiscordNotificationSettings),
    Slack(SlackNotificationSettings),
    Email(EmailNotificationSettings),
    Pushover(PushoverNotificationSettings),
    Telegram(TelegramNotificationSettings),
}

impl NotificationChannel {
    pub fn is_enabled(&self) -> bool {
        match self {
            NotificationChannel::Discord(s) => s.base.enabled,
            NotificationChannel::Slack(s) => s.base.enabled,
            NotificationChannel::Email(s) => s.base.enabled,
            NotificationChannel::Pushover(s) => s.base.enabled,
            NotificationChannel::Telegram(s) => s.base.enabled,
        }
    }

    pub fn team_id(&self) -> Uuid {
        match self {
            NotificationChannel::Discord(s) => s.base.team_id,
            NotificationChannel::Slack(s) => s.base.team_id,
            NotificationChannel::Email(s) => s.base.team_id,
            NotificationChannel::Pushover(s) => s.base.team_id,
            NotificationChannel::Telegram(s) => s.base.team_id,
        }
    }

    pub fn channel_type(&self) -> &'static str {
        match self {
            NotificationChannel::Discord(_) => "discord",
            NotificationChannel::Slack(_) => "slack",
            NotificationChannel::Email(_) => "email",
            NotificationChannel::Pushover(_) => "pushover",
            NotificationChannel::Telegram(_) => "telegram",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discord_settings() {
        let team_id = Uuid::new_v4();
        let settings = DiscordNotificationSettings::new(
            team_id,
            "https://discord.com/api/webhooks/123/abc".to_string(),
        );

        assert!(settings.is_valid_webhook());
        assert!(!settings.base.enabled);
    }

    #[test]
    fn test_slack_settings() {
        let team_id = Uuid::new_v4();
        let settings = SlackNotificationSettings::new(
            team_id,
            "https://hooks.slack.com/services/T00/B00/xxxx".to_string(),
        );

        assert!(settings.is_valid_webhook());
    }

    #[test]
    fn test_email_recipients() {
        let team_id = Uuid::new_v4();
        let settings = EmailNotificationSettings::new(
            team_id,
            "smtp.example.com".to_string(),
            "noreply@example.com".to_string(),
            "user1@example.com, user2@example.com".to_string(),
        );

        let recipients = settings.recipient_list();
        assert_eq!(recipients.len(), 2);
        assert_eq!(recipients[0], "user1@example.com");
    }

    #[test]
    fn test_notification_channel_enum() {
        let team_id = Uuid::new_v4();
        let discord = NotificationChannel::Discord(DiscordNotificationSettings::new(
            team_id,
            "https://discord.com/api/webhooks/123/abc".to_string(),
        ));

        assert!(!discord.is_enabled());
        assert_eq!(discord.team_id(), team_id);
        assert_eq!(discord.channel_type(), "discord");
    }
}
