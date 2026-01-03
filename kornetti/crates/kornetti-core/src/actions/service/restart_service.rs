//! Restart Service Action
//!
//! Restarts a Docker Compose service.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::{info, instrument};

use crate::models::{Server, Service};
use super::service_configuration_dir;
use crate::actions::{Action, ActionError, ActionResult, CommandBuilder};
use kornetti_ssh::SshClient;

pub struct RestartServiceInput<'a> {
    pub server: &'a Server,
    pub service: &'a Service,
}

pub struct RestartService {
    ssh: Arc<SshClient>,
}

impl RestartService {
    pub fn new(ssh: Arc<SshClient>) -> Self {
        Self { ssh }
    }
}

#[async_trait]
impl Action for RestartService {
    type Input = RestartServiceInput<'static>;
    type Output = ActionResult;

    fn name(&self) -> &'static str {
        "restart_service"
    }

    #[instrument(skip(self, input), fields(service_id = %input.service.id))]
    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        let server = input.server;
        let service = input.service;
        let start = std::time::Instant::now();

        let uuid = service.uuid.to_string();
        let config_dir = service_configuration_dir(&uuid);

        info!("Restarting service {} on server {}", service.id, server.id);

        let session = self.ssh.connect(server)
            .await
            .map_err(|e| ActionError::ssh_connection_failed(e.to_string()))?;

        let mut builder = CommandBuilder::new();

        builder.echo(&format!("Restarting service {}...", service.name.as_deref().unwrap_or(&uuid)));

        // Check if compose file exists
        let compose_exists = session.execute(&format!(
            "test -f {}/docker-compose.yml && echo 'yes'",
            config_dir
        ))
            .await
            .map(|r| r.stdout.trim() == "yes")
            .unwrap_or(false);

        if compose_exists {
            builder.add(format!(
                "docker compose -f {}/docker-compose.yml restart",
                config_dir
            ));
        } else {
            // Fallback: restart containers by label
            builder.add(format!(
                "docker ps -aq --filter 'label=coolify.serviceId={}' | xargs -r docker restart 2>/dev/null || true",
                uuid
            ));
        }

        builder.echo("Service restarted successfully!");

        let commands = builder.build();

        for cmd in &commands {
            let result = session.execute(cmd)
                .await
                .map_err(|e| ActionError::command_failed(cmd, e.to_string()))?;

            if result.exit_code != 0 && !cmd.contains("|| true") && !cmd.starts_with("echo") {
                return Err(ActionError::command_failed(cmd, result.stderr));
            }
        }

        Ok(ActionResult::success("Service restarted successfully")
            .with_duration(start.elapsed())
            .with_commands(commands))
    }
}
