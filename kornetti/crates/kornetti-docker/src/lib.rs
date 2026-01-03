//! Kornetti Docker
//!
//! Docker client and container orchestration library providing full
//! lifecycle management of containers, images, networks, and volumes.
//!
//! # Features
//!
//! - **Container Lifecycle**: Create, start, stop, restart, remove containers
//! - **Image Building**: Build images from Dockerfiles with build args
//! - **Compose Support**: Parse and execute docker-compose files
//! - **Network Management**: Create and manage Docker networks
//! - **Volume Management**: Create and manage Docker volumes
//! - **Remote Execution**: All operations work over SSH on remote servers

pub mod client;
pub mod compose;
pub mod container;
pub mod image;
pub mod network;
pub mod volume;
pub mod builder;
pub mod error;

pub use client::DockerClient;
pub use container::{ContainerConfig, PortBinding, Protocol, RestartPolicy, ResourceConfig};
pub use image::ImageRef;
pub use network::{NetworkConfig, NetworkDriver};
pub use volume::{VolumeConfig, VolumeDriver, VolumeMount};
pub use compose::{ComposeFile, ComposeService};
pub use builder::{ImageBuilder, BuildConfig, BuildResult, DockerfileGenerator};
pub use error::{DockerError, Result};

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

use kornetti_core::{
    Result as CoreResult, Error as CoreError,
    models::Server,
    traits::{ContainerOrchestrator, Container, CommandOutput},
};
use kornetti_ssh::{SshClient, Command, CommandBatch};

/// Remote Docker executor with full lifecycle management
pub struct DockerExecutor {
    ssh: Arc<SshClient>,
}

impl DockerExecutor {
    pub fn new(ssh: Arc<SshClient>) -> Self {
        Self { ssh }
    }

    /// Get an image builder
    pub fn builder(&self) -> ImageBuilder {
        ImageBuilder::new(self.ssh.clone())
    }

    // ===== Container Operations =====

    /// Create and start a container
    #[instrument(skip(self, server, config))]
    pub async fn run_container(
        &self,
        server: &Server,
        config: &ContainerConfig,
    ) -> Result<String> {
        info!(name = %config.name, image = %config.image, "Running container");

        let cmd = config.to_docker_run_command();
        let result = self.ssh.execute(server, &cmd).await
            .map_err(|e| DockerError::CommandFailed(e.to_string()))?;

        if !result.output.success() {
            if result.output.stderr.contains("already in use") {
                return Err(DockerError::ContainerExists(config.name.clone()));
            }
            return Err(DockerError::CommandFailed(result.output.stderr));
        }

        let container_id = result.output.stdout.trim().to_string();
        info!(container_id = %container_id, "Container started");

        Ok(container_id)
    }

    /// Stop a container
    #[instrument(skip(self, server))]
    pub async fn stop_container(
        &self,
        server: &Server,
        container: &str,
        timeout: Option<Duration>,
    ) -> Result<()> {
        let timeout_secs = timeout.map(|d| d.as_secs()).unwrap_or(10);
        let cmd = format!("docker stop -t {} {}", timeout_secs, container);

        let result = self.ssh.execute(server, &cmd).await
            .map_err(|e| DockerError::CommandFailed(e.to_string()))?;

        if !result.output.success() {
            if result.output.stderr.contains("No such container") {
                return Err(DockerError::ContainerNotFound(container.to_string()));
            }
            return Err(DockerError::CommandFailed(result.output.stderr));
        }

        Ok(())
    }

    /// Start a stopped container
    pub async fn start_container(&self, server: &Server, container: &str) -> Result<()> {
        let result = self.ssh.execute(server, &format!("docker start {}", container)).await
            .map_err(|e| DockerError::CommandFailed(e.to_string()))?;

        if !result.output.success() {
            if result.output.stderr.contains("No such container") {
                return Err(DockerError::ContainerNotFound(container.to_string()));
            }
            return Err(DockerError::CommandFailed(result.output.stderr));
        }

        Ok(())
    }

    /// Restart a container
    pub async fn restart_container(
        &self,
        server: &Server,
        container: &str,
        timeout: Option<Duration>,
    ) -> Result<()> {
        let timeout_secs = timeout.map(|d| d.as_secs()).unwrap_or(10);
        let cmd = format!("docker restart -t {} {}", timeout_secs, container);

        let result = self.ssh.execute(server, &cmd).await
            .map_err(|e| DockerError::CommandFailed(e.to_string()))?;

        if !result.output.success() {
            return Err(DockerError::CommandFailed(result.output.stderr));
        }

        Ok(())
    }

