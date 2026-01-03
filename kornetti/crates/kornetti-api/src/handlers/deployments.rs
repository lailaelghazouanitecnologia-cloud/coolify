//! Deployment handlers

use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use kornetti_core::models::{Deployment, DeploymentLog, DeploymentStatus};
use crate::{state::{AppState, AuthContext}, ApiError};

#[derive(Deserialize)]
pub struct ListDeploymentsQuery {
    /// Filter by application UUID
    pub application_id: Option<Uuid>,
    /// Filter by status
    pub status: Option<String>,
    /// Limit results (default 50)
    pub limit: Option<i64>,
    /// Offset for pagination
    pub offset: Option<i64>,
}

#[derive(Serialize)]
pub struct DeploymentWithLogs {
    #[serde(flatten)]
    pub deployment: Deployment,
    pub logs: Vec<DeploymentLog>,
}

#[derive(Serialize)]
pub struct DeploymentStats {
    pub total: i64,
    pub successful: i64,
    pub failed: i64,
    pub in_progress: i64,
    pub avg_duration_seconds: Option<f64>,
}

/// List deployments for the current team
pub async fn list(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Query(query): Query<ListDeploymentsQuery>,
) -> Result<Json<Vec<Deployment>>, ApiError> {
    let repo = state.deployments();

    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    let deployments = if let Some(app_id) = query.application_id {
        // Verify application belongs to team
        let app = state.applications().find_by_uuid(app_id)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to fetch application: {}", e)))?
            .ok_or_else(|| ApiError::not_found("Application not found"))?;

        // Get project to verify team ownership
        let project = state.projects().find_by_id_raw(app.project_id)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to fetch project: {}", e)))?
            .ok_or_else(|| ApiError::not_found("Project not found"))?;

        if project.team_id != auth.team_id && !auth.is_admin {
            return Err(ApiError::forbidden("Application belongs to another team"));
        }

        repo.find_by_application(app.id as i64, limit, offset)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to fetch deployments: {}", e)))?
    } else {
        repo.find_by_team(auth.team_id, limit, offset)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to fetch deployments: {}", e)))?
    };

    // Filter by status if specified
    let deployments = if let Some(ref status) = query.status {
        deployments.into_iter()
            .filter(|d| d.status.as_str() == status)
            .collect()
    } else {
        deployments
    };

    Ok(Json(deployments))
}

