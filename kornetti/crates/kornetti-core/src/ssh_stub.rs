//! SSH Client Stub
//!
//! This module provides a minimal SSH client interface that can be implemented
//! by the kornetti-ssh crate. This breaks the circular dependency by defining
//! the interface in kornetti-core.

use async_trait::async_trait;
use std::sync::Arc;

/// SSH connection result
pub type SshResult<T> = Result<T, SshError>;

/// SSH error type
#[derive(Debug, Clone)]
pub struct SshError {
    pub message: String,
    pub code: String,
    pub retryable: bool,
}

impl SshError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: "SSH_ERROR".to_string(),
            retryable: false,
        }
    }

    pub fn connection_failed(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: "SSH_CONNECTION_FAILED".to_string(),
            retryable: true,
        }
    }

    pub fn command_failed(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: "SSH_COMMAND_FAILED".to_string(),
            retryable: false,
        }
    }
}

impl std::fmt::Display for SshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for SshError {}

/// Result of executing a command over SSH
#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl CommandOutput {
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }

    pub fn output(&self) -> &str {
        if self.stdout.is_empty() {
            &self.stderr
        } else {
            &self.stdout
        }
    }
}

/// SSH client trait that can be implemented by kornetti-ssh
#[async_trait]
pub trait SshClient: Send + Sync {
    /// Execute a command on the remote server
    async fn execute(&self, command: &str) -> SshResult<CommandOutput>;

    /// Execute multiple commands sequentially
    async fn execute_batch(&self, commands: &[String]) -> SshResult<Vec<CommandOutput>>;

    /// Check if the connection is alive
    async fn is_connected(&self) -> bool;

    /// Close the connection
    async fn disconnect(&self) -> SshResult<()>;
}

/// SSH connection configuration
#[derive(Debug, Clone)]
pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub private_key: String,
    pub connection_timeout_seconds: u64,
    pub command_timeout_seconds: u64,
}

impl SshConfig {
    pub fn new(host: String, user: String, private_key: String) -> Self {
        Self {
            host,
            port: 22,
            user,
            private_key,
            connection_timeout_seconds: 30,
            command_timeout_seconds: 120,
        }
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }
}

/// A no-op SSH client for testing or when SSH is not available
pub struct NoOpSshClient;

#[async_trait]
impl SshClient for NoOpSshClient {
    async fn execute(&self, _command: &str) -> SshResult<CommandOutput> {
        Ok(CommandOutput {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 0,
        })
    }

    async fn execute_batch(&self, commands: &[String]) -> SshResult<Vec<CommandOutput>> {
        Ok(commands.iter().map(|_| CommandOutput {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 0,
        }).collect())
    }

    async fn is_connected(&self) -> bool {
        false
    }

    async fn disconnect(&self) -> SshResult<()> {
        Ok(())
    }
}

/// Type alias for a shared SSH client
pub type SharedSshClient = Arc<dyn SshClient>;
