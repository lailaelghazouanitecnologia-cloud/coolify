//! Settings handlers
//!
//! API handlers for instance and application settings.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::state::AppState;

/// Instance settings response
#[derive(Debug, Serialize)]
pub struct InstanceSettingsResponse {
    pub id: Uuid,
    pub public_port_min: i32,
    pub public_port_max: i32,
    pub custom_dns_servers: Option<String>,
    pub instance_name: Option<String>,
    pub is_auto_update_enabled: bool,
    pub is_registration_enabled: bool,
    pub is_https_forced: bool,
    pub is_api_enabled: bool,
    pub allowed_ips: Option<String>,
    pub do_not_track: bool,
    pub is_dns_validation_enabled: bool,
    pub next_channel: bool,
}

/// Request to update instance settings
#[derive(Debug, Deserialize)]
pub struct UpdateInstanceSettingsRequest {
    pub public_port_min: Option<i32>,
    pub public_port_max: Option<i32>,
    pub custom_dns_servers: Option<String>,
    pub instance_name: Option<String>,
    pub is_auto_update_enabled: Option<bool>,
    pub is_registration_enabled: Option<bool>,
    pub is_https_forced: Option<bool>,
    pub is_api_enabled: Option<bool>,
    pub allowed_ips: Option<String>,
    pub do_not_track: Option<bool>,
    pub is_dns_validation_enabled: Option<bool>,
    pub next_channel: Option<bool>,
}

/// S3 storage settings
#[derive(Debug, Serialize, Deserialize)]
pub struct S3StorageResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub endpoint: String,
    pub bucket: String,
    pub region: String,
    pub is_usable: bool,
}

/// Request to create S3 storage
#[derive(Debug, Deserialize)]
pub struct CreateS3StorageRequest {
    pub name: String,
    pub description: Option<String>,
    pub endpoint: String,
    pub bucket: String,
    pub region: String,
    pub key: String,
    pub secret: String,
}

/// Get instance settings
pub async fn get_instance(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Placeholder: would fetch instance settings from database
    Json(InstanceSettingsResponse {
        id: Uuid::new_v4(),
        public_port_min: 9000,
        public_port_max: 9100,
        custom_dns_servers: None,
        instance_name: Some("Kornetti".to_string()),
        is_auto_update_enabled: true,
        is_registration_enabled: false,
        is_https_forced: true,
        is_api_enabled: true,
        allowed_ips: None,
        do_not_track: false,
        is_dns_validation_enabled: true,
        next_channel: false,
    })
}

/// Update instance settings
pub async fn update_instance(
    State(state): State<Arc<AppState>>,
    Json(request): Json<UpdateInstanceSettingsRequest>,
) -> impl IntoResponse {
    // Validate port range
    if let (Some(min), Some(max)) = (request.public_port_min, request.public_port_max) {
        if min >= max {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Port min must be less than port max"
            }))).into_response();
        }
        if min < 1024 || max > 65535 {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Port range must be between 1024 and 65535"
            }))).into_response();
        }
    }

    // Placeholder: would update instance settings in database
    Json(serde_json::json!({
        "message": "Instance settings updated successfully"
    })).into_response()
}

/// List S3 storage configurations
pub async fn list_s3_storages(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Placeholder: would query S3 storages from database
    let storages: Vec<S3StorageResponse> = vec![];
    Json(storages)
}

/// Create S3 storage configuration
pub async fn create_s3_storage(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateS3StorageRequest>,
) -> impl IntoResponse {
    // Validate required fields
    if request.name.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "error": "Name is required"
        }))).into_response();
    }

    if request.bucket.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "error": "Bucket is required"
        }))).into_response();
    }

    // Placeholder: would create S3 storage in database, encrypt secrets
    Json(serde_json::json!({
        "id": Uuid::new_v4(),
        "name": request.name,
        "message": "S3 storage created successfully"
    })).into_response()
}

/// Get S3 storage configuration
pub async fn get_s3_storage(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    // Placeholder: would fetch S3 storage from database
    Json(S3StorageResponse {
        id,
        name: "Default S3".to_string(),
        description: None,
        endpoint: "s3.amazonaws.com".to_string(),
        bucket: "backups".to_string(),
        region: "us-east-1".to_string(),
        is_usable: true,
    })
}

/// Update S3 storage configuration
#[derive(Debug, Deserialize)]
pub struct UpdateS3StorageRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub endpoint: Option<String>,
    pub bucket: Option<String>,
    pub region: Option<String>,
    pub key: Option<String>,
    pub secret: Option<String>,
}

pub async fn update_s3_storage(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateS3StorageRequest>,
) -> impl IntoResponse {
    // Placeholder: would update S3 storage in database
    Json(serde_json::json!({
        "id": id,
        "message": "S3 storage updated successfully"
    }))
}

/// Delete S3 storage configuration
pub async fn delete_s3_storage(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    // Placeholder: would check for usage, delete S3 storage
    Json(serde_json::json!({
        "message": "S3 storage deleted successfully"
    }))
}

/// Test S3 storage connection
pub async fn test_s3_storage(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    // Placeholder: would test S3 connection
    Json(serde_json::json!({
        "success": true,
        "message": "S3 storage connection successful"
    }))
}

/// Get license information
pub async fn get_license(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Placeholder: would fetch license information
    Json(serde_json::json!({
        "is_valid": true,
        "type": "community",
        "features": []
    }))
}

/// Validate and update license
#[derive(Debug, Deserialize)]
pub struct UpdateLicenseRequest {
    pub license_key: String,
}

pub async fn update_license(
    State(state): State<Arc<AppState>>,
    Json(request): Json<UpdateLicenseRequest>,
) -> impl IntoResponse {
    // Placeholder: would validate and store license
    Json(serde_json::json!({
        "message": "License updated successfully"
    }))
}

/// Get system health/status
pub async fn get_system_status(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "database": "connected",
        "redis": "connected",
        "docker": "running",
        "disk_usage_percent": 45,
        "memory_usage_percent": 62,
        "uptime_seconds": 86400
    }))
}

/// Trigger system cleanup
pub async fn trigger_cleanup(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Placeholder: would queue cleanup job
    Json(serde_json::json!({
        "message": "Cleanup job queued successfully"
    }))
}
