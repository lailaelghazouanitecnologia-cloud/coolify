//! Project handlers

use axum::{extract::{Path, State}, Json};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;
use kornetti_core::models::Project;
use crate::{state::AppState, ApiError};

#[derive(Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
}

pub async fn list(State(_state): State<Arc<AppState>>) -> Result<Json<Vec<Project>>, ApiError> {
    Ok(Json(vec![]))
}

pub async fn get(State(_state): State<Arc<AppState>>, Path(_id): Path<Uuid>) -> Result<Json<Project>, ApiError> {
    Err(ApiError { status: axum::http::StatusCode::NOT_FOUND, message: "Project not found".to_string(), code: None })
}

pub async fn create(State(_state): State<Arc<AppState>>, Json(_body): Json<CreateProjectRequest>) -> Result<Json<Project>, ApiError> {
    Err(ApiError { status: axum::http::StatusCode::NOT_IMPLEMENTED, message: "Not implemented".to_string(), code: None })
}

pub async fn update(State(_state): State<Arc<AppState>>, Path(_id): Path<Uuid>, Json(_body): Json<CreateProjectRequest>) -> Result<Json<Project>, ApiError> {
    Err(ApiError { status: axum::http::StatusCode::NOT_IMPLEMENTED, message: "Not implemented".to_string(), code: None })
}

pub async fn delete(State(_state): State<Arc<AppState>>, Path(_id): Path<Uuid>) -> Result<Json<()>, ApiError> {
    Ok(Json(()))
}
