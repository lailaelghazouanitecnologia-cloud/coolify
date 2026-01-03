//! Start Service Action
//!
//! Starts a Docker Compose service.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::{info, instrument};

use crate::models::{Server, Service};
use super::service_configuration_dir;
use crate::actions::{Action, ActionError, ActionResult, CommandBuilder};
use crate::ssh_stub::SshClient;

pub struct StartServiceInput<'a> {
    pub server: &'a Server,
    pub service: &'a Service,
    /// Compose file content (if overriding)
    pub compose_content: Option<String>,
    /// Pull images before starting
    pub pull: bool,
}

pub struct StartService {
    ssh: Arc<dyn SshClient>,
}

impl StartService {
    pub fn new(ssh: Arc<dyn SshClient>) -> Self {
        Self { ssh }
    }
}

#[async_trait]
impl Action for StartService {
    type Input = StartServiceInput<'static>;
    type Output = ActionResult;

    fn name(&self) -> &'static str {
        "start_service"
    }

    #[instrument(skip(self, input), fields(service_id = %input.service.id))]
    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        let server = input.server;
        let service = input.service;
        let start = std::time::Instant::now();

        let uuid = service.uuid.to_string();
        let config_dir = service_configuration_dir(&uuid);

        info!("Starting service {} on server {}", service.id, server.id);

        let session = self.ssh.connect(server)
            .await
            .map_err(|e| ActionError::ssh_connection_failed(e.to_string()))?;

        let mut builder = CommandBuilder::new();

        builder.echo(&format!("Starting service {}...", service.name.as_deref().unwrap_or(&uuid)));
        builder.mkdir(&config_dir);

        // Write compose file if provided
        if let Some(ref content) = input.compose_content {
            let content_base64 = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                content
            );
            builder.add(format!(
                "echo '{}' | base64 -d > {}/docker-compose.yml",
                content_base64, config_dir
            ));
        }

        // Ensure network exists
        builder.add("docker network create --attachable coolify 2>/dev/null || true");

        // Pull images if requested
        if input.pull {
            builder.echo("Pulling images...");
            builder.add(format!(
                "docker compose -f {}/docker-compose.yml pull 2>/dev/null || true",
                config_dir
            ));
        }

        // Start the service
        builder.echo("Starting containers...");
        builder.add(format!(
            "docker compose -f {}/docker-compose.yml up -d --remove-orphans",
            config_dir
        ));

        builder.echo("Service started successfully!");

        let commands = builder.build();

        for cmd in &commands {
            let result = session.execute(cmd)
                .await
                .map_err(|e| ActionError::command_failed(cmd, e.to_string()))?;

            if result.exit_code != 0
                && !cmd.contains("|| true")
                && !cmd.starts_with("echo")
            {
                return Err(ActionError::command_failed(cmd, result.stderr));
            }
        }

        Ok(ActionResult::success("Service started successfully")
            .with_duration(start.elapsed())
            .with_commands(commands))
    }
}
