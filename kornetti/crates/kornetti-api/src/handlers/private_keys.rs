//! Private key API handlers

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

/// Private key response (never includes the actual key)
#[derive(Debug, Serialize)]
pub struct PrivateKeyResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub team_id: Uuid,
    /// Public key fingerprint
    pub fingerprint: Option<String>,
    /// Whether the key is encrypted with a passphrase
    pub is_encrypted: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Create private key request
#[derive(Debug, Deserialize)]
pub struct CreatePrivateKeyRequest {
    pub name: String,
    pub description: Option<String>,
    /// The private key content (will be encrypted at rest)
    pub private_key: String,
    /// Optional passphrase for encrypted keys
    pub passphrase: Option<String>,
}

/// Update private key request
#[derive(Debug, Deserialize)]
pub struct UpdatePrivateKeyRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    /// New private key (optional, to replace existing)
    pub private_key: Option<String>,
    pub passphrase: Option<String>,
}

/// Generate private key request
#[derive(Debug, Deserialize)]
pub struct GeneratePrivateKeyRequest {
    pub name: String,
    pub description: Option<String>,
    /// Key type: "ed25519" (default) or "rsa"
    pub key_type: Option<String>,
    /// RSA key size (2048, 4096)
    pub key_size: Option<u32>,
}

/// Generate key response (includes public key for display)
#[derive(Debug, Serialize)]
pub struct GenerateKeyResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub team_id: Uuid,
    /// The public key to add to authorized_keys
    pub public_key: String,
    pub fingerprint: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// List all private keys for the team
pub async fn list(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<PrivateKeyResponse>>, ApiError> {
    // TODO: Implement listing from database
    Ok(Json(vec![]))
}

/// Get a private key by ID (metadata only)
pub async fn get(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<PrivateKeyResponse>, ApiError> {
    Err(ApiError::not_found(format!("Private key {} not found", id)))
}

/// Create a new private key
pub async fn create(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreatePrivateKeyRequest>,
) -> Result<Json<PrivateKeyResponse>, ApiError> {
    // Validate the private key format
    if !req.private_key.contains("PRIVATE KEY") {
        return Err(ApiError::validation("Invalid private key format"));
    }

    let now = chrono::Utc::now();

    // TODO: Calculate fingerprint from key
    let fingerprint = Some("SHA256:...".to_string());

    let response = PrivateKeyResponse {
        id: Uuid::new_v4(),
        name: req.name,
        description: req.description,
        team_id: Uuid::new_v4(), // TODO: Get from auth context
        fingerprint,
        is_encrypted: req.passphrase.is_some(),
        created_at: now,
        updated_at: now,
    };

    // TODO: Encrypt and persist to database
    Ok(Json(response))
}

/// Generate a new SSH key pair
pub async fn generate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GeneratePrivateKeyRequest>,
) -> Result<Json<GenerateKeyResponse>, ApiError> {
    let key_type = req.key_type.as_deref().unwrap_or("ed25519");

    // TODO: Actually generate the key using ssh-keygen or a Rust library
    // For now, return a placeholder response
    let now = chrono::Utc::now();

    let response = GenerateKeyResponse {
        id: Uuid::new_v4(),
        name: req.name,
        description: req.description,
        team_id: Uuid::new_v4(), // TODO: Get from auth context
        public_key: format!("ssh-{} AAAA... generated-key@kornetti", key_type),
        fingerprint: "SHA256:placeholder".to_string(),
        created_at: now,
    };

    // TODO: Generate key, encrypt private key, and persist
    Ok(Json(response))
}

/// Update a private key
pub async fn update(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdatePrivateKeyRequest>,
) -> Result<Json<PrivateKeyResponse>, ApiError> {
    Err(ApiError::not_found(format!("Private key {} not found", id)))
}

/// Delete a private key
pub async fn delete(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // TODO: Check if key is in use by any server or source
    // TODO: Delete from database

    Ok(Json(serde_json::json!({
        "message": "Private key deleted successfully"
    })))
}

/// Get the public key for a private key
pub async fn get_public_key(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // TODO: Retrieve and return the public key
    Err(ApiError::not_found(format!("Private key {} not found", id)))
}
