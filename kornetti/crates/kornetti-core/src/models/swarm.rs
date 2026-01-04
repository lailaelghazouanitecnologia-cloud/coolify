//! Docker Swarm Models
//!
//! Models for Docker Swarm destinations and configurations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Docker Swarm destination (overlay network configuration)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmDocker {
    pub id: Uuid,
    /// Name of the swarm destination
    pub name: String,
    /// Docker network name (typically overlay)
    pub network: String,
    /// Associated server ID (swarm manager)
    pub server_id: Uuid,
    /// UUID for external references
    pub uuid: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SwarmDocker {
    pub fn new(name: String, network: String, server_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            network,
            server_id,
            uuid: Uuid::new_v4().to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Create the default coolify-overlay swarm destination
    pub fn coolify_overlay(server_id: Uuid) -> Self {
        Self::new(
            "coolify-overlay".to_string(),
            "coolify-overlay".to_string(),
            server_id,
        )
    }
}

/// Swarm node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmNode {
    pub id: String,
    pub hostname: String,
    pub status: SwarmNodeStatus,
    pub availability: SwarmNodeAvailability,
    pub role: SwarmNodeRole,
    pub engine_version: String,
    pub addr: String,
    pub labels: std::collections::HashMap<String, String>,
    pub manager_status: Option<ManagerStatus>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SwarmNodeStatus {
    Ready,
    Down,
    Unknown,
    Disconnected,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SwarmNodeAvailability {
    Active,
    Pause,
    Drain,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SwarmNodeRole {
    Manager,
    Worker,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerStatus {
    pub leader: bool,
    pub reachability: String,
    pub addr: String,
}

/// Swarm service information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmService {
    pub id: String,
    pub name: String,
    pub mode: SwarmServiceMode,
    pub replicas: SwarmServiceReplicas,
    pub image: String,
    pub ports: Vec<SwarmServicePort>,
    pub labels: std::collections::HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SwarmServiceMode {
    Replicated,
    Global,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmServiceReplicas {
    /// Running replicas
    pub running: u32,
    /// Desired replicas
    pub desired: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmServicePort {
    /// Protocol (tcp, udp)
    pub protocol: String,
    /// Target port (container)
    pub target_port: u16,
    /// Published port (host)
    pub published_port: u16,
    /// Publish mode (ingress, host)
    pub publish_mode: String,
}

/// Swarm task (individual container in a service)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmTask {
    pub id: String,
    pub service_id: String,
    pub node_id: String,
    pub status: SwarmTaskStatus,
    pub desired_state: String,
    pub current_state: String,
    pub error: Option<String>,
    pub container_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SwarmTaskStatus {
    New,
    Pending,
    Assigned,
    Accepted,
    Preparing,
    Ready,
    Starting,
    Running,
    Complete,
    Shutdown,
    Failed,
    Rejected,
    Remove,
    Orphaned,
}

/// Swarm stack (group of services from compose file)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmStack {
    pub id: Uuid,
    /// Stack name
    pub name: String,
    /// Swarm destination ID
    pub swarm_docker_id: Uuid,
    /// Docker Compose file content
    pub compose_file: String,
    /// Services in this stack
    pub services: Vec<String>,
    /// Environment variables
    pub environment: std::collections::HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SwarmStack {
    pub fn new(name: String, swarm_docker_id: Uuid, compose_file: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            swarm_docker_id,
            compose_file,
            services: Vec::new(),
            environment: std::collections::HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// Swarm configuration for an application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmDeploymentConfig {
    /// Number of replicas
    pub replicas: u32,
    /// Placement constraints (JSON array as string in Coolify)
    pub placement_constraints: Vec<String>,
    /// Only deploy to worker nodes
    pub worker_nodes_only: bool,
    /// Update configuration
    pub update_config: Option<SwarmUpdateConfig>,
    /// Rollback configuration
    pub rollback_config: Option<SwarmRollbackConfig>,
    /// Resource limits
    pub resources: Option<SwarmResourceConfig>,
}

impl Default for SwarmDeploymentConfig {
    fn default() -> Self {
        Self {
            replicas: 1,
            placement_constraints: Vec::new(),
            worker_nodes_only: false,
            update_config: Some(SwarmUpdateConfig::default()),
            rollback_config: Some(SwarmRollbackConfig::default()),
            resources: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmUpdateConfig {
    /// Number of containers to update at a time
    pub parallelism: u32,
    /// Delay between updates
    pub delay: String,
    /// Action on failure (pause, continue, rollback)
    pub failure_action: String,
    /// Monitor duration
    pub monitor: String,
    /// Maximum failure ratio
    pub max_failure_ratio: f64,
    /// Update order (stop-first, start-first)
    pub order: String,
}

impl Default for SwarmUpdateConfig {
    fn default() -> Self {
        Self {
            parallelism: 1,
            delay: "10s".to_string(),
            failure_action: "pause".to_string(),
            monitor: "5s".to_string(),
            max_failure_ratio: 0.0,
            order: "stop-first".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmRollbackConfig {
    pub parallelism: u32,
    pub delay: String,
    pub failure_action: String,
    pub monitor: String,
    pub max_failure_ratio: f64,
    pub order: String,
}

impl Default for SwarmRollbackConfig {
    fn default() -> Self {
        Self {
            parallelism: 1,
            delay: "10s".to_string(),
            failure_action: "pause".to_string(),
            monitor: "5s".to_string(),
            max_failure_ratio: 0.0,
            order: "stop-first".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmResourceConfig {
    /// Memory limit (e.g., "512M", "1G")
    pub limits_memory: Option<String>,
    /// CPU limit (e.g., 0.5, 1.0)
    pub limits_cpus: Option<f64>,
    /// Memory reservation
    pub reservations_memory: Option<String>,
    /// CPU reservation
    pub reservations_cpus: Option<f64>,
}

/// Parse swarm placement constraints from base64-encoded JSON
pub fn parse_placement_constraints(encoded: &str) -> Result<Vec<String>, String> {
    use base64::Engine;

    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|e| format!("Invalid base64: {}", e))?;

    let json_str = String::from_utf8(decoded)
        .map_err(|e| format!("Invalid UTF-8: {}", e))?;

    serde_json::from_str(&json_str)
        .map_err(|e| format!("Invalid JSON: {}", e))
}

/// Encode placement constraints to base64 JSON
pub fn encode_placement_constraints(constraints: &[String]) -> String {
    use base64::Engine;

    let json = serde_json::to_string(constraints).unwrap_or_default();
    base64::engine::general_purpose::STANDARD.encode(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swarm_docker_new() {
        let server_id = Uuid::new_v4();
        let swarm = SwarmDocker::new(
            "test-swarm".to_string(),
            "test-overlay".to_string(),
            server_id,
        );

        assert_eq!(swarm.name, "test-swarm");
        assert_eq!(swarm.network, "test-overlay");
        assert_eq!(swarm.server_id, server_id);
    }

    #[test]
    fn test_coolify_overlay() {
        let server_id = Uuid::new_v4();
        let swarm = SwarmDocker::coolify_overlay(server_id);

        assert_eq!(swarm.name, "coolify-overlay");
        assert_eq!(swarm.network, "coolify-overlay");
    }

    #[test]
    fn test_placement_constraints_encoding() {
        let constraints = vec![
            "node.role==worker".to_string(),
            "node.labels.env==production".to_string(),
        ];

        let encoded = encode_placement_constraints(&constraints);
        let decoded = parse_placement_constraints(&encoded).unwrap();

        assert_eq!(constraints, decoded);
    }
}
