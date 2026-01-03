//! Service model (Docker Compose based)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub id: Uuid,
    pub environment_id: Uuid,
    pub server_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub docker_compose: String,
    pub docker_compose_raw: String,
    pub status: ServiceStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceStatus {
    Stopped,
    Starting,
    Running,
    PartiallyRunning,
    Stopping,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceApplication {
    pub id: Uuid,
    pub service_id: Uuid,
    pub name: String,
    pub fqdn: Option<String>,
    pub image: String,
    pub exclude_from_status: bool,
    pub required_fqdn: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDatabase {
    pub id: Uuid,
    pub service_id: Uuid,
    pub name: String,
    pub image: String,
}

impl Service {
    pub fn is_running(&self) -> bool {
        matches!(self.status, ServiceStatus::Running | ServiceStatus::PartiallyRunning)
    }
}
