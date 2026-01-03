//! Docker Cleanup Job
//!
//! Cleans up unused Docker resources (images, containers, volumes, networks).

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{info, instrument, warn};
use uuid::Uuid;

use kornetti_core::{Result, Error, models::Server};
use kornetti_ssh::SshClient;

/// Cleanup context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerCleanupContext {
    pub server_id: Uuid,
    /// Cleanup unused images
    pub cleanup_images: bool,
    /// Cleanup stopped containers
    pub cleanup_containers: bool,
    /// Cleanup unused volumes
    pub cleanup_volumes: bool,
    /// Cleanup unused networks
    pub cleanup_networks: bool,
    /// Cleanup build cache
    pub cleanup_builder_cache: bool,
    /// Force cleanup (bypass confirmation)
    pub force: bool,
}

impl Default for DockerCleanupContext {
    fn default() -> Self {
        Self {
            server_id: Uuid::nil(),
            cleanup_images: true,
            cleanup_containers: true,
            cleanup_volumes: false, // Don't clean volumes by default (data loss risk)
            cleanup_networks: true,
            cleanup_builder_cache: true,
            force: true,
        }
    }
}

/// Docker cleanup job
pub struct DockerCleanupJob {
    ssh: Arc<SshClient>,
    context: DockerCleanupContext,
}

impl DockerCleanupJob {
    pub fn new(ssh: Arc<SshClient>, context: DockerCleanupContext) -> Self {
        Self { ssh, context }
    }

    /// Run the cleanup
    #[instrument(skip(self, server))]
    pub async fn run(&self, server: &Server) -> Result<CleanupResult> {
        info!(server_id = %server.id, "Starting Docker cleanup");

        let start_time = Utc::now();
        let mut result = CleanupResult {
            server_id: server.id,
            started_at: start_time,
            completed_at: start_time,
            space_reclaimed_bytes: 0,
            images_removed: 0,
            containers_removed: 0,
            volumes_removed: 0,
            networks_removed: 0,
            build_cache_cleared: false,
            errors: Vec::new(),
        };

        // Cleanup stopped containers
        if self.context.cleanup_containers {
            match self.cleanup_containers(server).await {
                Ok(count) => {
                    result.containers_removed = count;
                    info!(count = count, "Removed stopped containers");
                }
                Err(e) => {
                    result.errors.push(format!("Container cleanup failed: {}", e));
                }
            }
        }

        // Cleanup unused images
        if self.context.cleanup_images {
            match self.cleanup_images(server).await {
                Ok((count, bytes)) => {
                    result.images_removed = count;
                    result.space_reclaimed_bytes += bytes;
                    info!(count = count, bytes = bytes, "Removed unused images");
                }
                Err(e) => {
                    result.errors.push(format!("Image cleanup failed: {}", e));
                }
            }
        }

        // Cleanup unused volumes (careful!)
        if self.context.cleanup_volumes {
            match self.cleanup_volumes(server).await {
                Ok(count) => {
                    result.volumes_removed = count;
                    info!(count = count, "Removed unused volumes");
                }
                Err(e) => {
                    result.errors.push(format!("Volume cleanup failed: {}", e));
                }
            }
        }

        // Cleanup unused networks
        if self.context.cleanup_networks {
            match self.cleanup_networks(server).await {
                Ok(count) => {
                    result.networks_removed = count;
                    info!(count = count, "Removed unused networks");
                }
                Err(e) => {
                    result.errors.push(format!("Network cleanup failed: {}", e));
                }
            }
        }

        // Cleanup build cache
        if self.context.cleanup_builder_cache {
            match self.cleanup_build_cache(server).await {
                Ok(bytes) => {
                    result.build_cache_cleared = true;
                    result.space_reclaimed_bytes += bytes;
                    info!(bytes = bytes, "Cleared build cache");
                }
                Err(e) => {
                    result.errors.push(format!("Build cache cleanup failed: {}", e));
                }
            }
        }

        result.completed_at = Utc::now();
        Ok(result)
    }

