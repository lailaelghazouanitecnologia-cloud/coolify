//! Kornetti SSH
//!
//! SSH client for remote command execution.

pub mod client;
pub mod executor;
pub mod key;

pub use client::SshClient;
pub use executor::SshExecutor;
pub use key::PrivateKey;

use async_trait::async_trait;
use kornetti_core::{
    Result,
    models::Server,
    traits::{RemoteExecutor, CommandOutput},
};

/// Default SSH executor implementation
pub struct DefaultSshExecutor;

#[async_trait]
impl RemoteExecutor for DefaultSshExecutor {
    async fn execute(&self, server: &Server, command: &str) -> Result<CommandOutput> {
        let client = SshClient::connect(server).await?;
        client.execute(command).await
    }

    async fn execute_many(&self, server: &Server, commands: &[&str]) -> Result<Vec<CommandOutput>> {
        let client = SshClient::connect(server).await?;
        let mut results = Vec::with_capacity(commands.len());
        for cmd in commands {
            results.push(client.execute(cmd).await?);
        }
        Ok(results)
    }

    async fn upload_file(
        &self,
        server: &Server,
        local_path: &str,
        remote_path: &str,
    ) -> Result<()> {
        let client = SshClient::connect(server).await?;
        client.upload_file(local_path, remote_path).await
    }

    async fn download_file(
        &self,
        server: &Server,
        remote_path: &str,
        local_path: &str,
    ) -> Result<()> {
        let client = SshClient::connect(server).await?;
        client.download_file(remote_path, local_path).await
    }

    async fn check_connection(&self, server: &Server) -> Result<bool> {
        match SshClient::connect(server).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}
