//! Get Proxy Configuration Action
//!
//! Retrieves the current proxy configuration from a server.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use crate::models::Server;
use super::{ProxyType, proxy_path, traefik_path, caddy_path};
use crate::actions::{Action, ActionError};
use crate::ssh_stub::SshClient;

/// Proxy status and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyStatus {
    pub running: bool,
    pub proxy_type: ProxyType,
    pub container_id: Option<String>,
    pub image: Option<String>,
    pub uptime: Option<String>,
    pub config_content: Option<String>,
    pub dynamic_configs: Vec<String>,
}

pub struct GetProxyConfigurationInput<'a> {
    pub server: &'a Server,
}

pub struct GetProxyConfiguration {
    ssh: Arc<dyn SshClient>,
}

impl GetProxyConfiguration {
    pub fn new(ssh: Arc<dyn SshClient>) -> Self {
        Self { ssh }
    }
}

#[async_trait]
impl Action for GetProxyConfiguration {
    type Input = GetProxyConfigurationInput<'static>;
    type Output = ProxyStatus;

    fn name(&self) -> &'static str {
        "get_proxy_configuration"
    }

    #[instrument(skip(self, input), fields(server_id = %input.server.id))]
    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        let server = input.server;

        info!("Getting proxy configuration for server {}", server.id);

        let session = self.ssh.connect(server)
            .await
            .map_err(|e| ActionError::ssh_connection_failed(e.to_string()))?;

        let mut status = ProxyStatus {
            running: false,
            proxy_type: ProxyType::None,
            container_id: None,
            image: None,
            uptime: None,
            config_content: None,
            dynamic_configs: Vec::new(),
        };

        // Check if proxy container is running
        let container_info = session.execute(
            "docker inspect coolify-proxy --format '{{.Id}}|{{.Config.Image}}|{{.State.Running}}|{{.State.StartedAt}}' 2>/dev/null"
        )
            .await
            .ok();

        if let Some(output) = container_info {
            if output.exit_code == 0 {
                let parts: Vec<&str> = output.stdout.trim().split('|').collect();
                if parts.len() >= 4 {
                    status.container_id = Some(parts[0][..12].to_string());
                    status.image = Some(parts[1].to_string());
                    status.running = parts[2] == "true";
                    status.uptime = Some(parts[3].to_string());

                    // Detect proxy type from image
                    if let Some(ref image) = status.image {
                        if image.contains("traefik") {
                            status.proxy_type = ProxyType::Traefik;
                        } else if image.contains("caddy") {
                            status.proxy_type = ProxyType::Caddy;
                        }
                    }
                }
            }
        }

        // Get main config file
        let config_path = match status.proxy_type {
            ProxyType::Traefik => format!("{}/traefik.yml", traefik_path()),
            ProxyType::Caddy => format!("{}/Caddyfile", caddy_path()),
            ProxyType::None => String::new(),
        };

        if !config_path.is_empty() {
            let config = session.execute(&format!("cat {} 2>/dev/null", config_path))
                .await
                .ok();

            if let Some(output) = config {
                if output.exit_code == 0 {
                    status.config_content = Some(output.stdout);
                }
            }
        }

        // Get dynamic configs
        let dynamic_path = match status.proxy_type {
            ProxyType::Traefik => format!("{}/dynamic", traefik_path()),
            ProxyType::Caddy => format!("{}/dynamic", caddy_path()),
            ProxyType::None => String::new(),
        };

        if !dynamic_path.is_empty() {
            let files = session.execute(&format!("ls -1 {} 2>/dev/null", dynamic_path))
                .await
                .ok();

            if let Some(output) = files {
                if output.exit_code == 0 {
                    status.dynamic_configs = output.stdout
                        .lines()
                        .map(|s| s.to_string())
                        .collect();
                }
            }
        }

        Ok(status)
    }
}