    /// Cleanup stopped containers
    async fn cleanup_containers(&self, server: &Server) -> Result<u32> {
        let force_flag = if self.context.force { "-f" } else { "" };
        let result = self.ssh.execute(
            server,
            &format!("docker container prune {} 2>&1 | grep -oP 'Total reclaimed space: \\K[0-9]+' || echo 0", force_flag)
        ).await?;

        // Count removed containers
        let list_result = self.ssh.execute(
            server,
            "docker ps -a -q -f status=exited | wc -l"
        ).await?;

        let count: u32 = list_result.output.stdout.trim().parse().unwrap_or(0);
        Ok(count)
    }

    /// Cleanup unused images
    async fn cleanup_images(&self, server: &Server) -> Result<(u32, i64)> {
        let force_flag = if self.context.force { "-f" } else { "" };

        // Get before size
        let before = self.ssh.execute(
            server,
            "docker system df --format '{{.Size}}' | head -1"
        ).await?;

        // Prune images
        let result = self.ssh.execute(
            server,
            &format!("docker image prune -a {} 2>&1", force_flag)
        ).await?;

        // Get after size to calculate reclaimed
        let after = self.ssh.execute(
            server,
            "docker system df --format '{{.Size}}' | head -1"
        ).await?;

        // Parse output to get count
        let count = result.output.stdout
            .lines()
            .filter(|l| l.starts_with("deleted:") || l.contains("Deleted"))
            .count() as u32;

        // Estimate bytes reclaimed (would need actual parsing)
        let bytes_reclaimed = 0i64;

        Ok((count, bytes_reclaimed))
    }

    /// Cleanup unused volumes
    async fn cleanup_volumes(&self, server: &Server) -> Result<u32> {
        let force_flag = if self.context.force { "-f" } else { "" };
        let result = self.ssh.execute(
            server,
            &format!("docker volume prune {} 2>&1", force_flag)
        ).await?;

        let count = result.output.stdout
            .lines()
            .filter(|l| l.contains("Deleted") || l.len() == 64) // Volume IDs are 64 chars
            .count() as u32;

        Ok(count)
    }

    /// Cleanup unused networks
    async fn cleanup_networks(&self, server: &Server) -> Result<u32> {
        let force_flag = if self.context.force { "-f" } else { "" };
        let result = self.ssh.execute(
            server,
            &format!("docker network prune {} 2>&1", force_flag)
        ).await?;

        let count = result.output.stdout
            .lines()
            .filter(|l| l.contains("Deleted"))
            .count() as u32;

        Ok(count)
    }

    /// Cleanup build cache
    async fn cleanup_build_cache(&self, server: &Server) -> Result<i64> {
        let force_flag = if self.context.force { "-f" } else { "" };
        let result = self.ssh.execute(
            server,
            &format!("docker builder prune {} --all 2>&1", force_flag)
        ).await?;

        // Parse reclaimed space from output
        let bytes = 0i64; // Would parse from output

        Ok(bytes)
    }
}

/// Cleanup result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupResult {
    pub server_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub space_reclaimed_bytes: i64,
    pub images_removed: u32,
    pub containers_removed: u32,
    pub volumes_removed: u32,
    pub networks_removed: u32,
    pub build_cache_cleared: bool,
    pub errors: Vec<String>,
}

impl CleanupResult {
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn space_reclaimed_formatted(&self) -> String {
        let bytes = self.space_reclaimed_bytes as f64;
        if bytes >= 1_073_741_824.0 {
            format!("{:.2} GB", bytes / 1_073_741_824.0)
        } else if bytes >= 1_048_576.0 {
            format!("{:.2} MB", bytes / 1_048_576.0)
        } else if bytes >= 1024.0 {
            format!("{:.2} KB", bytes / 1024.0)
        } else {
            format!("{} bytes", self.space_reclaimed_bytes)
        }
    }

    pub fn duration_seconds(&self) -> i64 {
        (self.completed_at - self.started_at).num_seconds()
    }
}
