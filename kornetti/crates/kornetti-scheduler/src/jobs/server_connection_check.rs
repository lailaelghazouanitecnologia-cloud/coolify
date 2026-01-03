//! Server Connection Check Job
//!
//! Periodically checks if servers are reachable and Docker is available.
//! Mirrors Coolify's ServerConnectionCheckJob.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::{Job, JobContext, JobResult};

/// Context for server connection check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConnectionCheckContext {
    pub server_id: Uuid,
    pub server_name: String,
    pub server_ip: String,
    pub server_port: u16,
    pub server_user: String,
    /// Whether to disable SSH multiplexing for this check
    pub disable_mux: bool,
}

/// Result of server connection check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionCheckResult {
    /// Whether the server is reachable via SSH
    pub is_reachable: bool,
    /// Whether Docker is available and working
    pub is_usable: bool,
    /// Docker version if available
    pub docker_version: Option<String>,
    /// Error message if check failed
    pub error: Option<String>,
    /// Server uptime if available
    pub uptime: Option<String>,
}

/// Job to check server SSH connectivity and Docker availability
pub struct ServerConnectionCheckJob {
    timeout_seconds: u64,
}

impl ServerConnectionCheckJob {
    pub fn new() -> Self {
        Self {
            timeout_seconds: 30,
        }
    }

    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    /// Check basic SSH connectivity
    async fn check_connection(&self, ctx: &JobContext<ServerConnectionCheckContext>) -> Result<bool, String> {
        // In a real implementation, this would use the SSH client
        // to run a simple command like "ls -la /"
        debug!(
            server_id = %ctx.payload.server_id,
            server_ip = %ctx.payload.server_ip,
            "Checking SSH connection"
        );

        // Placeholder: actual implementation would use kornetti_ssh
        // let output = ssh_client.execute_with_timeout(
        //     &ctx.payload.server_ip,
        //     ctx.payload.server_port,
        //     &ctx.payload.server_user,
        //     "ls -la /",
        //     Duration::from_secs(self.timeout_seconds)
        // ).await;

        Ok(true) // Placeholder
    }

    /// Check Docker availability
    async fn check_docker(&self, ctx: &JobContext<ServerConnectionCheckContext>) -> Result<Option<String>, String> {
        debug!(
            server_id = %ctx.payload.server_id,
            "Checking Docker availability"
        );

        // Placeholder: actual implementation would run "docker version --format json"
        // and parse the output
        // let output = ssh_client.execute(
        //     &ctx.payload.server_ip,
        //     ctx.payload.server_port,
        //     &ctx.payload.server_user,
        //     "docker version --format json"
        // ).await;
        //
        // if let Ok(json) = serde_json::from_str::<DockerVersion>(&output) {
        //     return Ok(Some(json.server.version));
        // }

        Ok(Some("24.0.0".to_string())) // Placeholder
    }

    /// Get server uptime
    async fn get_uptime(&self, _ctx: &JobContext<ServerConnectionCheckContext>) -> Option<String> {
        // Placeholder: would run "uptime" command
        Some("1 day, 2 hours".to_string())
    }
}

impl Default for ServerConnectionCheckJob {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Job for ServerConnectionCheckJob {
    type Context = ServerConnectionCheckContext;
    type Result = ConnectionCheckResult;

    fn name(&self) -> &'static str {
        "server_connection_check"
    }

    fn max_tries(&self) -> u32 {
        1
    }

    fn timeout_seconds(&self) -> u64 {
        self.timeout_seconds
    }

    async fn handle(&self, ctx: JobContext<Self::Context>) -> JobResult<Self::Result> {
        info!(
            server_id = %ctx.payload.server_id,
            server_name = %ctx.payload.server_name,
            "Starting connection check"
        );

        // Check basic connectivity
        let is_reachable = match self.check_connection(&ctx).await {
            Ok(reachable) => reachable,
            Err(e) => {
                warn!(
                    server_id = %ctx.payload.server_id,
                    error = %e,
                    "Server not reachable"
                );
                return Ok(ConnectionCheckResult {
                    is_reachable: false,
                    is_usable: false,
                    docker_version: None,
                    error: Some(e),
                    uptime: None,
                });
            }
        };

        if !is_reachable {
            return Ok(ConnectionCheckResult {
                is_reachable: false,
                is_usable: false,
                docker_version: None,
                error: Some("Server not reachable via SSH".to_string()),
                uptime: None,
            });
        }

        // Check Docker availability
        let (is_usable, docker_version) = match self.check_docker(&ctx).await {
            Ok(Some(version)) => (true, Some(version)),
            Ok(None) => (false, None),
            Err(e) => {
                warn!(
                    server_id = %ctx.payload.server_id,
                    error = %e,
                    "Docker not available"
                );
                (false, None)
            }
        };

        let uptime = self.get_uptime(&ctx).await;

        info!(
            server_id = %ctx.payload.server_id,
            is_reachable = is_reachable,
            is_usable = is_usable,
            docker_version = ?docker_version,
            "Connection check complete"
        );

        Ok(ConnectionCheckResult {
            is_reachable,
            is_usable,
            docker_version,
            error: None,
            uptime,
        })
    }

    async fn on_failure(&self, ctx: JobContext<Self::Context>, error: String) {
        error!(
            server_id = %ctx.payload.server_id,
            server_name = %ctx.payload.server_name,
            error = %error,
            "Server connection check failed"
        );

        // In a real implementation, update the database:
        // UPDATE server_settings SET is_reachable = false, is_usable = false WHERE server_id = ?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_context() -> JobContext<ServerConnectionCheckContext> {
        JobContext {
            job_id: Uuid::new_v4(),
            attempt: 1,
            max_attempts: 1,
            queued_at: Utc::now(),
            payload: ServerConnectionCheckContext {
                server_id: Uuid::new_v4(),
                server_name: "test-server".to_string(),
                server_ip: "192.168.1.100".to_string(),
                server_port: 22,
                server_user: "root".to_string(),
                disable_mux: false,
            },
        }
    }

    #[tokio::test]
    async fn test_server_connection_check() {
        let job = ServerConnectionCheckJob::new();
        let ctx = create_test_context();

        let result = job.handle(ctx).await.unwrap();
        assert!(result.is_reachable);
    }
}
