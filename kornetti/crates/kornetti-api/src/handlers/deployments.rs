//! Deployment handlers

use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use kornetti_core::models::{Deployment, DeploymentStatus, DeploymentType, DeploymentLog, LogLevel};
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
        let app = state.applications().find_by_id(app_id)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to fetch application: {}", e)))?
            .ok_or_else(|| ApiError::not_found("Application not found"))?;

        // Get project to verify team ownership
        let env = state.projects().find_environment_by_id(app.environment_id)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to fetch environment: {}", e)))?
            .ok_or_else(|| ApiError::not_found("Environment not found"))?;

        let project = state.projects().find_by_id(env.project_id)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to fetch project: {}", e)))?
            .ok_or_else(|| ApiError::not_found("Project not found"))?;

        if project.team_id != auth.team_id && !auth.is_admin {
            return Err(ApiError::forbidden("Application belongs to another team"));
        }

        repo.find_by_application(app.id, limit, offset)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to fetch deployments: {}", e)))?
    } else {
        repo.find_by_team(auth.team_id, limit, offset)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to fetch deployments: {}", e)))?
    };

    // Filter by status if specified
    let deployments = if let Some(ref status) = query.status {
        let target_status = parse_status(status);
        deployments.into_iter()
            .filter(|d| Some(d.status) == target_status)
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

    let deployment = repo.find_by_id(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch deployment: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Deployment not found"))?;

    // Verify team ownership via application -> environment -> project -> team
    let app = state.applications().find_by_id(deployment.application_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch application: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Application not found"))?;

    let env = state.projects().find_environment_by_id(app.environment_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch environment: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Environment not found"))?;

    let project = state.projects().find_by_id(env.project_id)
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
    let deployment = get_deployment_with_auth(&state, id, &auth).await?;
    Ok(Json(deployment.logs))
}

/// Cancel a running deployment
pub async fn cancel(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<Deployment>, ApiError> {
    let repo = state.deployments();

    let mut deployment = get_deployment_with_auth(&state, id, &auth).await?;

    // Can only cancel in-progress deployments
    match deployment.status {
        DeploymentStatus::Queued | DeploymentStatus::InProgress => {
            deployment.status = DeploymentStatus::Cancelled;
            deployment.finished_at = Some(chrono::Utc::now());

            // Add cancellation log
            deployment.add_log(
                LogLevel::Info,
                format!("Deployment cancelled by user {}", auth.email),
                Some("cancel".to_string()),
            );

            let updated = repo.update(&deployment)
                .await
                .map_err(|e| ApiError::internal(format!("Failed to cancel deployment: {}", e)))?;

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
    let app = state.applications().find_by_id(app_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch application: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Application not found"))?;

    let env = state.projects().find_environment_by_id(app.environment_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch environment: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Environment not found"))?;

    let project = state.projects().find_by_id(env.project_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch project: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    if project.team_id != auth.team_id && !auth.is_admin {
        return Err(ApiError::forbidden("Application belongs to another team"));
    }

    let stats = repo.get_stats(app.id)
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

    let original = get_deployment_with_auth(&state, id, &auth).await?;

    // Can only retry failed or cancelled deployments
    if !matches!(original.status, DeploymentStatus::Failed | DeploymentStatus::Cancelled) {
        return Err(ApiError::bad_request("Can only retry failed or cancelled deployments"));
    }

    // Create new deployment with same parameters
    let mut new_deployment = Deployment::new(
        original.application_id,
        original.server_id,
        DeploymentType::Redeploy,
    );
    new_deployment.commit_sha = original.commit_sha.clone();
    new_deployment.commit_message = original.commit_message.clone();
    new_deployment.triggered_by = Some(auth.user_id);

    // Add initial log
    new_deployment.add_log(
        LogLevel::Info,
        format!("Deployment queued (retry of {})", original.id),
        Some("queue".to_string()),
    );

    let created = repo.create(&new_deployment)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to create deployment: {}", e)))?;

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
    let app = state.applications().find_by_id(app_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch application: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Application not found"))?;

    let env = state.projects().find_environment_by_id(app.environment_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch environment: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Environment not found"))?;

    let project = state.projects().find_by_id(env.project_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch project: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    if project.team_id != auth.team_id && !auth.is_admin {
        return Err(ApiError::forbidden("Application belongs to another team"));
    }

    let latest = repo.find_latest_by_application(app.id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch deployment: {}", e)))?;

    Ok(Json(latest))
}

// Helper function to get deployment with authorization check
async fn get_deployment_with_auth(
    state: &AppState,
    id: Uuid,
    auth: &AuthContext,
) -> Result<Deployment, ApiError> {
    let repo = state.deployments();

    let deployment = repo.find_by_id(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch deployment: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Deployment not found"))?;

    // Verify team ownership
    let app = state.applications().find_by_id(deployment.application_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch application: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Application not found"))?;

    let env = state.projects().find_environment_by_id(app.environment_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch environment: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Environment not found"))?;

    let project = state.projects().find_by_id(env.project_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch project: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    if project.team_id != auth.team_id && !auth.is_admin {
        return Err(ApiError::forbidden("Deployment belongs to another team"));
    }

    Ok(deployment)
}

fn parse_status(s: &str) -> Option<DeploymentStatus> {
    match s.to_lowercase().as_str() {
        "queued" => Some(DeploymentStatus::Queued),
        "in_progress" | "inprogress" => Some(DeploymentStatus::InProgress),
        "finished" | "success" | "successful" => Some(DeploymentStatus::Finished),
        "failed" | "error" => Some(DeploymentStatus::Failed),
        "cancelled" | "canceled" => Some(DeploymentStatus::Cancelled),
        _ => None,
    }
}
