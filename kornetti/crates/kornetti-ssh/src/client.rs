//! High-level SSH client
//!
//! Provides a unified interface for SSH operations with automatic
//! connection pooling, retry logic, and command batching.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tracing::{debug, info, instrument};
use uuid::Uuid;

use kornetti_core::{
    Result as CoreResult,
    models::Server,
    traits::{RemoteExecutor, CommandOutput as CoreCommandOutput},
};

use crate::error::{Result, SshError};
use crate::multiplexer::{ConnectionPool, PoolConfig};
use crate::session::SshSession;
use crate::command::{Command, CommandBatch, CommandExecutor, CommandResult};
use crate::sftp::SftpClient;
use crate::retry::RetryPolicy;

/// High-level SSH client with connection pooling
pub struct SshClient {
    pool: Arc<ConnectionPool>,
}

impl SshClient {
    /// Create a new SSH client with default configuration
    pub fn new() -> Self {
        Self::with_config(PoolConfig::default())
    }

    /// Create a new SSH client with custom configuration
    pub fn with_config(config: PoolConfig) -> Self {
        Self {
            pool: Arc::new(ConnectionPool::new(config)),
        }
    }

    /// Register a private key for authentication
    pub fn register_key(&self, key_id: Uuid, private_key: String) {
        self.pool.register_key(key_id, private_key);
    }

    /// Execute a command on a server
    #[instrument(skip(self, server))]
    pub async fn execute(&self, server: &Server, command: &str) -> Result<CommandResult> {
        let conn = self.pool.get_connection(server).await?;
        let executor = CommandExecutor::new(conn.session());
        executor.execute(&Command::new(command)).await
    }

    /// Execute a command with sudo
    pub async fn execute_sudo(&self, server: &Server, command: &str) -> Result<CommandResult> {
        let conn = self.pool.get_connection(server).await?;
        let executor = CommandExecutor::new(conn.session());
        executor.execute(&Command::new(command).sudo()).await
    }

    /// Execute a command batch
    pub async fn execute_batch(&self, server: &Server, batch: &CommandBatch) -> Result<CommandResult> {
        let conn = self.pool.get_connection(server).await?;
        let executor = CommandExecutor::new(conn.session());
        executor.execute_batch(batch).await
    }

    /// Execute multiple commands sequentially
    pub async fn execute_many(&self, server: &Server, commands: &[&str]) -> Result<Vec<CommandResult>> {
        let conn = self.pool.get_connection(server).await?;
        let executor = CommandExecutor::new(conn.session());

        let mut results = Vec::with_capacity(commands.len());
        for cmd in commands {
            results.push(executor.execute(&Command::new(*cmd)).await?);
        }
        Ok(results)
    }

    /// Get an SFTP client for file operations
    pub async fn sftp(&self, server: &Server) -> Result<SftpClient> {
        let conn = self.pool.get_connection(server).await?;
        Ok(SftpClient::new(conn.session()))
    }

    /// Upload a file to a server
    pub async fn upload_file(
        &self,
        server: &Server,
        local_path: &str,
        remote_path: &str,
    ) -> Result<()> {
        let sftp = self.sftp(server).await?;
        sftp.upload_file(local_path, remote_path).await
    }

    /// Upload content as a file
    pub async fn upload_string(
        &self,
        server: &Server,
        remote_path: &str,
        content: &str,
    ) -> Result<()> {
        let sftp = self.sftp(server).await?;
        sftp.upload_string(remote_path, content).await
    }

    /// Download a file from a server
    pub async fn download_file(
        &self,
        server: &Server,
        remote_path: &str,
        local_path: &str,
    ) -> Result<()> {
        let sftp = self.sftp(server).await?;
        sftp.download_file(remote_path, local_path).await
    }

    /// Download file content as string
    pub async fn download_string(&self, server: &Server, remote_path: &str) -> Result<String> {
        let sftp = self.sftp(server).await?;
        sftp.download_string(remote_path).await
    }

    /// Check if a server is reachable
    #[instrument(skip(self, server))]
    pub async fn check_connection(&self, server: &Server) -> Result<bool> {
        match self.pool.get_connection(server).await {
            Ok(conn) => {
                let session = conn.session();
                Ok(session.is_alive().await)
            }
            Err(_) => Ok(false),
        }
    }