    /// Remove a container
    pub async fn remove_container(
        &self,
        server: &Server,
        container: &str,
        force: bool,
        volumes: bool,
    ) -> Result<()> {
        let mut cmd = vec!["docker", "rm"];
        if force {
            cmd.push("-f");
        }
        if volumes {
            cmd.push("-v");
        }
        cmd.push(container);

        let result = self.ssh.execute(server, &cmd.join(" ")).await
            .map_err(|e| DockerError::CommandFailed(e.to_string()))?;

        if !result.output.success() && !result.output.stderr.contains("No such container") {
            return Err(DockerError::CommandFailed(result.output.stderr));
        }

        Ok(())
    }

    /// Get container logs
    pub async fn logs(
        &self,
        server: &Server,
        container: &str,
        tail: Option<usize>,
        since: Option<&str>,
    ) -> Result<String> {
        let mut cmd = vec!["docker", "logs"];

        if let Some(n) = tail {
            cmd.push("--tail");
            cmd.push(&n.to_string());
        }

        if let Some(s) = since {
            cmd.push("--since");
            cmd.push(s);
        }

        cmd.push(container);

        let result = self.ssh.execute(server, &cmd.join(" ")).await
            .map_err(|e| DockerError::CommandFailed(e.to_string()))?;

        Ok(result.output.combined())
    }

    /// Execute a command in a running container
    pub async fn exec(
        &self,
        server: &Server,
        container: &str,
        command: &str,
        interactive: bool,
    ) -> Result<kornetti_ssh::CommandOutput> {
        let mut cmd = vec!["docker", "exec"];
        if interactive {
            cmd.push("-it");
        }
        cmd.push(container);
        cmd.push("sh");
        cmd.push("-c");
        cmd.push(&format!("'{}'", command));

        let result = self.ssh.execute(server, &cmd.join(" ")).await
            .map_err(|e| DockerError::CommandFailed(e.to_string()))?;

        Ok(result.output)
    }

    /// Get container stats
    pub async fn stats(&self, server: &Server, container: &str) -> Result<ContainerStats> {
        let cmd = format!(
            "docker stats {} --no-stream --format '{{{{json .}}}}'",
            container
        );

        let result = self.ssh.execute(server, &cmd).await
            .map_err(|e| DockerError::CommandFailed(e.to_string()))?;

        if !result.output.success() {
            return Err(DockerError::CommandFailed(result.output.stderr));
        }

        serde_json::from_str(&result.output.stdout)
            .map_err(|e| DockerError::CommandFailed(format!("Failed to parse stats: {}", e)))
    }

    /// Wait for a container to be healthy
    pub async fn wait_healthy(
        &self,
        server: &Server,
        container: &str,
        timeout: Duration,
    ) -> Result<()> {
        let start = std::time::Instant::now();
        let check_interval = Duration::from_secs(2);

        while start.elapsed() < timeout {
            let result = self.ssh.execute(
                server,
                &format!("docker inspect --format='{{{{.State.Health.Status}}}}' {}", container)
            ).await.map_err(|e| DockerError::CommandFailed(e.to_string()))?;

            let status = result.output.stdout.trim();
            match status {
                "healthy" => return Ok(()),
                "unhealthy" => return Err(DockerError::HealthCheckFailed(container.to_string())),
                _ => {
                    tokio::time::sleep(check_interval).await;
                }
            }
        }

        Err(DockerError::Timeout)
    }

    /// Inspect a container
    pub async fn inspect(&self, server: &Server, container: &str) -> Result<ContainerInspect> {
        let result = self.ssh.execute(
            server,
            &format!("docker inspect {}", container)
        ).await.map_err(|e| DockerError::CommandFailed(e.to_string()))?;

        if !result.output.success() {
            if result.output.stderr.contains("No such") {
                return Err(DockerError::ContainerNotFound(container.to_string()));
            }
            return Err(DockerError::CommandFailed(result.output.stderr));
        }

        let inspects: Vec<ContainerInspect> = serde_json::from_str(&result.output.stdout)
            .map_err(|e| DockerError::CommandFailed(format!("Failed to parse inspect: {}", e)))?;

        inspects.into_iter().next()
            .ok_or_else(|| DockerError::ContainerNotFound(container.to_string()))
    }

    // ===== Network Operations =====

