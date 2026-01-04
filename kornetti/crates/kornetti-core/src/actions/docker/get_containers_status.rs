//! Get Containers Status Action
//!
//! Retrieves the status of all containers on a server.

use crate::actions::{Action, ActionContext, ActionResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::ContainerInfo;

/// Get containers status action
pub struct GetContainersStatus;

/// Input for getting container status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetContainersStatusInput {
    /// Server ID
    pub server_id: Uuid,
    /// Filter by container name prefix
    pub name_prefix: Option<String>,
    /// Include stopped containers
    pub include_stopped: bool,
    /// Filter by labels
    pub labels: Option<Vec<String>>,
}

/// Output of container status check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetContainersStatusOutput {
    /// Server ID
    pub server_id: Uuid,
    /// All containers found
    pub containers: Vec<ContainerStatusInfo>,
    /// Summary counts
    pub summary: ContainerSummary,
}

/// Container status with parsed information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStatusInfo {
    pub container_id: String,
    pub name: String,
    pub image: String,
    pub status: ContainerStatus,
    pub health: Option<HealthStatus>,
    pub uptime: Option<String>,
    pub ports: Vec<PortMapping>,
    pub labels: std::collections::HashMap<String, String>,
    /// Coolify resource type (if labeled)
    pub resource_type: Option<String>,
    /// Coolify resource UUID (if labeled)
    pub resource_uuid: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContainerStatus {
    Running,
    Stopped,
    Exited,
    Paused,
    Restarting,
    Dead,
    Created,
    Unknown,
}

impl ContainerStatus {
    pub fn from_docker_status(s: &str) -> Self {
        let lower = s.to_lowercase();
        if lower.starts_with("up") {
            ContainerStatus::Running
        } else if lower.starts_with("exited") {
            ContainerStatus::Exited
        } else if lower.contains("paused") {
            ContainerStatus::Paused
        } else if lower.contains("restarting") {
            ContainerStatus::Restarting
        } else if lower.contains("dead") {
            ContainerStatus::Dead
        } else if lower.contains("created") {
            ContainerStatus::Created
        } else {
            ContainerStatus::Unknown
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Starting,
    None,
}

impl HealthStatus {
    pub fn from_docker_status(s: &str) -> Self {
        let lower = s.to_lowercase();
        if lower.contains("(healthy)") {
            HealthStatus::Healthy
        } else if lower.contains("(unhealthy)") {
            HealthStatus::Unhealthy
        } else if lower.contains("(health: starting)") {
            HealthStatus::Starting
        } else {
            HealthStatus::None
        }
    }
}

/// Port mapping information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    pub host_ip: Option<String>,
    pub host_port: Option<u16>,
    pub container_port: u16,
    pub protocol: String,
}

impl PortMapping {
    /// Parse docker ports string like "0.0.0.0:8080->80/tcp, :::8080->80/tcp"
    pub fn parse_ports(ports_str: &str) -> Vec<Self> {
        let mut mappings = Vec::new();

        for port in ports_str.split(',') {
            let port = port.trim();
            if port.is_empty() {
                continue;
            }

            // Parse formats like "80/tcp" or "0.0.0.0:8080->80/tcp"
            if let Some(arrow_pos) = port.find("->") {
                // Has host mapping
                let host_part = &port[..arrow_pos];
                let container_part = &port[arrow_pos + 2..];

                let (host_ip, host_port) = if let Some(colon) = host_part.rfind(':') {
                    let ip = &host_part[..colon];
                    let port = host_part[colon + 1..].parse().ok();
                    (if ip.is_empty() { None } else { Some(ip.to_string()) }, port)
                } else {
                    (None, None)
                };

                let (container_port, protocol) = parse_port_protocol(container_part);

                if let Some(cp) = container_port {
                    mappings.push(PortMapping {
                        host_ip,
                        host_port,
                        container_port: cp,
                        protocol,
                    });
                }
            } else {
                // No host mapping, just "80/tcp"
                let (container_port, protocol) = parse_port_protocol(port);
                if let Some(cp) = container_port {
                    mappings.push(PortMapping {
                        host_ip: None,
                        host_port: None,
                        container_port: cp,
                        protocol,
                    });
                }
            }
        }

        mappings
    }
}

fn parse_port_protocol(s: &str) -> (Option<u16>, String) {
    if let Some(slash) = s.rfind('/') {
        let port = s[..slash].parse().ok();
        let proto = s[slash + 1..].to_string();
        (port, proto)
    } else {
        (s.parse().ok(), "tcp".to_string())
    }
}

/// Summary of container counts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerSummary {
    pub total: usize,
    pub running: usize,
    pub stopped: usize,
    pub healthy: usize,
    pub unhealthy: usize,
    pub coolify_managed: usize,
}

