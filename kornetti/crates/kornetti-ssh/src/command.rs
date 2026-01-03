//! Command execution and batching
//!
//! Provides command batching to reduce SSH round-trips by combining
//! multiple commands into single executions.

use std::fmt;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use tracing::{debug, instrument};
use uuid::Uuid;

use crate::error::{Result, SshError};
use crate::session::{SshSession, CommandOutput};
use crate::retry::{RetryPolicy, with_retry, Retryable};

/// A command to execute on a remote server
#[derive(Debug, Clone)]
pub struct Command {
    /// Unique identifier for this command
    pub id: Uuid,
    /// The command to execute
    pub command: String,
    /// Whether to run with sudo
    pub sudo: bool,
    /// Working directory (cd before running)
    pub working_dir: Option<String>,
    /// Environment variables to set
    pub env: Vec<(String, String)>,
    /// Timeout in seconds (0 = no timeout)
    pub timeout_secs: u64,
    /// Whether this command can be retried on failure
    pub retryable: bool,
}

impl Command {
    /// Create a new command
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            command: command.into(),
            sudo: false,
            working_dir: None,
            env: Vec::new(),
            timeout_secs: 0,
            retryable: false,
        }
    }

    /// Set sudo flag
    pub fn sudo(mut self) -> Self {
        self.sudo = true;
        self
    }

    /// Set working directory
    pub fn working_dir(mut self, dir: impl Into<String>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Add environment variable
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// Set timeout
    pub fn timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Mark as retryable
    pub fn retryable(mut self) -> Self {
        self.retryable = true;
        self
    }

    /// Build the full command string with environment and cd
    pub fn build(&self) -> String {
        let mut parts = Vec::new();

        // Environment variables
        for (key, value) in &self.env {
            parts.push(format!("export {}={}", key, shell_escape(value)));
        }

        // Working directory
        if let Some(dir) = &self.working_dir {
            parts.push(format!("cd {}", shell_escape(dir)));
        }

        // Main command
        let cmd = if self.sudo {
            format!("sudo -n {}", &self.command)
        } else {
            self.command.clone()
        };
        parts.push(cmd);

        parts.join(" && ")
    }
}

/// Escape a string for shell usage
fn shell_escape(s: &str) -> String {
    if s.contains('\'') {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        format!("'{}'", s)
    }
}

/// Result of executing a command
#[derive(Debug, Clone)]
pub struct CommandResult {
    /// The command that was executed
    pub command_id: Uuid,
    /// The output
    pub output: CommandOutput,
    /// When execution started
    pub started_at: DateTime<Utc>,
    /// When execution finished
    pub finished_at: DateTime<Utc>,
    /// Number of retry attempts
    pub attempts: u32,
}

impl CommandResult {
    /// Check if the command succeeded
    pub fn success(&self) -> bool {
        self.output.success()
    }

    /// Get execution duration
    pub fn duration(&self) -> chrono::Duration {
        self.finished_at - self.started_at
    }
}

/// A batch of commands to execute together
#[derive(Debug, Clone)]
pub struct CommandBatch {
    /// Batch identifier
    pub id: Uuid,
    /// Commands in this batch
    commands: Vec<Command>,
    /// Whether to stop on first error
    stop_on_error: bool,
    /// Separator between commands
    separator: BatchSeparator,
}

/// How to separate commands in a batch
#[derive(Debug, Clone, Copy)]
pub enum BatchSeparator {
    /// Use && (stop on error)
    And,
    /// Use ; (continue on error)
    Semicolon,
    /// Use || (run next only on error)
    Or,
}

impl fmt::Display for BatchSeparator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BatchSeparator::And => write!(f, " && "),
            BatchSeparator::Semicolon => write!(f, "; "),
            BatchSeparator::Or => write!(f, " || "),
        }
    }
}

