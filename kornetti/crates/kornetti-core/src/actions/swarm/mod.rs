//! Docker Swarm Actions
//!
//! Actions for managing Docker Swarm clusters, services, and deployments.

mod init_swarm;
mod join_swarm;
mod deploy_service;
mod manage_nodes;

pub use init_swarm::{InitSwarm, InitSwarmInput, InitSwarmOutput};
pub use join_swarm::{JoinSwarm, JoinSwarmInput, JoinSwarmOutput};
pub use deploy_service::{DeploySwarmService, DeploySwarmServiceInput, DeploySwarmServiceOutput};
pub use manage_nodes::{
    ListSwarmNodes, SwarmNode, SwarmNodeStatus, SwarmNodeRole,
    PromoteNode, DemoteNode, DrainNode, RemoveNode,
};

use serde::{Deserialize, Serialize};

/// Swarm placement constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementConstraint {
    /// Constraint expression (e.g., "node.role==worker")
    pub expression: String,
}

impl PlacementConstraint {
    pub fn new(expression: impl Into<String>) -> Self {
        Self {
            expression: expression.into(),
        }
    }

    /// Only deploy to manager nodes
    pub fn managers_only() -> Self {
        Self::new("node.role==manager")
    }

    /// Only deploy to worker nodes
    pub fn workers_only() -> Self {
        Self::new("node.role==worker")
    }

    /// Deploy to nodes with specific label
    pub fn with_label(key: &str, value: &str) -> Self {
        Self::new(format!("node.labels.{}=={}", key, value))
    }
}

/// Swarm service update configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    /// Number of containers to update at a time
    pub parallelism: u32,
    /// Delay between updates
    pub delay: String,
    /// Action on failure (pause, continue, rollback)
    pub failure_action: FailureAction,
    /// Monitor duration after update
    pub monitor: String,
    /// Maximum failure ratio before action
    pub max_failure_ratio: f64,
    /// Order of update (stop-first, start-first)
    pub order: UpdateOrder,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            parallelism: 1,
            delay: "10s".to_string(),
            failure_action: FailureAction::Pause,
            monitor: "5s".to_string(),
            max_failure_ratio: 0.0,
            order: UpdateOrder::StopFirst,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FailureAction {
    Pause,
    Continue,
    Rollback,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum UpdateOrder {
    StopFirst,
    StartFirst,
}

/// Swarm rollback configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackConfig {
    /// Number of containers to rollback at a time
    pub parallelism: u32,
    /// Delay between rollbacks
    pub delay: String,
    /// Action on failure
    pub failure_action: FailureAction,
    /// Monitor duration
    pub monitor: String,
    /// Maximum failure ratio
    pub max_failure_ratio: f64,
    /// Order of rollback
    pub order: UpdateOrder,
}

impl Default for RollbackConfig {
    fn default() -> Self {
        Self {
            parallelism: 1,
            delay: "10s".to_string(),
            failure_action: FailureAction::Pause,
            monitor: "5s".to_string(),
            max_failure_ratio: 0.0,
            order: UpdateOrder::StopFirst,
        }
    }
}

/// Swarm network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmNetwork {
    /// Network name
    pub name: String,
    /// Network driver (overlay, bridge)
    pub driver: String,
    /// Is attachable by non-swarm containers
    pub attachable: bool,
    /// Is external (pre-existing)
    pub external: bool,
    /// Network options
    pub options: std::collections::HashMap<String, String>,
}

impl SwarmNetwork {
    /// Create a new overlay network
    pub fn overlay(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            driver: "overlay".to_string(),
            attachable: true,
            external: false,
            options: std::collections::HashMap::new(),
        }
    }

    /// Create an external network reference
    pub fn external(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            driver: "overlay".to_string(),
            attachable: true,
            external: true,
            options: std::collections::HashMap::new(),
        }
    }
}

/// Generate docker service create/update command
pub fn generate_service_command(
    service_name: &str,
    image: &str,
    replicas: u32,
    networks: &[String],
    constraints: &[PlacementConstraint],
    labels: &[(String, String)],
    env_vars: &[(String, String)],
    ports: &[(u16, u16)],
    update_config: Option<&UpdateConfig>,
    is_update: bool,
) -> String {
    let mut cmd = if is_update {
        format!("docker service update")
    } else {
        format!("docker service create --name {}", service_name)
    };

    // Image
    cmd.push_str(&format!(" --image {}", image));

    // Replicas
    cmd.push_str(&format!(" --replicas {}", replicas));

    // Networks
    for network in networks {
        cmd.push_str(&format!(" --network {}", network));
    }

    // Placement constraints
    for constraint in constraints {
        cmd.push_str(&format!(" --constraint '{}'", constraint.expression));
    }

    // Labels
    for (key, value) in labels {
        cmd.push_str(&format!(" --label {}=\"{}\"", key, value));
    }

    // Environment variables
    for (key, value) in env_vars {
        cmd.push_str(&format!(" --env {}=\"{}\"", key, value));
    }

    // Ports
    for (host, container) in ports {
        cmd.push_str(&format!(" --publish {}:{}", host, container));
    }

    // Update config
    if let Some(config) = update_config {
        cmd.push_str(&format!(" --update-parallelism {}", config.parallelism));
        cmd.push_str(&format!(" --update-delay {}", config.delay));
        cmd.push_str(&format!(
            " --update-failure-action {}",
            match config.failure_action {
                FailureAction::Pause => "pause",
                FailureAction::Continue => "continue",
                FailureAction::Rollback => "rollback",
            }
        ));
        cmd.push_str(&format!(" --update-monitor {}", config.monitor));
        cmd.push_str(&format!(
            " --update-order {}",
            match config.order {
                UpdateOrder::StopFirst => "stop-first",
                UpdateOrder::StartFirst => "start-first",
            }
        ));
    }

    if is_update {
        cmd.push_str(&format!(" {}", service_name));
    }

    cmd
}