    /// Create a network
    pub async fn create_network(&self, server: &Server, config: &NetworkConfig) -> Result<String> {
        let cmd = config.to_create_command();
        let result = self.ssh.execute(server, &cmd).await
            .map_err(|e| DockerError::CommandFailed(e.to_string()))?;

        if !result.output.success() {
            return Err(DockerError::CommandFailed(result.output.stderr));
        }

        Ok(result.output.stdout.trim().to_string())
    }

    /// Remove a network
    pub async fn remove_network(&self, server: &Server, network: &str) -> Result<()> {
        let result = self.ssh.execute(server, &format!("docker network rm {}", network)).await
            .map_err(|e| DockerError::CommandFailed(e.to_string()))?;

        if !result.output.success() && !result.output.stderr.contains("No such network") {
            return Err(DockerError::CommandFailed(result.output.stderr));
        }

        Ok(())
    }

    /// Connect a container to a network
    pub async fn connect_network(
        &self,
        server: &Server,
        network: &str,
        container: &str,
    ) -> Result<()> {
        let result = self.ssh.execute(
            server,
            &format!("docker network connect {} {}", network, container)
        ).await.map_err(|e| DockerError::CommandFailed(e.to_string()))?;

        if !result.output.success() {
            return Err(DockerError::CommandFailed(result.output.stderr));
        }

        Ok(())
    }

    // ===== Volume Operations =====

    /// Create a volume
    pub async fn create_volume(&self, server: &Server, config: &VolumeConfig) -> Result<String> {
        let cmd = config.to_create_command();
        let result = self.ssh.execute(server, &cmd).await
            .map_err(|e| DockerError::CommandFailed(e.to_string()))?;

        if !result.output.success() {
            return Err(DockerError::CommandFailed(result.output.stderr));
        }

        Ok(result.output.stdout.trim().to_string())
    }

    /// Remove a volume
    pub async fn remove_volume(&self, server: &Server, volume: &str, force: bool) -> Result<()> {
        let cmd = if force {
            format!("docker volume rm -f {}", volume)
        } else {
            format!("docker volume rm {}", volume)
        };

        let result = self.ssh.execute(server, &cmd).await
            .map_err(|e| DockerError::CommandFailed(e.to_string()))?;

        if !result.output.success() && !result.output.stderr.contains("No such volume") {
            return Err(DockerError::CommandFailed(result.output.stderr));
        }

        Ok(())
    }

    // ===== Compose Operations =====

    /// Deploy with docker compose
    #[instrument(skip(self, server, compose_content))]
    pub async fn compose_up(
        &self,
        server: &Server,
        working_dir: &str,
        compose_content: &str,
        project_name: &str,
    ) -> Result<()> {
        info!(project = %project_name, "Deploying with docker compose");

        // Write compose file
        let compose_path = format!("{}/docker-compose.yml", working_dir);
        self.ssh.upload_string(server, &compose_path, compose_content).await
            .map_err(|e| DockerError::ComposeError(e.to_string()))?;

        // Pull images
        let batch = CommandBatch::new()
            .cmd(format!("cd {} && docker compose -p {} pull", working_dir, project_name))
            .cmd(format!("cd {} && docker compose -p {} up -d --remove-orphans", working_dir, project_name));

        let result = self.ssh.execute_batch(server, &batch).await
            .map_err(|e| DockerError::ComposeError(e.to_string()))?;

        if !result.output.success() {
            return Err(DockerError::ComposeError(result.output.stderr));
        }

        Ok(())
    }

    /// Stop and remove compose services
    pub async fn compose_down(
        &self,
        server: &Server,
        working_dir: &str,
        project_name: &str,
        volumes: bool,
    ) -> Result<()> {
        let cmd = if volumes {
            format!("cd {} && docker compose -p {} down -v", working_dir, project_name)
        } else {
            format!("cd {} && docker compose -p {} down", working_dir, project_name)
        };

        let result = self.ssh.execute(server, &cmd).await
            .map_err(|e| DockerError::ComposeError(e.to_string()))?;

        if !result.output.success() {
            warn!(stderr = %result.output.stderr, "Compose down had warnings");
        }

        Ok(())
    }

    /// Get compose service logs
    pub async fn compose_logs(
        &self,
        server: &Server,
        working_dir: &str,
        project_name: &str,
        service: Option<&str>,
        tail: Option<usize>,
    ) -> Result<String> {
        let mut cmd = format!("cd {} && docker compose -p {} logs", working_dir, project_name);

        if let Some(n) = tail {
            cmd.push_str(&format!(" --tail {}", n));
        }

        if let Some(svc) = service {
            cmd.push_str(&format!(" {}", svc));
        }

        let result = self.ssh.execute(server, &cmd).await
            .map_err(|e| DockerError::ComposeError(e.to_string()))?;

        Ok(result.output.combined())
    }