impl Action for GetContainersStatus {
    type Input = GetContainersStatusInput;
    type Output = GetContainersStatusOutput;

    fn name(&self) -> &'static str {
        "get_containers_status"
    }

    fn execute(
        &self,
        ctx: &ActionContext,
        input: &Self::Input,
    ) -> ActionResult<Self::Output> {
        // Would SSH to server and run:
        // docker ps -a --format '{{json .}}'
        // Then parse and categorize containers

        // Placeholder implementation
        Ok(GetContainersStatusOutput {
            server_id: input.server_id,
            containers: Vec::new(),
            summary: ContainerSummary {
                total: 0,
                running: 0,
                stopped: 0,
                healthy: 0,
                unhealthy: 0,
                coolify_managed: 0,
            },
        })
    }
}

impl GetContainersStatus {
    /// Convert raw ContainerInfo to ContainerStatusInfo
    pub fn process_container(info: &ContainerInfo) -> ContainerStatusInfo {
        let labels = info.parse_labels();
        let ports = PortMapping::parse_ports(&info.ports);

        ContainerStatusInfo {
            container_id: info.id.clone(),
            name: info.container_name().to_string(),
            image: info.image.clone(),
            status: ContainerStatus::from_docker_status(&info.status),
            health: Some(HealthStatus::from_docker_status(&info.status)),
            uptime: if info.is_running() {
                Some(info.status.clone())
            } else {
                None
            },
            ports,
            resource_type: labels.get("coolify.type").cloned(),
            resource_uuid: labels.get("coolify.uuid").cloned(),
            labels,
        }
    }

    /// Calculate summary from containers
    pub fn calculate_summary(containers: &[ContainerStatusInfo]) -> ContainerSummary {
        let mut summary = ContainerSummary {
            total: containers.len(),
            running: 0,
            stopped: 0,
            healthy: 0,
            unhealthy: 0,
            coolify_managed: 0,
        };

        for c in containers {
            match c.status {
                ContainerStatus::Running => summary.running += 1,
                ContainerStatus::Stopped | ContainerStatus::Exited => summary.stopped += 1,
                _ => {}
            }

            match c.health {
                Some(HealthStatus::Healthy) => summary.healthy += 1,
                Some(HealthStatus::Unhealthy) => summary.unhealthy += 1,
                _ => {}
            }

            if c.resource_type.is_some() {
                summary.coolify_managed += 1;
            }
        }

        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_status_from_docker() {
        assert_eq!(
            ContainerStatus::from_docker_status("Up 2 hours"),
            ContainerStatus::Running
        );
        assert_eq!(
            ContainerStatus::from_docker_status("Exited (0) 5 minutes ago"),
            ContainerStatus::Exited
        );
    }

    #[test]
    fn test_health_status() {
        assert_eq!(
            HealthStatus::from_docker_status("Up 2 hours (healthy)"),
            HealthStatus::Healthy
        );
        assert_eq!(
            HealthStatus::from_docker_status("Up 5 minutes (unhealthy)"),
            HealthStatus::Unhealthy
        );
    }

    #[test]
    fn test_parse_ports() {
        let ports = PortMapping::parse_ports("0.0.0.0:8080->80/tcp, 443/tcp");
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].host_port, Some(8080));
        assert_eq!(ports[0].container_port, 80);
        assert_eq!(ports[1].container_port, 443);
    }
}
