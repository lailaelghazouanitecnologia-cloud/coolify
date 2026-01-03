//! SSH client implementation

use kornetti_core::{Result, Error, models::Server, traits::CommandOutput};
use russh::*;
use russh_keys::*;
use std::sync::Arc;
use tracing::{debug, instrument};

pub struct SshClient {
    session: client::Handle<ClientHandler>,
}

struct ClientHandler;

impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // TODO: Implement proper host key verification
        Ok(true)
    }
}

impl SshClient {
    #[instrument(skip(server))]
    pub async fn connect(server: &Server) -> Result<Self> {
        debug!(ip = %server.ip, port = %server.port, user = %server.user, "Connecting to server");

        let config = client::Config::default();
        let config = Arc::new(config);

        let addr = format!("{}:{}", server.ip, server.port);
        let mut session = client::connect(config, &addr, ClientHandler)
            .await
            .map_err(|e| Error::Ssh(format!("Failed to connect: {}", e)))?;

        // TODO: Load private key from database
        // For now, use a placeholder
        let key_pair = russh_keys::PrivateKey::random(&mut rand::thread_rng(), russh_keys::Algorithm::Ed25519)
            .map_err(|e| Error::Ssh(format!("Failed to generate key: {}", e)))?;

        let auth_result = session
            .authenticate_publickey(&server.user, Arc::new(key_pair))
            .await
            .map_err(|e| Error::Ssh(format!("Authentication failed: {}", e)))?;

        if !auth_result {
            return Err(Error::Ssh("Authentication rejected".to_string()));
        }

        Ok(Self { session })
    }

    #[instrument(skip(self))]
    pub async fn execute(&self, command: &str) -> Result<CommandOutput> {
        debug!(command = %command, "Executing command");

        let mut channel = self
            .session
            .channel_open_session()
            .await
            .map_err(|e| Error::Ssh(format!("Failed to open channel: {}", e)))?;

        channel
            .exec(true, command)
            .await
            .map_err(|e| Error::Ssh(format!("Failed to execute command: {}", e)))?;

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut exit_code = 0;

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

        Ok(CommandOutput {
            stdout: String::from_utf8_lossy(&stdout).to_string(),
            stderr: String::from_utf8_lossy(&stderr).to_string(),
            exit_code,
        })
    }

    pub async fn upload_file(&self, _local_path: &str, _remote_path: &str) -> Result<()> {
        // TODO: Implement SCP/SFTP file upload
        Err(Error::Ssh("File upload not yet implemented".to_string()))
    }

    pub async fn download_file(&self, _remote_path: &str, _local_path: &str) -> Result<()> {
        // TODO: Implement SCP/SFTP file download
        Err(Error::Ssh("File download not yet implemented".to_string()))
    }
}
