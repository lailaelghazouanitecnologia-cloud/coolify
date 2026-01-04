//! Dockerfile Generator Action
//!
//! Generates optimized Dockerfiles for various application types.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::actions::{Action, ActionError, ActionResult};

/// Build pack types for application deployment
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BuildPackType {
    /// Auto-detect using Nixpacks
    Nixpacks,
    /// Use existing Dockerfile
    Dockerfile,
    /// Docker Compose
    DockerCompose,
    /// Docker Image (pre-built)
    DockerImage,
    /// Static file serving
    Static,
}

impl std::fmt::Display for BuildPackType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildPackType::Nixpacks => write!(f, "nixpacks"),
            BuildPackType::Dockerfile => write!(f, "dockerfile"),
            BuildPackType::DockerCompose => write!(f, "dockercompose"),
            BuildPackType::DockerImage => write!(f, "dockerimage"),
            BuildPackType::Static => write!(f, "static"),
        }
    }
}

/// Input for Dockerfile generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerfileGenerateInput {
    /// Application ID
    pub application_id: Uuid,
    /// Build pack type
    pub build_pack: BuildPackType,
    /// Base directory containing source code
    pub workdir: String,
    /// Custom Dockerfile location (relative to workdir)
    pub dockerfile_location: Option<String>,
    /// Target image name
    pub image_name: String,
    /// Base image to use (for custom Dockerfiles)
    pub base_image: Option<String>,
    /// Build arguments
    pub build_args: HashMap<String, String>,
    /// Environment variables
    pub env_vars: HashMap<String, String>,
    /// Exposed ports
    pub ports: Vec<u16>,
    /// Custom build command
    pub build_command: Option<String>,
    /// Custom start command
    pub start_command: Option<String>,
    /// Additional Dockerfile content
    pub additional_dockerfile: Option<String>,
    /// Whether to use multi-stage build
    pub multi_stage: bool,
    /// Whether this is a static site
    pub is_static: bool,
    /// Static image for serving (e.g., nginx:alpine)
    pub static_image: Option<String>,
    /// Custom nginx configuration
    pub nginx_conf: Option<String>,
}

/// Output from Dockerfile generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerfileGenerateOutput {
    /// Generated Dockerfile content
    pub dockerfile: String,
    /// Path where Dockerfile was written
    pub dockerfile_path: String,
    /// Docker build command
    pub build_command: String,
    /// Detected or configured ports
    pub ports: Vec<u16>,
    /// Build arguments used
    pub build_args: HashMap<String, String>,
    /// Success status
    pub success: bool,
    /// Message
    pub message: String,
}

/// Dockerfile generator action
pub struct DockerfileGenerate;

impl DockerfileGenerate {
    pub fn new() -> Self {
        Self
    }

    /// Generate Dockerfile for a Node.js application
    pub fn generate_node_dockerfile(&self, input: &DockerfileGenerateInput) -> String {
        let node_version = input.build_args
            .get("NODE_VERSION")
            .cloned()
            .unwrap_or_else(|| "22".to_string());

        let mut dockerfile = format!(
            r#"# Build stage
FROM node:{node_version}-alpine AS builder

WORKDIR /app

# Copy package files
COPY package*.json ./

# Install dependencies
RUN npm ci --only=production

# Copy source code
COPY . .

"#
        );

        // Add build command if provided
        if let Some(ref build_cmd) = input.build_command {
            dockerfile.push_str(&format!(
                r#"# Build application
RUN {}

"#,
                build_cmd
            ));
        }

        // Add multi-stage production image
        if input.multi_stage {
            dockerfile.push_str(&format!(
                r#"# Production stage
FROM node:{}-alpine

WORKDIR /app

# Copy built files
COPY --from=builder /app ./

"#,
                node_version
            ));
        }

        // Add environment variables
        if !input.env_vars.is_empty() {
            dockerfile.push_str("# Environment variables\n");
            for (key, value) in &input.env_vars {
                dockerfile.push_str(&format!("ENV {}=\"{}\"\n", key, value));
            }
            dockerfile.push('\n');
        }

        // Expose ports
        if !input.ports.is_empty() {
            for port in &input.ports {
                dockerfile.push_str(&format!("EXPOSE {}\n", port));
            }
            dockerfile.push('\n');
        }

        // Start command
        let start_cmd = input.start_command.as_deref().unwrap_or("node server.js");
        dockerfile.push_str(&format!(
            r#"# Start application
CMD ["sh", "-c", "{}"]
"#,
            start_cmd
        ));

        dockerfile
    }

