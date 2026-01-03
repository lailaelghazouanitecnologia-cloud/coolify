//! Docker-specific error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DockerError {
    #[error("Container not found: {0}")]
    ContainerNotFound(String),

    #[error("Image not found: {0}")]
    ImageNotFound(String),

    #[error("Network not found: {0}")]
    NetworkNotFound(String),

    #[error("Volume not found: {0}")]
    VolumeNotFound(String),

    #[error("Build failed: {0}")]
    BuildFailed(String),

    #[error("Pull failed: {0}")]
    PullFailed(String),

    #[error("Command failed: {0}")]
    CommandFailed(String),

    #[error("Container already exists: {0}")]
    ContainerExists(String),

    #[error("Container not running: {0}")]
    ContainerNotRunning(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Compose error: {0}")]
    ComposeError(String),

    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),

    #[error("Timeout waiting for container")]
    Timeout,

    #[error("SSH error: {0}")]
    Ssh(#[from] kornetti_ssh::SshError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, DockerError>;

impl From<DockerError> for kornetti_core::Error {
    fn from(e: DockerError) -> Self {
        kornetti_core::Error::Docker(e.to_string())
    }
}
