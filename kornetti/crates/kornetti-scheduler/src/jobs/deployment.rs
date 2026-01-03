//! Application Deployment Job
//!
//! This is the core deployment engine that orchestrates the full deployment
//! process for applications. It handles:
//!
//! - Git repository cloning/pulling
//! - Dockerfile generation or detection
//! - Docker image building
//! - Container orchestration (stop old, start new)
//! - Health checks
//! - Rollback on failure
//! - Deployment logging

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use kornetti_core::{
    Result, Error,
    models::{Application, Server, Deployment, DeploymentStatus, DeploymentType},
    repositories::{ApplicationRepository, DeploymentRepository, ServerRepository},
};
use kornetti_ssh::{SshClient, Command, CommandBatch};
use kornetti_docker::{
    DockerExecutor, ContainerConfig, BuildConfig, NetworkConfig, VolumeMount,
    container::{PortBinding, Protocol, RestartPolicy},
};

/// Deployment context containing all necessary data
#[derive(Debug, Clone)]
pub struct DeploymentContext {
    pub deployment_id: Uuid,
    pub application_id: Uuid,
    pub server_id: Uuid,
    pub deployment_type: DeploymentType,
    pub commit_sha: Option<String>,
    pub force_rebuild: bool,
    pub rollback_to: Option<Uuid>,
}

/// Steps in the deployment process
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeploymentStep {
    Queued,
    PreparingServer,
    CloningRepository,
    GeneratingDockerfile,
    BuildingImage,
    StoppingOldContainers,
    StartingNewContainers,
    ConfiguringProxy,
    RunningHealthChecks,
    Cleanup,
    Finished,
    Failed,
    RollingBack,
}

impl DeploymentStep {
    pub fn display(&self) -> &'static str {
        match self {
            DeploymentStep::Queued => "Queued",
            DeploymentStep::PreparingServer => "Preparing server...",
            DeploymentStep::CloningRepository => "Cloning repository...",
            DeploymentStep::GeneratingDockerfile => "Generating Dockerfile...",
            DeploymentStep::BuildingImage => "Building image...",
            DeploymentStep::StoppingOldContainers => "Stopping old containers...",
            DeploymentStep::StartingNewContainers => "Starting new containers...",
            DeploymentStep::ConfiguringProxy => "Configuring proxy...",
            DeploymentStep::RunningHealthChecks => "Running health checks...",
            DeploymentStep::Cleanup => "Cleaning up...",
            DeploymentStep::Finished => "Deployment finished",
            DeploymentStep::Failed => "Deployment failed",
            DeploymentStep::RollingBack => "Rolling back...",
        }
    }
}

/// Deployment log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentLog {
    pub timestamp: DateTime<Utc>,
    pub step: DeploymentStep,
    pub message: String,
    pub is_error: bool,
}

/// Application deployment job
pub struct ApplicationDeploymentJob {
    ssh: Arc<SshClient>,
    docker: Arc<DockerExecutor>,
    context: DeploymentContext,
    logs: Vec<DeploymentLog>,
    current_step: DeploymentStep,
}

impl ApplicationDeploymentJob {
    pub fn new(
        ssh: Arc<SshClient>,
        docker: Arc<DockerExecutor>,
        context: DeploymentContext,
    ) -> Self {
        Self {
            ssh,
            docker,
            context,
            logs: Vec::new(),
            current_step: DeploymentStep::Queued,
        }
    }

    /// Run the deployment
    #[instrument(skip(self, app, server))]
    pub async fn run(
        &mut self,
        app: &Application,
        server: &Server,
    ) -> Result<DeploymentResult> {
        info!(
            deployment_id = %self.context.deployment_id,
            app_name = %app.name,
            "Starting deployment"
        );

        let start_time = Utc::now();
        let mut result = DeploymentResult::default();

        // Execute deployment steps
        match self.execute_deployment(app, server).await {
            Ok(_) => {
                self.set_step(DeploymentStep::Finished);
                self.log("Deployment completed successfully");
                result.success = true;
            }
            Err(e) => {
                self.set_step(DeploymentStep::Failed);
                self.log_error(&format!("Deployment failed: {}", e));
                result.success = false;
                result.error = Some(e.to_string());

                // Attempt rollback if not already a rollback
                if self.context.deployment_type != DeploymentType::Rollback {
                    if let Err(rollback_err) = self.rollback(app, server).await {
                        self.log_error(&format!("Rollback failed: {}", rollback_err));
                    }
                }
            }
        }

        result.duration = Utc::now() - start_time;
        result.logs = self.logs.clone();

        Ok(result)
    }

