//! Application model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Application {
    pub id: Uuid,
    pub environment_id: Uuid,
    pub server_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub fqdn: Option<String>,
    pub source: ApplicationSource,
    pub build_config: BuildConfig,
    pub deploy_config: DeployConfig,
    pub status: ApplicationStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ApplicationSource {
    Git {
        repository_url: String,
        branch: String,
        commit_sha: Option<String>,
        private_key_id: Option<Uuid>,
    },
    DockerImage {
        image: String,
        tag: String,
        registry_id: Option<Uuid>,
    },
    DockerCompose {
        content: String,
    },
    Dockerfile {
        content: String,
        context: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    pub build_pack: BuildPack,
    pub dockerfile_path: Option<String>,
    pub build_command: Option<String>,
    pub install_command: Option<String>,
    pub start_command: Option<String>,
    pub base_directory: Option<String>,
    pub publish_directory: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BuildPack {
    Nixpacks,
    Dockerfile,
    DockerImage,
    DockerCompose,
    Static,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployConfig {
    pub replicas: u32,
    pub health_check: Option<HealthCheck>,
    pub resources: ResourceLimits,
    pub ports: Vec<PortMapping>,
    pub volumes: Vec<VolumeMapping>,
    pub environment_variables: Vec<EnvVar>,
    pub labels: Vec<Label>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub path: String,
    pub port: u16,
    pub interval: u32,
    pub timeout: u32,
    pub retries: u32,
    pub start_period: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub memory_limit: Option<String>,
    pub memory_reservation: Option<String>,
    pub cpu_limit: Option<f64>,
    pub cpu_reservation: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    pub container_port: u16,
    pub host_port: Option<u16>,
    pub protocol: Protocol,
    pub public: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    #[default]
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMapping {
    pub container_path: String,
    pub host_path: Option<String>,
    pub volume_name: Option<String>,
}

/// Simple environment variable for deploy config (embedded)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVar {
    pub key: String,
    pub value: String,
    pub is_secret: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApplicationStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Restarting,
    Degraded,
    Error,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            build_pack: BuildPack::Nixpacks,
            dockerfile_path: None,
            build_command: None,
            install_command: None,
            start_command: None,
            base_directory: None,
            publish_directory: None,
        }
    }
}

impl Default for DeployConfig {
    fn default() -> Self {
        Self {
            replicas: 1,
            health_check: None,
            resources: ResourceLimits::default(),
            ports: vec![],
            volumes: vec![],
            environment_variables: vec![],
            labels: vec![],
        }
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            memory_limit: None,
            memory_reservation: None,
            cpu_limit: None,
            cpu_reservation: None,
        }
    }
}
