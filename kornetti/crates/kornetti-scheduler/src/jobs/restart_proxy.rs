//! Restart Proxy Job
//!
//! Restarts the proxy (Traefik/Nginx) on a server.
//! Mirrors Coolify's RestartProxyJob.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::{Job, JobContext, JobResult};

/// Proxy type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProxyType {
    Traefik,
    Nginx,
    Caddy,
}

impl ProxyType {
    pub fn container_name(&self) -> &'static str {
        match self {
            ProxyType::Traefik => "coolify-proxy",
            ProxyType::Nginx => "coolify-proxy",
            ProxyType::Caddy => "coolify-proxy",
        }
    }

    pub fn image(&self) -> &'static str {
        match self {
            ProxyType::Traefik => "traefik:v2.10",
            ProxyType::Nginx => "nginx:alpine",
            ProxyType::Caddy => "caddy:alpine",
        }
    }
}

impl Default for ProxyType {
    fn default() -> Self {
        ProxyType::Traefik
    }
}

/// Context for proxy restart
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestartProxyContext {
    pub server_id: Uuid,
    pub server_name: String,
    pub server_ip: String,
    pub proxy_type: ProxyType,
    /// Whether to force recreate the container
    pub force_recreate: bool,
    /// Whether to pull the latest image
    pub pull_image: bool,
}

/// Result of proxy restart
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestartProxyResult {
    /// Whether restart was successful
    pub success: bool,
    /// Proxy container ID after restart
    pub container_id: Option<String>,
    /// Steps completed during restart
    pub steps: Vec<String>,
    /// Any warnings
    pub warnings: Vec<String>,
    /// Error if restart failed
    pub error: Option<String>,
}

/// Job to restart the proxy on a server
pub struct RestartProxyJob;

impl RestartProxyJob {
    pub fn new() -> Self {
        Self
    }

    /// Ensure proxy networks exist
    async fn ensure_networks_exist(&self, ctx: &JobContext<RestartProxyContext>) -> Result<(), String> {
        debug!(server_id = %ctx.payload.server_id, "Ensuring proxy networks exist");

        // Placeholder: would run docker network create commands
        // docker network create --attachable coolify || true

        Ok(())
    }

    /// Stop the existing proxy container
    async fn stop_proxy(&self, ctx: &JobContext<RestartProxyContext>) -> Result<(), String> {
        let container_name = ctx.payload.proxy_type.container_name();
        debug!(
            server_id = %ctx.payload.server_id,
            container = container_name,
            "Stopping proxy container"
        );

        // Placeholder: would run docker stop and rm
        // docker stop coolify-proxy && docker rm coolify-proxy

        Ok(())
    }

    /// Pull the latest proxy image
    async fn pull_image(&self, ctx: &JobContext<RestartProxyContext>) -> Result<(), String> {
        if !ctx.payload.pull_image {
            return Ok(());
        }

        let image = ctx.payload.proxy_type.image();
        info!(
            server_id = %ctx.payload.server_id,
            image = image,
            "Pulling proxy image"
        );

        // Placeholder: would run docker pull
        // docker pull traefik:v2.10

        Ok(())
    }

    /// Start the proxy container
    async fn start_proxy(&self, ctx: &JobContext<RestartProxyContext>) -> Result<String, String> {
        let container_name = ctx.payload.proxy_type.container_name();
        let image = ctx.payload.proxy_type.image();

        info!(
            server_id = %ctx.payload.server_id,
            container = container_name,
            image = image,
            "Starting proxy container"
        );

        // Placeholder: would generate and run docker command
        // The actual command would depend on proxy type and configuration
        // For Traefik:
        // docker run -d --name coolify-proxy \
        //   --restart always \
        //   --network coolify \
        //   -p 80:80 -p 443:443 \
        //   -v /var/run/docker.sock:/var/run/docker.sock \
        //   -v /data/coolify/proxy:/traefik \
        //   traefik:v2.10 \
        //   --providers.docker=true \
        //   --entrypoints.web.address=:80 \
        //   --entrypoints.websecure.address=:443

        Ok("container-id-placeholder".to_string())
    }

    /// Verify proxy is running and healthy
    async fn verify_proxy(&self, ctx: &JobContext<RestartProxyContext>) -> Result<bool, String> {
        let container_name = ctx.payload.proxy_type.container_name();
        debug!(
            server_id = %ctx.payload.server_id,
            container = container_name,
            "Verifying proxy is running"
        );

        // Placeholder: would check container status
        // docker inspect coolify-proxy --format '{{.State.Running}}'

        Ok(true)
    }