    /// Execute the full deployment pipeline
    async fn execute_deployment(
        &mut self,
        app: &Application,
        server: &Server,
    ) -> Result<()> {
        // Step 1: Prepare server
        self.set_step(DeploymentStep::PreparingServer);
        self.prepare_server(app, server).await?;

        // Step 2: Clone/pull repository
        self.set_step(DeploymentStep::CloningRepository);
        let commit_sha = self.clone_repository(app, server).await?;

        // Step 3: Generate Dockerfile if needed
        self.set_step(DeploymentStep::GeneratingDockerfile);
        self.prepare_dockerfile(app, server).await?;

        // Step 4: Build image
        self.set_step(DeploymentStep::BuildingImage);
        let image_name = self.build_image(app, server, &commit_sha).await?;

        // Step 5: Stop old containers
        self.set_step(DeploymentStep::StoppingOldContainers);
        self.stop_old_containers(app, server).await?;

        // Step 6: Start new containers
        self.set_step(DeploymentStep::StartingNewContainers);
        self.start_new_containers(app, server, &image_name).await?;

        // Step 7: Configure proxy
        self.set_step(DeploymentStep::ConfiguringProxy);
        self.configure_proxy(app, server).await?;

        // Step 8: Health checks
        self.set_step(DeploymentStep::RunningHealthChecks);
        self.run_health_checks(app, server).await?;

        // Step 9: Cleanup
        self.set_step(DeploymentStep::Cleanup);
        self.cleanup(app, server).await?;

        Ok(())
    }

    /// Prepare the server for deployment
    async fn prepare_server(&mut self, app: &Application, server: &Server) -> Result<()> {
        self.log("Preparing deployment directory");

        let deployment_dir = format!("/data/coolify/applications/{}", app.id);

        let batch = CommandBatch::new()
            .cmd(format!("mkdir -p {}/source", deployment_dir))
            .cmd(format!("mkdir -p {}/volumes", deployment_dir));

        let result = self.ssh.execute_batch(server, &batch).await?;

        if !result.output.success() {
            return Err(Error::Deployment(format!(
                "Failed to prepare directory: {}",
                result.output.stderr
            )));
        }

        // Ensure network exists
        let network_name = format!("coolify-{}", app.id);
        let result = self.ssh.execute(
            server,
            &format!("docker network create {} 2>/dev/null || true", network_name)
        ).await?;

        self.log("Server prepared");
        Ok(())
    }

    /// Clone or pull the git repository
    async fn clone_repository(&mut self, app: &Application, server: &Server) -> Result<String> {
        let source_dir = format!("/data/coolify/applications/{}/source", app.id);
        let git_url = &app.git_repository;
        let branch = app.git_branch.as_deref().unwrap_or("main");

        self.log(&format!("Cloning {} (branch: {})", git_url, branch));

        // Check if repo already exists
        let exists_result = self.ssh.execute(
            server,
            &format!("test -d {}/.git && echo 1 || echo 0", source_dir)
        ).await?;

        let repo_exists = exists_result.output.stdout.trim() == "1";

        let batch = if repo_exists {
            self.log("Repository exists, pulling latest changes");
            CommandBatch::new()
                .cmd(format!("cd {} && git fetch origin", source_dir))
                .cmd(format!("cd {} && git checkout {}", source_dir, branch))
                .cmd(format!("cd {} && git reset --hard origin/{}", source_dir, branch))
        } else {
            self.log("Cloning repository for the first time");
            CommandBatch::new()
                .cmd(format!("rm -rf {}", source_dir))
                .cmd(format!("git clone -b {} --single-branch {} {}", branch, git_url, source_dir))
        };

        let result = self.ssh.execute_batch(server, &batch).await?;

        if !result.output.success() {
            return Err(Error::Deployment(format!(
                "Failed to clone repository: {}",
                result.output.stderr
            )));
        }

        // Get current commit SHA
        let sha_result = self.ssh.execute(
            server,
            &format!("cd {} && git rev-parse HEAD", source_dir)
        ).await?;

        let commit_sha = sha_result.output.stdout.trim().to_string();
        self.log(&format!("At commit: {}", &commit_sha[..8]));

        Ok(commit_sha)
    }

