//! Server model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub id: Uuid,
    pub team_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub ip: String,
    pub port: u16,
    pub user: String,
    pub private_key_id: Uuid,
    pub status: ServerStatus,
    pub provider: Option<CloudProvider>,
    pub provider_id: Option<String>,
    pub region: Option<String>,
    pub settings: ServerSettings,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServerStatus {
    New,
    Validating,
    Reachable,
    Unreachable,
    Installing,
    Ready,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CloudProvider {
    Vultr,
    Hetzner,
    DigitalOcean,
    Aws,
    Linode,
    Gcp,
    Azure,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSettings {
    pub docker_installed: bool,
    pub docker_version: Option<String>,
    pub proxy_type: ProxyType,
    pub wildcard_domain: Option<String>,
    pub concurrent_builds: u32,
    pub sentinel_enabled: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProxyType {
    #[default]
    Traefik,
    Caddy,
    None,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            docker_installed: false,
            docker_version: None,
            proxy_type: ProxyType::default(),
            wildcard_domain: None,
            concurrent_builds: 2,
            sentinel_enabled: false,
        }
    }
}

impl Server {
    pub fn is_ready(&self) -> bool {
        self.status == ServerStatus::Ready
    }

    pub fn is_reachable(&self) -> bool {
        matches!(self.status, ServerStatus::Reachable | ServerStatus::Ready)
    }
}
