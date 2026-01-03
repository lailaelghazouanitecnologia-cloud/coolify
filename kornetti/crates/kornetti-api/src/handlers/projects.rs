//! Project handlers

use axum::{
    extract::{Extension, Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use kornetti_core::models::{Project, Environment};
use crate::{state::{AppState, AuthContext}, ApiError};

#[derive(Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateEnvironmentRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Serialize)]
pub struct ProjectWithEnvironments {
    #[serde(flatten)]
    pub project: Project,
    pub environments: Vec<Environment>,
}

/// List projects for the current team
pub async fn list(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<Vec<Project>>, ApiError> {
    let repo = state.projects();

    let projects = repo.find_by_team(auth.team_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch projects: {}", e)))?;

    Ok(Json(projects))
}

/// Get a specific project with its environments
pub async fn get(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<ProjectWithEnvironments>, ApiError> {
    let repo = state.projects();

    let project = repo.find_by_id_and_team(id, auth.team_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch project: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    let environments = repo.find_environments(project.id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch environments: {}", e)))?;

    Ok(Json(ProjectWithEnvironments { project, environments }))
}

/// Create a new project
pub async fn create(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<CreateProjectRequest>,
) -> Result<Json<Project>, ApiError> {
    // Validate name
    if body.name.trim().is_empty() {
        return Err(ApiError::validation("Project name cannot be empty"));
    }

    if body.name.len() > 255 {
        return Err(ApiError::validation("Project name is too long"));
    }

    let repo = state.projects();

    // Create project with default production environment
    let mut project = Project::new(auth.team_id, body.name);
    if let Some(desc) = body.description {
        project.description = Some(desc);
    }

    let (created, _env) = repo.create_with_environment(&project)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to create project: {}", e)))?;

    Ok(Json(created))
}

/// Update a project
pub async fn update(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateProjectRequest>,
) -> Result<Json<Project>, ApiError> {
    let repo = state.projects();

    let mut project = repo.find_by_id_and_team(id, auth.team_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch project: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    // Apply updates
    if let Some(name) = body.name {
        if name.trim().is_empty() {
            return Err(ApiError::validation("Project name cannot be empty"));
        }
        project.name = name;
    }

    if let Some(description) = body.description {
        project.description = Some(description);
    }

    let updated = repo.update(&project)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to update project: {}", e)))?;

    Ok(Json(updated))
}

/// Delete a project
pub async fn delete(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<()>, ApiError> {
    let repo = state.projects();

    let project = repo.find_by_id_and_team(id, auth.team_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch project: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    // Check if project has applications
    let apps = state.applications().find_by_environment(project.id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to check applications: {}", e)))?;

    if !apps.is_empty() {
        return Err(ApiError::conflict(
            "Cannot delete project with active applications. Delete all applications first."
        ));
    }

    repo.delete(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to delete project: {}", e)))?;

    Ok(Json(()))
}

/// List environments in a project
pub async fn list_environments(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<Environment>>, ApiError> {
    let repo = state.projects();

    let project = repo.find_by_id_and_team(id, auth.team_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch project: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    let environments = repo.find_environments(project.id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch environments: {}", e)))?;

    Ok(Json(environments))
}

/// Create a new environment in a project
pub async fn create_environment(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Json(body): Json<CreateEnvironmentRequest>,
) -> Result<Json<Environment>, ApiError> {
    let repo = state.projects();

    let project = repo.find_by_id_and_team(id, auth.team_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch project: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    // Validate name
    if body.name.trim().is_empty() {
        return Err(ApiError::validation("Environment name cannot be empty"));
    }

    // Check for duplicate name within project
    let existing_envs = repo.find_environments(project.id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to check environment name: {}", e)))?;

    if existing_envs.iter().any(|e| e.name == body.name) {
        return Err(ApiError::conflict("An environment with this name already exists"));
    }

    let mut environment = Environment::new(project.id, body.name);
    if let Some(desc) = body.description {
        environment.description = Some(desc);
    }

    let created = repo.create_environment(&environment)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to create environment: {}", e)))?;

    Ok(Json(created))
}

/// Delete an environment
pub async fn delete_environment(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path((project_id, env_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<()>, ApiError> {
    let repo = state.projects();

    let project = repo.find_by_id_and_team(project_id, auth.team_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch project: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    let environment = repo.find_environment_by_id(env_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch environment: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Environment not found"))?;

    // Verify environment belongs to project
    if environment.project_id != project.id {
        return Err(ApiError::not_found("Environment not found in this project"));
    }

    // Check if environment has applications
    let apps = state.applications().find_by_environment(environment.id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to check applications: {}", e)))?;

    if !apps.is_empty() {
        return Err(ApiError::conflict(
            "Cannot delete environment with active applications. Delete all applications first."
        ));
    }

    // Cannot delete the last environment
    let envs = repo.find_environments(project.id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to count environments: {}", e)))?;

    if envs.len() <= 1 {
        return Err(ApiError::bad_request("Cannot delete the last environment"));
    }

    repo.delete_environment(env_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to delete environment: {}", e)))?;

    Ok(Json(()))
}
