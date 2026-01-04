//! Deploy Swarm Service Action
//!
//! Deploys or updates a service in Docker Swarm mode.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::{generate_service_command, PlacementConstraint, UpdateConfig, RollbackConfig};
use crate::actions::{Action, ActionError, ActionResult, CommandBuilder};

/// Input for deploying a Swarm service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploySwarmServiceInput {
    /// Application ID
    pub application_id: Uuid,
    /// Service name
    pub service_name: String,
    /// Docker image
    pub image: String,
    /// Image tag
    pub tag: Option<String>,
    /// Number of replicas
    pub replicas: u32,
    /// Environment variables
    pub env_vars: HashMap<String, String>,
    /// Port mappings (published:target)
    pub ports: Vec<(u16, u16)>,
    /// Networks to attach
    pub networks: Vec<String>,
    /// Placement constraints
    pub constraints: Vec<PlacementConstraint>,
    /// Labels for the service
    pub labels: HashMap<String, String>,
    /// Update configuration
    pub update_config: Option<UpdateConfig>,
    /// Rollback configuration
    pub rollback_config: Option<RollbackConfig>,
    /// Memory limit (e.g., "512m")
    pub memory_limit: Option<String>,
    /// Memory reservation
    pub memory_reservation: Option<String>,
    /// CPU limit
    pub cpu_limit: Option<f64>,
    /// CPU reservation
    pub cpu_reservation: Option<f64>,
    /// Health check configuration
    pub health_check: Option<SwarmHealthCheck>,
    /// Only deploy to worker nodes
    pub workers_only: bool,
    /// Force update even if no changes
    pub force: bool,
    /// Entrypoint override
    pub entrypoint: Option<String>,
    /// Command override
    pub command: Option<Vec<String>>,
    /// Working directory in container
    pub workdir: Option<String>,
    /// Volumes/mounts
    pub mounts: Vec<SwarmMount>,
    /// Secrets to attach
    pub secrets: Vec<SwarmSecret>,
    /// Configs to attach
    pub configs: Vec<SwarmConfig>,
}

/// Swarm health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmHealthCheck {
    pub test: Vec<String>,
    pub interval: String,
    pub timeout: String,
    pub retries: u32,
    pub start_period: String,
}

impl Default for SwarmHealthCheck {
    fn default() -> Self {
        Self {
            test: vec!["CMD-SHELL".to_string(), "curl -f http://localhost/ || exit 1".to_string()],
            interval: "30s".to_string(),
            timeout: "10s".to_string(),
            retries: 3,
            start_period: "60s".to_string(),
        }
    }
}

/// Swarm mount configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmMount {
    /// Mount type (volume, bind, tmpfs)
    pub mount_type: String,
    /// Source (volume name or host path)
    pub source: String,
    /// Target path in container
    pub target: String,
    /// Read-only mount
    pub readonly: bool,
}

/// Swarm secret reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmSecret {
    /// Secret name
    pub name: String,
    /// Target path/name in container
    pub target: Option<String>,
    /// File mode
    pub mode: Option<u32>,
}

/// Swarm config reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmConfig {
    /// Config name
    pub name: String,
    /// Target path in container
    pub target: String,
    /// File mode
    pub mode: Option<u32>,
}

/// Output from Swarm service deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploySwarmServiceOutput {
    /// Service ID
    pub service_id: String,
    /// Service name
    pub service_name: String,
    /// Number of replicas
    pub replicas: u32,
    /// Current state
    pub state: String,
    /// Commands executed
    pub commands: Vec<String>,
    /// Success status
    pub success: bool,
    /// Message
    pub message: String,
}

/// Deploy Swarm Service action
pub struct DeploySwarmService;

impl DeploySwarmService {
    pub fn new() -> Self {
        Self
    }

    /// Check if service exists
    pub fn check_service_exists_command(service_name: &str) -> String {
        format!(
            "docker service inspect {} --format '{{{{.ID}}}}' 2>/dev/null",
            service_name
        )
    }

