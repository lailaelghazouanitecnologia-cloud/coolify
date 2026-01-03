//! Sentinel Job
//!
//! Manages the Sentinel monitoring container on servers.
//! Sentinel collects metrics and pushes them to the Kornetti instance.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error, instrument};
use uuid::Uuid;

use super::{Job, JobContext, JobResult, JobError};

/// Sentinel container configuration
const SENTINEL_IMAGE: &str = "ghcr.io/coollabsio/sentinel:latest";
const SENTINEL_CONTAINER_NAME: &str = "coolify-sentinel";

/// Sentinel job for starting/checking the monitoring container
pub struct CheckAndStartSentinelJob;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentinelContext {
    pub server_id: Uuid,
    pub push_endpoint: String,
    pub token: String,
    pub refresh_rate_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentinelResult {
    pub running: bool,
    pub container_id: Option<String>,
    pub version: Option<String>,
    pub started: bool,
}

#[async_trait]
impl Job for CheckAndStartSentinelJob {
    type Context = SentinelContext;
    type Result = SentinelResult;

    fn name(&self) -> &'static str {
        "check_and_start_sentinel"
    }

    #[instrument(skip(self, ctx), fields(server_id = %ctx.payload.server_id))]
    async fn handle(&self, ctx: JobContext<Self::Context>) -> JobResult<Self::Result> {
        let data = &ctx.payload;
        info!("Checking Sentinel status on server {}", data.server_id);

        // Check if Sentinel is already running
        let check_cmd = format!(
            "docker ps --filter name={} --format '{{{{.ID}}}}' 2>/dev/null || true",
            SENTINEL_CONTAINER_NAME
        );

        // TODO: Execute via SSH
        // let output = ssh.execute(&check_cmd).await?;

        let container_id: Option<String> = None; // Placeholder

        if container_id.is_some() {
            info!("Sentinel is already running");
            return Ok(SentinelResult {
                running: true,
                container_id,
                version: Some("latest".to_string()),
                started: false,
            });
        }

        // Sentinel not running, start it
        info!("Starting Sentinel container");

        let start_cmd = format!(
            r#"docker run -d \
                --name {} \
                --restart unless-stopped \
                --network host \
                -v /var/run/docker.sock:/var/run/docker.sock:ro \
                -e PUSH_ENDPOINT={} \
                -e PUSH_INTERVAL_SECONDS={} \
                -e TOKEN={} \
                {}"#,
            SENTINEL_CONTAINER_NAME,
            data.push_endpoint,
            data.refresh_rate_seconds,
            data.token,
            SENTINEL_IMAGE
        );

        // TODO: Execute via SSH
        // let output = ssh.execute(&start_cmd).await?;

        info!("Sentinel container started");
        Ok(SentinelResult {
            running: true,
            container_id: Some("placeholder".to_string()),
            version: Some("latest".to_string()),
            started: true,
        })
    }
}

/// Job to stop Sentinel container
pub struct StopSentinelJob;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopSentinelContext {
    pub server_id: Uuid,
}

#[async_trait]
impl Job for StopSentinelJob {
    type Context = StopSentinelContext;
    type Result = bool;

    fn name(&self) -> &'static str {
        "stop_sentinel"
    }

    #[instrument(skip(self, ctx), fields(server_id = %ctx.payload.server_id))]
    async fn handle(&self, ctx: JobContext<Self::Context>) -> JobResult<Self::Result> {
        info!("Stopping Sentinel on server {}", ctx.payload.server_id);

        let stop_cmd = format!(
            "docker stop {} && docker rm {} 2>/dev/null || true",
            SENTINEL_CONTAINER_NAME,
            SENTINEL_CONTAINER_NAME
        );

        // TODO: Execute via SSH
        // ssh.execute(&stop_cmd).await?;

        info!("Sentinel stopped");
        Ok(true)
    }
}

/// Job to update Sentinel container
pub struct UpdateSentinelJob;

#[async_trait]
impl Job for UpdateSentinelJob {
    type Context = SentinelContext;
    type Result = SentinelResult;

    fn name(&self) -> &'static str {
        "update_sentinel"
    }

    #[instrument(skip(self, ctx), fields(server_id = %ctx.payload.server_id))]
    async fn handle(&self, ctx: JobContext<Self::Context>) -> JobResult<Self::Result> {
        let data = &ctx.payload;
        info!("Updating Sentinel on server {}", data.server_id);

        // Pull latest image
        let pull_cmd = format!("docker pull {}", SENTINEL_IMAGE);
        // TODO: ssh.execute(&pull_cmd).await?;

        // Stop existing container
        let stop_cmd = format!(
            "docker stop {} 2>/dev/null || true && docker rm {} 2>/dev/null || true",
            SENTINEL_CONTAINER_NAME,
            SENTINEL_CONTAINER_NAME
        );
        // TODO: ssh.execute(&stop_cmd).await?;

        // Start new container
        let start_cmd = format!(
            r#"docker run -d \
                --name {} \
                --restart unless-stopped \
                --network host \
                -v /var/run/docker.sock:/var/run/docker.sock:ro \
                -e PUSH_ENDPOINT={} \
                -e PUSH_INTERVAL_SECONDS={} \
                -e TOKEN={} \
                {}"#,
            SENTINEL_CONTAINER_NAME,
            data.push_endpoint,
            data.refresh_rate_seconds,
            data.token,
            SENTINEL_IMAGE
        );
        // TODO: ssh.execute(&start_cmd).await?;

        info!("Sentinel updated successfully");
        Ok(SentinelResult {
            running: true,
            container_id: Some("placeholder".to_string()),
            version: Some("latest".to_string()),
            started: true,
        })
    }
}

/// Sentinel push data from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentinelPushData {
    pub server_id: Uuid,
    pub timestamp: i64,
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub memory_total_mb: u64,
    pub memory_used_mb: u64,
    pub disk_usage_percent: f64,
    pub disk_total_gb: u64,
    pub disk_used_gb: u64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub container_count: u32,
    pub container_running: u32,
    pub load_average: [f64; 3],
    pub uptime_seconds: u64,
}

/// Process incoming Sentinel push data
pub struct ProcessSentinelPushJob;

#[async_trait]
impl Job for ProcessSentinelPushJob {
    type Context = SentinelPushData;
    type Result = bool;

    fn name(&self) -> &'static str {
        "process_sentinel_push"
    }

    #[instrument(skip(self, ctx), fields(server_id = %ctx.payload.server_id))]
    async fn handle(&self, ctx: JobContext<Self::Context>) -> JobResult<Self::Result> {
        let data = &ctx.payload;

        info!(
            "Processing Sentinel data for server {}: CPU {}%, Memory {}%, Disk {}%",
            data.server_id,
            data.cpu_usage_percent,
            data.memory_usage_percent,
            data.disk_usage_percent
        );

        // Check for alerts
        if data.disk_usage_percent > 90.0 {
            warn!("Server {} disk usage critical: {}%", data.server_id, data.disk_usage_percent);
            // TODO: Send notification
        }

        if data.memory_usage_percent > 90.0 {
            warn!("Server {} memory usage high: {}%", data.server_id, data.memory_usage_percent);
            // TODO: Send notification
        }

        // TODO: Store metrics in database
        // TODO: Update server last_seen timestamp

        Ok(true)
    }
}
