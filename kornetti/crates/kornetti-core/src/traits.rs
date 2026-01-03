//! Core traits for Kornetti

use async_trait::async_trait;
use uuid::Uuid;

use crate::{Result, models::Server};

/// Trait for cloud providers that can provision servers
#[async_trait]
pub trait CloudProvider: Send + Sync {
    /// Get the provider name
    fn name(&self) -> &'static str;

    /// List available regions
    async fn list_regions(&self) -> Result<Vec<Region>>;

    /// List available server sizes/plans
    async fn list_sizes(&self, region: &str) -> Result<Vec<ServerSize>>;

    /// List available OS images
    async fn list_images(&self, region: &str) -> Result<Vec<OsImage>>;

    /// Create a new server
    async fn create_server(&self, config: CreateServerConfig) -> Result<ProvisionedServer>;

    /// Get server status
    async fn get_server(&self, provider_id: &str) -> Result<ProvisionedServer>;

    /// Delete a server
    async fn delete_server(&self, provider_id: &str) -> Result<()>;

    /// Reboot a server
    async fn reboot_server(&self, provider_id: &str) -> Result<()>;
}

/// Trait for executing commands on remote servers
#[async_trait]
pub trait RemoteExecutor: Send + Sync {
    /// Execute a command on the server
    async fn execute(&self, server: &Server, command: &str) -> Result<CommandOutput>;

    /// Execute multiple commands
    async fn execute_many(&self, server: &Server, commands: &[&str]) -> Result<Vec<CommandOutput>>;

    /// Upload a file to the server
    async fn upload_file(
        &self,
        server: &Server,
        local_path: &str,
        remote_path: &str,
    ) -> Result<()>;

    /// Download a file from the server
    async fn download_file(
        &self,
        server: &Server,
        remote_path: &str,
        local_path: &str,
    ) -> Result<()>;

    /// Check if the server is reachable
    async fn check_connection(&self, server: &Server) -> Result<bool>;
}

/// Trait for container orchestration
#[async_trait]
pub trait ContainerOrchestrator: Send + Sync {
    /// List containers
    async fn list_containers(&self, server: &Server) -> Result<Vec<Container>>;

    /// Start a container
    async fn start_container(&self, server: &Server, container_id: &str) -> Result<()>;

    /// Stop a container
    async fn stop_container(&self, server: &Server, container_id: &str) -> Result<()>;

    /// Remove a container
    async fn remove_container(&self, server: &Server, container_id: &str) -> Result<()>;

    /// Get container logs
    async fn container_logs(
        &self,
        server: &Server,
        container_id: &str,
        tail: Option<usize>,
    ) -> Result<String>;

    /// Execute command in container
    async fn exec_in_container(
        &self,
        server: &Server,
        container_id: &str,
        command: &str,
    ) -> Result<CommandOutput>;
}

#[derive(Debug, Clone)]
pub struct Region {
    pub id: String,
    pub name: String,
    pub country: Option<String>,
    pub available: bool,
}

#[derive(Debug, Clone)]
pub struct ServerSize {
    pub id: String,
    pub name: String,
    pub vcpus: u32,
    pub memory_mb: u32,
    pub disk_gb: u32,
    pub bandwidth_tb: Option<f64>,
    pub price_monthly: f64,
    pub price_hourly: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct OsImage {
    pub id: String,
    pub name: String,
    pub distribution: String,
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct CreateServerConfig {
    pub name: String,
    pub region: String,
    pub size: String,
    pub image: String,
    pub ssh_key_ids: Vec<String>,
    pub user_data: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ProvisionedServer {
    pub provider_id: String,
    pub name: String,
    pub ip_address: Option<String>,
    pub ipv6_address: Option<String>,
    pub status: ProvisionedServerStatus,
    pub region: String,
    pub size: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvisionedServerStatus {
    Pending,
    Running,
    Stopped,
    Error,
}

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl CommandOutput {
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

#[derive(Debug, Clone)]
pub struct Container {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub created_at: String,
    pub labels: std::collections::HashMap<String, String>,
}
