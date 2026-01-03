//! Stop Service Action
//!
//! Stops a Docker Compose service.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::{info, instrument};

use crate::models::{Server, Service};
use super::service_configuration_dir;
use crate::actions::{Action, ActionError, ActionResult, CommandBuilder};
use crate::ssh_stub::SshClient;

pub struct StopServiceInput<'a> {
    pub server: &'a Server,
    pub service: &'a Service,
    /// Remove containers after stopping
    pub remove: bool,
}

pub struct StopService {
    ssh: Arc<dyn SshClient>,
}

impl StopService {
    pub fn new(ssh: Arc<dyn SshClient>) -> Self {
        Self { ssh }
    }
}

#[async_trait]
impl Action for StopService {
    type Input = StopServiceInput<'static>;
    type Output = ActionResult;

    fn name(&self) -> &'static str {
        "stop_service"
    }

    #[instrument(skip(self, input), fields(service_id = %input.service.id))]
    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        let server = input.server;
        let service = input.service;
        let start = std::time::Instant::now();

        let uuid = service.uuid.to_string();
        let config_dir = service_configuration_dir(&uuid);

        info!("Stopping service {} on server {}", service.id, server.id);

        let session = self.ssh.connect(server)
            .await
            .map_err(|e| ActionError::ssh_connection_failed(e.to_string()))?;

        let mut builder = CommandBuilder::new();

        builder.echo(&format!("Stopping service {}...", service.name.as_deref().unwrap_or(&uuid)));

        // Check if compose file exists
        let compose_exists = session.execute(&format!(
            "test -f {}/docker-compose.yml && echo 'yes'",
            config_dir
        ))
            .await
            .map(|r| r.stdout.trim() == "yes")
            .unwrap_or(false);

        if compose_exists {
            if input.remove {
                builder.add(format!(
                    "docker compose -f {}/docker-compose.yml down --remove-orphans",
                    config_dir
                ));
            } else {
                builder.add(format!(
                    "docker compose -f {}/docker-compose.yml stop",
                    config_dir
                ));
            }
        } else {
            // Fallback: stop containers by label
            builder.add(format!(
                "docker ps -aq --filter 'label=coolify.serviceId={}' | xargs -r docker stop 2>/dev/null || true",
                uuid
            ));

            if input.remove {
                builder.add(format!(
                    "docker ps -aq --filter 'label=coolify.serviceId={}' | xargs -r docker rm -f 2>/dev/null || true",
                    uuid
                ));
            }
        }

        builder.echo("Service stopped successfully!");

        let commands = builder.build();

        for cmd in &commands {
            let result = session.execute(cmd)
                .await
                .map_err(|e| ActionError::command_failed(cmd, e.to_string()))?;

            if result.exit_code != 0 && !cmd.contains("|| true") && !cmd.starts_with("echo") {
                return Err(ActionError::command_failed(cmd, result.stderr));
            }
        }

        Ok(ActionResult::success("Service stopped successfully")
            .with_duration(start.elapsed())
            .with_commands(commands))
    }
}
