//! Docker client for local connections

use bollard::Docker;
use kornetti_core::{Result, Error};

pub struct DockerClient {
    inner: Docker,
}

impl DockerClient {
    /// Connect to local Docker daemon
    pub fn connect_local() -> Result<Self> {
        let inner = Docker::connect_with_local_defaults()
            .map_err(|e| Error::Docker(format!("Failed to connect to Docker: {}", e)))?;
        Ok(Self { inner })
    }

    /// Connect to remote Docker daemon via TCP
    pub fn connect_tcp(url: &str) -> Result<Self> {
        let inner = Docker::connect_with_http_defaults()
            .map_err(|e| Error::Docker(format!("Failed to connect to Docker: {}", e)))?;
        Ok(Self { inner })
    }

    /// Get the inner bollard client
    pub fn inner(&self) -> &Docker {
        &self.inner
    }
}