    /// Get server system info
    pub async fn get_system_info(&self, server: &Server) -> Result<SystemInfo> {
        let batch = CommandBatch::new()
            .cmd("uname -s")
            .cmd("uname -r")
            .cmd("uname -m")
            .cmd("hostname")
            .cmd("cat /etc/os-release 2>/dev/null | grep -E '^(ID|VERSION_ID)=' || true");

        let result = self.execute_batch(server, &batch).await?;

        if !result.output.success() {
            return Err(SshError::CommandFailed(result.output.stderr));
        }

        let lines: Vec<&str> = result.output.stdout.lines().collect();

        Ok(SystemInfo {
            os: lines.get(0).unwrap_or(&"unknown").to_string(),
            kernel: lines.get(1).unwrap_or(&"unknown").to_string(),
            arch: lines.get(2).unwrap_or(&"unknown").to_string(),
            hostname: lines.get(3).unwrap_or(&"unknown").to_string(),
            distro: parse_distro_info(&result.output.stdout),
        })
    }

    /// Check Docker availability
    pub async fn check_docker(&self, server: &Server) -> Result<DockerInfo> {
        let result = self.execute(server, "docker version --format '{{.Server.Version}}'").await?;

        if !result.output.success() {
            return Ok(DockerInfo {
                installed: false,
                version: None,
                compose_version: None,
            });
        }

        let docker_version = result.output.stdout.trim().to_string();

        let compose_result = self.execute(
            server,
            "docker compose version --short 2>/dev/null || docker-compose version --short 2>/dev/null || true"
        ).await?;

        let compose_version = if compose_result.output.success() && !compose_result.output.stdout.trim().is_empty() {
            Some(compose_result.output.stdout.trim().to_string())
        } else {
            None
        };

        Ok(DockerInfo {
            installed: true,
            version: Some(docker_version),
            compose_version,
        })
    }

    /// Clean up idle connections
    pub async fn cleanup(&self) {
        self.pool.cleanup().await;
    }

    /// Get connection pool statistics
    pub fn pool_stats(&self) -> crate::multiplexer::PoolStats {
        self.pool.stats()
    }
}

impl Default for SshClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Implement the RemoteExecutor trait for integration with kornetti-core
#[async_trait]
impl RemoteExecutor for SshClient {
    async fn execute(&self, server: &Server, command: &str) -> CoreResult<CoreCommandOutput> {
        let result = SshClient::execute(self, server, command).await?;
        Ok(result.output.into())
    }

    async fn execute_many(&self, server: &Server, commands: &[&str]) -> CoreResult<Vec<CoreCommandOutput>> {
        let results = SshClient::execute_many(self, server, commands).await?;
        Ok(results.into_iter().map(|r| r.output.into()).collect())
    }

    async fn upload_file(
        &self,
        server: &Server,
        local_path: &str,
        remote_path: &str,
    ) -> CoreResult<()> {
        SshClient::upload_file(self, server, local_path, remote_path).await?;
        Ok(())
    }

    async fn download_file(
        &self,
        server: &Server,
        remote_path: &str,
        local_path: &str,
    ) -> CoreResult<()> {
        SshClient::download_file(self, server, remote_path, local_path).await?;
        Ok(())
    }

    async fn check_connection(&self, server: &Server) -> CoreResult<bool> {
        SshClient::check_connection(self, server).await.map_err(Into::into)
    }
}

/// System information from a server
#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub os: String,
    pub kernel: String,
    pub arch: String,
    pub hostname: String,
    pub distro: Option<DistroInfo>,
}

/// Linux distribution info
#[derive(Debug, Clone)]
pub struct DistroInfo {
    pub id: String,
    pub version: String,
}

/// Docker installation info
#[derive(Debug, Clone)]
pub struct DockerInfo {
    pub installed: bool,
    pub version: Option<String>,
    pub compose_version: Option<String>,
}

fn parse_distro_info(output: &str) -> Option<DistroInfo> {
    let mut id = None;
    let mut version = None;

    for line in output.lines() {
        if line.starts_with("ID=") {
            id = Some(line.trim_start_matches("ID=").trim_matches('"').to_string());
        } else if line.starts_with("VERSION_ID=") {
            version = Some(line.trim_start_matches("VERSION_ID=").trim_matches('"').to_string());
        }
    }

    match (id, version) {
        (Some(id), Some(version)) => Some(DistroInfo { id, version }),
        _ => None,
    }
}
