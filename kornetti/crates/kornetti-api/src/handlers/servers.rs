//! Server handlers

use axum::{
    extract::{Extension, Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn, instrument};
use uuid::Uuid;

use kornetti_core::models::{Server, ServerStatus, ServerSettings, ProxyType};
use kornetti_core::actions::server::{
    ValidateServer, ValidateServerInput, InstallDocker, InstallDockerInput,
    CleanupDocker, CleanupDockerInput, CleanupOptions,
};
use kornetti_core::actions::Action;
use crate::{state::{AppState, AuthContext}, ApiError};

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateServerRequest {
    pub name: String,
    pub description: Option<String>,
    pub ip: String,
    pub port: Option<u16>,
    pub user: Option<String>,
    pub private_key_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct UpdateServerRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub ip: Option<String>,
    pub port: Option<u16>,
    pub user: Option<String>,
    pub settings: Option<UpdateServerSettings>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateServerSettings {
    pub proxy_type: Option<ProxyType>,
    pub wildcard_domain: Option<String>,
    pub concurrent_builds: Option<u32>,
    pub sentinel_enabled: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ServerResourcesResponse {
    pub cpu_usage: f64,
    pub memory_used: u64,
    pub memory_total: u64,
    pub disk_used: u64,
    pub disk_total: u64,
    pub containers_running: u32,
    pub containers_total: u32,
}

#[derive(Debug, Serialize)]
pub struct ValidationResponse {
    pub is_valid: bool,
    pub os_type: String,
    pub docker_installed: bool,
    pub docker_version: Option<String>,
    pub docker_running: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

// ============================================================================
// Handlers
// ============================================================================

/// List all servers for the current team
#[instrument(skip(state, auth))]
pub async fn list(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<Vec<Server>>, ApiError> {
    let repo = state.servers();

    let servers = repo.find_by_team(auth.team_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch servers: {}", e)))?;

    Ok(Json(servers))
}

/// Get a specific server by ID
#[instrument(skip(state, auth))]
pub async fn get(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<Server>, ApiError> {
    let repo = state.servers();

    let server = repo.find_by_id_and_team(id, auth.team_id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found(format!("Server {} not found", id)))?;

    Ok(Json(server))
}

/// Create a new server
#[instrument(skip(state, auth, body))]
pub async fn create(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<CreateServerRequest>,
) -> Result<Json<Server>, ApiError> {
    let repo = state.servers();

    let now = chrono::Utc::now();
    let server = Server {
        id: Uuid::new_v4(),
        team_id: auth.team_id,
        name: body.name,
        description: body.description,
        ip: body.ip,
        port: body.port.unwrap_or(22),
        user: body.user.unwrap_or_else(|| "root".to_string()),
        private_key_id: body.private_key_id,
        status: ServerStatus::New,
        provider: None,
        provider_id: None,
        region: None,
        settings: ServerSettings::default(),
        created_at: now,
        updated_at: now,
    };

    let created = repo.create(&server)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to create server: {}", e)))?;

    info!(server_id = %created.id, "Created new server");

    Ok(Json(created))
}

/// Update an existing server
#[instrument(skip(state, auth, body))]
pub async fn update(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateServerRequest>,
) -> Result<Json<Server>, ApiError> {
    let repo = state.servers();

    // Fetch existing server (also verifies team ownership)
    let mut server = repo.find_by_id_and_team(id, auth.team_id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found(format!("Server {} not found", id)))?;

    // Apply updates
    if let Some(name) = body.name {
        server.name = name;
    }
    if let Some(description) = body.description {
        server.description = Some(description);
    }
    if let Some(ip) = body.ip {
        server.ip = ip;
    }
    if let Some(port) = body.port {
        server.port = port;
    }
    if let Some(user) = body.user {
        server.user = user;
    }

    if let Some(settings) = body.settings {
        if let Some(proxy_type) = settings.proxy_type {
            server.settings.proxy_type = proxy_type;
        }
        if let Some(domain) = settings.wildcard_domain {
            server.settings.wildcard_domain = Some(domain);
        }
        if let Some(builds) = settings.concurrent_builds {
            server.settings.concurrent_builds = builds;
        }
        if let Some(sentinel) = settings.sentinel_enabled {
            server.settings.sentinel_enabled = sentinel;
        }
    }

    let updated = repo.update(&server)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to update server: {}", e)))?;

    info!(server_id = %id, "Updated server");

    Ok(Json(updated))
}

/// Delete a server
#[instrument(skip(state, auth))]
pub async fn delete(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<()>, ApiError> {
    let repo = state.servers();

    // Verify ownership
    let _ = repo.find_by_id_and_team(id, auth.team_id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found(format!("Server {} not found", id)))?;

    repo.delete(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to delete server: {}", e)))?;

    info!(server_id = %id, "Deleted server");

    Ok(Json(()))
}

/// Validate server connection and check Docker
#[instrument(skip(state, auth))]
pub async fn validate(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<ValidationResponse>, ApiError> {
    let repo = state.servers();

    let server = repo.find_by_id_and_team(id, auth.team_id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found(format!("Server {} not found", id)))?;

    // Update status to validating
    repo.update_status(id, ServerStatus::Validating)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to update status: {}", e)))?;

    // Run validation
    let validator = ValidateServer::new(state.ssh.clone());
    let result = validator.handle(ValidateServerInput { server: &server })
        .await;

    match result {
        Ok(validation) => {
            let new_status = if validation.is_valid {
                if validation.docker_running {
                    ServerStatus::Ready
                } else {
                    ServerStatus::Reachable
                }
            } else {
                ServerStatus::Unreachable
            };

            repo.update_status(id, new_status)
                .await
                .map_err(|e| ApiError::internal(format!("Failed to update status: {}", e)))?;

            Ok(Json(ValidationResponse {
                is_valid: validation.is_valid,
                os_type: format!("{:?}", validation.os_type),
                docker_installed: validation.docker_installed,
                docker_version: validation.docker_version,
                docker_running: validation.docker_running,
                errors: validation.errors,
                warnings: validation.warnings,
            }))
        }
        Err(e) => {
            warn!(server_id = %id, error = %e, "Server validation failed");
            repo.update_status(id, ServerStatus::Unreachable).await.ok();
            Err(ApiError::internal(format!("Validation failed: {}", e)))
        }
    }
}

/// Install Docker on a server
#[instrument(skip(state, auth))]
pub async fn install_docker(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<()>, ApiError> {
    let repo = state.servers();

    let server = repo.find_by_id_and_team(id, auth.team_id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found(format!("Server {} not found", id)))?;

    repo.update_status(id, ServerStatus::Installing)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to update status: {}", e)))?;

    let installer = InstallDocker::new(state.ssh.clone());
    let result = installer.handle(InstallDockerInput { server: &server, force: false }).await;

    match result {
        Ok(action_result) => {
            info!(server_id = %id, duration_ms = action_result.duration_ms, "Docker installed");
            repo.update_status(id, ServerStatus::Ready).await.ok();
            Ok(Json(()))
        }
        Err(e) => {
            warn!(server_id = %id, error = %e, "Docker installation failed");
            repo.update_status(id, ServerStatus::Error).await.ok();
            Err(ApiError::internal(format!("Docker installation failed: {}", e)))
        }
    }
}

/// Get server resources (CPU, memory, disk)
#[instrument(skip(state, auth))]
pub async fn resources(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<ServerResourcesResponse>, ApiError> {
    let repo = state.servers();

    let server = repo.find_by_id_and_team(id, auth.team_id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found(format!("Server {} not found", id)))?;

    let session = state.ssh.connect(&server)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to connect: {}", e)))?;

    // Get resource info via SSH
    let cpu_output = session.execute("top -bn1 | grep 'Cpu(s)' | awk '{print $2}'").await.ok();
    let cpu_usage: f64 = cpu_output.and_then(|o| o.stdout.trim().parse().ok()).unwrap_or(0.0);

    let mem_output = session.execute("free -b | grep Mem | awk '{print $2,$3}'").await.ok();
    let (memory_total, memory_used) = mem_output.map(|o| {
        let parts: Vec<&str> = o.stdout.trim().split_whitespace().collect();
        (
            parts.first().and_then(|s| s.parse().ok()).unwrap_or(0),
            parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0),
        )
    }).unwrap_or((0, 0));

    let disk_output = session.execute("df -B1 / | tail -1 | awk '{print $2,$3}'").await.ok();
    let (disk_total, disk_used) = disk_output.map(|o| {
        let parts: Vec<&str> = o.stdout.trim().split_whitespace().collect();
        (
            parts.first().and_then(|s| s.parse().ok()).unwrap_or(0),
            parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0),
        )
    }).unwrap_or((0, 0));

    let container_output = session.execute("docker ps -q 2>/dev/null | wc -l").await.ok();
    let containers_running: u32 = container_output.and_then(|o| o.stdout.trim().parse().ok()).unwrap_or(0);

    let container_total_output = session.execute("docker ps -aq 2>/dev/null | wc -l").await.ok();
    let containers_total: u32 = container_total_output.and_then(|o| o.stdout.trim().parse().ok()).unwrap_or(0);

    Ok(Json(ServerResourcesResponse {
        cpu_usage,
        memory_used,
        memory_total,
        disk_used,
        disk_total,
        containers_running,
        containers_total,
    }))
}

/// Cleanup Docker resources
#[instrument(skip(state, auth))]
pub async fn cleanup(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let repo = state.servers();

    let server = repo.find_by_id_and_team(id, auth.team_id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found(format!("Server {} not found", id)))?;

    let cleaner = CleanupDocker::new(state.ssh.clone());
    let stats = cleaner.handle(CleanupDockerInput { server: &server, options: CleanupOptions::safe() })
        .await
        .map_err(|e| ApiError::internal(format!("Cleanup failed: {}", e)))?;

    info!(server_id = %id, space_mb = stats.space_reclaimed_mb, "Docker cleanup completed");

    Ok(Json(serde_json::json!({
        "containers_removed": stats.containers_removed,
        "images_removed": stats.images_removed,
        "volumes_removed": stats.volumes_removed,
        "networks_removed": stats.networks_removed,
        "space_reclaimed_mb": stats.space_reclaimed_mb,
    })))
}
