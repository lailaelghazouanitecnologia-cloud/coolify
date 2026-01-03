//! Service Actions
//!
//! Actions for managing Docker Compose services.

mod start_service;
mod stop_service;
mod restart_service;
mod delete_service;

pub use start_service::StartService;
pub use stop_service::StopService;
pub use restart_service::RestartService;
pub use delete_service::DeleteService;

use serde::{Deserialize, Serialize};

/// Service configuration directory
pub fn service_configuration_dir(uuid: &str) -> String {
    format!("/data/coolify/services/{}", uuid)
}

/// Service status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub uuid: String,
    pub running: bool,
    pub containers: Vec<ContainerStatus>,
    pub healthy: bool,
}

/// Container status within a service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStatus {
    pub name: String,
    pub id: String,
    pub status: String,
    pub health: Option<String>,
    pub ports: Vec<String>,
}
