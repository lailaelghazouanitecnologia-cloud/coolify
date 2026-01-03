//! Server handlers

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use kornetti_core::models::{Server, ServerStatus, ServerSettings, ProxyType};
use crate::{state::AppState, ApiError};

#[derive(Deserialize)]
pub struct CreateServerRequest {
    pub name: String,
    pub description: Option<String>,
    pub ip: String,
    pub port: u16,
    pub user: String,
    pub private_key_id: Uuid,
}

#[derive(Deserialize)]
pub struct UpdateServerRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub settings: Option<UpdateServerSettings>,
}

#[derive(Deserialize)]
pub struct UpdateServerSettings {
    pub proxy_type: Option<ProxyType>,
    pub wildcard_domain: Option<String>,
    pub concurrent_builds: Option<u32>,
    pub sentinel_enabled: Option<bool>,
}

#[derive(Serialize)]
pub struct ServerResourcesResponse {
    pub cpu_usage: f64,
    pub memory_used: u64,
    pub memory_total: u64,
    pub disk_used: u64,
    pub disk_total: u64,
    pub containers_running: u32,
    pub containers_total: u32,
}

pub async fn list(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<Vec<Server>>, ApiError> {
    // TODO: Fetch from database
    Ok(Json(vec![]))
}

pub async fn get(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Server>, ApiError> {
    // TODO: Fetch from database
    Err(ApiError {
        status: axum::http::StatusCode::NOT_FOUND,
        message: format!("Server {} not found", id),
        code: Some("NOT_FOUND".to_string()),
    })
}

pub async fn create(
    State(_state): State<Arc<AppState>>,
    Json(body): Json<CreateServerRequest>,
) -> Result<Json<Server>, ApiError> {
    // TODO: Save to database
    let now = chrono::Utc::now();
    Ok(Json(Server {
        id: Uuid::new_v4(),
        team_id: Uuid::new_v4(),
        name: body.name,
        description: body.description,
        ip: body.ip,
        port: body.port,
        user: body.user,
        private_key_id: body.private_key_id,
        status: ServerStatus::New,
        provider: None,
        provider_id: None,
        region: None,
        settings: ServerSettings::default(),
        created_at: now,
        updated_at: now,
    }))
}

pub async fn update(
    State(_state): State<Arc<AppState>>,
    Path(_id): Path<Uuid>,
    Json(_body): Json<UpdateServerRequest>,
) -> Result<Json<Server>, ApiError> {
    // TODO: Update in database
    Err(ApiError {
        status: axum::http::StatusCode::NOT_IMPLEMENTED,
        message: "Not implemented".to_string(),
        code: None,
    })
}

pub async fn delete(
    State(_state): State<Arc<AppState>>,
    Path(_id): Path<Uuid>,
) -> Result<Json<()>, ApiError> {
    // TODO: Delete from database
    Ok(Json(()))
}

pub async fn validate(
    State(_state): State<Arc<AppState>>,
    Path(_id): Path<Uuid>,
) -> Result<Json<()>, ApiError> {
    // TODO: Validate server connection
    Ok(Json(()))
}

pub async fn install_docker(
    State(_state): State<Arc<AppState>>,
    Path(_id): Path<Uuid>,
) -> Result<Json<()>, ApiError> {
    // TODO: Install Docker on server
    Ok(Json(()))
}

pub async fn resources(
    State(_state): State<Arc<AppState>>,
    Path(_id): Path<Uuid>,
) -> Result<Json<ServerResourcesResponse>, ApiError> {
    // TODO: Fetch resources from server
    Ok(Json(ServerResourcesResponse {
        cpu_usage: 0.0,
        memory_used: 0,
        memory_total: 0,
        disk_used: 0,
        disk_total: 0,
        containers_running: 0,
        containers_total: 0,
    }))
}
