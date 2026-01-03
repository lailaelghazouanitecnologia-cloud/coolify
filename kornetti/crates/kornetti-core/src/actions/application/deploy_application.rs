//! Deploy Application Action
//!
//! Handles the complete deployment process for an application.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::actions::{Action, ActionError, ActionResult, CommandBuilder};
use super::{DeploymentConfig, generate_traefik_labels};

/// Input for deploying an application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployApplicationInput {
    /// Application ID
    pub application_id: Uuid,
    /// Server ID
    pub server_id: Uuid,
    /// Container name
    pub container_name: String,
    /// Docker image to deploy
    pub image: String,
    /// Optional image tag (default: latest)
    pub tag: Option<String>,
    /// Environment variables
    pub env_vars: Vec<(String, String)>,
    /// Port mappings (host:container)
    pub ports: Vec<(u16, u16)>,
    /// Volume mounts
    pub volumes: Vec<(String, String)>,
    /// Domains for routing
    pub domains: Vec<String>,
    /// Container port for proxy
    pub container_port: u16,
    /// Enable HTTPS
    pub https: bool,
    /// Force HTTPS redirect
    pub force_https: bool,
    /// Memory limit (e.g., "512m", "1g")
    pub memory_limit: Option<String>,
    /// CPU limit (e.g., 0.5, 1.0)
    pub cpu_limit: Option<f64>,
    /// Health check enabled
    pub health_check_enabled: bool,
    /// Health check path
    pub health_check_path: Option<String>,
    /// Number of replicas
    pub replicas: u32,
    /// Docker network
    pub network: String,
    /// Working directory for deployment
    pub workdir: String,
}

/// Output from deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployApplicationOutput {
    /// Deployed container ID
    pub container_id: String,
    /// Container name
    pub container_name: String,
    /// Whether deployment was successful
    pub success: bool,
    /// Deployment message
    pub message: String,
    /// Domains configured
    pub domains: Vec<String>,
    /// Port mappings
    pub ports: Vec<(u16, u16)>,
}

/// Deploy application action
pub struct DeployApplication;

impl DeployApplication {
    pub fn new() -> Self {
        Self
    }