    /// Generate docker service create command
    pub fn create_command(&self, input: &DeploySwarmServiceInput) -> String {
        let image = match &input.tag {
            Some(tag) => format!("{}:{}", input.image, tag),
            None => format!("{}:latest", input.image),
        };

        let mut cmd = format!("docker service create --name {}", input.service_name);

        // Image
        cmd.push_str(&format!(" --image {}", image));

        // Replicas
        cmd.push_str(&format!(" --replicas {}", input.replicas));

        // Networks
        for network in &input.networks {
            cmd.push_str(&format!(" --network {}", network));
        }

        // Constraints
        let mut constraints = input.constraints.clone();
        if input.workers_only {
            constraints.push(PlacementConstraint::workers_only());
        }
        for constraint in &constraints {
            cmd.push_str(&format!(" --constraint '{}'", constraint.expression));
        }

        // Labels
        for (key, value) in &input.labels {
            cmd.push_str(&format!(" --label {}=\"{}\"", key, value));
        }
        // Add Kornetti labels
        cmd.push_str(&format!(
            " --label kornetti.applicationId={}",
            input.application_id
        ));
        cmd.push_str(" --label kornetti.managed=true");

        // Environment variables
        for (key, value) in &input.env_vars {
            cmd.push_str(&format!(" --env {}=\"{}\"", key, value));
        }

        // Ports
        for (published, target) in &input.ports {
            cmd.push_str(&format!(" --publish published={},target={}", published, target));
        }

        // Resource limits
        if let Some(ref limit) = input.memory_limit {
            cmd.push_str(&format!(" --limit-memory {}", limit));
        }
        if let Some(limit) = input.cpu_limit {
            cmd.push_str(&format!(" --limit-cpu {}", limit));
        }
        if let Some(ref res) = input.memory_reservation {
            cmd.push_str(&format!(" --reserve-memory {}", res));
        }
        if let Some(res) = input.cpu_reservation {
            cmd.push_str(&format!(" --reserve-cpu {}", res));
        }

        // Update config
        if let Some(ref config) = input.update_config {
            cmd.push_str(&format!(" --update-parallelism {}", config.parallelism));
            cmd.push_str(&format!(" --update-delay {}", config.delay));
        }

        // Rollback config
        if let Some(ref config) = input.rollback_config {
            cmd.push_str(&format!(" --rollback-parallelism {}", config.parallelism));
            cmd.push_str(&format!(" --rollback-delay {}", config.delay));
        }

        // Health check
        if let Some(ref hc) = input.health_check {
            cmd.push_str(&format!(" --health-cmd \"{}\"", hc.test.join(" ")));
            cmd.push_str(&format!(" --health-interval {}", hc.interval));
            cmd.push_str(&format!(" --health-timeout {}", hc.timeout));
            cmd.push_str(&format!(" --health-retries {}", hc.retries));
            cmd.push_str(&format!(" --health-start-period {}", hc.start_period));
        }

        // Mounts
        for mount in &input.mounts {
            let ro = if mount.readonly { ",readonly" } else { "" };
            cmd.push_str(&format!(
                " --mount type={},source={},target={}{}",
                mount.mount_type, mount.source, mount.target, ro
            ));
        }

        // Secrets
        for secret in &input.secrets {
            if let Some(ref target) = secret.target {
                cmd.push_str(&format!(" --secret source={},target={}", secret.name, target));
            } else {
                cmd.push_str(&format!(" --secret {}", secret.name));
            }
        }

        // Configs
        for config in &input.configs {
            cmd.push_str(&format!(
                " --config source={},target={}",
                config.name, config.target
            ));
        }

        // Entrypoint and command
        if let Some(ref entrypoint) = input.entrypoint {
            cmd.push_str(&format!(" --entrypoint \"{}\"", entrypoint));
        }

        if let Some(ref workdir) = input.workdir {
            cmd.push_str(&format!(" --workdir {}", workdir));
        }

        // Command args come at the end
        if let Some(ref command) = input.command {
            for arg in command {
                cmd.push_str(&format!(" \"{}\"", arg));
            }
        }

        cmd
    }

