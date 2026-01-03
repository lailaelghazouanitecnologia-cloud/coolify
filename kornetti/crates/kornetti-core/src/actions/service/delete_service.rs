//! Delete Service Action
//!
//! Completely removes a Docker Compose service including volumes and configuration.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::{info, instrument, warn};

use crate::models::{Server, Service};
use super::service_configuration_dir;
use crate::actions::{Action, ActionError, ActionResult, CommandBuilder};
use kornetti_ssh::SshClient;

pub struct DeleteServiceInput<'a> {
    pub server: &'a Server,
    pub service: &'a Service,
    /// Remove associated volumes
    pub remove_volumes: bool,
    /// Remove configuration files
    pub remove_config: bool,
}

pub struct DeleteService {
    ssh: Arc<SshClient>,
}

impl DeleteService {
    pub fn new(ssh: Arc<SshClient>) -> Self {
        Self { ssh }
    }
}

#[async_trait]
impl Action for DeleteService {
    type Input = DeleteServiceInput<'static>;
    type Output = ActionResult;

    fn name(&self) -> &'static str {
        "delete_service"
    }

    #[instrument(skip(self, input), fields(service_id = %input.service.id))]
    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        let server = input.server;
        let service = input.service;
        let start = std::time::Instant::now();

        let uuid = service.uuid.to_string();
        let config_dir = service_configuration_dir(&uuid);

        info!("Deleting service {} on server {}", service.id, server.id);

        let session = self.ssh.connect(server)
            .await
            .map_err(|e| ActionError::ssh_connection_failed(e.to_string()))?;

        let mut builder = CommandBuilder::new();

        builder.echo(&format!("Deleting service {}...", service.name.as_deref().unwrap_or(&uuid)));

        // Check if compose file exists
        let compose_exists = session.execute(&format!(
            "test -f {}/docker-compose.yml && echo 'yes'",
            config_dir
        ))
            .await
            .map(|r| r.stdout.trim() == "yes")
            .unwrap_or(false);

        if compose_exists {
            let down_cmd = if input.remove_volumes {
                format!(
                    "docker compose -f {}/docker-compose.yml down --volumes --remove-orphans",
                    config_dir
                )
            } else {
                format!(
                    "docker compose -f {}/docker-compose.yml down --remove-orphans",
                    config_dir
                )
            };
            builder.add(down_cmd);
        } else {
            // Fallback: remove containers by label
            builder.add(format!(
                "docker ps -aq --filter 'label=coolify.serviceId={}' | xargs -r docker rm -f 2>/dev/null || true",
                uuid
            ));
        }

        // Remove proxy configuration
        builder.add(format!(
            "rm -f /data/coolify/proxy/dynamic/{}.yml 2>/dev/null || true",
            uuid
        ));
        builder.add(format!(
            "rm -f /data/coolify/proxy/dynamic/{}.caddy 2>/dev/null || true",
            uuid
        ));

        // Remove configuration directory if requested
        if input.remove_config {
            builder.add(format!("rm -rf {} 2>/dev/null || true", config_dir));
        }

        // Clean up any orphaned volumes with this service's label
        if input.remove_volumes {
            builder.add(format!(
                "docker volume ls -q --filter 'label=coolify.serviceId={}' | xargs -r docker volume rm 2>/dev/null || true",
                uuid
            ));
        }

        builder.echo("Service deleted successfully!");

        let commands = builder.build();

        for cmd in &commands {
            let result = session.execute(cmd)
                .await
                .map_err(|e| ActionError::command_failed(cmd, e.to_string()))?;

            if result.exit_code != 0 && !cmd.contains("|| true") && !cmd.starts_with("echo") {
                warn!("Delete command had non-zero exit: {} - {}", cmd, result.stderr);
            }
        }

        Ok(ActionResult::success("Service deleted successfully")
            .with_duration(start.elapsed())
            .with_commands(commands))
    }
}
