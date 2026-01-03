//! Authentication handlers

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::{state::AppState, ApiError};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserResponse,
    pub teams: Vec<TeamResponse>,
}

#[derive(Serialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub name: String,
}

#[derive(Serialize)]
pub struct TeamResponse {
    pub id: String,
    pub name: String,
    pub personal: bool,
}

pub async fn login(
    State(_state): State<Arc<AppState>>,
    Json(_body): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    // TODO: Implement actual authentication
    Ok(Json(LoginResponse {
        token: "dummy_token".to_string(),
        user: UserResponse {
            id: "1".to_string(),
            email: "user@example.com".to_string(),
            name: "User".to_string(),
        },
        teams: vec![TeamResponse {
            id: "1".to_string(),
            name: "Personal".to_string(),
            personal: true,
        }],
    }))
}

pub async fn logout() -> Result<Json<()>, ApiError> {
    Ok(Json(()))
}

pub async fn me(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<UserResponse>, ApiError> {
    // TODO: Extract user from token
    Ok(Json(UserResponse {
        id: "1".to_string(),
        email: "user@example.com".to_string(),
        name: "User".to_string(),
    }))
}
