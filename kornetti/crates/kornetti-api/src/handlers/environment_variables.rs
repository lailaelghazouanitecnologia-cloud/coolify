//! Environment variable API handlers

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};
use kornetti_core::models::environment_variable::ResourceType;

/// Environment variable response
#[derive(Debug, Serialize)]
pub struct EnvVarResponse {
    pub id: Uuid,
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    pub is_secret: bool,
    pub is_build_time: bool,
    pub resource_type: String,
    pub resource_id: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Create environment variable request
#[derive(Debug, Deserialize)]
pub struct CreateEnvVarRequest {
    pub key: String,
    pub value: String,
    pub is_secret: Option<bool>,
    pub is_build_time: Option<bool>,
    pub resource_type: String,
    pub resource_id: Uuid,
}

/// Update environment variable request
#[derive(Debug, Deserialize)]
pub struct UpdateEnvVarRequest {
    pub key: Option<String>,
    pub value: Option<String>,
    pub is_secret: Option<bool>,
    pub is_build_time: Option<bool>,
}

/// Bulk create environment variables request
#[derive(Debug, Deserialize)]
pub struct BulkCreateEnvVarsRequest {
    pub resource_type: String,
    pub resource_id: Uuid,
    pub variables: Vec<EnvVarItem>,
}

#[derive(Debug, Deserialize)]
pub struct EnvVarItem {
    pub key: String,
    pub value: String,
    pub is_secret: Option<bool>,
    pub is_build_time: Option<bool>,
}

/// Query parameters for listing env vars
#[derive(Debug, Deserialize)]
pub struct ListEnvVarsQuery {
    pub resource_type: Option<String>,
    pub resource_id: Option<Uuid>,
}

/// List environment variables
pub async fn list(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListEnvVarsQuery>,
) -> Result<Json<Vec<EnvVarResponse>>, ApiError> {
    // TODO: Implement listing from database filtered by resource
    Ok(Json(vec![]))
}

/// Get an environment variable by ID
pub async fn get(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<EnvVarResponse>, ApiError> {
    Err(ApiError::not_found(format!("Environment variable {} not found", id)))
}

/// Create a new environment variable
pub async fn create(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateEnvVarRequest>,
) -> Result<Json<EnvVarResponse>, ApiError> {
    let now = chrono::Utc::now();

    let response = EnvVarResponse {
        id: Uuid::new_v4(),
        key: req.key,
        value: if req.is_secret.unwrap_or(false) { None } else { Some(req.value) },
        is_secret: req.is_secret.unwrap_or(false),
        is_build_time: req.is_build_time.unwrap_or(false),
        resource_type: req.resource_type,
        resource_id: req.resource_id,
        created_at: now,
        updated_at: now,
    };

    // TODO: Persist to database
    Ok(Json(response))
}

/// Bulk create environment variables
pub async fn bulk_create(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BulkCreateEnvVarsRequest>,
) -> Result<Json<Vec<EnvVarResponse>>, ApiError> {
    let now = chrono::Utc::now();

    let responses: Vec<EnvVarResponse> = req.variables.into_iter().map(|var| {
        EnvVarResponse {
            id: Uuid::new_v4(),
            key: var.key,
            value: if var.is_secret.unwrap_or(false) { None } else { Some(var.value) },
            is_secret: var.is_secret.unwrap_or(false),
            is_build_time: var.is_build_time.unwrap_or(false),
            resource_type: req.resource_type.clone(),
            resource_id: req.resource_id,
            created_at: now,
            updated_at: now,
        }
    }).collect();

    // TODO: Persist to database
    Ok(Json(responses))
}

/// Update an environment variable
pub async fn update(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateEnvVarRequest>,
) -> Result<Json<EnvVarResponse>, ApiError> {
    Err(ApiError::not_found(format!("Environment variable {} not found", id)))
}

/// Delete an environment variable
pub async fn delete(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // TODO: Delete from database
    Ok(Json(serde_json::json!({
        "message": "Environment variable deleted successfully"
    })))
}

/// Delete all environment variables for a resource
pub async fn delete_for_resource(
    State(state): State<Arc<AppState>>,
    Path((resource_type, resource_id)): Path<(String, Uuid)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // TODO: Delete from database
    Ok(Json(serde_json::json!({
        "message": "Environment variables deleted successfully",
        "resource_type": resource_type,
        "resource_id": resource_id
    })))
}
