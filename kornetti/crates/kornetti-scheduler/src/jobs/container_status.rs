//! Container status check job
//!
//! This job periodically checks the status of all containers on a server
//! and updates the database accordingly.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Container status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub container_id: String,
    pub container_name: String,
    pub image: String,
    pub status: ContainerState,
    pub health: Option<HealthStatus>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub ports: Vec<PortMapping>,
    pub labels: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContainerState {
    Created,
    Running,
    Paused,
    Restarting,
    Removing,
    Exited,
    Dead,
}

impl ContainerState {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContainerState::Created => "created",
            ContainerState::Running => "running",
            ContainerState::Paused => "paused",
            ContainerState::Restarting => "restarting",
            ContainerState::Removing => "removing",
            ContainerState::Exited => "exited",
            ContainerState::Dead => "dead",
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(self, ContainerState::Running)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Starting,
    Healthy,
    Unhealthy,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    pub container_port: u16,
    pub host_port: Option<u16>,
    pub protocol: String,
}

/// Context for the container status job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStatusContext {
    pub server_id: Uuid,
    /// Only check containers matching these labels (if specified)
    pub label_filter: Option<String>,
}

/// Result of checking container status
#[derive(Debug, Serialize, Deserialize)]
pub struct ContainerStatusResult {
    pub server_id: Uuid,
    pub containers: Vec<ContainerInfo>,
    pub running_count: usize,
    pub total_count: usize,
    pub checked_at: DateTime<Utc>,
    /// Containers that changed state since last check
    pub state_changes: Vec<ContainerStateChange>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContainerStateChange {
    pub container_id: String,
    pub container_name: String,
    pub previous_state: Option<ContainerState>,
    pub current_state: ContainerState,
    pub resource_type: Option<String>,
    pub resource_id: Option<Uuid>,
}

/// Container status check job
pub struct ContainerStatusJob;

impl ContainerStatusJob {
    pub fn new() -> Self {
        Self
    }

    /// Run the container status check job
    pub async fn run(&self, ctx: ContainerStatusContext) -> Result<ContainerStatusResult> {
        tracing::info!(
            server_id = %ctx.server_id,
            "Checking container status"
        );

        // Build docker ps command
        let format = "{{.ID}}|{{.Names}}|{{.Image}}|{{.State}}|{{.Status}}|{{.Ports}}|{{.Labels}}";
        let mut cmd = format!("docker ps -a --format '{}'", format);

        if let Some(filter) = &ctx.label_filter {
            cmd = format!("{} --filter 'label={}'", cmd, filter);
        }

        // TODO: Execute command on server via SSH
        // let output = ssh_client.execute(&cmd).await?;

        // For now, return empty result
        let result = ContainerStatusResult {
            server_id: ctx.server_id,
            containers: vec![],
            running_count: 0,
            total_count: 0,
            checked_at: Utc::now(),
            state_changes: vec![],
        };

        Ok(result)
    }

    /// Parse docker ps output into ContainerInfo
    pub fn parse_docker_ps_output(output: &str) -> Vec<ContainerInfo> {
        let mut containers = Vec::new();

        for line in output.lines() {
            if line.trim().is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() < 7 {
                continue;
            }

            let container = ContainerInfo {
                container_id: parts[0].to_string(),
                container_name: parts[1].to_string(),
                image: parts[2].to_string(),
                status: Self::parse_state(parts[3]),
                health: Self::parse_health_from_status(parts[4]),
                started_at: None,
                finished_at: None,
                exit_code: Self::parse_exit_code(parts[4]),
                ports: Self::parse_ports(parts[5]),
                labels: Self::parse_labels(parts[6]),
            };

            containers.push(container);
        }

        containers
    }

    fn parse_state(state: &str) -> ContainerState {
        match state.to_lowercase().as_str() {
            "created" => ContainerState::Created,
            "running" => ContainerState::Running,
            "paused" => ContainerState::Paused,
            "restarting" => ContainerState::Restarting,
            "removing" => ContainerState::Removing,
            "exited" => ContainerState::Exited,
            "dead" => ContainerState::Dead,
            _ => ContainerState::Dead,
        }
    }

