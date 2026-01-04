//! Docker Actions
//!
//! Actions for Docker container management and status checking.

mod get_containers_status;
mod container_logs;
mod container_stats;

pub use get_containers_status::*;
pub use container_logs::*;
pub use container_stats::*;

use crate::actions::Action;

/// Docker-related actions collection
pub struct DockerActions;

impl DockerActions {
    /// Parse docker container status from JSON
    pub fn parse_container_status(json: &str) -> Result<Vec<ContainerInfo>, String> {
        serde_json::from_str(json).map_err(|e| format!("Failed to parse container status: {}", e))
    }

    /// Get docker command for listing containers
    pub fn list_containers_command(all: bool) -> String {
        if all {
            "docker ps -a --format '{{json .}}'".to_string()
        } else {
            "docker ps --format '{{json .}}'".to_string()
        }
    }

    /// Get docker command for inspecting a container
    pub fn inspect_container_command(container_id: &str) -> String {
        format!("docker inspect {}", container_id)
    }

    /// Get docker command for container logs
    pub fn logs_command(container_id: &str, lines: Option<u32>, follow: bool) -> String {
        let mut cmd = format!("docker logs {}", container_id);
        if let Some(n) = lines {
            cmd.push_str(&format!(" --tail {}", n));
        }
        if follow {
            cmd.push_str(" -f");
        }
        cmd
    }

    /// Get docker command for container stats
    pub fn stats_command(container_id: &str, no_stream: bool) -> String {
        let mut cmd = format!("docker stats {} --format '{{{{json .}}}}'", container_id);
        if no_stream {
            cmd.push_str(" --no-stream");
        }
        cmd
    }
}

/// Container information from docker ps
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerInfo {
    pub id: String,
    #[serde(alias = "ID")]
    pub container_id: Option<String>,
    pub image: String,
    pub command: String,
    pub created: String,
    pub status: String,
    pub ports: String,
    pub names: String,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub labels: Option<String>,
}

impl ContainerInfo {
    /// Check if container is running
    pub fn is_running(&self) -> bool {
        self.status.to_lowercase().starts_with("up")
            || self.state.as_deref() == Some("running")
    }

    /// Check if container is healthy
    pub fn is_healthy(&self) -> bool {
        self.status.to_lowercase().contains("healthy")
    }

    /// Check if container is unhealthy
    pub fn is_unhealthy(&self) -> bool {
        self.status.to_lowercase().contains("unhealthy")
    }

    /// Check if container has exited
    pub fn has_exited(&self) -> bool {
        self.status.to_lowercase().starts_with("exited")
            || self.state.as_deref() == Some("exited")
    }

    /// Get the container name (first name without leading /)
    pub fn container_name(&self) -> &str {
        self.names.split(',').next().unwrap_or(&self.names).trim_start_matches('/')
    }

    /// Parse labels into a hashmap
    pub fn parse_labels(&self) -> std::collections::HashMap<String, String> {
        self.labels
            .as_deref()
            .map(|s| {
                s.split(',')
                    .filter_map(|pair| {
                        let mut parts = pair.splitn(2, '=');
                        Some((parts.next()?.to_string(), parts.next()?.to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Container statistics from docker stats
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerStats {
    #[serde(alias = "Container")]
    pub container: String,
    pub name: String,
    #[serde(alias = "CPUPerc")]
    pub cpu_perc: String,
    #[serde(alias = "MemUsage")]
    pub mem_usage: String,
    #[serde(alias = "MemPerc")]
    pub mem_perc: String,
    #[serde(alias = "NetIO")]
    pub net_io: String,
    #[serde(alias = "BlockIO")]
    pub block_io: String,
    #[serde(alias = "PIDs")]
    pub pids: String,
}

impl ContainerStats {
    /// Parse CPU percentage as f64
    pub fn cpu_percent(&self) -> f64 {
        self.cpu_perc
            .trim_end_matches('%')
            .parse()
            .unwrap_or(0.0)
    }

    /// Parse memory percentage as f64
    pub fn mem_percent(&self) -> f64 {
        self.mem_perc
            .trim_end_matches('%')
            .parse()
            .unwrap_or(0.0)
    }

    /// Parse memory usage in bytes
    pub fn mem_used_bytes(&self) -> Option<u64> {
        let usage = self.mem_usage.split('/').next()?.trim();
        parse_size(usage)
    }

    /// Parse memory limit in bytes
    pub fn mem_limit_bytes(&self) -> Option<u64> {
        let parts: Vec<&str> = self.mem_usage.split('/').collect();
        if parts.len() >= 2 {
            parse_size(parts[1].trim())
        } else {
            None
        }
    }
}

/// Parse size string like "1.5GiB" or "512MiB" to bytes
fn parse_size(s: &str) -> Option<u64> {
    let s = s.to_uppercase();
    let (num_str, multiplier) = if s.ends_with("GIB") || s.ends_with("GB") {
        (s.trim_end_matches(|c| c == 'G' || c == 'I' || c == 'B'), 1024 * 1024 * 1024)
    } else if s.ends_with("MIB") || s.ends_with("MB") {
        (s.trim_end_matches(|c| c == 'M' || c == 'I' || c == 'B'), 1024 * 1024)
    } else if s.ends_with("KIB") || s.ends_with("KB") {
        (s.trim_end_matches(|c| c == 'K' || c == 'I' || c == 'B'), 1024)
    } else if s.ends_with("B") {
        (s.trim_end_matches('B'), 1)
    } else {
        return None;
    };

    let num: f64 = num_str.parse().ok()?;
    Some((num * multiplier as f64) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_info_is_running() {
        let info = ContainerInfo {
            id: "abc123".to_string(),
            container_id: None,
            image: "nginx".to_string(),
            command: "nginx -g".to_string(),
            created: "2 hours ago".to_string(),
            status: "Up 2 hours".to_string(),
            ports: "80/tcp".to_string(),
            names: "my-nginx".to_string(),
            state: None,
            labels: None,
        };

        assert!(info.is_running());
        assert!(!info.has_exited());
    }

    #[test]
    fn test_container_name() {
        let info = ContainerInfo {
            id: "abc".to_string(),
            container_id: None,
            image: "nginx".to_string(),
            command: "".to_string(),
            created: "".to_string(),
            status: "Up".to_string(),
            ports: "".to_string(),
            names: "/my-container,alias".to_string(),
            state: None,
            labels: None,
        };

        assert_eq!(info.container_name(), "my-container");
    }

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("1GiB"), Some(1073741824));
        assert_eq!(parse_size("512MiB"), Some(536870912));
        assert_eq!(parse_size("1024KiB"), Some(1048576));
    }
}
