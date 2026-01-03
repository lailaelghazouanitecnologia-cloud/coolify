//! Notification settings API handlers

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

/// Notification settings response
#[derive(Debug, Serialize)]
pub struct NotificationSettingsResponse {
    pub id: Uuid,
    pub team_id: Uuid,
    pub name: String,
    pub notification_type: String,
    pub enabled: bool,
    pub settings: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Discord notification settings
#[derive(Debug, Serialize, Deserialize)]
pub struct DiscordSettings {
    pub webhook_url: String,
    pub notify_on_success: bool,
    pub notify_on_failure: bool,
    pub notify_on_status_change: bool,
    pub notify_on_backup: bool,
}

/// Telegram notification settings
#[derive(Debug, Serialize, Deserialize)]
pub struct TelegramSettings {
    pub bot_token: String,
    pub chat_id: String,
    pub notify_on_success: bool,
    pub notify_on_failure: bool,
    pub notify_on_status_change: bool,
    pub notify_on_backup: bool,
}

/// Slack notification settings
#[derive(Debug, Serialize, Deserialize)]
pub struct SlackSettings {
    pub webhook_url: String,
    pub channel: Option<String>,
    pub notify_on_success: bool,
    pub notify_on_failure: bool,
    pub notify_on_status_change: bool,
    pub notify_on_backup: bool,
}

/// Email notification settings
#[derive(Debug, Serialize, Deserialize)]
pub struct EmailSettings {
    pub recipients: Vec<String>,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<u16>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub use_tls: bool,
    pub notify_on_success: bool,
    pub notify_on_failure: bool,
    pub notify_on_status_change: bool,
    pub notify_on_backup: bool,
}

/// Create notification settings request
#[derive(Debug, Deserialize)]
pub struct CreateNotificationRequest {
    pub name: String,
    pub notification_type: String,
    pub settings: serde_json::Value,
    pub enabled: Option<bool>,
}

/// Update notification settings request
#[derive(Debug, Deserialize)]
pub struct UpdateNotificationRequest {
    pub name: Option<String>,
    pub settings: Option<serde_json::Value>,
    pub enabled: Option<bool>,
}

/// Test notification request
#[derive(Debug, Deserialize)]
pub struct TestNotificationRequest {
    pub message: Option<String>,
}

/// List all notification settings for the team
pub async fn list(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<NotificationSettingsResponse>>, ApiError> {
    // TODO: Implement listing from database
    Ok(Json(vec![]))
}

/// Get notification settings by ID
pub async fn get(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<NotificationSettingsResponse>, ApiError> {
    Err(ApiError::not_found(format!("Notification settings {} not found", id)))
}

/// Create new notification settings
pub async fn create(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateNotificationRequest>,
) -> Result<Json<NotificationSettingsResponse>, ApiError> {
    // Validate notification type
    let valid_types = ["discord", "telegram", "slack", "email"];
    if !valid_types.contains(&req.notification_type.as_str()) {
        return Err(ApiError::validation(format!(
            "Invalid notification type. Must be one of: {}",
            valid_types.join(", ")
        )));
    }

    let now = chrono::Utc::now();

    let response = NotificationSettingsResponse {
        id: Uuid::new_v4(),
        team_id: Uuid::new_v4(), // TODO: Get from auth context
        name: req.name,
        notification_type: req.notification_type,
        enabled: req.enabled.unwrap_or(true),
        settings: req.settings,
        created_at: now,
        updated_at: now,
    };

    // TODO: Persist to database
    Ok(Json(response))
}

/// Update notification settings
pub async fn update(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateNotificationRequest>,
) -> Result<Json<NotificationSettingsResponse>, ApiError> {
    Err(ApiError::not_found(format!("Notification settings {} not found", id)))
}

/// Delete notification settings
pub async fn delete(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // TODO: Delete from database
    Ok(Json(serde_json::json!({
        "message": "Notification settings deleted successfully"
    })))
}

/// Enable notification settings
pub async fn enable(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // TODO: Update database
    Ok(Json(serde_json::json!({
        "message": "Notifications enabled"
    })))
}

/// Disable notification settings
pub async fn disable(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // TODO: Update database
    Ok(Json(serde_json::json!({
        "message": "Notifications disabled"
    })))
}

/// Test notification (send a test message)
pub async fn test(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<TestNotificationRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let message = req.message.unwrap_or_else(|| "This is a test notification from Kornetti".to_string());

    // TODO: Retrieve notification settings and send test message
    // TODO: Queue a test notification job

    Ok(Json(serde_json::json!({
        "message": "Test notification sent",
        "test_message": message
    })))
}