impl CommandBatch {
    /// Create a new command batch
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            commands: Vec::new(),
            stop_on_error: true,
            separator: BatchSeparator::And,
        }
    }

    /// Add a command to the batch
    pub fn add(mut self, command: Command) -> Self {
        self.commands.push(command);
        self
    }

    /// Add a simple command string
    pub fn cmd(self, command: impl Into<String>) -> Self {
        self.add(Command::new(command))
    }

    /// Set whether to stop on first error
    pub fn stop_on_error(mut self, stop: bool) -> Self {
        self.stop_on_error = stop;
        if !stop {
            self.separator = BatchSeparator::Semicolon;
        }
        self
    }

    /// Set the separator between commands
    pub fn separator(mut self, sep: BatchSeparator) -> Self {
        self.separator = sep;
        self
    }

    /// Build the combined command string
    pub fn build(&self) -> String {
        self.commands
            .iter()
            .map(|c| c.build())
            .collect::<Vec<_>>()
            .join(&self.separator.to_string())
    }

    /// Get the number of commands
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Check if the batch is empty
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

impl Default for CommandBatch {
    fn default() -> Self {
        Self::new()
    }
}

/// Command executor with retry support
pub struct CommandExecutor {
    session: Arc<SshSession>,
    retry_policy: RetryPolicy,
}

impl CommandExecutor {
    /// Create a new executor
    pub fn new(session: Arc<SshSession>) -> Self {
        Self {
            session,
            retry_policy: RetryPolicy::command(),
        }
    }

    /// Set the retry policy
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = policy;
        self
    }

    /// Execute a single command
    #[instrument(skip(self))]
    pub async fn execute(&self, command: &Command) -> Result<CommandResult> {
        let started_at = Utc::now();
        let cmd_str = command.build();

        debug!(command_id = %command.id, cmd = %cmd_str, "Executing command");

        let (output, attempts) = if command.retryable {
            let mut attempts = 0;
            let output = with_retry(&self.retry_policy, "ssh_command", || {
                attempts += 1;
                let session = self.session.clone();
                let cmd = cmd_str.clone();
                async move { session.execute(&cmd).await }
            })
            .await?;
            (output, attempts)
        } else {
            let output = self.session.execute(&cmd_str).await?;
            (output, 1)
        };

        let finished_at = Utc::now();

        Ok(CommandResult {
            command_id: command.id,
            output,
            started_at,
            finished_at,
            attempts,
        })
    }

    /// Execute a batch of commands
    #[instrument(skip(self))]
    pub async fn execute_batch(&self, batch: &CommandBatch) -> Result<CommandResult> {
        let started_at = Utc::now();
        let cmd_str = batch.build();

        debug!(
            batch_id = %batch.id,
            num_commands = batch.len(),
            "Executing command batch"
        );

        let output = self.session.execute(&cmd_str).await?;
        let finished_at = Utc::now();

        Ok(CommandResult {
            command_id: batch.id,
            output,
            started_at,
            finished_at,
            attempts: 1,
        })
    }

    /// Execute multiple commands in parallel
    pub async fn execute_parallel(&self, commands: Vec<Command>) -> Vec<Result<CommandResult>> {
        let mut handles = Vec::with_capacity(commands.len());

        for cmd in commands {
            let session = self.session.clone();
            let policy = self.retry_policy.clone();

            handles.push(tokio::spawn(async move {
                let executor = CommandExecutor::new(session).with_retry_policy(policy);
                executor.execute(&cmd).await
            }));
        }

        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(Err(SshError::CommandFailed(e.to_string()))),
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_build() {
        let cmd = Command::new("ls -la")
            .working_dir("/tmp")
            .env("FOO", "bar")
            .sudo();

        let built = cmd.build();
        assert!(built.contains("export FOO='bar'"));
        assert!(built.contains("cd '/tmp'"));
        assert!(built.contains("sudo -n ls -la"));
    }

    #[test]
    fn test_command_batch_build() {
        let batch = CommandBatch::new()
            .cmd("echo hello")
            .cmd("echo world");

        assert_eq!(batch.build(), "echo hello && echo world");
    }

    #[test]
    fn test_batch_with_semicolon() {
        let batch = CommandBatch::new()
            .cmd("cmd1")
            .cmd("cmd2")
            .separator(BatchSeparator::Semicolon);

        assert_eq!(batch.build(), "cmd1; cmd2");
    }

    #[test]
    fn test_shell_escape() {
        assert_eq!(shell_escape("simple"), "'simple'");
        assert_eq!(shell_escape("with space"), "'with space'");
        assert_eq!(shell_escape("it's"), "\"it's\"");
    }
}
