//! SSH-specific error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SshError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Command execution failed: {0}")]
    CommandFailed(String),

    #[error("Session error: {0}")]
    SessionError(String),

    #[error("Channel error: {0}")]
    ChannelError(String),

    #[error("Key parsing error: {0}")]
    KeyError(String),

    #[error("SFTP error: {0}")]
    SftpError(String),

    #[error("Timeout: operation took longer than {0} seconds")]
    Timeout(u64),

    #[error("Connection pool exhausted")]
    PoolExhausted,

    #[error("Max retries exceeded: {0}")]
    MaxRetriesExceeded(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, SshError>;

impl From<SshError> for kornetti_core::Error {
    fn from(e: SshError) -> Self {
        kornetti_core::Error::Ssh(e.to_string())
    }
}