/// Get a specific deployment
pub async fn get(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<Deployment>, ApiError> {
    let repo = state.deployments();

    let deployment = repo.find_by_uuid(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch deployment: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Deployment not found"))?;

    // Verify team ownership via application -> project -> team
    let app = state.applications().find_by_id_raw(deployment.application_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch application: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Application not found"))?;

    let project = state.projects().find_by_id_raw(app.project_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch project: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    if project.team_id != auth.team_id && !auth.is_admin {
        return Err(ApiError::forbidden("Deployment belongs to another team"));
    }

    Ok(Json(deployment))
}

/// Get deployment logs
pub async fn logs(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<DeploymentLog>>, ApiError> {
    let repo = state.deployments();

    let deployment = repo.find_by_uuid(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch deployment: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Deployment not found"))?;

    // Verify team ownership
    let app = state.applications().find_by_id_raw(deployment.application_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch application: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Application not found"))?;

    let project = state.projects().find_by_id_raw(app.project_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch project: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    if project.team_id != auth.team_id && !auth.is_admin {
        return Err(ApiError::forbidden("Deployment belongs to another team"));
    }

    let logs = repo.get_logs(deployment.id as i64)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch logs: {}", e)))?;

    Ok(Json(logs))
}

/// Cancel a running deployment
pub async fn cancel(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<Deployment>, ApiError> {
    let repo = state.deployments();

    let mut deployment = repo.find_by_uuid(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch deployment: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Deployment not found"))?;

    // Verify team ownership
    let app = state.applications().find_by_id_raw(deployment.application_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch application: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Application not found"))?;

    let project = state.projects().find_by_id_raw(app.project_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch project: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    if project.team_id != auth.team_id && !auth.is_admin {
        return Err(ApiError::forbidden("Deployment belongs to another team"));
    }

    // Can only cancel in-progress deployments
    match deployment.status {
        DeploymentStatus::Queued | DeploymentStatus::InProgress => {
            deployment.status = DeploymentStatus::Cancelled;
            deployment.finished_at = Some(chrono::Utc::now());

            let updated = repo.update(deployment)
                .await
                .map_err(|e| ApiError::internal(format!("Failed to cancel deployment: {}", e)))?;

            // Add cancellation log
            let log = DeploymentLog {
                id: 0,
                deployment_id: updated.id,
                output: format!("Deployment cancelled by user {}", auth.email),
                level: "info".to_string(),
                order: 9999,
                hidden: false,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };

            repo.add_log(log)
                .await
                .map_err(|e| ApiError::internal(format!("Failed to add log: {}", e)))?;

            Ok(Json(updated))
        }
        _ => Err(ApiError::bad_request("Deployment is not in progress")),
    }
}

/// Get deployment statistics for an application
pub async fn stats(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(app_id): Path<Uuid>,
) -> Result<Json<DeploymentStats>, ApiError> {
    let repo = state.deployments();

    // Verify application belongs to team
    let app = state.applications().find_by_uuid(app_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch application: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Application not found"))?;

    let project = state.projects().find_by_id_raw(app.project_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch project: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    if project.team_id != auth.team_id && !auth.is_admin {
        return Err(ApiError::forbidden("Application belongs to another team"));
    }

    let stats = repo.get_stats(app.id as i64)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch stats: {}", e)))?;

    Ok(Json(stats))
}

/// Retry a failed deployment
pub async fn retry(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<Deployment>, ApiError> {
    let repo = state.deployments();

    let original = repo.find_by_uuid(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch deployment: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Deployment not found"))?;

    // Verify team ownership
    let app = state.applications().find_by_id_raw(original.application_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch application: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Application not found"))?;

    let project = state.projects().find_by_id_raw(app.project_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch project: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    if project.team_id != auth.team_id && !auth.is_admin {
        return Err(ApiError::forbidden("Deployment belongs to another team"));
    }

    // Can only retry failed deployments
    if !matches!(original.status, DeploymentStatus::Failed | DeploymentStatus::Cancelled) {
        return Err(ApiError::bad_request("Can only retry failed or cancelled deployments"));
    }

    // Create new deployment with same parameters
    let new_deployment = Deployment {
        id: 0,
        uuid: Uuid::new_v4(),
        application_id: original.application_id,
        status: DeploymentStatus::Queued,
        commit: original.commit.clone(),
        commit_message: original.commit_message.clone(),
        branch: original.branch.clone(),
        pull_request_id: original.pull_request_id,
        force_rebuild: original.force_rebuild,
        rollback: false,
        only_this_server: original.only_this_server,
        server_id: original.server_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        started_at: None,
        finished_at: None,
    };

    let created = repo.create(new_deployment)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to create deployment: {}", e)))?;

    // Add initial log
    let log = DeploymentLog {
        id: 0,
        deployment_id: created.id,
        output: format!("Deployment queued (retry of {})", original.uuid),
        level: "info".to_string(),
        order: 0,
        hidden: false,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    repo.add_log(log)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to add log: {}", e)))?;

    // TODO: Trigger deployment job via queue

    Ok(Json(created))
}

/// Get the current/latest deployment for an application
pub async fn current(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(app_id): Path<Uuid>,
) -> Result<Json<Option<Deployment>>, ApiError> {
    let repo = state.deployments();

    // Verify application belongs to team
    let app = state.applications().find_by_uuid(app_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch application: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Application not found"))?;

    let project = state.projects().find_by_id_raw(app.project_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch project: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    if project.team_id != auth.team_id && !auth.is_admin {
        return Err(ApiError::forbidden("Application belongs to another team"));
    }

    let latest = repo.find_latest_by_application(app.id as i64)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch deployment: {}", e)))?;

    Ok(Json(latest))
}