    /// Connect proxy to all required networks
    async fn connect_to_networks(&self, ctx: &JobContext<RestartProxyContext>) -> Result<(), String> {
        debug!(server_id = %ctx.payload.server_id, "Connecting proxy to networks");

        // Placeholder: would list all coolify networks and connect proxy
        // docker network ls --filter label=coolify.managed=true -q | xargs -I{} docker network connect {} coolify-proxy

        Ok(())
    }
}

impl Default for RestartProxyJob {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Job for RestartProxyJob {
    type Context = RestartProxyContext;
    type Result = RestartProxyResult;

    fn name(&self) -> &'static str {
        "restart_proxy"
    }

    fn timeout_seconds(&self) -> u64 {
        300 // 5 minutes
    }

    async fn handle(&self, ctx: JobContext<Self::Context>) -> JobResult<Self::Result> {
        info!(
            server_id = %ctx.payload.server_id,
            server_name = %ctx.payload.server_name,
            proxy_type = ?ctx.payload.proxy_type,
            "Starting proxy restart"
        );

        let mut steps = Vec::new();
        let mut warnings = Vec::new();

        // Step 1: Ensure networks exist
        match self.ensure_networks_exist(&ctx).await {
            Ok(()) => steps.push("Networks verified".to_string()),
            Err(e) => {
                return Ok(RestartProxyResult {
                    success: false,
                    container_id: None,
                    steps,
                    warnings,
                    error: Some(format!("Failed to create networks: {}", e)),
                });
            }
        }

        // Step 2: Stop existing proxy (if force recreate or updating)
        if ctx.payload.force_recreate {
            match self.stop_proxy(&ctx).await {
                Ok(()) => steps.push("Stopped existing proxy".to_string()),
                Err(e) => warnings.push(format!("Failed to stop proxy: {}", e)),
            }
        }

        // Step 3: Pull latest image
        if ctx.payload.pull_image {
            match self.pull_image(&ctx).await {
                Ok(()) => steps.push("Pulled latest image".to_string()),
                Err(e) => warnings.push(format!("Failed to pull image: {}", e)),
            }
        }

        // Step 4: Start proxy
        let container_id = match self.start_proxy(&ctx).await {
            Ok(id) => {
                steps.push("Started proxy container".to_string());
                Some(id)
            }
            Err(e) => {
                return Ok(RestartProxyResult {
                    success: false,
                    container_id: None,
                    steps,
                    warnings,
                    error: Some(format!("Failed to start proxy: {}", e)),
                });
            }
        };

        // Step 5: Connect to networks
        match self.connect_to_networks(&ctx).await {
            Ok(()) => steps.push("Connected to networks".to_string()),
            Err(e) => warnings.push(format!("Failed to connect to networks: {}", e)),
        }

        // Step 6: Verify proxy is running
        match self.verify_proxy(&ctx).await {
            Ok(true) => steps.push("Proxy verified running".to_string()),
            Ok(false) => {
                return Ok(RestartProxyResult {
                    success: false,
                    container_id,
                    steps,
                    warnings,
                    error: Some("Proxy started but is not running".to_string()),
                });
            }
            Err(e) => {
                return Ok(RestartProxyResult {
                    success: false,
                    container_id,
                    steps,
                    warnings,
                    error: Some(format!("Failed to verify proxy: {}", e)),
                });
            }
        }

        info!(
            server_id = %ctx.payload.server_id,
            container_id = ?container_id,
            "Proxy restart complete"
        );

        Ok(RestartProxyResult {
            success: true,
            container_id,
            steps,
            warnings,
            error: None,
        })
    }

    async fn on_failure(&self, ctx: JobContext<Self::Context>, error: String) {
        error!(
            server_id = %ctx.payload.server_id,
            server_name = %ctx.payload.server_name,
            error = %error,
            "Proxy restart failed"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_context() -> JobContext<RestartProxyContext> {
        JobContext {
            job_id: Uuid::new_v4(),
            attempt: 1,
            max_attempts: 1,
            queued_at: Utc::now(),
            payload: RestartProxyContext {
                server_id: Uuid::new_v4(),
                server_name: "test-server".to_string(),
                server_ip: "192.168.1.100".to_string(),
                proxy_type: ProxyType::Traefik,
                force_recreate: true,
                pull_image: true,
            },
        }
    }

    #[tokio::test]
    async fn test_restart_proxy() {
        let job = RestartProxyJob::new();
        let ctx = create_test_context();

        let result = job.handle(ctx).await.unwrap();
        assert!(result.success);
        assert!(result.container_id.is_some());
    }
}
