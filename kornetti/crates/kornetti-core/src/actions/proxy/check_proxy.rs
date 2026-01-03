//! Check Proxy Action
//!
//! Checks if the proxy is running and healthy.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument, warn};

use crate::models::Server;
use super::ProxyType;
use crate::actions::{Action, ActionError};
use kornetti_ssh::SshClient;

/// Proxy health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyHealthCheck {
    pub running: bool,
    pub healthy: bool,
    pub proxy_type: ProxyType,
    pub http_port_open: bool,
    pub https_port_open: bool,
    pub dashboard_accessible: bool,
    pub container_status: Option<String>,
    pub errors: Vec<String>,
}

impl Default for ProxyHealthCheck {
    fn default() -> Self {
        Self {
            running: false,
            healthy: false,
            proxy_type: ProxyType::None,
            http_port_open: false,
            https_port_open: false,
            dashboard_accessible: false,
            container_status: None,
            errors: Vec::new(),
        }
    }
}

pub struct CheckProxyInput<'a> {
    pub server: &'a Server,
}

pub struct CheckProxy {
    ssh: Arc<SshClient>,
}

impl CheckProxy {
    pub fn new(ssh: Arc<SshClient>) -> Self {
        Self { ssh }
    }
}

#[async_trait]
impl Action for CheckProxy {
    type Input = CheckProxyInput<'static>;
    type Output = ProxyHealthCheck;

    fn name(&self) -> &'static str {
        "check_proxy"
    }

    #[instrument(skip(self, input), fields(server_id = %input.server.id))]
    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        let server = input.server;

        info!("Checking proxy health on server {}", server.id);

        let session = self.ssh.connect(server)
            .await
            .map_err(|e| ActionError::ssh_connection_failed(e.to_string()))?;

        let mut health = ProxyHealthCheck::default();

        // Check if container exists and is running
        let status = session.execute(
            "docker inspect coolify-proxy --format '{{.State.Status}}|{{.State.Health.Status}}|{{.Config.Image}}' 2>/dev/null"
        )
            .await
            .ok();

        if let Some(output) = status {
            if output.exit_code == 0 {
                let parts: Vec<&str> = output.stdout.trim().split('|').collect();
                if !parts.is_empty() {
                    health.container_status = Some(parts[0].to_string());
                    health.running = parts[0] == "running";

                    if parts.len() > 1 {
                        health.healthy = parts[1] == "healthy" || parts[1].is_empty();
                    }

                    if parts.len() > 2 {
                        let image = parts[2];
                        if image.contains("traefik") {
                            health.proxy_type = ProxyType::Traefik;
                        } else if image.contains("caddy") {
                            health.proxy_type = ProxyType::Caddy;
                        }
                    }
                }
            }
        } else {
            health.errors.push("Proxy container not found".to_string());
        }

        // Check if ports are listening
        let port_check = session.execute(
            "ss -tlnp 2>/dev/null | grep -E ':80|:443|:8080' | head -5"
        )
            .await
            .ok();

        if let Some(output) = port_check {
            health.http_port_open = output.stdout.contains(":80 ");
            health.https_port_open = output.stdout.contains(":443 ");
            health.dashboard_accessible = output.stdout.contains(":8080 ");
        }

        // Additional checks for running proxy
        if health.running {
            // Check proxy logs for errors
            let logs = session.execute(
                "docker logs coolify-proxy --tail 50 2>&1 | grep -i error | tail -5"
            )
                .await
                .ok();

            if let Some(output) = logs {
                for line in output.stdout.lines() {
                    if !line.is_empty() {
                        health.errors.push(line.to_string());
                    }
                }
            }

            // Try to reach the proxy locally
            let curl_check = session.execute(
                "curl -s -o /dev/null -w '%{http_code}' http://localhost:80 2>/dev/null || echo '000'"
            )
                .await
                .ok();

            if let Some(output) = curl_check {
                let status_code = output.stdout.trim();
                if status_code == "000" {
                    health.errors.push("Proxy not responding on port 80".to_string());
                }
            }
        }

        // Determine overall health
        health.healthy = health.running
            && health.http_port_open
            && health.errors.is_empty();

        if !health.healthy && health.running {
            warn!("Proxy is running but unhealthy: {:?}", health.errors);
        }

        Ok(health)
    }
}
