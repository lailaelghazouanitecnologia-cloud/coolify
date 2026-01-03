//! Error types for Kornetti

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("SSH connection error: {0}")]
    Ssh(String),

    #[error("Docker error: {0}")]
    Docker(String),

    #[error("Deployment error: {0}")]
    Deployment(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Error::Ssh(_) | Error::Docker(_) | Error::Provider(_)
        )
    }
}