    fn parse_health_from_status(status: &str) -> Option<HealthStatus> {
        let status_lower = status.to_lowercase();
        if status_lower.contains("(healthy)") {
            Some(HealthStatus::Healthy)
        } else if status_lower.contains("(unhealthy)") {
            Some(HealthStatus::Unhealthy)
        } else if status_lower.contains("(health: starting)") {
            Some(HealthStatus::Starting)
        } else {
            None
        }
    }

    fn parse_exit_code(status: &str) -> Option<i32> {
        // Status format: "Exited (0) 2 hours ago"
        if status.to_lowercase().starts_with("exited") {
            let start = status.find('(')?;
            let end = status.find(')')?;
            let code_str = &status[start + 1..end];
            code_str.parse().ok()
        } else {
            None
        }
    }

    fn parse_ports(ports_str: &str) -> Vec<PortMapping> {
        let mut ports = Vec::new();

        // Format: "0.0.0.0:8080->80/tcp, 443/tcp"
        for port_mapping in ports_str.split(", ") {
            if port_mapping.is_empty() {
                continue;
            }

            let (host_port, container_part) = if port_mapping.contains("->") {
                let parts: Vec<&str> = port_mapping.split("->").collect();
                let host = parts[0].split(':').last().and_then(|p| p.parse().ok());
                (host, parts.get(1).copied().unwrap_or(""))
            } else {
                (None, port_mapping)
            };

            let (container_port, protocol) = if container_part.contains('/') {
                let parts: Vec<&str> = container_part.split('/').collect();
                (
                    parts[0].parse().unwrap_or(0),
                    parts.get(1).copied().unwrap_or("tcp").to_string(),
                )
            } else {
                (container_part.parse().unwrap_or(0), "tcp".to_string())
            };

            if container_port > 0 {
                ports.push(PortMapping {
                    container_port,
                    host_port,
                    protocol,
                });
            }
        }

        ports
    }

    fn parse_labels(labels_str: &str) -> std::collections::HashMap<String, String> {
        let mut labels = std::collections::HashMap::new();

        // Format: "key=value,key2=value2"
        for label in labels_str.split(',') {
            if let Some((key, value)) = label.split_once('=') {
                labels.insert(key.to_string(), value.to_string());
            }
        }

        labels
    }

    /// Extract resource info from container labels
    pub fn extract_resource_from_labels(
        labels: &std::collections::HashMap<String, String>,
    ) -> (Option<String>, Option<Uuid>) {
        // Kornetti labels
        let resource_type = labels.get("kornetti.resource.type").cloned();
        let resource_id = labels
            .get("kornetti.resource.id")
            .and_then(|id| Uuid::parse_str(id).ok());

        // Fallback to Coolify labels for compatibility
        let resource_type = resource_type.or_else(|| labels.get("coolify.resource.type").cloned());
        let resource_id = resource_id.or_else(|| {
            labels
                .get("coolify.resource.id")
                .and_then(|id| Uuid::parse_str(id).ok())
        });

        (resource_type, resource_id)
    }
}

impl Default for ContainerStatusJob {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_docker_ps_output() {
        let output = "abc123|my-app|nginx:latest|running|Up 2 hours (healthy)|0.0.0.0:8080->80/tcp|kornetti.resource.type=application";
        let containers = ContainerStatusJob::parse_docker_ps_output(output);

        assert_eq!(containers.len(), 1);
        assert_eq!(containers[0].container_name, "my-app");
        assert_eq!(containers[0].status, ContainerState::Running);
        assert_eq!(containers[0].health, Some(HealthStatus::Healthy));
    }

    #[test]
    fn test_parse_ports() {
        let ports = ContainerStatusJob::parse_ports("0.0.0.0:8080->80/tcp, 443/tcp");
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].host_port, Some(8080));
        assert_eq!(ports[0].container_port, 80);
    }

    #[test]
    fn test_parse_exit_code() {
        assert_eq!(ContainerStatusJob::parse_exit_code("Exited (0) 2 hours ago"), Some(0));
        assert_eq!(ContainerStatusJob::parse_exit_code("Exited (137) 1 minute ago"), Some(137));
        assert_eq!(ContainerStatusJob::parse_exit_code("Up 2 hours"), None);
    }
}