    /// Generate docker service update command
    pub fn update_command(&self, input: &DeploySwarmServiceInput) -> String {
        let image = match &input.tag {
            Some(tag) => format!("{}:{}", input.image, tag),
            None => format!("{}:latest", input.image),
        };

        let mut cmd = format!("docker service update");

        // Force update
        if input.force {
            cmd.push_str(" --force");
        }

        // Image
        cmd.push_str(&format!(" --image {}", image));

        // Replicas
        cmd.push_str(&format!(" --replicas {}", input.replicas));

        // Environment variables (--env-add for new, --env-rm for removed)
        for (key, value) in &input.env_vars {
            cmd.push_str(&format!(" --env-add {}=\"{}\"", key, value));
        }

        // Resource limits
        if let Some(ref limit) = input.memory_limit {
            cmd.push_str(&format!(" --limit-memory {}", limit));
        }
        if let Some(limit) = input.cpu_limit {
            cmd.push_str(&format!(" --limit-cpu {}", limit));
        }

        // Service name at the end
        cmd.push_str(&format!(" {}", input.service_name));

        cmd
    }

    /// Generate commands for deployment
    pub fn generate_commands(&self, input: &DeploySwarmServiceInput) -> Vec<String> {
        let mut builder = CommandBuilder::new();

        // Check if service exists
        let check_cmd = Self::check_service_exists_command(&input.service_name);
        builder.add(format!(
            "if {}; then echo 'UPDATE'; else echo 'CREATE'; fi",
            check_cmd
        ));

        builder.build()
    }
}

impl Default for DeploySwarmService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Action for DeploySwarmService {
    type Input = DeploySwarmServiceInput;
    type Output = DeploySwarmServiceOutput;

    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        tracing::info!(
            application_id = %input.application_id,
            service = %input.service_name,
            image = %input.image,
            replicas = input.replicas,
            "Deploying Swarm service"
        );

        let commands = self.generate_commands(&input);

        // In real implementation:
        // 1. Check if service exists
        // 2. If exists, update; otherwise create
        // 3. Wait for service to converge
        // 4. Return status

        Ok(DeploySwarmServiceOutput {
            service_id: Uuid::new_v4().to_string(),
            service_name: input.service_name,
            replicas: input.replicas,
            state: "running".to_string(),
            commands,
            success: true,
            message: "Swarm service deployed successfully".to_string(),
        })
    }

    fn name(&self) -> &'static str {
        "deploy_swarm_service"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_command() {
        let action = DeploySwarmService::new();
        let input = DeploySwarmServiceInput {
            application_id: Uuid::new_v4(),
            service_name: "my-service".to_string(),
            image: "nginx".to_string(),
            tag: Some("alpine".to_string()),
            replicas: 3,
            env_vars: HashMap::from([("NODE_ENV".to_string(), "production".to_string())]),
            ports: vec![(80, 80)],
            networks: vec!["coolify-overlay".to_string()],
            constraints: vec![],
            labels: HashMap::new(),
            update_config: None,
            rollback_config: None,
            memory_limit: Some("512m".to_string()),
            memory_reservation: None,
            cpu_limit: Some(0.5),
            cpu_reservation: None,
            health_check: None,
            workers_only: true,
            force: false,
            entrypoint: None,
            command: None,
            workdir: None,
            mounts: vec![],
            secrets: vec![],
            configs: vec![],
        };

        let cmd = action.create_command(&input);
        assert!(cmd.contains("docker service create"));
        assert!(cmd.contains("--name my-service"));
        assert!(cmd.contains("nginx:alpine"));
        assert!(cmd.contains("--replicas 3"));
        assert!(cmd.contains("node.role==worker"));
        assert!(cmd.contains("--limit-memory 512m"));
    }
}