    // ===== System Operations =====

    /// Prune unused Docker resources
    pub async fn system_prune(&self, server: &Server, all: bool, volumes: bool) -> Result<()> {
        let mut cmd = vec!["docker", "system", "prune", "-f"];
        if all {
            cmd.push("-a");
        }
        if volumes {
            cmd.push("--volumes");
        }

        let result = self.ssh.execute(server, &cmd.join(" ")).await
            .map_err(|e| DockerError::CommandFailed(e.to_string()))?;

        if !result.output.success() {
            return Err(DockerError::CommandFailed(result.output.stderr));
        }

        Ok(())
    }

    /// Get Docker version info
    pub async fn version(&self, server: &Server) -> Result<DockerVersion> {
        let result = self.ssh.execute(server, "docker version --format '{{json .}}'").await
            .map_err(|e| DockerError::CommandFailed(e.to_string()))?;

        if !result.output.success() {
            return Err(DockerError::CommandFailed(result.output.stderr));
        }

        serde_json::from_str(&result.output.stdout)
            .map_err(|e| DockerError::CommandFailed(format!("Failed to parse version: {}", e)))
    }
}

#[async_trait]
impl ContainerOrchestrator for DockerExecutor {
    async fn list_containers(&self, server: &Server) -> CoreResult<Vec<Container>> {
        let result = self.ssh.execute(
            server,
            "docker ps -a --format '{{json .}}'"
        ).await?;

        let containers = result.output.stdout
            .lines()
            .filter_map(|line| serde_json::from_str::<DockerPsOutput>(line).ok())
            .map(|c| Container {
                id: c.id,
                name: c.names,
                image: c.image,
                status: c.status,
                created_at: c.created_at,
                labels: HashMap::new(),
            })
            .collect();

        Ok(containers)
    }

    async fn start_container(&self, server: &Server, container_id: &str) -> CoreResult<()> {
        DockerExecutor::start_container(self, server, container_id).await?;
        Ok(())
    }

    async fn stop_container(&self, server: &Server, container_id: &str) -> CoreResult<()> {
        DockerExecutor::stop_container(self, server, container_id, None).await?;
        Ok(())
    }

    async fn remove_container(&self, server: &Server, container_id: &str) -> CoreResult<()> {
        DockerExecutor::remove_container(self, server, container_id, true, false).await?;
        Ok(())
    }

    async fn container_logs(
        &self,
        server: &Server,
        container_id: &str,
        tail: Option<usize>,
    ) -> CoreResult<String> {
        let logs = DockerExecutor::logs(self, server, container_id, tail, None).await?;
        Ok(logs)
    }

    async fn exec_in_container(
        &self,
        server: &Server,
        container_id: &str,
        command: &str,
    ) -> CoreResult<CommandOutput> {
        let output = DockerExecutor::exec(self, server, container_id, command, false).await?;
        Ok(CommandOutput {
            stdout: output.stdout,
            stderr: output.stderr,
            exit_code: output.exit_code,
        })
    }
}

// ===== Types =====

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct DockerPsOutput {
    #[serde(rename = "ID")]
    id: String,
    names: String,
    image: String,
    status: String,
    #[serde(default)]
    created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerStats {
    #[serde(rename = "Container")]
    pub container: String,
    #[serde(rename = "CPUPerc")]
    pub cpu_perc: String,
    #[serde(rename = "MemUsage")]
    pub mem_usage: String,
    #[serde(rename = "MemPerc")]
    pub mem_perc: String,
    #[serde(rename = "NetIO")]
    pub net_io: String,
    #[serde(rename = "BlockIO")]
    pub block_io: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerInspect {
    pub id: String,
    pub name: String,
    pub state: ContainerState,
    pub config: ContainerInspectConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerState {
    pub status: String,
    pub running: bool,
    pub paused: bool,
    pub restarting: bool,
    #[serde(rename = "OOMKilled")]
    pub oom_killed: bool,
    pub exit_code: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerInspectConfig {
    pub image: String,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DockerVersion {
    pub client: DockerVersionInfo,
    pub server: DockerVersionInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DockerVersionInfo {
    pub version: String,
    pub api_version: String,
    pub os: String,
    pub arch: String,
}
