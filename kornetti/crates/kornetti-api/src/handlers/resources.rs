//! Resources handlers
//!
//! API handlers for listing all resources across projects.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::state::AppState;

/// Resource type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    Application,
    Service,
    Database,
    PostgreSql,
    MySql,
    MariaDb,
    MongoDB,
    Redis,
    KeyDb,
    Dragonfly,
    Clickhouse,
}

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceType::Application => write!(f, "application"),
            ResourceType::Service => write!(f, "service"),
            ResourceType::Database => write!(f, "database"),
            ResourceType::PostgreSql => write!(f, "postgresql"),
            ResourceType::MySql => write!(f, "mysql"),
            ResourceType::MariaDb => write!(f, "mariadb"),
            ResourceType::MongoDB => write!(f, "mongodb"),
            ResourceType::Redis => write!(f, "redis"),
            ResourceType::KeyDb => write!(f, "keydb"),
            ResourceType::Dragonfly => write!(f, "dragonfly"),
            ResourceType::Clickhouse => write!(f, "clickhouse"),
        }
    }
}

/// Resource summary for listing
#[derive(Debug, Serialize)]
pub struct ResourceSummary {
    pub id: Uuid,
    pub uuid: String,
    pub name: String,
    pub description: Option<String>,
    pub resource_type: ResourceType,
    pub status: String,
    pub project_id: Uuid,
    pub project_name: String,
    pub environment_id: Uuid,
    pub environment_name: String,
    pub server_id: Option<Uuid>,
    pub server_name: Option<String>,
    pub fqdn: Option<String>,
}

/// Query parameters for resource listing
#[derive(Debug, Deserialize)]
pub struct ResourceQuery {
    /// Filter by resource type
    pub resource_type: Option<String>,
    /// Filter by project ID
    pub project_id: Option<Uuid>,
    /// Filter by environment ID
    pub environment_id: Option<Uuid>,
    /// Filter by server ID
    pub server_id: Option<Uuid>,
    /// Filter by status
    pub status: Option<String>,
    /// Search by name
    pub search: Option<String>,
}

/// List all resources
///
/// GET /api/resources
pub async fn list(
    State(_state): State<Arc<AppState>>,
    Query(query): Query<ResourceQuery>,
) -> impl IntoResponse {
    // Would query all resources from database:
    // - Applications
    // - Services
    // - All database types (PostgreSQL, MySQL, etc.)

    // Apply filters from query parameters

    let resources: Vec<ResourceSummary> = vec![
        // Placeholder
    ];

    Json(serde_json::json!({
        "resources": resources,
        "total": 0
    }))
}

/// Get resource by UUID (type-agnostic)
///
/// GET /api/resources/:uuid
pub async fn get_by_uuid(
    State(_state): State<Arc<AppState>>,
    Path(uuid): Path<String>,
) -> impl IntoResponse {
    // Would search across all resource types for the UUID

    // Not found for now
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({
            "error": "Resource not found"
        })),
    )
        .into_response()
}

/// Get resources for a specific project
///
/// GET /api/projects/:id/resources
pub async fn list_for_project(
    State(_state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
) -> impl IntoResponse {
    let resources: Vec<ResourceSummary> = vec![];

    Json(serde_json::json!({
        "project_id": project_id,
        "resources": resources,
        "total": 0
    }))
}

/// Get resources for a specific server
///
/// GET /api/servers/:id/resources
pub async fn list_for_server(
    State(_state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
) -> impl IntoResponse {
    let resources: Vec<ResourceSummary> = vec![];

    Json(serde_json::json!({
        "server_id": server_id,
        "resources": resources,
        "total": 0
    }))
}

/// Resource statistics
#[derive(Debug, Serialize)]
pub struct ResourceStats {
    pub total_applications: u32,
    pub total_services: u32,
    pub total_databases: u32,
    pub running: u32,
    pub stopped: u32,
    pub error: u32,
    pub by_type: std::collections::HashMap<String, u32>,
}

/// Get resource statistics
///
/// GET /api/resources/stats
pub async fn stats(
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    Json(ResourceStats {
        total_applications: 0,
        total_services: 0,
        total_databases: 0,
        running: 0,
        stopped: 0,
        error: 0,
        by_type: std::collections::HashMap::new(),
    })
}