    /// Generate docker-compose.yml content
    pub fn generate_compose(&self, input: &DeployApplicationInput) -> String {
        let uuid = input.application_id.to_string();
        let image_with_tag = match &input.tag {
            Some(tag) => format!("{}:{}", input.image, tag),
            None => format!("{}:latest", input.image),
        };

        // Generate Traefik labels
        let traefik_labels = generate_traefik_labels(
            &uuid,
            &input.domains,
            input.container_port,
            input.https,
            input.force_https,
        );

        // Build compose content
        let mut compose = format!(r#"version: '3.8'

services:
  {}:
    image: {}
    container_name: {}
    restart: unless-stopped
    networks:
      - {}
"#,
            input.container_name,
            image_with_tag,
            input.container_name,
            input.network
        );

        // Labels
        if !traefik_labels.is_empty() {
            compose.push_str("    labels:\n");
            for (key, value) in &traefik_labels {
                compose.push_str(&format!("      - \"{}={}\"\n", key, value));
            }
            compose.push_str(&format!("      - \"kornetti.applicationId={}\"\n", uuid));
            compose.push_str("      - \"kornetti.managed=true\"\n");
        }

        // Ports
        if !input.ports.is_empty() {
            compose.push_str("    ports:\n");
            for (host, container) in &input.ports {
                compose.push_str(&format!("      - \"{}:{}\"\n", host, container));
            }
        }

        // Volumes
        if !input.volumes.is_empty() {
            compose.push_str("    volumes:\n");
            for (host, container) in &input.volumes {
                compose.push_str(&format!("      - \"{}:{}\"\n", host, container));
            }
        }

        // Environment variables
        if !input.env_vars.is_empty() {
            compose.push_str("    environment:\n");
            for (key, value) in &input.env_vars {
                // Escape special characters in values
                let escaped_value = value.replace('\"', "\\\"");
                compose.push_str(&format!("      - \"{}={}\"\n", key, escaped_value));
            }
        }

        // Health check
        if input.health_check_enabled {
            let path = input.health_check_path.as_deref().unwrap_or("/");
            compose.push_str(&format!(r#"    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:{}{}"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
"#, input.container_port, path));
        }

        // Resource limits
        if input.memory_limit.is_some() || input.cpu_limit.is_some() {
            compose.push_str("    deploy:\n      resources:\n        limits:\n");
            if let Some(ref mem) = input.memory_limit {
                compose.push_str(&format!("          memory: {}\n", mem));
            }
            if let Some(cpus) = input.cpu_limit {
                compose.push_str(&format!("          cpus: '{}'\n", cpus));
            }
        }

        // Network definition
        compose.push_str(&format!(r#"
networks:
  {}:
    external: true
"#, input.network));

        compose
    }

    /// Generate deployment commands
    pub fn generate_commands(&self, input: &DeployApplicationInput) -> Vec<String> {
        let mut builder = CommandBuilder::new();
        let workdir = &input.workdir;
        let compose_file = format!("{}/docker-compose.yml", workdir);

        // Ensure workdir exists
        builder.mkdir(workdir);

        // Pull latest image
        let image_with_tag = match &input.tag {
            Some(tag) => format!("{}:{}", input.image, tag),
            None => format!("{}:latest", input.image),
        };
        builder.add(format!("docker pull {}", image_with_tag));

        // Stop existing container if running
        builder.add(format!(
            "docker stop {} 2>/dev/null || true",
            input.container_name
        ));
        builder.add(format!(
            "docker rm {} 2>/dev/null || true",
            input.container_name
        ));

        // Write compose file (will be done separately)
        // Deploy with compose
        builder.add(format!(
            "cd {} && docker compose -f docker-compose.yml up -d --remove-orphans",
            workdir
        ));

        // Wait for container to be healthy
        if input.health_check_enabled {
            builder.add(format!(
                "timeout 120 sh -c 'until docker inspect --format=\"{{{{.State.Health.Status}}}}\" {} 2>/dev/null | grep -q healthy; do sleep 2; done' || true",
                input.container_name
            ));
        } else {
            // Just wait a bit for container to start
            builder.add(format!(
                "timeout 30 sh -c 'until docker inspect --format=\"{{{{.State.Running}}}}\" {} 2>/dev/null | grep -q true; do sleep 1; done'",
                input.container_name
            ));
        }

        // Get container ID
        builder.add(format!(
            "docker inspect --format='{{{{.Id}}}}' {}",
            input.container_name
        ));

        builder.build()
    }
}

impl Default for DeployApplication {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Action for DeployApplication {
    type Input = DeployApplicationInput;
    type Output = DeployApplicationOutput;

    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        tracing::info!(
            application_id = %input.application_id,
            container = %input.container_name,
            image = %input.image,
            "Deploying application"
        );

        // Generate compose file
        let compose_content = self.generate_compose(&input);

        // Generate commands
        let commands = self.generate_commands(&input);

        // TODO: Execute commands via SSH
        // 1. Write compose file to server
        // 2. Execute deployment commands
        // 3. Verify container is running

        tracing::info!(
            container = %input.container_name,
            domains = ?input.domains,
            "Application deployment initiated"
        );

        Ok(DeployApplicationOutput {
            container_id: String::new(), // Would be filled from actual deployment
            container_name: input.container_name,
            success: true,
            message: "Deployment initiated".to_string(),
            domains: input.domains,
            ports: input.ports,
        })
    }

    fn name(&self) -> &'static str {
        "deploy_application"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_compose() {
        let action = DeployApplication::new();
        let input = DeployApplicationInput {
            application_id: Uuid::new_v4(),
            server_id: Uuid::new_v4(),
            container_name: "my-app".to_string(),
            image: "nginx".to_string(),
            tag: Some("alpine".to_string()),
            env_vars: vec![("PORT".to_string(), "3000".to_string())],
            ports: vec![(8080, 80)],
            volumes: vec![],
            domains: vec!["example.com".to_string()],
            container_port: 80,
            https: true,
            force_https: true,
            memory_limit: Some("512m".to_string()),
            cpu_limit: Some(0.5),
            health_check_enabled: true,
            health_check_path: Some("/health".to_string()),
            replicas: 1,
            network: "coolify".to_string(),
            workdir: "/data/coolify/applications/my-app".to_string(),
        };

        let compose = action.generate_compose(&input);
        assert!(compose.contains("nginx:alpine"));
        assert!(compose.contains("traefik.enable"));
        assert!(compose.contains("memory: 512m"));
        assert!(compose.contains("healthcheck"));
    }
}
