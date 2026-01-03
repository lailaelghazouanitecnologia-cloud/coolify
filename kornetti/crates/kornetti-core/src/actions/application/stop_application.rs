//! Stop Application Action
//!
//! Stops all containers associated with an application.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::{info, instrument, warn};

use crate::models::{Server, Application};
use crate::actions::{Action, ActionError, ActionResult, CommandBuilder};
use kornetti_ssh::SshClient;

/// Options for stopping an application
#[derive(Debug, Clone)]
pub struct StopOptions {
    /// Graceful shutdown timeout
    pub timeout: u32,
    /// Remove containers after stopping
    pub remove: bool,
    /// Remove associated volumes
    pub remove_volumes: bool,
    /// Remove from all servers (for multi-server deployments)
    pub all_servers: bool,
}

impl Default for StopOptions {
    fn default() -> Self {
        Self {
            timeout: 30,
            remove: true,
            remove_volumes: false,
            all_servers: false,
        }
    }
}

pub struct StopApplicationInput<'a> {
    pub server: &'a Server,
    pub application: &'a Application,
    pub options: StopOptions,
}

pub struct StopApplication {
    ssh: Arc<SshClient>,
}

impl StopApplication {
    pub fn new(ssh: Arc<SshClient>) -> Self {
        Self { ssh }
    }
}

#[async_trait]
impl Action for StopApplication {
    type Input = StopApplicationInput<'static>;
    type Output = ActionResult;

    fn name(&self) -> &'static str {
        "stop_application"
    }

    #[instrument(skip(self, input), fields(application_id = %input.application.id))]
    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        let server = input.server;
        let app = input.application;
        let options = &input.options;
        let start = std::time::Instant::now();

        let container_name = app.uuid.to_string();

        info!("Stopping application {} on server {}", app.id, server.id);

        let session = self.ssh.connect(server)
            .await
            .map_err(|e| ActionError::ssh_connection_failed(e.to_string()))?;

        let mut builder = CommandBuilder::new();

        builder.echo(&format!("Stopping application {}...", app.name.as_deref().unwrap_or(&container_name)));

        // Find all containers with this application label
        let find_cmd = format!(
            "docker ps -aq --filter 'label=coolify.applicationId={}' 2>/dev/null",
            app.uuid
        );

        let containers = session.execute(&find_cmd)
            .await
            .map_err(|e| ActionError::command_failed(&find_cmd, e.to_string()))?;

        let container_ids: Vec<&str> = containers.stdout
            .lines()
            .filter(|s| !s.is_empty())
            .collect();

        if container_ids.is_empty() {
            // Try finding by container name
            let name_check = session.execute(&format!(
                "docker ps -aq --filter 'name={}' 2>/dev/null",
                container_name
            ))
                .await
                .ok();

            if name_check.map(|r| r.stdout.trim().is_empty()).unwrap_or(true) {
                return Ok(ActionResult::success("No containers found for this application")
                    .with_duration(start.elapsed()));
            }

            builder.add(format!(
                "docker stop -t {} {} 2>/dev/null || true",
                options.timeout, container_name
            ));

            if options.remove {
                builder.add(format!(
                    "docker rm -f {} 2>/dev/null || true",
                    container_name
                ));
            }
        } else {
            // Stop all found containers
            for container_id in &container_ids {
                builder.add(format!(
                    "docker stop -t {} {} 2>/dev/null || true",
                    options.timeout, container_id
                ));

                if options.remove {
                    let rm_cmd = if options.remove_volumes {
                        format!("docker rm -fv {} 2>/dev/null || true", container_id)
                    } else {
                        format!("docker rm -f {} 2>/dev/null || true", container_id)
                    };
                    builder.add(rm_cmd);
                }
            }
        }

        // Also stop any preview/staging containers
        let preview_cmd = format!(
            "docker ps -aq --filter 'label=coolify.applicationId={}' --filter 'label=coolify.preview=true' 2>/dev/null | xargs -r docker stop -t {} 2>/dev/null || true",
            app.uuid, options.timeout
        );
        builder.add(preview_cmd);

        // Clean up networks if this was the last container
        builder.add(format!(
            "docker network disconnect coolify {} 2>/dev/null || true",
            container_name
        ));

        builder.echo("Application stopped successfully!");

        let commands = builder.build();

        for cmd in &commands {
            let result = session.execute(cmd)
                .await
                .map_err(|e| ActionError::command_failed(cmd, e.to_string()))?;

            if result.exit_code != 0 && !cmd.contains("|| true") && !cmd.contains("2>/dev/null") {
                warn!("Stop command had non-zero exit: {} - {}", cmd, result.stderr);
            }
        }

        // Remove proxy configuration if removing the container
        if options.remove {
            let proxy_config = format!("/data/coolify/proxy/dynamic/{}.yml", app.uuid);
            let _ = session.execute(&format!("rm -f {} 2>/dev/null || true", proxy_config)).await;
        }

        Ok(ActionResult::success(format!(
            "Stopped {} container(s) for application {}",
            container_ids.len().max(1),
            app.id
        ))
            .with_duration(start.elapsed())
            .with_commands(commands))
    }
}
