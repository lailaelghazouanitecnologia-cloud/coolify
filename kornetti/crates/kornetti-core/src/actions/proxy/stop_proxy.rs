//! Stop Proxy Action
//!
//! Stops the reverse proxy on a server.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::{info, instrument};

use crate::models::Server;
use super::proxy_path;
use crate::actions::{Action, ActionError, ActionResult, CommandBuilder};
use crate::ssh_stub::SshClient;

pub struct StopProxyInput<'a> {
    pub server: &'a Server,
}

pub struct StopProxy {
    ssh: Arc<dyn SshClient>,
}

impl StopProxy {
    pub fn new(ssh: Arc<dyn SshClient>) -> Self {
        Self { ssh }
    }
}

#[async_trait]
impl Action for StopProxy {
    type Input = StopProxyInput<'static>;
    type Output = ActionResult;

    fn name(&self) -> &'static str {
        "stop_proxy"
    }

    #[instrument(skip(self, input), fields(server_id = %input.server.id))]
    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        let server = input.server;
        let start = std::time::Instant::now();

        info!("Stopping proxy on server {}", server.id);

        let session = self.ssh.connect(server)
            .await
            .map_err(|e| ActionError::ssh_connection_failed(e.to_string()))?;

        let mut builder = CommandBuilder::new();

        builder.echo("Stopping proxy...");

        // Check if proxy is running
        let running = session.execute("docker ps -q -f name=coolify-proxy")
            .await
            .map(|r| !r.stdout.trim().is_empty())
            .unwrap_or(false);

        if !running {
            return Ok(ActionResult::success("Proxy is not running")
                .with_duration(start.elapsed()));
        }

        // Stop gracefully
        builder.add("docker stop -t 30 coolify-proxy 2>/dev/null || true");
        builder.add("docker rm -f coolify-proxy 2>/dev/null || true");

        builder.echo("Proxy stopped successfully!");

        let commands = builder.build();

        for cmd in &commands {
            let result = session.execute(cmd)
                .await
                .map_err(|e| ActionError::command_failed(cmd, e.to_string()))?;

            if result.exit_code != 0 && !cmd.contains("|| true") {
                return Err(ActionError::command_failed(cmd, result.stderr));
            }
        }

        Ok(ActionResult::success("Proxy stopped successfully")
            .with_duration(start.elapsed())
            .with_commands(commands))
    }
}
