//! Kornetti SSH
//!
//! A robust SSH client library with connection multiplexing, automatic retry,
//! and command batching for efficient remote server management.
//!
//! # Features
//!
//! - **Connection Pooling**: Reuse SSH connections across multiple operations
//! - **Automatic Retry**: Exponential backoff for transient failures
//! - **Command Batching**: Combine commands to reduce round-trips
//! - **SFTP Operations**: File upload/download via SSH
//!
//! # Example
//!
//! ```ignore
//! use kornetti_ssh::{SshClient, Command, CommandBatch};
//! use kornetti_core::models::Server;
//!
//! let client = SshClient::new();
//!
//! // Register private key
//! client.register_key(server.private_key_id.unwrap(), private_key);
//!
//! // Execute a single command
//! let result = client.execute(&server, "docker ps").await?;
//! println!("Output: {}", result.output.stdout);
//!
//! // Execute a batch of commands
//! let batch = CommandBatch::new()
//!     .cmd("cd /app")
//!     .cmd("docker compose pull")
//!     .cmd("docker compose up -d");
//! let result = client.execute_batch(&server, &batch).await?;
//!
//! // Upload a file
//! client.upload_string(&server, "/etc/nginx/conf.d/app.conf", &nginx_config).await?;
//! ```

pub mod client;
pub mod session;
pub mod multiplexer;
pub mod command;
pub mod retry;
pub mod sftp;
pub mod error;

// Re-export main types
pub use client::{SshClient, SystemInfo, DistroInfo, DockerInfo};
pub use session::{SshSession, CommandOutput};
pub use multiplexer::{ConnectionPool, PoolConfig, PoolStats, ServerKey};
pub use command::{Command, CommandBatch, CommandExecutor, CommandResult, BatchSeparator};
pub use retry::{RetryPolicy, with_retry, Retryable};
pub use sftp::SftpClient;
pub use error::{SshError, Result};

use async_trait::async_trait;
use kornetti_core::{
    Result as CoreResult,
    models::Server,
    traits::{RemoteExecutor, CommandOutput as CoreCommandOutput},
};

/// Default SSH executor using the connection-pooled client
pub struct DefaultSshExecutor {
    client: SshClient,
}

impl DefaultSshExecutor {
    pub fn new() -> Self {
        Self {
            client: SshClient::new(),
        }
    }

    pub fn with_client(client: SshClient) -> Self {
        Self { client }
    }

    /// Get a reference to the underlying client
    pub fn client(&self) -> &SshClient {
        &self.client
    }
}

impl Default for DefaultSshExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RemoteExecutor for DefaultSshExecutor {
    async fn execute(&self, server: &Server, command: &str) -> CoreResult<CoreCommandOutput> {
        self.client.execute(server, command).await
    }

    async fn execute_many(&self, server: &Server, commands: &[&str]) -> CoreResult<Vec<CoreCommandOutput>> {
        self.client.execute_many(server, commands).await
    }

    async fn upload_file(
        &self,
        server: &Server,
        local_path: &str,
        remote_path: &str,
    ) -> CoreResult<()> {
        self.client.upload_file(server, local_path, remote_path).await
    }

    async fn download_file(
        &self,
        server: &Server,
        remote_path: &str,
        local_path: &str,
    ) -> CoreResult<()> {
        self.client.download_file(server, remote_path, local_path).await
    }

    async fn check_connection(&self, server: &Server) -> CoreResult<bool> {
        self.client.check_connection(server).await
    }
}