    /// Generate Dockerfile for a Python application
    pub fn generate_python_dockerfile(&self, input: &DockerfileGenerateInput) -> String {
        let python_version = input.build_args
            .get("PYTHON_VERSION")
            .cloned()
            .unwrap_or_else(|| "3.12".to_string());

        let mut dockerfile = format!(
            r#"# Python application
FROM python:{python_version}-slim

WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy requirements and install dependencies
COPY requirements.txt* ./
RUN pip install --no-cache-dir -r requirements.txt || true

# Copy application code
COPY . .

"#
        );

        // Add environment variables
        if !input.env_vars.is_empty() {
            dockerfile.push_str("# Environment variables\n");
            for (key, value) in &input.env_vars {
                dockerfile.push_str(&format!("ENV {}=\"{}\"\n", key, value));
            }
            dockerfile.push('\n');
        }

        // Expose ports
        if !input.ports.is_empty() {
            for port in &input.ports {
                dockerfile.push_str(&format!("EXPOSE {}\n", port));
            }
            dockerfile.push('\n');
        }

        // Start command
        let start_cmd = input.start_command.as_deref().unwrap_or("python app.py");
        dockerfile.push_str(&format!(
            r#"# Start application
CMD ["sh", "-c", "{}"]
"#,
            start_cmd
        ));

        dockerfile
    }

