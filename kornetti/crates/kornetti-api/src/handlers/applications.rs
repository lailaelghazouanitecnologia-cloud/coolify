//! Application handlers

use axum::{extract::{Path, State}, Json};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;
use kornetti_core::models::Application;
use crate::{state::AppState, ApiError};

#[derive(Deserialize)]
pub struct CreateApplicationRequest {
    pub name: String,
    pub environment_id: Uuid,
    pub server_id: Uuid,
}

pub async fn list(State(_state): State<Arc<AppState>>) -> Result<Json<Vec<Application>>, ApiError> {
    Ok(Json(vec![]))
}

pub async fn get(State(_state): State<Arc<AppState>>, Path(_id): Path<Uuid>) -> Result<Json<Application>, ApiError> {
    Err(ApiError { status: axum::http::StatusCode::NOT_FOUND, message: "Application not found".to_string(), code: None })
}

pub async fn create(State(_state): State<Arc<AppState>>, Json(_body): Json<CreateApplicationRequest>) -> Result<Json<Application>, ApiError> {
    Err(ApiError { status: axum::http::StatusCode::NOT_IMPLEMENTED, message: "Not implemented".to_string(), code: None })
}

pub async fn update(State(_state): State<Arc<AppState>>, Path(_id): Path<Uuid>, Json(_body): Json<CreateApplicationRequest>) -> Result<Json<Application>, ApiError> {
    Err(ApiError { status: axum::http::StatusCode::NOT_IMPLEMENTED, message: "Not implemented".to_string(), code: None })
}

pub async fn delete(State(_state): State<Arc<AppState>>, Path(_id): Path<Uuid>) -> Result<Json<()>, ApiError> {
    Ok(Json(()))
}

pub async fn deploy(State(_state): State<Arc<AppState>>, Path(_id): Path<Uuid>) -> Result<Json<()>, ApiError> {
    Ok(Json(()))
}

pub async fn stop(State(_state): State<Arc<AppState>>, Path(_id): Path<Uuid>) -> Result<Json<()>, ApiError> {
    Ok(Json(()))
}

pub async fn restart(State(_state): State<Arc<AppState>>, Path(_id): Path<Uuid>) -> Result<Json<()>, ApiError> {
    Ok(Json(()))
}