    /// Prepare Dockerfile (generate if needed)
    async fn prepare_dockerfile(&mut self, app: &Application, server: &Server) -> Result<()> {
        let source_dir = format!("/data/coolify/applications/{}/source", app.id);

        // Check if Dockerfile exists
        let exists_result = self.ssh.execute(
            server,
            &format!("test -f {}/Dockerfile && echo 1 || echo 0", source_dir)
        ).await?;

        if exists_result.output.stdout.trim() == "1" {
            self.log("Found existing Dockerfile");
            return Ok(());
        }

        // Auto-detect and generate Dockerfile
        self.log("No Dockerfile found, auto-detecting project type");

        let project_type = self.detect_project_type(server, &source_dir).await?;
        let dockerfile = self.generate_dockerfile(&project_type, app)?;

        // Write Dockerfile
        let dockerfile_path = format!("{}/Dockerfile", source_dir);
        self.ssh.upload_string(server, &dockerfile_path, &dockerfile).await?;

        self.log(&format!("Generated Dockerfile for {} project", project_type));

        Ok(())
    }

    /// Detect project type from source files
    async fn detect_project_type(&self, server: &Server, source_dir: &str) -> Result<String> {
        // Check for package.json (Node.js)
        let node_check = self.ssh.execute(
            server,
            &format!("test -f {}/package.json && echo 1 || echo 0", source_dir)
        ).await?;

        if node_check.output.stdout.trim() == "1" {
            return Ok("nodejs".to_string());
        }

        // Check for requirements.txt (Python)
        let python_check = self.ssh.execute(
            server,
            &format!("test -f {}/requirements.txt && echo 1 || echo 0", source_dir)
        ).await?;

        if python_check.output.stdout.trim() == "1" {
            return Ok("python".to_string());
        }

        // Check for Cargo.toml (Rust)
        let rust_check = self.ssh.execute(
            server,
            &format!("test -f {}/Cargo.toml && echo 1 || echo 0", source_dir)
        ).await?;

        if rust_check.output.stdout.trim() == "1" {
            return Ok("rust".to_string());
        }

        // Check for go.mod (Go)
        let go_check = self.ssh.execute(
            server,
            &format!("test -f {}/go.mod && echo 1 || echo 0", source_dir)
        ).await?;

        if go_check.output.stdout.trim() == "1" {
            return Ok("golang".to_string());
        }

        // Check for composer.json (PHP)
        let php_check = self.ssh.execute(
            server,
            &format!("test -f {}/composer.json && echo 1 || echo 0", source_dir)
        ).await?;

        if php_check.output.stdout.trim() == "1" {
            return Ok("php".to_string());
        }

        // Default to static
        Ok("static".to_string())
    }

