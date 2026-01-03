//! Cleanup Docker Action
//!
//! Cleans up unused Docker resources (containers, images, volumes, networks).

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument, warn};

use crate::models::Server;
use crate::actions::{Action, ActionError, ActionResult, CommandBuilder};
use kornetti_ssh::SshClient;

/// Cleanup options
#[derive(Debug, Clone, Default)]
pub struct CleanupOptions {
    /// Remove stopped containers
    pub containers: bool,
    /// Remove unused images
    pub images: bool,
    /// Remove unused volumes
    pub volumes: bool,
    /// Remove unused networks
    pub networks: bool,
    /// Remove build cache
    pub build_cache: bool,
    /// Force removal without confirmation
    pub force: bool,
    /// Only remove resources older than this (e.g., "24h", "7d")
    pub older_than: Option<String>,
}

impl CleanupOptions {
    /// Clean everything
    pub fn all() -> Self {
        Self {
            containers: true,
            images: true,
            volumes: true,
            networks: true,
            build_cache: true,
            force: true,
            older_than: None,
        }
    }

    /// Conservative cleanup (no volumes)
    pub fn safe() -> Self {
        Self {
            containers: true,
            images: true,
            volumes: false,
            networks: true,
            build_cache: true,
            force: true,
            older_than: Some("24h".to_string()),
        }
    }
}

/// Cleanup statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CleanupStats {
    pub containers_removed: u32,
    pub images_removed: u32,
    pub volumes_removed: u32,
    pub networks_removed: u32,
    pub space_reclaimed_mb: u64,
}

/// Input for Docker cleanup
pub struct CleanupDockerInput<'a> {
    pub server: &'a Server,
    pub options: CleanupOptions,
}

/// Action to clean up Docker resources
pub struct CleanupDocker {
    ssh: Arc<SshClient>,
}

impl CleanupDocker {
    pub fn new(ssh: Arc<SshClient>) -> Self {
        Self { ssh }
    }

    /// Parse space reclaimed from Docker prune output
    fn parse_space_reclaimed(output: &str) -> u64 {
        // "Total reclaimed space: 1.234GB"
        for line in output.lines() {
            if line.contains("reclaimed space") {
                if let Some(size_str) = line.split(':').last() {
                    return Self::parse_size(size_str.trim());
                }
            }
        }
        0
    }

    /// Parse size string to MB
    fn parse_size(s: &str) -> u64 {
        let s = s.trim();
        let (num, unit) = if s.ends_with("GB") {
            (s.trim_end_matches("GB").trim(), 1024u64)
        } else if s.ends_with("MB") {
            (s.trim_end_matches("MB").trim(), 1u64)
        } else if s.ends_with("KB") || s.ends_with("kB") {
            return 0; // Less than 1 MB
        } else if s.ends_with('B') {
            return 0;
        } else {
            return 0;
        };

        num.parse::<f64>()
            .map(|n| (n * unit as f64) as u64)
            .unwrap_or(0)
    }

    /// Count items in Docker list output
    fn count_items(output: &str) -> u32 {
        output.lines()
            .filter(|line| !line.is_empty() && !line.starts_with("CONTAINER") && !line.starts_with("IMAGE"))
            .count() as u32
    }
}

#[async_trait]
impl Action for CleanupDocker {
    type Input = CleanupDockerInput<'static>;
    type Output = CleanupStats;

    fn name(&self) -> &'static str {
        "cleanup_docker"
    }

    #[instrument(skip(self, input), fields(server_id = %input.server.id))]
    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        let server = input.server;
        let options = &input.options;
        let start = std::time::Instant::now();

        info!("Starting Docker cleanup on server {}", server.id);

        let session = self.ssh.connect(server)
            .await
            .map_err(|e| ActionError::ssh_connection_failed(e.to_string()))?;

        let mut stats = CleanupStats::default();
        let filter = options.older_than
            .as_ref()
            .map(|t| format!(" --filter \"until={}\"", t))
            .unwrap_or_default();

        // Count resources before cleanup
        if options.containers {
            let before = session.execute("docker ps -aq --filter status=exited 2>/dev/null | wc -l")
                .await
                .ok()
                .map(|r| r.stdout.trim().parse::<u32>().unwrap_or(0))
                .unwrap_or(0);

            let cmd = format!("docker container prune -f{}", filter);
            let result = session.execute(&cmd).await
                .map_err(|e| ActionError::command_failed(&cmd, e.to_string()))?;

            stats.containers_removed = before;
            stats.space_reclaimed_mb += Self::parse_space_reclaimed(&result.stdout);
            info!("Removed {} containers", stats.containers_removed);
        }

        if options.images {
            let cmd = format!("docker image prune -af{}", filter);
            let result = session.execute(&cmd).await
                .map_err(|e| ActionError::command_failed(&cmd, e.to_string()))?;

            // Count "Deleted:" lines
            stats.images_removed = result.stdout
                .lines()
                .filter(|line| line.starts_with("Deleted:") || line.contains("deleted:"))
                .count() as u32;

            stats.space_reclaimed_mb += Self::parse_space_reclaimed(&result.stdout);
            info!("Removed {} images", stats.images_removed);
        }

        if options.volumes {
            let cmd = "docker volume prune -f";
            let result = session.execute(cmd).await
                .map_err(|e| ActionError::command_failed(cmd, e.to_string()))?;

            stats.volumes_removed = result.stdout
                .lines()
                .filter(|line| !line.is_empty() && !line.contains("Total") && !line.contains("reclaimed"))
                .count() as u32;

            stats.space_reclaimed_mb += Self::parse_space_reclaimed(&result.stdout);
            info!("Removed {} volumes", stats.volumes_removed);
        }

        if options.networks {
            let cmd = "docker network prune -f";
            let result = session.execute(cmd).await
                .map_err(|e| ActionError::command_failed(cmd, e.to_string()))?;

            stats.networks_removed = result.stdout
                .lines()
                .filter(|line| !line.is_empty() && !line.contains("Deleted"))
                .count() as u32;

            info!("Removed {} networks", stats.networks_removed);
        }

        if options.build_cache {
            let cmd = "docker builder prune -af";
            let result = session.execute(cmd).await;

            if let Ok(result) = result {
                stats.space_reclaimed_mb += Self::parse_space_reclaimed(&result.stdout);
            }
        }

        let duration = start.elapsed();
        info!(
            "Docker cleanup complete in {:?}: {} containers, {} images, {} volumes, {} networks, {} MB reclaimed",
            duration,
            stats.containers_removed,
            stats.images_removed,
            stats.volumes_removed,
            stats.networks_removed,
            stats.space_reclaimed_mb
        );

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size() {
        assert_eq!(CleanupDocker::parse_size("1.5GB"), 1536);
        assert_eq!(CleanupDocker::parse_size("100MB"), 100);
        assert_eq!(CleanupDocker::parse_size("500KB"), 0);
    }

    #[test]
    fn test_parse_space_reclaimed() {
        let output = "Deleted Images:\nsha256:abc123\nTotal reclaimed space: 2.5GB";
        assert_eq!(CleanupDocker::parse_space_reclaimed(output), 2560);
    }
}