    /// Generate Dockerfile for a static site
    pub fn generate_static_dockerfile(&self, input: &DockerfileGenerateInput) -> String {
        let static_image = input.static_image.as_deref().unwrap_or("nginx:alpine");

        let mut dockerfile = String::new();

        // If build command is provided, use a build stage
        if input.build_command.is_some() {
            dockerfile.push_str(
                r#"# Build stage
FROM node:22-alpine AS builder

WORKDIR /app

COPY package*.json ./
RUN npm ci

COPY . .

"#,
            );

            if let Some(ref build_cmd) = input.build_command {
                dockerfile.push_str(&format!("RUN {}\n\n", build_cmd));
            }
        }

        // Static serving stage
        dockerfile.push_str(&format!(
            r#"# Serve stage
FROM {}

"#,
            static_image
        ));

        // Custom nginx configuration if provided
        if let Some(ref nginx_conf) = input.nginx_conf {
            dockerfile.push_str("# Copy custom nginx configuration\n");
            dockerfile.push_str("COPY nginx.conf /etc/nginx/nginx.conf\n\n");
        } else {
            // Default nginx configuration for SPA
            dockerfile.push_str(
                r#"# Default nginx configuration for SPA
RUN echo 'server { \
    listen 80; \
    root /usr/share/nginx/html; \
    index index.html; \
    location / { \
        try_files $uri $uri/ /index.html; \
    } \
}' > /etc/nginx/conf.d/default.conf

"#,
            );
        }

        // Copy built files
        if input.build_command.is_some() {
            let dist_dir = input.build_args
                .get("DIST_DIR")
                .cloned()
                .unwrap_or_else(|| "dist".to_string());
            dockerfile.push_str(&format!(
                "COPY --from=builder /app/{} /usr/share/nginx/html\n\n",
                dist_dir
            ));
        } else {
            dockerfile.push_str("COPY . /usr/share/nginx/html\n\n");
        }

        // Expose port
        dockerfile.push_str("EXPOSE 80\n\n");

        // Start nginx
        dockerfile.push_str(r#"CMD ["nginx", "-g", "daemon off;"]
"#);

        dockerfile
    }

    /// Generate Dockerfile for a Rust application
    pub fn generate_rust_dockerfile(&self, input: &DockerfileGenerateInput) -> String {
        let rust_version = input.build_args
            .get("RUST_VERSION")
            .cloned()
            .unwrap_or_else(|| "1.75".to_string());

        let mut dockerfile = format!(
            r#"# Build stage
FROM rust:{rust_version} AS builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock* ./

# Create dummy source for dependency caching
RUN mkdir src && echo "fn main() {{}}" > src/main.rs

# Build dependencies only
RUN cargo build --release && rm -rf src

# Copy actual source
COPY . .

# Build application
RUN touch src/main.rs && cargo build --release

# Production stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/app ./app

"#
        );

        // Add environment variables
        if !input.env_vars.is_empty() {
            dockerfile.push_str("# Environment variables\n");
            for (key, value) in &input.env_vars {
                dockerfile.push_str(&format!("ENV {}=\"{}\"\n", key, value));
            }
            dockerfile.push('\n');
        }

        // Expose ports
        if !input.ports.is_empty() {
            for port in &input.ports {
                dockerfile.push_str(&format!("EXPOSE {}\n", port));
            }
            dockerfile.push('\n');
        }

        // Start command
        let start_cmd = input.start_command.as_deref().unwrap_or("./app");
        dockerfile.push_str(&format!(
            r#"# Start application
CMD ["{}"]
"#,
            start_cmd
        ));

        dockerfile
    }

    /// Generate Dockerfile for a Go application
    pub fn generate_go_dockerfile(&self, input: &DockerfileGenerateInput) -> String {
        let go_version = input.build_args
            .get("GO_VERSION")
            .cloned()
            .unwrap_or_else(|| "1.22".to_string());

        let mut dockerfile = format!(
            r#"# Build stage
FROM golang:{go_version}-alpine AS builder

WORKDIR /app

# Install git for go modules
RUN apk add --no-cache git

# Copy go mod files
COPY go.mod go.sum* ./

# Download dependencies
RUN go mod download

# Copy source
COPY . .

# Build
RUN CGO_ENABLED=0 GOOS=linux go build -a -installsuffix cgo -o app .

# Production stage
FROM alpine:latest

RUN apk --no-cache add ca-certificates

WORKDIR /app

COPY --from=builder /app/app .

"#
        );

        // Add environment variables
        if !input.env_vars.is_empty() {
            dockerfile.push_str("# Environment variables\n");
            for (key, value) in &input.env_vars {
                dockerfile.push_str(&format!("ENV {}=\"{}\"\n", key, value));
            }
            dockerfile.push('\n');
        }

        // Expose ports
        if !input.ports.is_empty() {
            for port in &input.ports {
                dockerfile.push_str(&format!("EXPOSE {}\n", port));
            }
            dockerfile.push('\n');
        }

        // Start command
        dockerfile.push_str(r#"CMD ["./app"]
"#);

        dockerfile
    }

    /// Generate docker build command
    pub fn generate_build_command(&self, input: &DockerfileGenerateInput) -> String {
        let dockerfile_path = input.dockerfile_location.as_deref().unwrap_or("Dockerfile");

        let mut cmd = format!(
            "DOCKER_BUILDKIT=1 docker build --network host -f {} --progress plain",
            dockerfile_path
        );

        // Add build args
        for (key, value) in &input.build_args {
            cmd.push_str(&format!(" --build-arg {}=\"{}\"", key, value));
        }

        cmd.push_str(&format!(" -t {} {}", input.image_name, input.workdir));

        cmd
    }

    /// Select and generate appropriate Dockerfile based on detection
    pub fn generate_dockerfile(&self, input: &DockerfileGenerateInput) -> String {
        // If user provides additional dockerfile content, use it as base
        if let Some(ref additional) = input.additional_dockerfile {
            return additional.clone();
        }

        if input.is_static {
            return self.generate_static_dockerfile(input);
        }

        // Detect based on files in workdir or build args
        // In real implementation, this would check actual files
        let detected_type = input.build_args
            .get("APP_TYPE")
            .cloned()
            .unwrap_or_else(|| "node".to_string());

        match detected_type.as_str() {
            "node" | "nodejs" => self.generate_node_dockerfile(input),
            "python" => self.generate_python_dockerfile(input),
            "rust" => self.generate_rust_dockerfile(input),
            "go" | "golang" => self.generate_go_dockerfile(input),
            "static" => self.generate_static_dockerfile(input),
            _ => self.generate_node_dockerfile(input), // Default to Node.js
        }
    }
}

impl Default for DockerfileGenerate {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Action for DockerfileGenerate {
    type Input = DockerfileGenerateInput;
    type Output = DockerfileGenerateOutput;

    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        tracing::info!(
            application_id = %input.application_id,
            build_pack = %input.build_pack,
            image = %input.image_name,
            "Generating Dockerfile"
        );

        let dockerfile = self.generate_dockerfile(&input);
        let dockerfile_path = input.dockerfile_location
            .clone()
            .unwrap_or_else(|| format!("{}/Dockerfile", input.workdir));
        let build_command = self.generate_build_command(&input);

        Ok(DockerfileGenerateOutput {
            dockerfile,
            dockerfile_path,
            build_command,
            ports: input.ports.clone(),
            build_args: input.build_args.clone(),
            success: true,
            message: "Dockerfile generated successfully".to_string(),
        })
    }

    fn name(&self) -> &'static str {
        "dockerfile_generate"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_node_dockerfile() {
        let action = DockerfileGenerate::new();
        let input = DockerfileGenerateInput {
            application_id: Uuid::new_v4(),
            build_pack: BuildPackType::Dockerfile,
            workdir: "/app".to_string(),
            dockerfile_location: None,
            image_name: "my-app:latest".to_string(),
            base_image: None,
            build_args: HashMap::new(),
            env_vars: HashMap::from([("NODE_ENV".to_string(), "production".to_string())]),
            ports: vec![3000],
            build_command: Some("npm run build".to_string()),
            start_command: Some("npm start".to_string()),
            additional_dockerfile: None,
            multi_stage: true,
            is_static: false,
            static_image: None,
            nginx_conf: None,
        };

        let dockerfile = action.generate_node_dockerfile(&input);
        assert!(dockerfile.contains("FROM node:22-alpine"));
        assert!(dockerfile.contains("npm ci"));
        assert!(dockerfile.contains("npm run build"));
        assert!(dockerfile.contains("EXPOSE 3000"));
    }

    #[test]
    fn test_generate_static_dockerfile() {
        let action = DockerfileGenerate::new();
        let input = DockerfileGenerateInput {
            application_id: Uuid::new_v4(),
            build_pack: BuildPackType::Static,
            workdir: "/app".to_string(),
            dockerfile_location: None,
            image_name: "my-static:latest".to_string(),
            base_image: None,
            build_args: HashMap::new(),
            env_vars: HashMap::new(),
            ports: vec![80],
            build_command: Some("npm run build".to_string()),
            start_command: None,
            additional_dockerfile: None,
            multi_stage: false,
            is_static: true,
            static_image: Some("nginx:alpine".to_string()),
            nginx_conf: None,
        };

        let dockerfile = action.generate_static_dockerfile(&input);
        assert!(dockerfile.contains("FROM nginx:alpine"));
        assert!(dockerfile.contains("npm run build"));
        assert!(dockerfile.contains("EXPOSE 80"));
    }
}
