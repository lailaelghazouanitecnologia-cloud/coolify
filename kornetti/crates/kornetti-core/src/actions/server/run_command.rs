//! Run Command Action
//!
//! Executes arbitrary commands on a server with proper error handling.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument, warn};

use crate::models::Server;
use crate::actions::{Action, ActionError};
use crate::ssh_stub::SshClient;

/// Command execution options
#[derive(Debug, Clone)]
pub struct CommandOptions {
    /// Working directory for the command
    pub working_dir: Option<String>,
    /// Environment variables
    pub env: Vec<(String, String)>,
    /// Run as sudo
    pub sudo: bool,
    /// Command timeout
    pub timeout: Option<Duration>,
    /// Stream output (for long-running commands)
    pub stream: bool,
}

impl Default for CommandOptions {
    fn default() -> Self {
        Self {
            working_dir: None,
            env: Vec::new(),
            sudo: false,
            timeout: Some(Duration::from_secs(300)), // 5 min default
            stream: false,
        }
    }
}

/// Command execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub timed_out: bool,
}

impl CommandResult {
    pub fn success(&self) -> bool {
        self.exit_code == 0 && !self.timed_out
    }
}

/// Input for command execution
pub struct RunCommandInput<'a> {
    pub server: &'a Server,
    pub command: String,
    pub options: CommandOptions,
}

/// Action to run a command on a server
pub struct RunCommand {
    ssh: Arc<dyn SshClient>,
}

impl RunCommand {
    pub fn new(ssh: Arc<dyn SshClient>) -> Self {
        Self { ssh }
    }

    /// Build the full command with options
    fn build_command(command: &str, options: &CommandOptions) -> String {
        let mut parts = Vec::new();

        // Add environment variables
        for (key, value) in &options.env {
            parts.push(format!("{}={}", key, shell_escape(value)));
        }

        // Add working directory change
        if let Some(ref dir) = options.working_dir {
            parts.push(format!("cd {} &&", shell_escape(dir)));
        }

        // Add sudo if needed
        if options.sudo {
            parts.push("sudo".to_string());
        }

        // Add the actual command
        parts.push(command.to_string());

        parts.join(" ")
    }
}

/// Escape a string for shell usage
fn shell_escape(s: &str) -> String {
    if s.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.' || c == '/') {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

#[async_trait]
impl Action for RunCommand {
    type Input = RunCommandInput<'static>;
    type Output = CommandResult;

    fn name(&self) -> &'static str {
        "run_command"
    }

    #[instrument(skip(self, input), fields(server_id = %input.server.id, command = %input.command))]
    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        let server = input.server;
        let start = std::time::Instant::now();

        let full_command = Self::build_command(&input.command, &input.options);

        info!("Executing command on server {}: {}", server.id, full_command);

        let session = self.ssh.connect(server)
            .await
            .map_err(|e| ActionError::ssh_connection_failed(e.to_string()))?;

        let timeout = input.options.timeout.unwrap_or(Duration::from_secs(300));

        // Execute with timeout
        let result = tokio::time::timeout(timeout, session.execute(&full_command))
            .await;

        let (exit_code, stdout, stderr, timed_out) = match result {
            Ok(Ok(output)) => {
                (output.exit_code, output.stdout, output.stderr, false)
            }
            Ok(Err(e)) => {
                return Err(ActionError::command_failed(&input.command, e.to_string()));
            }
            Err(_) => {
                warn!("Command timed out after {:?}", timeout);
                (-1, String::new(), "Command timed out".to_string(), true)
            }
        };

        let duration = start.elapsed();

        let result = CommandResult {
            exit_code,
            stdout,
            stderr,
            duration_ms: duration.as_millis() as u64,
            timed_out,
        };

        if !result.success() {
            warn!(
                "Command exited with code {}: {}",
                result.exit_code,
                result.stderr.lines().next().unwrap_or("")
            );
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_escape() {
        assert_eq!(shell_escape("simple"), "simple");
        assert_eq!(shell_escape("has spaces"), "'has spaces'");
        assert_eq!(shell_escape("has'quote"), "'has'\\''quote'");
    }

    #[test]
    fn test_build_command() {
        let options = CommandOptions {
            working_dir: Some("/app".to_string()),
            env: vec![("FOO".to_string(), "bar".to_string())],
            sudo: true,
            ..Default::default()
        };

        let cmd = RunCommand::build_command("ls -la", &options);
        assert!(cmd.contains("FOO=bar"));
        assert!(cmd.contains("cd /app"));
        assert!(cmd.contains("sudo"));
        assert!(cmd.contains("ls -la"));
    }
}
