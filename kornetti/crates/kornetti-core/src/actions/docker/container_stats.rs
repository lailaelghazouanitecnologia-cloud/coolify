//! Container Stats Action
//!
//! Retrieves resource usage statistics from Docker containers.

use crate::actions::{Action, ActionContext, ActionResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::ContainerStats;

/// Get container stats action
pub struct GetContainerStats;

/// Input for getting container stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetContainerStatsInput {
    /// Server ID
    pub server_id: Uuid,
    /// Container names or IDs (empty = all containers)
    pub containers: Vec<String>,
    /// Stream stats continuously (false = one-shot)
    pub stream: bool,
}

/// Output of container stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetContainerStatsOutput {
    /// Server ID
    pub server_id: Uuid,
    /// Container stats
    pub stats: Vec<ContainerStatsInfo>,
    /// Server-wide summary
    pub summary: ServerResourceSummary,
}

/// Extended container stats with parsed values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStatsInfo {
    pub container_id: String,
    pub name: String,
    /// CPU usage percentage
    pub cpu_percent: f64,
    /// Memory usage in bytes
    pub memory_used: u64,
    /// Memory limit in bytes
    pub memory_limit: u64,
    /// Memory usage percentage
    pub memory_percent: f64,
    /// Network I/O received bytes
    pub network_rx: u64,
    /// Network I/O transmitted bytes
    pub network_tx: u64,
    /// Block I/O read bytes
    pub block_read: u64,
    /// Block I/O write bytes
    pub block_write: u64,
    /// Number of processes
    pub pids: u32,
}

/// Server-wide resource summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerResourceSummary {
    /// Total CPU usage across all containers
    pub total_cpu_percent: f64,
    /// Total memory used across all containers
    pub total_memory_used: u64,
    /// Total memory limit (sum of limits)
    pub total_memory_limit: u64,
    /// Number of containers monitored
    pub container_count: usize,
}

impl Action for GetContainerStats {
    type Input = GetContainerStatsInput;
    type Output = GetContainerStatsOutput;

    fn name(&self) -> &'static str {
        "get_container_stats"
    }

    fn execute(
        &self,
        ctx: &ActionContext,
        input: &Self::Input,
    ) -> ActionResult<Self::Output> {
        // Would SSH to server and run:
        // docker stats --no-stream --format '{{json .}}'

        Ok(GetContainerStatsOutput {
            server_id: input.server_id,
            stats: Vec::new(),
            summary: ServerResourceSummary {
                total_cpu_percent: 0.0,
                total_memory_used: 0,
                total_memory_limit: 0,
                container_count: 0,
            },
        })
    }
}

impl GetContainerStats {
    /// Build the docker stats command
    pub fn build_command(containers: &[String], stream: bool) -> String {
        let mut cmd = "docker stats --format '{{json .}}'".to_string();

        if !stream {
            cmd.push_str(" --no-stream");
        }

        for container in containers {
            cmd.push(' ');
            cmd.push_str(container);
        }

        cmd
    }

    /// Parse ContainerStats into ContainerStatsInfo
    pub fn parse_stats(raw: &ContainerStats) -> ContainerStatsInfo {
        let cpu_percent = raw.cpu_percent();
        let memory_percent = raw.mem_percent();
        let memory_used = raw.mem_used_bytes().unwrap_or(0);
        let memory_limit = raw.mem_limit_bytes().unwrap_or(0);

        let (network_rx, network_tx) = parse_io(&raw.net_io);
        let (block_read, block_write) = parse_io(&raw.block_io);

        ContainerStatsInfo {
            container_id: raw.container.clone(),
            name: raw.name.clone(),
            cpu_percent,
            memory_used,
            memory_limit,
            memory_percent,
            network_rx,
            network_tx,
            block_read,
            block_write,
            pids: raw.pids.parse().unwrap_or(0),
        }
    }

    /// Calculate server summary from stats
    pub fn calculate_summary(stats: &[ContainerStatsInfo]) -> ServerResourceSummary {
        let total_cpu: f64 = stats.iter().map(|s| s.cpu_percent).sum();
        let total_mem: u64 = stats.iter().map(|s| s.memory_used).sum();
        let total_limit: u64 = stats.iter().map(|s| s.memory_limit).sum();

        ServerResourceSummary {
            total_cpu_percent: total_cpu,
            total_memory_used: total_mem,
            total_memory_limit: total_limit,
            container_count: stats.len(),
        }
    }
}

/// Parse I/O strings like "1.5GB / 500MB" into (rx, tx) bytes
fn parse_io(s: &str) -> (u64, u64) {
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() >= 2 {
        let rx = parse_size_string(parts[0].trim()).unwrap_or(0);
        let tx = parse_size_string(parts[1].trim()).unwrap_or(0);
        (rx, tx)
    } else {
        (0, 0)
    }
}

/// Parse size string like "1.5GB" to bytes
fn parse_size_string(s: &str) -> Option<u64> {
    let s = s.to_uppercase();

    // Find where the number ends
    let num_end = s
        .chars()
        .position(|c| !c.is_ascii_digit() && c != '.')
        .unwrap_or(s.len());

    let num_str = &s[..num_end];
    let unit = &s[num_end..];

    let num: f64 = num_str.parse().ok()?;

    let multiplier: u64 = match unit.trim() {
        "TB" | "TIB" => 1024 * 1024 * 1024 * 1024,
        "GB" | "GIB" => 1024 * 1024 * 1024,
        "MB" | "MIB" => 1024 * 1024,
        "KB" | "KIB" => 1024,
        "B" | "" => 1,
        _ => return None,
    };

    Some((num * multiplier as f64) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_command() {
        let cmd = GetContainerStats::build_command(
            &["container1".to_string(), "container2".to_string()],
            false,
        );

        assert!(cmd.contains("docker stats"));
        assert!(cmd.contains("--no-stream"));
        assert!(cmd.contains("container1"));
        assert!(cmd.contains("container2"));
    }

    #[test]
    fn test_parse_io() {
        let (rx, tx) = parse_io("1GB / 500MB");
        assert_eq!(rx, 1024 * 1024 * 1024);
        assert_eq!(tx, 500 * 1024 * 1024);
    }

    #[test]
    fn test_parse_size_string() {
        assert_eq!(parse_size_string("1GB"), Some(1024 * 1024 * 1024));
        assert_eq!(parse_size_string("512MB"), Some(512 * 1024 * 1024));
        assert_eq!(parse_size_string("1.5GB"), Some((1.5 * 1024.0 * 1024.0 * 1024.0) as u64));
    }
}
