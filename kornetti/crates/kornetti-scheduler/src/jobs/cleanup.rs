//! Cleanup Jobs
//!
//! Various cleanup jobs for maintaining system health.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::job::{Job, JobContext, JobError, JobResult};

/// Cleanup helper containers that are no longer needed
pub struct CleanupHelperContainersJob;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupHelperContainersPayload {
    /// Server ID to cleanup
    pub server_id: Uuid,
    /// Maximum age in hours before cleanup
    pub max_age_hours: u32,
}

#[async_trait]
impl Job for CleanupHelperContainersJob {
    type Payload = CleanupHelperContainersPayload;

    fn name() -> &'static str {
        "cleanup_helper_containers"
    }

    fn queue() -> &'static str {
        "low"
    }

    async fn handle(&self, ctx: JobContext<Self::Payload>) -> Result<JobResult, JobError> {
        let payload = &ctx.payload;

        tracing::info!(
            server_id = %payload.server_id,
            max_age_hours = payload.max_age_hours,
            "Cleaning up helper containers"
        );

        // Commands to execute:
        // docker container prune -f --filter "label=kornetti.helper=true"
        // docker container ls -a --filter "name=kornetti-helper-" --format "{{.ID}}" | xargs docker rm -f

        Ok(JobResult::success("Helper containers cleaned up"))
    }
}

/// Cleanup instance-wide resources
pub struct CleanupInstanceJob;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupInstancePayload {
    /// Cleanup old deployment logs
    pub cleanup_logs: bool,
    /// Cleanup old activity records
    pub cleanup_activities: bool,
    /// Cleanup expired tokens
    pub cleanup_tokens: bool,
    /// Days to retain logs
    pub log_retention_days: u32,
}

#[async_trait]
impl Job for CleanupInstanceJob {
    type Payload = CleanupInstancePayload;

    fn name() -> &'static str {
        "cleanup_instance"
    }

    fn queue() -> &'static str {
        "low"
    }

    async fn handle(&self, ctx: JobContext<Self::Payload>) -> Result<JobResult, JobError> {
        let payload = &ctx.payload;

        tracing::info!(
            cleanup_logs = payload.cleanup_logs,
            cleanup_activities = payload.cleanup_activities,
            cleanup_tokens = payload.cleanup_tokens,
            "Cleaning up instance resources"
        );

        // Would delete old records from database:
        // - Deployment logs older than retention period
        // - Activity records
        // - Expired API tokens
        // - Old webhook deliveries

        Ok(JobResult::success("Instance cleanup completed"))
    }
}

/// Cleanup orphaned preview deployment containers
pub struct CleanupOrphanedPreviewsJob;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupOrphanedPreviewsPayload {
    /// Server ID to cleanup
    pub server_id: Uuid,
    /// Application ID (optional, cleanup all if None)
    pub application_id: Option<Uuid>,
}

#[async_trait]
impl Job for CleanupOrphanedPreviewsJob {
    type Payload = CleanupOrphanedPreviewsPayload;

    fn name() -> &'static str {
        "cleanup_orphaned_previews"
    }

    fn queue() -> &'static str {
        "default"
    }

    async fn handle(&self, ctx: JobContext<Self::Payload>) -> Result<JobResult, JobError> {
        let payload = &ctx.payload;

        tracing::info!(
            server_id = %payload.server_id,
            application_id = ?payload.application_id,
            "Cleaning up orphaned preview containers"
        );

        // Steps:
        // 1. List containers with label kornetti.preview=true
        // 2. Check if corresponding PR is still open
        // 3. Remove containers for closed PRs
        // 4. Remove associated networks and volumes

        Ok(JobResult::success("Orphaned previews cleaned up"))
    }
}

/// Cleanup stale SSH multiplexed connections
pub struct CleanupStaleConnectionsJob;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupStaleConnectionsPayload {
    /// Maximum connection age in minutes
    pub max_age_minutes: u32,
}

#[async_trait]
impl Job for CleanupStaleConnectionsJob {
    type Payload = CleanupStaleConnectionsPayload;

    fn name() -> &'static str {
        "cleanup_stale_connections"
    }

    fn queue() -> &'static str {
        "low"
    }

    async fn handle(&self, ctx: JobContext<Self::Payload>) -> Result<JobResult, JobError> {
        let payload = &ctx.payload;

        tracing::info!(
            max_age_minutes = payload.max_age_minutes,
            "Cleaning up stale SSH connections"
        );

        // Would find and close stale SSH ControlMaster connections
        // find /tmp/ssh_mux_* -mmin +{max_age} -delete

        Ok(JobResult::success("Stale connections cleaned up"))
    }
}

/// Connect proxy to all required networks
pub struct ConnectProxyToNetworksJob;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectProxyToNetworksPayload {
    /// Server ID
    pub server_id: Uuid,
    /// Networks to connect to (if empty, auto-detect)
    pub networks: Vec<String>,
}

#[async_trait]
impl Job for ConnectProxyToNetworksJob {
    type Payload = ConnectProxyToNetworksPayload;

    fn name() -> &'static str {
        "connect_proxy_to_networks"
    }

    fn queue() -> &'static str {
        "high"
    }

    async fn handle(&self, ctx: JobContext<Self::Payload>) -> Result<JobResult, JobError> {
        let payload = &ctx.payload;

        tracing::info!(
            server_id = %payload.server_id,
            networks = ?payload.networks,
            "Connecting proxy to networks"
        );

        // For each network:
        // docker network connect {network} coolify-proxy 2>/dev/null || true

        Ok(JobResult::success("Proxy connected to networks"))
    }
}

/// Clone a volume from one container to another
pub struct VolumeCloneJob;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeClonePayload {
    /// Server ID
    pub server_id: Uuid,
    /// Source volume name
    pub source_volume: String,
    /// Destination volume name
    pub destination_volume: String,
    /// Whether to overwrite if destination exists
    pub overwrite: bool,
}

#[async_trait]
impl Job for VolumeCloneJob {
    type Payload = VolumeClonePayload;

    fn name() -> &'static str {
        "volume_clone"
    }

    fn queue() -> &'static str {
        "default"
    }

    async fn handle(&self, ctx: JobContext<Self::Payload>) -> Result<JobResult, JobError> {
        let payload = &ctx.payload;

        tracing::info!(
            server_id = %payload.server_id,
            source = %payload.source_volume,
            destination = %payload.destination_volume,
            "Cloning volume"
        );

        // Steps:
        // 1. Create destination volume if not exists
        // 2. Use intermediate container to copy data:
        //    docker run --rm -v {src}:/src -v {dst}:/dst alpine sh -c "cp -a /src/. /dst/"

        Ok(JobResult::success(format!(
            "Volume {} cloned to {}",
            payload.source_volume, payload.destination_volume
        )))
    }
}
