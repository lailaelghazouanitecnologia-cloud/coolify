//! Destinations handlers
//!
//! API handlers for Docker and Swarm destinations (networks).

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

/// Destination type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DestinationType {
    /// Standalone Docker (bridge network)
    StandaloneDocker,
    /// Docker Swarm (overlay network)
    SwarmDocker,
}

/// Destination response
#[derive(Debug, Serialize)]
pub struct DestinationResponse {
    pub id: Uuid,
    pub uuid: String,
    pub name: String,
    pub network: String,
    pub destination_type: DestinationType,
    pub server_id: Uuid,
    pub server_name: String,
    /// Whether this destination has resources attached
    pub has_resources: bool,
    /// Count of applications using this destination
    pub application_count: u32,
    /// Count of databases using this destination
    pub database_count: u32,
    /// Count of services using this destination
    pub service_count: u32,
}

/// Request to create a destination
#[derive(Debug, Deserialize)]
pub struct CreateDestinationRequest {
    /// Destination name
    pub name: String,
    /// Docker network name
    pub network: String,
    /// Server ID
    pub server_id: Uuid,
    /// Destination type (defaults to standalone_docker)
    pub destination_type: Option<DestinationType>,
}

/// Request to update a destination
#[derive(Debug, Deserialize)]
pub struct UpdateDestinationRequest {
    pub name: Option<String>,
    pub network: Option<String>,
}

/// List all destinations
///
/// GET /api/destinations
pub async fn list(
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Would query all destinations from database
    let destinations: Vec<DestinationResponse> = vec![];

    Json(serde_json::json!({
        "destinations": destinations
    }))
}

/// Get destinations for a server
///
/// GET /api/servers/:id/destinations
pub async fn list_for_server(
    State(_state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
) -> impl IntoResponse {
    let destinations: Vec<DestinationResponse> = vec![];

    Json(serde_json::json!({
        "server_id": server_id,
        "destinations": destinations
    }))
}

/// Create a new destination
///
/// POST /api/destinations
pub async fn create(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<CreateDestinationRequest>,
) -> impl IntoResponse {
    // Validate server exists
    // Create Docker network on server if doesn't exist
    // Create destination record

    let destination_type = request.destination_type.unwrap_or(DestinationType::StandaloneDocker);

    let network_cmd = match destination_type {
        DestinationType::StandaloneDocker => {
            format!("docker network create {} --attachable", request.network)
        }
        DestinationType::SwarmDocker => {
            format!("docker network create {} --driver overlay --attachable", request.network)
        }
    };

    // Would execute network creation command via SSH

    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": Uuid::new_v4(),
            "name": request.name,
            "network": request.network,
            "message": "Destination created successfully"
        })),
    )
}

/// Get a destination by ID
///
/// GET /api/destinations/:id
pub async fn get(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    // Would fetch from database
    Json(serde_json::json!({
        "id": id,
        "name": "coolify",
        "network": "coolify",
        "destination_type": "standalone_docker",
        "server_id": Uuid::new_v4(),
        "has_resources": false
    }))
}

/// Update a destination
///
/// PUT /api/destinations/:id
pub async fn update(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateDestinationRequest>,
) -> impl IntoResponse {
    // Would update in database
    // Note: Changing network name requires recreating the network

    Json(serde_json::json!({
        "id": id,
        "message": "Destination updated successfully"
    }))
}

/// Delete a destination
///
/// DELETE /api/destinations/:id
pub async fn delete(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    // Check if destination has resources attached
    // If attached, return error
    // Otherwise, optionally remove Docker network and delete record

    Json(serde_json::json!({
        "message": "Destination deleted successfully"
    }))
}

/// Verify a destination's network exists
///
/// POST /api/destinations/:id/verify
pub async fn verify(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    // Would SSH to server and check:
    // docker network inspect {network_name}

    Json(serde_json::json!({
        "id": id,
        "exists": true,
        "driver": "bridge",
        "scope": "local"
    }))
}

/// Recreate a destination's network
///
/// POST /api/destinations/:id/recreate
pub async fn recreate(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    // Would:
    // 1. docker network rm {network} (if exists)
    // 2. docker network create {network} --attachable

    Json(serde_json::json!({
        "id": id,
        "message": "Network recreated successfully"
    }))
}

// === Swarm-specific endpoints ===

/// Swarm node information
#[derive(Debug, Serialize)]
pub struct SwarmNodeInfo {
    pub id: String,
    pub hostname: String,
    pub status: String,
    pub availability: String,
    pub role: String,
    pub engine_version: String,
    pub addr: String,
}

/// Get Swarm nodes for a destination
///
/// GET /api/destinations/:id/swarm/nodes
pub async fn swarm_nodes(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    // Would SSH to server and run:
    // docker node ls --format '{{json .}}'

    let nodes: Vec<SwarmNodeInfo> = vec![];

    Json(serde_json::json!({
        "destination_id": id,
        "nodes": nodes
    }))
}

/// Swarm service information
#[derive(Debug, Serialize)]
pub struct SwarmServiceInfo {
    pub id: String,
    pub name: String,
    pub mode: String,
    pub replicas: String,
    pub image: String,
    pub ports: String,
}

/// List Swarm services for a destination
///
/// GET /api/destinations/:id/swarm/services
pub async fn swarm_services(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    // Would SSH to server and run:
    // docker service ls --format '{{json .}}'

    let services: Vec<SwarmServiceInfo> = vec![];

    Json(serde_json::json!({
        "destination_id": id,
        "services": services
    }))
}

/// Initialize Swarm on a destination's server
///
/// POST /api/destinations/:id/swarm/init
#[derive(Debug, Deserialize)]
pub struct SwarmInitRequest {
    /// Advertise address (IP:port)
    pub advertise_addr: Option<String>,
    /// Force re-init
    pub force: bool,
}

pub async fn swarm_init(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(request): Json<SwarmInitRequest>,
) -> impl IntoResponse {
    // Would SSH to server and run:
    // docker swarm init --advertise-addr {addr}

    // Return join tokens
    Json(serde_json::json!({
        "destination_id": id,
        "manager_token": "SWMTKN-manager-...",
        "worker_token": "SWMTKN-worker-...",
        "manager_addr": request.advertise_addr.unwrap_or_else(|| "0.0.0.0:2377".to_string()),
        "message": "Swarm initialized successfully"
    }))
}

/// Join token response
#[derive(Debug, Serialize)]
pub struct JoinTokenResponse {
    pub manager_token: String,
    pub worker_token: String,
    pub manager_addr: String,
}

/// Get Swarm join tokens
///
/// GET /api/destinations/:id/swarm/tokens
pub async fn swarm_tokens(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    // Would SSH to server and run:
    // docker swarm join-token manager -q
    // docker swarm join-token worker -q

    Json(JoinTokenResponse {
        manager_token: "SWMTKN-manager-...".to_string(),
        worker_token: "SWMTKN-worker-...".to_string(),
        manager_addr: "0.0.0.0:2377".to_string(),
    })
}

/// Leave Swarm
///
/// POST /api/destinations/:id/swarm/leave
#[derive(Debug, Deserialize)]
pub struct SwarmLeaveRequest {
    pub force: bool,
}

pub async fn swarm_leave(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(request): Json<SwarmLeaveRequest>,
) -> impl IntoResponse {
    // Would SSH to server and run:
    // docker swarm leave [--force]

    Json(serde_json::json!({
        "destination_id": id,
        "message": "Left swarm successfully"
    }))
}
