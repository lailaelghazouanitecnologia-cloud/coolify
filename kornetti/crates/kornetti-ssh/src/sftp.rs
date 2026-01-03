//! SFTP operations for file transfer
//!
//! Provides secure file upload/download over SSH.

use std::path::Path;
use std::sync::Arc;

use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, instrument};

use crate::error::{Result, SshError};
use crate::session::SshSession;

/// SFTP client for file operations
pub struct SftpClient {
    session: Arc<SshSession>,
}

impl SftpClient {
    /// Create a new SFTP client
    pub fn new(session: Arc<SshSession>) -> Self {
        Self { session }
    }

    /// Upload a file to the remote server
    #[instrument(skip(self, content))]
    pub async fn upload_bytes(&self, remote_path: &str, content: &[u8]) -> Result<()> {
        debug!(path = %remote_path, size = content.len(), "Uploading file");

        // Use base64 encoding to safely transfer binary content
        let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, content);

        // Create parent directory
        if let Some(parent) = Path::new(remote_path).parent() {
            let mkdir_cmd = format!("mkdir -p '{}'", parent.display());
            let result = self.session.execute(&mkdir_cmd).await?;
            if !result.success() {
                return Err(SshError::SftpError(format!(
                    "Failed to create directory: {}",
                    result.stderr
                )));
            }
        }

        // Write file using base64 decode
        let write_cmd = format!(
            "echo '{}' | base64 -d > '{}'",
            encoded, remote_path
        );
        let result = self.session.execute(&write_cmd).await?;

        if !result.success() {
            return Err(SshError::SftpError(format!(
                "Failed to write file: {}",
                result.stderr
            )));
        }

        debug!(path = %remote_path, "File uploaded successfully");
        Ok(())
    }

    /// Upload a file from local filesystem
    #[instrument(skip(self))]
    pub async fn upload_file(&self, local_path: &str, remote_path: &str) -> Result<()> {
        let mut file = File::open(local_path)
            .await
            .map_err(|e| SshError::Io(e))?;

        let mut content = Vec::new();
        file.read_to_end(&mut content)
            .await
            .map_err(|e| SshError::Io(e))?;

        self.upload_bytes(remote_path, &content).await
    }

    /// Upload a string as a file
    pub async fn upload_string(&self, remote_path: &str, content: &str) -> Result<()> {
        self.upload_bytes(remote_path, content.as_bytes()).await
    }

    /// Download file content from remote server
    #[instrument(skip(self))]
    pub async fn download_bytes(&self, remote_path: &str) -> Result<Vec<u8>> {
        debug!(path = %remote_path, "Downloading file");

        let cmd = format!("base64 '{}'", remote_path);
        let result = self.session.execute(&cmd).await?;

        if !result.success() {
            return Err(SshError::SftpError(format!(
                "Failed to read file: {}",
                result.stderr
            )));
        }

        let content = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            result.stdout.trim(),
        )
        .map_err(|e| SshError::SftpError(format!("Failed to decode file content: {}", e)))?;

        debug!(path = %remote_path, size = content.len(), "File downloaded");
        Ok(content)
    }

    /// Download a file to local filesystem
    #[instrument(skip(self))]
    pub async fn download_file(&self, remote_path: &str, local_path: &str) -> Result<()> {
        let content = self.download_bytes(remote_path).await?;

        // Create parent directory
        if let Some(parent) = Path::new(local_path).parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| SshError::Io(e))?;
        }

        let mut file = File::create(local_path)
            .await
            .map_err(|e| SshError::Io(e))?;

        file.write_all(&content)
            .await
            .map_err(|e| SshError::Io(e))?;

        Ok(())
    }

    /// Download file as string
    pub async fn download_string(&self, remote_path: &str) -> Result<String> {
        let bytes = self.download_bytes(remote_path).await?;
        String::from_utf8(bytes)
            .map_err(|e| SshError::SftpError(format!("Invalid UTF-8: {}", e)))
    }

    /// Check if a remote file exists
    pub async fn exists(&self, remote_path: &str) -> Result<bool> {
        let cmd = format!("test -e '{}' && echo 1 || echo 0", remote_path);
        let result = self.session.execute(&cmd).await?;
        Ok(result.stdout.trim() == "1")
    }

    /// Check if a remote path is a directory
    pub async fn is_dir(&self, remote_path: &str) -> Result<bool> {
        let cmd = format!("test -d '{}' && echo 1 || echo 0", remote_path);
        let result = self.session.execute(&cmd).await?;
        Ok(result.stdout.trim() == "1")
    }

    /// Create a remote directory
    pub async fn mkdir(&self, remote_path: &str) -> Result<()> {
        let cmd = format!("mkdir -p '{}'", remote_path);
        let result = self.session.execute(&cmd).await?;

        if !result.success() {
            return Err(SshError::SftpError(format!(
                "Failed to create directory: {}",
                result.stderr
            )));
        }

        Ok(())
    }

    /// Remove a remote file or directory
    pub async fn remove(&self, remote_path: &str, recursive: bool) -> Result<()> {
        let cmd = if recursive {
            format!("rm -rf '{}'", remote_path)
        } else {
            format!("rm -f '{}'", remote_path)
        };

        let result = self.session.execute(&cmd).await?;

        if !result.success() {
            return Err(SshError::SftpError(format!(
                "Failed to remove: {}",
                result.stderr
            )));
        }

        Ok(())
    }

    /// List directory contents
    pub async fn list_dir(&self, remote_path: &str) -> Result<Vec<String>> {
        let cmd = format!("ls -1 '{}'", remote_path);
        let result = self.session.execute(&cmd).await?;

        if !result.success() {
            return Err(SshError::SftpError(format!(
                "Failed to list directory: {}",
                result.stderr
            )));
        }

        Ok(result.stdout.lines().map(String::from).collect())
    }

    /// Get file size
    pub async fn file_size(&self, remote_path: &str) -> Result<u64> {
        let cmd = format!("stat -c%s '{}'", remote_path);
        let result = self.session.execute(&cmd).await?;

        if !result.success() {
            return Err(SshError::SftpError(format!(
                "Failed to get file size: {}",
                result.stderr
            )));
        }

        result.stdout.trim().parse()
            .map_err(|e| SshError::SftpError(format!("Invalid size: {}", e)))
    }

    /// Set file permissions
    pub async fn chmod(&self, remote_path: &str, mode: &str) -> Result<()> {
        let cmd = format!("chmod {} '{}'", mode, remote_path);
        let result = self.session.execute(&cmd).await?;

        if !result.success() {
            return Err(SshError::SftpError(format!(
                "Failed to set permissions: {}",
                result.stderr
            )));
        }

        Ok(())
    }

    /// Change file owner
    pub async fn chown(&self, remote_path: &str, owner: &str) -> Result<()> {
        let cmd = format!("chown {} '{}'", owner, remote_path);
        let result = self.session.execute_sudo(&cmd).await?;

        if !result.success() {
            return Err(SshError::SftpError(format!(
                "Failed to change owner: {}",
                result.stderr
            )));
        }

        Ok(())
    }
}
