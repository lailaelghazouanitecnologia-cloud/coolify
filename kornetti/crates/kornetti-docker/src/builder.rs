//! Docker image building
//!
//! Provides functionality for building Docker images from Dockerfiles
//! or from application source code with automatic Dockerfile generation.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use tracing::{debug, info, instrument};
use uuid::Uuid;

use kornetti_core::models::Server;
use kornetti_ssh::{SshClient, Command, CommandBatch};
use crate::error::{Result, DockerError};

/// Docker image builder
pub struct ImageBuilder {
    ssh: Arc<SshClient>,
}

impl ImageBuilder {
    pub fn new(ssh: Arc<SshClient>) -> Self {
        Self { ssh }
    }

    /// Build an image on a remote server
    #[instrument(skip(self, server, config))]
    pub async fn build(
        &self,
        server: &Server,
        config: &BuildConfig,
    ) -> Result<BuildResult> {
        info!(
            image = %config.image_name,
            context = %config.context_path,
            "Building Docker image"
        );

        let build_id = Uuid::new_v4();
        let mut cmd = vec!["docker", "build"];

        // Tag
        cmd.push("-t");
        cmd.push(&config.image_name);

        // Dockerfile
        if let Some(dockerfile) = &config.dockerfile_path {
            cmd.push("-f");
            cmd.push(dockerfile);
        }

        // Build args
        for (key, value) in &config.build_args {
            cmd.push("--build-arg");
            cmd.push(&format!("{}={}", key, value));
        }

        // Labels
        for (key, value) in &config.labels {
            cmd.push("--label");
            cmd.push(&format!("{}={}", key, value));
        }

        // Target stage
        if let Some(target) = &config.target {
            cmd.push("--target");
            cmd.push(target);
        }

        // Cache options
        if config.no_cache {
            cmd.push("--no-cache");
        }

        if config.pull {
            cmd.push("--pull");
        }

        // Platform
        if let Some(platform) = &config.platform {
            cmd.push("--platform");
            cmd.push(platform);
        }

        // Context path
        cmd.push(&config.context_path);

        let command = cmd.join(" ");
        debug!(command = %command, "Executing build command");

        let result = self.ssh.execute(server, &command).await
            .map_err(|e| DockerError::BuildFailed(e.to_string()))?;

        if !result.output.success() {
            return Err(DockerError::BuildFailed(result.output.stderr));
        }

        // Extract image ID from output
        let image_id = self.get_image_id(server, &config.image_name).await?;

        info!(
            image = %config.image_name,
            image_id = %image_id,
            "Image built successfully"
        );

        Ok(BuildResult {
            build_id,
            image_name: config.image_name.clone(),
            image_id,
            logs: result.output.combined(),
        })
    }

    /// Get image ID by name
    async fn get_image_id(&self, server: &Server, image_name: &str) -> Result<String> {
        let result = self.ssh.execute(
            server,
            &format!("docker images -q {}", image_name)
        ).await.map_err(|e| DockerError::CommandFailed(e.to_string()))?;

        if result.output.success() {
            Ok(result.output.stdout.trim().to_string())
        } else {
            Err(DockerError::ImageNotFound(image_name.to_string()))
        }
    }

    /// Pull an image from a registry
    #[instrument(skip(self, server))]
    pub async fn pull(&self, server: &Server, image: &str) -> Result<()> {
        info!(image = %image, "Pulling Docker image");

        let result = self.ssh.execute(
            server,
            &format!("docker pull {}", image)
        ).await.map_err(|e| DockerError::PullFailed(e.to_string()))?;

        if !result.output.success() {
            return Err(DockerError::PullFailed(result.output.stderr));
        }

        Ok(())
    }

    /// Push an image to a registry
    #[instrument(skip(self, server))]
    pub async fn push(&self, server: &Server, image: &str) -> Result<()> {
        info!(image = %image, "Pushing Docker image");

        let result = self.ssh.execute(
            server,
            &format!("docker push {}", image)
        ).await.map_err(|e| DockerError::CommandFailed(e.to_string()))?;

        if !result.output.success() {
            return Err(DockerError::CommandFailed(result.output.stderr));
        }

        Ok(())
    }

    /// Tag an image
    pub async fn tag(&self, server: &Server, source: &str, target: &str) -> Result<()> {
        let result = self.ssh.execute(
            server,
            &format!("docker tag {} {}", source, target)
        ).await.map_err(|e| DockerError::CommandFailed(e.to_string()))?;

        if !result.output.success() {
            return Err(DockerError::CommandFailed(result.output.stderr));
        }

        Ok(())
    }

    /// Remove an image
    pub async fn remove(&self, server: &Server, image: &str, force: bool) -> Result<()> {
        let cmd = if force {
            format!("docker rmi -f {}", image)
        } else {
            format!("docker rmi {}", image)
        };

        let result = self.ssh.execute(server, &cmd).await
            .map_err(|e| DockerError::CommandFailed(e.to_string()))?;

        if !result.output.success() {
            return Err(DockerError::CommandFailed(result.output.stderr));
        }

        Ok(())
    }

