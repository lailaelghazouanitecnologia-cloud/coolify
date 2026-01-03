//! Deployment handlers

use axum::{extract::{Path, State}, Json};
use std::sync::Arc;
use uuid::Uuid;
use kornetti_core::models::{Deployment, DeploymentLog};
use crate::{state::AppState, ApiError};

pub async fn list(State(_state): State<Arc<AppState>>) -> Result<Json<Vec<Deployment>>, ApiError> {
    Ok(Json(vec![]))
}

pub async fn get(State(_state): State<Arc<AppState>>, Path(_id): Path<Uuid>) -> Result<Json<Deployment>, ApiError> {
    Err(ApiError { status: axum::http::StatusCode::NOT_FOUND, message: "Deployment not found".to_string(), code: None })
}

pub async fn logs(State(_state): State<Arc<AppState>>, Path(_id): Path<Uuid>) -> Result<Json<Vec<DeploymentLog>>, ApiError> {
    Ok(Json(vec![]))
}

pub async fn cancel(State(_state): State<Arc<AppState>>, Path(_id): Path<Uuid>) -> Result<Json<()>, ApiError> {
    Ok(Json(()))
}
