//! Database handlers

use axum::{extract::{Path, State}, Json};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;
use kornetti_core::models::{StandaloneDatabase, DatabaseType};
use crate::{state::AppState, ApiError};

#[derive(Deserialize)]
pub struct CreateDatabaseRequest {
    pub name: String,
    pub environment_id: Uuid,
    pub server_id: Uuid,
    pub database_type: DatabaseType,
    pub version: String,
}

pub async fn list(State(_state): State<Arc<AppState>>) -> Result<Json<Vec<StandaloneDatabase>>, ApiError> {
    Ok(Json(vec![]))
}

pub async fn get(State(_state): State<Arc<AppState>>, Path(_id): Path<Uuid>) -> Result<Json<StandaloneDatabase>, ApiError> {
    Err(ApiError { status: axum::http::StatusCode::NOT_FOUND, message: "Database not found".to_string(), code: None })
}

pub async fn create(State(_state): State<Arc<AppState>>, Json(_body): Json<CreateDatabaseRequest>) -> Result<Json<StandaloneDatabase>, ApiError> {
    Err(ApiError { status: axum::http::StatusCode::NOT_IMPLEMENTED, message: "Not implemented".to_string(), code: None })
}

pub async fn delete(State(_state): State<Arc<AppState>>, Path(_id): Path<Uuid>) -> Result<Json<()>, ApiError> {
    Ok(Json(()))
}

pub async fn start(State(_state): State<Arc<AppState>>, Path(_id): Path<Uuid>) -> Result<Json<()>, ApiError> {
    Ok(Json(()))
}

pub async fn stop(State(_state): State<Arc<AppState>>, Path(_id): Path<Uuid>) -> Result<Json<()>, ApiError> {
    Ok(Json(()))
}
