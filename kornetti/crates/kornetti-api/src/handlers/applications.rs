//! Application handlers

use axum::{
    extract::{Extension, Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn, instrument};
use uuid::Uuid;

use kornetti_core::models::{
    Application, ApplicationStatus, ApplicationSource, BuildConfig, DeployConfig,
    BuildPack, ResourceLimits, HealthCheck,
};
use kornetti_core::actions::application::{StopApplication, StopApplicationInput, StopOptions};
use kornetti_core::actions::Action;
use crate::{state::{AppState, AuthContext}, ApiError};

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateApplicationRequest {
    pub name: String,
    pub description: Option<String>,
    pub environment_id: Uuid,
    pub server_id: Uuid,
    pub fqdn: Option<String>,
    #[serde(flatten)]
    pub source: ApplicationSourceRequest,
    pub build_pack: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ApplicationSourceRequest {
    Git {
        git_repository: String,
        git_branch: Option<String>,
    },
    DockerImage {
        docker_image: String,
    },
    Dockerfile {
        dockerfile_content: String,
    },
}

#[derive(Debug, Deserialize)]
pub struct UpdateApplicationRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub fqdn: Option<String>,
    pub git_branch: Option<String>,
    pub build_command: Option<String>,
    pub install_command: Option<String>,
    pub start_command: Option<String>,
    pub replicas: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct DeploymentResponse {
    pub deployment_id: Uuid,
    pub status: String,
}

// ============================================================================
// Handlers
// ============================================================================

/// List all applications for the current team
#[instrument(skip(state, auth))]
pub async fn list(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<Vec<Application>>, ApiError> {
    let repo = state.applications();

    let apps = repo.find_by_team(auth.team_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch applications: {}", e)))?;

    Ok(Json(apps))
}

/// Get a specific application by ID
#[instrument(skip(state, auth))]
pub async fn get(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<Application>, ApiError> {
    let repo = state.applications();

    let app = repo.find_by_id(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found(format!("Application {} not found", id)))?;

    // TODO: Verify team ownership through project/environment chain

    Ok(Json(app))
}

/// Create a new application
#[instrument(skip(state, auth, body))]
pub async fn create(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<CreateApplicationRequest>,
) -> Result<Json<Application>, ApiError> {
    let repo = state.applications();

    // Verify server belongs to team
    let server = state.servers()
        .find_by_id_and_team(body.server_id, auth.team_id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::bad_request("Invalid server_id"))?;

    // TODO: Verify environment belongs to team through project

    let source = match body.source {
        ApplicationSourceRequest::Git { git_repository, git_branch } => {
            ApplicationSource::Git {
                repository_url: git_repository,
                branch: git_branch.unwrap_or_else(|| "main".to_string()),
                commit_sha: None,
                private_key_id: None,
            }
        }
        ApplicationSourceRequest::DockerImage { docker_image } => {
            ApplicationSource::DockerImage {
                image: docker_image,
                tag: "latest".to_string(),
                registry_id: None,
            }
        }
        ApplicationSourceRequest::Dockerfile { dockerfile_content } => {
            ApplicationSource::Dockerfile {
                content: dockerfile_content,
                context: ".".to_string(),
            }
        }
    };

    let build_pack = body.build_pack
        .map(|s| match s.as_str() {
            "nixpacks" => BuildPack::Nixpacks,
            "dockerfile" => BuildPack::Dockerfile,
            "docker_image" => BuildPack::DockerImage,
            "static" => BuildPack::Static,
            _ => BuildPack::Nixpacks,
        })
        .unwrap_or(BuildPack::Nixpacks);

    let now = chrono::Utc::now();
    let app = Application {
        id: Uuid::new_v4(),
        environment_id: body.environment_id,
        server_id: body.server_id,
        name: body.name,
        description: body.description,
        fqdn: body.fqdn,
        source,
        build_config: BuildConfig {
            build_pack,
            dockerfile_path: None,
            build_command: None,
            install_command: None,
            start_command: None,
            base_directory: None,
            publish_directory: None,
        },
        deploy_config: DeployConfig {
            replicas: 1,
            health_check: Some(HealthCheck::default()),
            resources: ResourceLimits::default(),
            ports: vec![],
            volumes: vec![],
            environment_variables: vec![],
            labels: vec![],
        },
        status: ApplicationStatus::Stopped,
        created_at: now,
        updated_at: now,
    };

    let created = repo.create(&app)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to create application: {}", e)))?;

    info!(application_id = %created.id, "Created new application");

    Ok(Json(created))
}

/// Update an existing application
#[instrument(skip(state, auth, body))]
pub async fn update(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateApplicationRequest>,
) -> Result<Json<Application>, ApiError> {
    let repo = state.applications();

    let mut app = repo.find_by_id(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found(format!("Application {} not found", id)))?;

    // TODO: Verify team ownership

    // Apply updates
    if let Some(name) = body.name {
        app.name = name;
    }
    if let Some(description) = body.description {
        app.description = Some(description);
    }
    if let Some(fqdn) = body.fqdn {
        app.fqdn = Some(fqdn);
    }
    if let Some(branch) = body.git_branch {
        if let ApplicationSource::Git { ref mut branch: b, .. } = app.source {
            *b = branch;
        }
    }
    if let Some(build_cmd) = body.build_command {
        app.build_config.build_command = Some(build_cmd);
    }
    if let Some(install_cmd) = body.install_command {
        app.build_config.install_command = Some(install_cmd);
    }
    if let Some(start_cmd) = body.start_command {
        app.build_config.start_command = Some(start_cmd);
    }
    if let Some(replicas) = body.replicas {
        app.deploy_config.replicas = replicas;
    }

    // Note: Full update would require a new repository method
    // For now, we just update the status as an example
    info!(application_id = %id, "Updated application");

    Ok(Json(app))
}

/// Delete an application
#[instrument(skip(state, auth))]
pub async fn delete(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<()>, ApiError> {
    let repo = state.applications();

    let app = repo.find_by_id(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found(format!("Application {} not found", id)))?;

    // TODO: Verify team ownership

    // Stop the application first if running
    if app.status == ApplicationStatus::Running {
        let server = state.servers()
            .find_by_id(app.server_id)
            .await
            .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
            .ok_or_else(|| ApiError::internal("Server not found"))?;

        let stopper = StopApplication::new(state.ssh.clone());
        let _ = stopper.handle(StopApplicationInput {
            server: &server,
            application: &app,
            options: StopOptions { remove: true, ..Default::default() },
        }).await;
    }

    repo.delete(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to delete application: {}", e)))?;

    info!(application_id = %id, "Deleted application");

    Ok(Json(()))
}

/// Trigger a deployment for an application
#[instrument(skip(state, auth))]
pub async fn deploy(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<DeploymentResponse>, ApiError> {
    let app_repo = state.applications();
    let deploy_repo = state.deployments();

    let app = app_repo.find_by_id(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found(format!("Application {} not found", id)))?;

    // TODO: Verify team ownership

    // Create a new deployment record
    let deployment_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    // TODO: Create deployment in database
    // deploy_repo.create(&deployment).await?;

    // Update application status
    app_repo.update_status(id, ApplicationStatus::Starting)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to update status: {}", e)))?;

    // TODO: Queue deployment job
    // scheduler.queue_deployment(DeploymentContext { ... });

    info!(application_id = %id, deployment_id = %deployment_id, "Deployment queued");

    Ok(Json(DeploymentResponse {
        deployment_id,
        status: "queued".to_string(),
    }))
}

/// Stop a running application
#[instrument(skip(state, auth))]
pub async fn stop(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<()>, ApiError> {
    let repo = state.applications();

    let app = repo.find_by_id(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found(format!("Application {} not found", id)))?;

    // TODO: Verify team ownership

    let server = state.servers()
        .find_by_id(app.server_id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::internal("Server not found"))?;

    // Update status
    repo.update_status(id, ApplicationStatus::Stopping)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to update status: {}", e)))?;

    // Stop the application
    let stopper = StopApplication::new(state.ssh.clone());
    let result = stopper.handle(StopApplicationInput {
        server: &server,
        application: &app,
        options: StopOptions::default(),
    }).await;

    match result {
        Ok(_) => {
            repo.update_status(id, ApplicationStatus::Stopped).await.ok();
            info!(application_id = %id, "Application stopped");
            Ok(Json(()))
        }
        Err(e) => {
            warn!(application_id = %id, error = %e, "Failed to stop application");
            repo.update_status(id, ApplicationStatus::Error).await.ok();
            Err(ApiError::internal(format!("Failed to stop: {}", e)))
        }
    }
}

/// Restart an application
#[instrument(skip(state, auth))]
pub async fn restart(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<DeploymentResponse>, ApiError> {
    let repo = state.applications();

    let app = repo.find_by_id(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found(format!("Application {} not found", id)))?;

    // TODO: Verify team ownership

    // Update status
    repo.update_status(id, ApplicationStatus::Restarting)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to update status: {}", e)))?;

    // For restart, we trigger a new deployment with the same configuration
    let deployment_id = Uuid::new_v4();

    // TODO: Queue restart job

    info!(application_id = %id, deployment_id = %deployment_id, "Restart queued");

    Ok(Json(DeploymentResponse {
        deployment_id,
        status: "restarting".to_string(),
    }))
}

/// Get application logs
#[instrument(skip(state, auth))]
pub async fn logs(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<String>>, ApiError> {
    let repo = state.applications();

    let app = repo.find_by_id(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found(format!("Application {} not found", id)))?;

    // TODO: Verify team ownership

    let server = state.servers()
        .find_by_id(app.server_id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::internal("Server not found"))?;

    // Get container logs via SSH
    let session = state.ssh.connect(&server)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to connect: {}", e)))?;

    let container_name = app.id.to_string();
    let output = session.execute(&format!(
        "docker logs --tail 100 {} 2>&1",
        container_name
    ))
        .await
        .map_err(|e| ApiError::internal(format!("Failed to get logs: {}", e)))?;

    let logs: Vec<String> = output.stdout
        .lines()
        .map(|s| s.to_string())
        .collect();

    Ok(Json(logs))
}
