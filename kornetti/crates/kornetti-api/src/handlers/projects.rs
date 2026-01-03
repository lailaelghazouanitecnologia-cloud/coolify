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

    let project = repo.find_by_id(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch project: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    // Verify team ownership
    if project.team_id != auth.team_id && !auth.is_admin {
        return Err(ApiError::forbidden("Project belongs to another team"));
    }

    let environments = repo.get_environments(project.id as i64)
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

    // Check for duplicate name within team
    let existing = repo.find_by_name_and_team(&body.name, auth.team_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to check project name: {}", e)))?;

    if existing.is_some() {
        return Err(ApiError::conflict("A project with this name already exists"));
    }

    // Create project
    let project = Project {
        id: 0,
        uuid: Uuid::new_v4(),
        name: body.name,
        description: body.description,
        team_id: auth.team_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        deleted_at: None,
    };

    let created = repo.create(project)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to create project: {}", e)))?;

    // Create default "production" environment
    let default_env = Environment {
        id: 0,
        uuid: Uuid::new_v4(),
        name: "production".to_string(),
        description: Some("Production environment".to_string()),
        project_id: created.id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        deleted_at: None,
    };

    repo.create_environment(default_env)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to create default environment: {}", e)))?;

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

    let mut project = repo.find_by_id(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch project: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    // Verify team ownership
    if project.team_id != auth.team_id && !auth.is_admin {
        return Err(ApiError::forbidden("Project belongs to another team"));
    }

    // Apply updates
    if let Some(name) = body.name {
        if name.trim().is_empty() {
            return Err(ApiError::validation("Project name cannot be empty"));
        }

        // Check for duplicate name
        if name != project.name {
            let existing = repo.find_by_name_and_team(&name, auth.team_id)
                .await
                .map_err(|e| ApiError::internal(format!("Failed to check project name: {}", e)))?;

            if existing.is_some() {
                return Err(ApiError::conflict("A project with this name already exists"));
            }
        }

        project.name = name;
    }

    if let Some(description) = body.description {
        project.description = Some(description);
    }

    project.updated_at = chrono::Utc::now();

    let updated = repo.update(project)
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

    let project = repo.find_by_id(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch project: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    // Verify team ownership
    if project.team_id != auth.team_id && !auth.is_admin {
        return Err(ApiError::forbidden("Project belongs to another team"));
    }

    // Check if project has resources (applications, databases, services)
    let app_count = state.applications().count_by_project(project.id as i64)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to count applications: {}", e)))?;

    if app_count > 0 {
        return Err(ApiError::conflict(
            "Cannot delete project with active applications. Delete all applications first."
        ));
    }

    let db_count = state.databases().count_by_project(project.id as i64)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to count databases: {}", e)))?;

    if db_count > 0 {
        return Err(ApiError::conflict(
            "Cannot delete project with active databases. Delete all databases first."
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

    let project = repo.find_by_id(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch project: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    // Verify team ownership
    if project.team_id != auth.team_id && !auth.is_admin {
        return Err(ApiError::forbidden("Project belongs to another team"));
    }

    let environments = repo.get_environments(project.id as i64)
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

    let project = repo.find_by_id(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch project: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    // Verify team ownership
    if project.team_id != auth.team_id && !auth.is_admin {
        return Err(ApiError::forbidden("Project belongs to another team"));
    }

    // Validate name
    if body.name.trim().is_empty() {
        return Err(ApiError::validation("Environment name cannot be empty"));
    }

    // Check for duplicate name within project
    let existing = repo.find_environment_by_name(project.id as i64, &body.name)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to check environment name: {}", e)))?;

    if existing.is_some() {
        return Err(ApiError::conflict("An environment with this name already exists"));
    }

    let environment = Environment {
        id: 0,
        uuid: Uuid::new_v4(),
        name: body.name,
        description: body.description,
        project_id: project.id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        deleted_at: None,
    };

    let created = repo.create_environment(environment)
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

    let project = repo.find_by_id(project_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch project: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    // Verify team ownership
    if project.team_id != auth.team_id && !auth.is_admin {
        return Err(ApiError::forbidden("Project belongs to another team"));
    }

    let environment = repo.find_environment_by_id(env_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch environment: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Environment not found"))?;

    // Verify environment belongs to project
    if environment.project_id != project.id {
        return Err(ApiError::not_found("Environment not found in this project"));
    }

    // Check if environment has resources
    let app_count = state.applications().count_by_environment(environment.id as i64)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to count applications: {}", e)))?;

    if app_count > 0 {
        return Err(ApiError::conflict(
            "Cannot delete environment with active applications. Delete all applications first."
        ));
    }

    // Cannot delete the last environment
    let env_count = repo.count_environments(project.id as i64)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to count environments: {}", e)))?;

    if env_count <= 1 {
        return Err(ApiError::bad_request("Cannot delete the last environment"));
    }

    repo.delete_environment(env_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to delete environment: {}", e)))?;

    Ok(Json(()))
}
