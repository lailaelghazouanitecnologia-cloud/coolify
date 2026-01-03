//! SSH Session management
//!
//! Provides a managed SSH session with proper lifecycle handling.

use std::sync::Arc;
use std::time::Duration;

use russh::client::{self, Config, Handle};
use russh::ChannelMsg;
use russh_keys::key::PrivateKey;
use tokio::sync::Mutex;
use tracing::{debug, instrument, warn};

use crate::error::{Result, SshError};

/// SSH session wrapper with proper lifecycle management
pub struct SshSession {
    handle: Mutex<Handle<ClientHandler>>,
    host: String,
    port: u16,
    user: String,
}

/// Client handler for SSH events
struct ClientHandler;

impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh_keys::key::PublicKey,
    ) -> std::result::Result<bool, Self::Error> {
        // TODO: Implement proper host key verification
        // For now, accept all keys (similar to StrictHostKeyChecking=no)
        Ok(true)
    }
}

impl SshSession {
    /// Connect to a server
    #[instrument(skip(private_key_pem))]
    pub async fn connect(
        host: &str,
        port: u16,
        user: &str,
        private_key_pem: Option<&str>,
        timeout: Duration,
    ) -> Result<Self> {
        debug!(host = %host, port = %port, user = %user, "Establishing SSH connection");

        let config = Config {
            connection_timeout: Some(timeout),
            ..Default::default()
        };

        let addr = format!("{}:{}", host, port);

        let mut handle = tokio::time::timeout(
            timeout,
            client::connect(Arc::new(config), &addr, ClientHandler),
        )
        .await
        .map_err(|_| SshError::Timeout(timeout.as_secs()))?
        .map_err(|e| SshError::ConnectionFailed(e.to_string()))?;

        // Authenticate
        let authenticated = if let Some(pem) = private_key_pem {
            let key = Self::parse_private_key(pem)?;
            handle
                .authenticate_publickey(user, Arc::new(key))
                .await
                .map_err(|e| SshError::AuthenticationFailed(e.to_string()))?
        } else {
            // Try with agent or empty password as fallback
            warn!("No private key provided, authentication may fail");
            false
        };

        if !authenticated {
            return Err(SshError::AuthenticationFailed("Authentication rejected".into()));
        }

        debug!(host = %host, "SSH connection established");

        Ok(Self {
            handle: Mutex::new(handle),
            host: host.to_string(),
            port,
            user: user.to_string(),
        })
    }

    /// Parse a PEM-encoded private key
    fn parse_private_key(pem: &str) -> Result<PrivateKey> {
        russh_keys::decode_secret_key(pem, None)
            .map_err(|e| SshError::KeyError(e.to_string()))
    }

    /// Execute a command and return the output
    #[instrument(skip(self))]
    pub async fn execute(&self, command: &str) -> Result<CommandOutput> {
        debug!(command = %command, "Executing SSH command");

        let handle = self.handle.lock().await;
        let mut channel = handle
            .channel_open_session()
            .await
            .map_err(|e| SshError::ChannelError(e.to_string()))?;

        channel
            .exec(true, command)
            .await
            .map_err(|e| SshError::CommandFailed(e.to_string()))?;

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut exit_code = 0i32;

        loop {
            match channel.wait().await {
                Some(ChannelMsg::Data { data }) => {
                    stdout.extend_from_slice(&data);
                }
                Some(ChannelMsg::ExtendedData { data, ext }) if ext == 1 => {
                    stderr.extend_from_slice(&data);
                }
                Some(ChannelMsg::ExitStatus { exit_status }) => {
                    exit_code = exit_status as i32;
                }
                Some(ChannelMsg::Eof) | None => break,
                _ => {}
            }
        }

        let output = CommandOutput {
            stdout: String::from_utf8_lossy(&stdout).to_string(),
            stderr: String::from_utf8_lossy(&stderr).to_string(),
            exit_code,
        };

        if exit_code != 0 {
            debug!(
                exit_code = exit_code,
                stderr = %output.stderr,
                "Command exited with non-zero status"
            );
        }

        Ok(output)
    }

    /// Execute a command with sudo
    pub async fn execute_sudo(&self, command: &str) -> Result<CommandOutput> {
        let sudo_command = format!("sudo -n {}", command);
        self.execute(&sudo_command).await
    }

    /// Check if the session is still alive
    pub async fn is_alive(&self) -> bool {
        self.execute("echo 1").await.is_ok()
    }

    /// Get the host address
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Get the port
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Get the user
    pub fn user(&self) -> &str {
        &self.user
    }
}

/// Output from a command execution
#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl CommandOutput {
    /// Check if the command succeeded (exit code 0)
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }

    /// Get combined output (stdout + stderr)
    pub fn combined(&self) -> String {
        if self.stderr.is_empty() {
            self.stdout.clone()
        } else if self.stdout.is_empty() {
            self.stderr.clone()
        } else {
            format!("{}\n{}", self.stdout, self.stderr)
        }
    }

    /// Get stdout lines
    pub fn lines(&self) -> Vec<&str> {
        self.stdout.lines().collect()
    }
}

impl From<CommandOutput> for kornetti_core::traits::CommandOutput {
    fn from(output: CommandOutput) -> Self {
        kornetti_core::traits::CommandOutput {
            stdout: output.stdout,
            stderr: output.stderr,
            exit_code: output.exit_code,
        }
    }
}
