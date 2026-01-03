//! Stop Database Action
//!
//! Stops a running database container gracefully.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::{info, instrument, warn};

use crate::models::{Server, StandaloneDatabase};
use super::database_configuration_dir;
use crate::actions::{Action, ActionError, ActionResult, CommandBuilder};
use crate::ssh_stub::SshClient;

/// Options for stopping a database
#[derive(Debug, Clone)]
pub struct StopOptions {
    /// Graceful shutdown timeout in seconds
    pub timeout: u32,
    /// Also remove the container after stopping
    pub remove: bool,
    /// Force kill if graceful stop fails
    pub force: bool,
}

impl Default for StopOptions {
    fn default() -> Self {
        Self {
            timeout: 30,
            remove: false,
            force: false,
        }
    }
}

pub struct StopDatabaseInput<'a> {
    pub server: &'a Server,
    pub database: &'a StandaloneDatabase,
    pub options: StopOptions,
}

pub struct StopDatabase {
    ssh: Arc<dyn SshClient>,
}

impl StopDatabase {
    pub fn new(ssh: Arc<dyn SshClient>) -> Self {
        Self { ssh }
    }
}

#[async_trait]
impl Action for StopDatabase {
    type Input = StopDatabaseInput<'static>;
    type Output = ActionResult;

    fn name(&self) -> &'static str {
        "stop_database"
    }

    #[instrument(skip(self, input), fields(database_id = %input.database.id))]
    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        let server = input.server;
        let database = input.database;
        let options = &input.options;
        let start = std::time::Instant::now();

        let container_name = database.uuid.to_string();

        info!("Stopping database container {}", container_name);

        let session = self.ssh.connect(server)
            .await
            .map_err(|e| ActionError::ssh_connection_failed(e.to_string()))?;

        let mut builder = CommandBuilder::new();

        builder.echo(&format!("Stopping database {}...", container_name));

        // Check if container exists
        let check_cmd = format!(
            "docker inspect {} >/dev/null 2>&1 && echo 'exists'",
            container_name
        );

        let exists = session.execute(&check_cmd)
            .await
            .map(|r| r.stdout.trim() == "exists")
            .unwrap_or(false);

        if !exists {
            return Ok(ActionResult::success("Container not found, nothing to stop")
                .with_duration(start.elapsed()));
        }

        // Stop container with timeout
        builder.add(format!(
            "docker stop -t {} {}",
            options.timeout, container_name
        ));

        // Force kill if needed
        if options.force {
            builder.add(format!(
                "docker kill {} 2>/dev/null || true",
                container_name
            ));
        }

        // Remove container if requested
        if options.remove {
            builder.add(format!(
                "docker rm -f {} 2>/dev/null || true",
                container_name
            ));
        }

        builder.echo("Database stopped successfully!");

        let commands = builder.build();

        for cmd in &commands {
            let result = session.execute(cmd)
                .await
                .map_err(|e| ActionError::command_failed(cmd, e.to_string()))?;

            if result.exit_code != 0 && !cmd.contains("|| true") {
                warn!("Stop command failed: {}", result.stderr);
            }
        }

        Ok(ActionResult::success("Database stopped successfully")
            .with_duration(start.elapsed())
            .with_commands(commands))
    }
}
