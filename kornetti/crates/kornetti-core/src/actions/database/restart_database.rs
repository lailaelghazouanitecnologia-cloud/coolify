//! Restart Database Action
//!
//! Restarts a database container.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::{info, instrument};

use crate::models::{Server, StandaloneDatabase};
use super::database_configuration_dir;
use crate::actions::{Action, ActionError, ActionResult, CommandBuilder};
use kornetti_ssh::SshClient;

pub struct RestartDatabaseInput<'a> {
    pub server: &'a Server,
    pub database: &'a StandaloneDatabase,
    /// Timeout for stop operation
    pub timeout: u32,
}

pub struct RestartDatabase {
    ssh: Arc<SshClient>,
}

impl RestartDatabase {
    pub fn new(ssh: Arc<SshClient>) -> Self {
        Self { ssh }
    }
}

#[async_trait]
impl Action for RestartDatabase {
    type Input = RestartDatabaseInput<'static>;
    type Output = ActionResult;

    fn name(&self) -> &'static str {
        "restart_database"
    }

    #[instrument(skip(self, input), fields(database_id = %input.database.id))]
    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        let server = input.server;
        let database = input.database;
        let start = std::time::Instant::now();

        let container_name = database.uuid.to_string();
        let config_dir = database_configuration_dir(&container_name);

        info!("Restarting database container {}", container_name);

        let session = self.ssh.connect(server)
            .await
            .map_err(|e| ActionError::ssh_connection_failed(e.to_string()))?;

        let mut builder = CommandBuilder::new();

        builder.echo(&format!("Restarting database {}...", container_name));

        // Check if docker-compose file exists
        let compose_exists = session.execute(&format!(
            "test -f {}/docker-compose.yml && echo 'yes'",
            config_dir
        ))
            .await
            .map(|r| r.stdout.trim() == "yes")
            .unwrap_or(false);

        if compose_exists {
            // Use docker-compose restart
            builder.add(format!(
                "docker compose -f {}/docker-compose.yml restart",
                config_dir
            ));
        } else {
            // Use docker restart
            builder.add(format!(
                "docker restart -t {} {}",
                input.timeout, container_name
            ));
        }

        builder.echo("Database restarted successfully!");

        let commands = builder.build();

        for cmd in &commands {
            let result = session.execute(cmd)
                .await
                .map_err(|e| ActionError::command_failed(cmd, e.to_string()))?;

            if result.exit_code != 0 && !cmd.starts_with("echo") {
                return Err(ActionError::command_failed(cmd, result.stderr));
            }
        }

        // Verify container is running
        let status = session.execute(&format!(
            "docker inspect -f '{{{{.State.Running}}}}' {}",
            container_name
        ))
            .await
            .map_err(|e| ActionError::command_failed("check status", e.to_string()))?;

        if status.stdout.trim() != "true" {
            return Err(ActionError::new(
                "RESTART_FAILED",
                "Container is not running after restart"
            ));
        }

        Ok(ActionResult::success("Database restarted successfully")
            .with_duration(start.elapsed())
            .with_commands(commands))
    }
}