    /// Prune unused images
    pub async fn prune(&self, server: &Server, all: bool) -> Result<PruneResult> {
        let cmd = if all {
            "docker image prune -af"
        } else {
            "docker image prune -f"
        };

        let result = self.ssh.execute(server, cmd).await
            .map_err(|e| DockerError::CommandFailed(e.to_string()))?;

        // Parse space reclaimed from output
        let space_reclaimed = parse_prune_output(&result.output.stdout);

        Ok(PruneResult {
            images_deleted: 0, // Would need to parse
            space_reclaimed,
        })
    }

    /// Login to a Docker registry
    pub async fn login(
        &self,
        server: &Server,
        registry: &str,
        username: &str,
        password: &str,
    ) -> Result<()> {
        let cmd = format!(
            "echo {} | docker login {} -u {} --password-stdin",
            password, registry, username
        );

        let result = self.ssh.execute(server, &cmd).await
            .map_err(|e| DockerError::CommandFailed(e.to_string()))?;

        if !result.output.success() {
            return Err(DockerError::CommandFailed(result.output.stderr));
        }

        Ok(())
    }
}

/// Build configuration
#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub image_name: String,
    pub context_path: String,
    pub dockerfile_path: Option<String>,
    pub build_args: HashMap<String, String>,
    pub labels: HashMap<String, String>,
    pub target: Option<String>,
    pub platform: Option<String>,
    pub no_cache: bool,
    pub pull: bool,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            image_name: String::new(),
            context_path: ".".to_string(),
            dockerfile_path: None,
            build_args: HashMap::new(),
            labels: HashMap::new(),
            target: None,
            platform: None,
            no_cache: false,
            pull: false,
        }
    }
}

impl BuildConfig {
    pub fn new(image_name: impl Into<String>) -> Self {
        Self {
            image_name: image_name.into(),
            ..Default::default()
        }
    }

    pub fn context(mut self, path: impl Into<String>) -> Self {
        self.context_path = path.into();
        self
    }

    pub fn dockerfile(mut self, path: impl Into<String>) -> Self {
        self.dockerfile_path = Some(path.into());
        self
    }

    pub fn build_arg(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.build_args.insert(key.into(), value.into());
        self
    }

    pub fn label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    pub fn platform(mut self, platform: impl Into<String>) -> Self {
        self.platform = Some(platform.into());
        self
    }

    pub fn no_cache(mut self) -> Self {
        self.no_cache = true;
        self
    }

    pub fn pull(mut self) -> Self {
        self.pull = true;
        self
    }
}

/// Result of a build operation
#[derive(Debug, Clone)]
pub struct BuildResult {
    pub build_id: Uuid,
    pub image_name: String,
    pub image_id: String,
    pub logs: String,
}

/// Result of a prune operation
#[derive(Debug, Clone)]
pub struct PruneResult {
    pub images_deleted: usize,
    pub space_reclaimed: u64,
}

fn parse_prune_output(output: &str) -> u64 {
    // Parse "Total reclaimed space: 1.234GB" format
    for line in output.lines() {
        if line.contains("reclaimed space") {
            if let Some(size_str) = line.split(':').nth(1) {
                return parse_size(size_str.trim());
            }
        }
    }
    0
}

fn parse_size(s: &str) -> u64 {
    let s = s.to_uppercase();
    let multiplier = if s.ends_with("GB") {
        1024 * 1024 * 1024
    } else if s.ends_with("MB") {
        1024 * 1024
    } else if s.ends_with("KB") {
        1024
    } else if s.ends_with('B') {
        1
    } else {
        1
    };

    let num_str: String = s.chars().take_while(|c| c.is_numeric() || *c == '.').collect();
    num_str.parse::<f64>().unwrap_or(0.0) as u64 * multiplier
}

/// Dockerfile generator for common application types
pub struct DockerfileGenerator;

impl DockerfileGenerator {
    /// Generate Dockerfile for a Node.js application
    pub fn nodejs(node_version: &str, install_cmd: &str, build_cmd: &str, start_cmd: &str) -> String {
        format!(r#"FROM node:{node_version}-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN {install_cmd}
COPY . .
RUN {build_cmd}

FROM node:{node_version}-alpine
WORKDIR /app
COPY --from=builder /app .
EXPOSE 3000
CMD [{start_cmd}]
"#)
    }

    /// Generate Dockerfile for a static site
    pub fn static_site(build_output: &str) -> String {
        format!(r#"FROM nginx:alpine
COPY {build_output} /usr/share/nginx/html
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
"#)
    }

    /// Generate Dockerfile for a Python application
    pub fn python(python_version: &str) -> String {
        format!(r#"FROM python:{python_version}-slim
WORKDIR /app
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt
COPY . .
EXPOSE 8000
CMD ["python", "app.py"]
"#)
    }

    /// Generate Dockerfile for a Rust application
    pub fn rust() -> String {
        r#"FROM rust:alpine AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM alpine:latest
WORKDIR /app
COPY --from=builder /app/target/release/app .
EXPOSE 8080
CMD ["./app"]
"#.to_string()
    }

    /// Generate Dockerfile for a Go application
    pub fn golang() -> String {
        r#"FROM golang:alpine AS builder
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
"#.to_string()
    }
}