    /// Generate Dockerfile based on project type
    fn generate_dockerfile(&self, project_type: &str, app: &Application) -> Result<String> {
        let dockerfile = match project_type {
            "nodejs" => r#"FROM node:20-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build || true

FROM node:20-alpine
WORKDIR /app
COPY --from=builder /app .
EXPOSE 3000
CMD ["npm", "start"]
"#.to_string(),

            "python" => r#"FROM python:3.11-slim
WORKDIR /app
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt
COPY . .
EXPOSE 8000
CMD ["python", "app.py"]
"#.to_string(),

            "rust" => r#"FROM rust:alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY . .
RUN cargo build --release

FROM alpine:latest
WORKDIR /app
COPY --from=builder /app/target/release/* .
EXPOSE 8080
CMD ["./app"]
"#.to_string(),

            "golang" => r#"FROM golang:alpine AS builder
WORKDIR /app
COPY go.* ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 go build -o app .

FROM alpine:latest
WORKDIR /app
COPY --from=builder /app/app .
EXPOSE 8080
CMD ["./app"]
"#.to_string(),

            "php" => r#"FROM php:8.2-fpm-alpine
RUN apk add --no-cache nginx
COPY . /var/www/html
EXPOSE 80
CMD ["php-fpm"]
"#.to_string(),

            _ => r#"FROM nginx:alpine
COPY . /usr/share/nginx/html
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
"#.to_string(),
        };

        Ok(dockerfile)
    }

    /// Build Docker image
    async fn build_image(
        &mut self,
        app: &Application,
        server: &Server,
        commit_sha: &str,
    ) -> Result<String> {
        let source_dir = format!("/data/coolify/applications/{}/source", app.id);
        let image_name = format!("coolify-{}:{}", app.id, &commit_sha[..8]);

        self.log(&format!("Building image: {}", image_name));

        let build_config = BuildConfig::new(&image_name)
            .context(&source_dir)
            .label("coolify.application_id", app.id.to_string())
            .label("coolify.commit_sha", commit_sha);

        let builder = self.docker.builder();
        let result = builder.build(server, &build_config).await
            .map_err(|e| Error::Deployment(format!("Build failed: {}", e)))?;

        self.log(&format!("Image built: {}", result.image_id));

        Ok(image_name)
    }

    /// Stop old containers
    async fn stop_old_containers(&mut self, app: &Application, server: &Server) -> Result<()> {
        let container_name = format!("coolify-{}", app.id);

        self.log("Stopping old container if exists");

        // Try to stop, ignore errors if container doesn't exist
        let _ = self.docker.stop_container(server, &container_name, Some(Duration::from_secs(30))).await;
        let _ = self.docker.remove_container(server, &container_name, true, false).await;

        Ok(())
    }

    /// Start new containers
    async fn start_new_containers(
        &mut self,
        app: &Application,
        server: &Server,
        image_name: &str,
    ) -> Result<()> {
        let container_name = format!("coolify-{}", app.id);
        let network_name = format!("coolify-{}", app.id);

        self.log(&format!("Starting container: {}", container_name));

        // Build port mappings
        let mut ports = Vec::new();
        if let Some(port) = app.ports.first() {
            ports.push(PortBinding {
                container_port: *port as u16,
                host_port: None, // Let Docker assign
                protocol: Protocol::Tcp,
            });
        }

        // Build environment variables
        let environment: Vec<(String, String)> = app.environment_variables
            .iter()
            .map(|ev| (ev.key.clone(), ev.value.clone()))
            .collect();

        let config = ContainerConfig {
            name: container_name.clone(),
            image: image_name.to_string(),
            ports,
            volumes: Vec::new(),
            environment,
            labels: vec![
                ("coolify.managed".to_string(), "true".to_string()),
                ("coolify.application_id".to_string(), app.id.to_string()),
            ],
            network: Some(network_name),
            restart_policy: RestartPolicy::UnlessStopped,
            resources: Default::default(),
        };

        let container_id = self.docker.run_container(server, &config).await
            .map_err(|e| Error::Deployment(format!("Failed to start container: {}", e)))?;

        self.log(&format!("Container started: {}", &container_id[..12]));

        Ok(())
    }

    /// Configure proxy for the application
    async fn configure_proxy(&mut self, app: &Application, server: &Server) -> Result<()> {
        // Skip if no domain configured
        if app.fqdn.is_none() {
            self.log("No domain configured, skipping proxy setup");
            return Ok(());
        }

        let fqdn = app.fqdn.as_ref().unwrap();
        self.log(&format!("Configuring proxy for: {}", fqdn));

        // This would integrate with Traefik/Caddy proxy configuration
        // For now, we'll use Docker labels which Traefik can pick up automatically

        let container_name = format!("coolify-{}", app.id);
        let labels_cmd = format!(
            r#"docker container update --label-add "traefik.enable=true" \
               --label-add "traefik.http.routers.{}.rule=Host(`{}`)" \
               --label-add "traefik.http.routers.{}.entrypoints=websecure" \
               --label-add "traefik.http.routers.{}.tls.certresolver=letsencrypt" \
               {}"#,
            app.id, fqdn, app.id, app.id, container_name
        );

        let result = self.ssh.execute(server, &labels_cmd).await?;

        if !result.output.success() {
            warn!(stderr = %result.output.stderr, "Proxy label update had issues");
        }

        self.log("Proxy configured");
        Ok(())
    }

    /// Run health checks
    async fn run_health_checks(&mut self, app: &Application, server: &Server) -> Result<()> {
        let container_name = format!("coolify-{}", app.id);

        self.log("Running health checks");

        // Wait for container to be running
        tokio::time::sleep(Duration::from_secs(5)).await;

        // Check container status
        let result = self.ssh.execute(
            server,
            &format!("docker inspect --format='{{{{.State.Status}}}}' {}", container_name)
        ).await?;

        let status = result.output.stdout.trim();

        if status != "running" {
            // Get logs for debugging
            let logs = self.docker.logs(server, &container_name, Some(50), None).await
                .unwrap_or_else(|_| "Failed to get logs".to_string());

            return Err(Error::Deployment(format!(
                "Container not running (status: {}). Logs:\n{}",
                status, logs
            )));
        }

        // If container has healthcheck, wait for it
        if let Some(health_check) = &app.health_check_path {
            self.log(&format!("Waiting for health check: {}", health_check));

            match self.docker.wait_healthy(server, &container_name, Duration::from_secs(120)).await {
                Ok(_) => self.log("Health check passed"),
                Err(e) => {
                    return Err(Error::Deployment(format!("Health check failed: {}", e)));
                }
            }
        }

        self.log("Health checks passed");
        Ok(())
    }

    /// Cleanup old resources
    async fn cleanup(&mut self, app: &Application, server: &Server) -> Result<()> {
        self.log("Cleaning up old images");

        // Remove dangling images
        let _ = self.ssh.execute(
            server,
            "docker image prune -f --filter 'until=24h'"
        ).await;

        // Keep only last 3 images for this app
        let cleanup_cmd = format!(
            r#"docker images 'coolify-{}' --format '{{{{.ID}}}}' | tail -n +4 | xargs -r docker rmi"#,
            app.id
        );

        let _ = self.ssh.execute(server, &cleanup_cmd).await;

        self.log("Cleanup completed");
        Ok(())
    }

    /// Rollback to previous version
    async fn rollback(&mut self, app: &Application, server: &Server) -> Result<()> {
        self.set_step(DeploymentStep::RollingBack);
        self.log("Attempting rollback to previous version");

        // Find the previous working image
        let images_result = self.ssh.execute(
            server,
            &format!("docker images 'coolify-{}' --format '{{{{.Tag}}}}' | head -2 | tail -1", app.id)
        ).await?;

        let previous_tag = images_result.output.stdout.trim();

        if previous_tag.is_empty() {
            return Err(Error::Deployment("No previous version to rollback to".to_string()));
        }

        let previous_image = format!("coolify-{}:{}", app.id, previous_tag);
        self.log(&format!("Rolling back to: {}", previous_image));

        // Stop current container
        self.stop_old_containers(app, server).await?;

        // Start with previous image
        self.start_new_containers(app, server, &previous_image).await?;

        self.log("Rollback completed");
        Ok(())
    }

    /// Set current step and log it
    fn set_step(&mut self, step: DeploymentStep) {
        self.current_step = step;
        self.log(step.display());
    }

    /// Add a log entry
    fn log(&mut self, message: &str) {
        info!(
            deployment_id = %self.context.deployment_id,
            step = ?self.current_step,
            message = %message,
            "Deployment log"
        );

        self.logs.push(DeploymentLog {
            timestamp: Utc::now(),
            step: self.current_step,
            message: message.to_string(),
            is_error: false,
        });
    }

    /// Add an error log entry
    fn log_error(&mut self, message: &str) {
        error!(
            deployment_id = %self.context.deployment_id,
            step = ?self.current_step,
            message = %message,
            "Deployment error"
        );

        self.logs.push(DeploymentLog {
            timestamp: Utc::now(),
            step: self.current_step,
            message: message.to_string(),
            is_error: true,
        });
    }
}

/// Result of a deployment
#[derive(Debug, Clone, Default)]
pub struct DeploymentResult {
    pub success: bool,
    pub error: Option<String>,
    pub duration: chrono::Duration,
    pub logs: Vec<DeploymentLog>,
}
