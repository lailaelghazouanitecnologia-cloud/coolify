//! Docker Compose Parser
//!
//! Parses and manipulates Docker Compose YAML files.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod parser;
pub mod builder;

pub use parser::{parse_compose, parse_compose_str};
pub use builder::ComposeBuilder;

/// Docker Compose file representation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComposeFile {
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub services: HashMap<String, Service>,
    #[serde(default)]
    pub networks: HashMap<String, Network>,
    #[serde(default)]
    pub volumes: HashMap<String, Volume>,
    #[serde(default)]
    pub secrets: HashMap<String, Secret>,
    #[serde(default)]
    pub configs: HashMap<String, Config>,
}

/// Docker Compose service definition
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Service {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build: Option<BuildConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<StringOrList>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entrypoint: Option<StringOrList>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ports: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expose: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub volumes: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub environment: HashMap<String, StringOrNull>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_file: Option<StringOrList>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub networks: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restart: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub labels: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub healthcheck: Option<HealthCheck>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deploy: Option<Deploy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<Logging>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dns: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_hosts: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cap_add: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cap_drop: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privileged: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdin_open: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tty: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mem_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpus: Option<f64>,
}

/// Build configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BuildConfig {
    Simple(String),
    Extended {
        context: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        dockerfile: Option<String>,
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        args: HashMap<String, String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        target: Option<String>,
    },
}

/// String or list of strings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StringOrList {
    String(String),
    List(Vec<String>),
}

impl StringOrList {
    pub fn as_vec(&self) -> Vec<String> {
        match self {
            StringOrList::String(s) => vec![s.clone()],
            StringOrList::List(l) => l.clone(),
        }
    }
}

/// String or null (for environment variables)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StringOrNull {
    String(String),
    Null,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HealthCheck {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test: Option<StringOrList>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retries: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_period: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable: Option<bool>,
}

/// Deploy configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Deploy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replicas: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<Resources>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restart_policy: Option<RestartPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_config: Option<UpdateConfig>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub labels: HashMap<String, String>,
}

/// Resource limits and reservations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Resources {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limits: Option<ResourceSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reservations: Option<ResourceSpec>,
}

/// Resource specification
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpus: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<String>,
}

/// Restart policy
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RestartPolicy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_attempts: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window: Option<String>,
}

/// Update configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallelism: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<String>,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Logging {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub options: HashMap<String, String>,
}

/// Network definition
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Network {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub driver_opts: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub labels: HashMap<String, String>,
}

/// Volume definition
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Volume {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub driver_opts: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub labels: HashMap<String, String>,
}

/// Secret definition
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Secret {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Config definition
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl ComposeFile {
    /// Create a new empty compose file
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a service
    pub fn add_service(&mut self, name: impl Into<String>, service: Service) {
        self.services.insert(name.into(), service);
    }

    /// Add a network
    pub fn add_network(&mut self, name: impl Into<String>, network: Network) {
        self.networks.insert(name.into(), network);
    }

    /// Add a volume
    pub fn add_volume(&mut self, name: impl Into<String>, volume: Volume) {
        self.volumes.insert(name.into(), volume);
    }

    /// Get service names
    pub fn service_names(&self) -> Vec<&str> {
        self.services.keys().map(|s| s.as_str()).collect()
    }

    /// Get a service by name
    pub fn get_service(&self, name: &str) -> Option<&Service> {
        self.services.get(name)
    }

    /// Convert to YAML string
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }

    /// Parse ports and return (host, container) pairs
    pub fn parse_service_ports(service: &Service) -> Vec<(u16, u16)> {
        let mut result = Vec::new();
        for port in &service.ports {
            if let Some((host, container)) = Self::parse_port_mapping(port) {
                result.push((host, container));
            }
        }
        result
    }

    fn parse_port_mapping(port: &str) -> Option<(u16, u16)> {
        // Handle formats: "8080:80", "127.0.0.1:8080:80", "80"
        let parts: Vec<&str> = port.split(':').collect();
        match parts.len() {
            1 => {
                // Just container port
                let p: u16 = parts[0].split('/').next()?.parse().ok()?;
                Some((p, p))
            }
            2 => {
                // host:container
                let host: u16 = parts[0].parse().ok()?;
                let container: u16 = parts[1].split('/').next()?.parse().ok()?;
                Some((host, container))
            }
            3 => {
                // ip:host:container
                let host: u16 = parts[1].parse().ok()?;
                let container: u16 = parts[2].split('/').next()?.parse().ok()?;
                Some((host, container))
            }
            _ => None,
        }
    }

    /// Get environment variables as a HashMap
    pub fn service_env_vars(service: &Service) -> HashMap<String, String> {
        service.environment.iter()
            .filter_map(|(k, v)| {
                match v {
                    StringOrNull::String(s) => Some((k.clone(), s.clone())),
                    StringOrNull::Null => None,
                }
            })
            .collect()
    }
}

impl Service {
    /// Create a new service with just an image
    pub fn from_image(image: impl Into<String>) -> Self {
        Self {
            image: Some(image.into()),
            ..Default::default()
        }
    }

    /// Add a port mapping
    pub fn with_port(mut self, host: u16, container: u16) -> Self {
        self.ports.push(format!("{}:{}", host, container));
        self
    }

    /// Add an environment variable
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.environment.insert(key.into(), StringOrNull::String(value.into()));
        self
    }

    /// Add a volume
    pub fn with_volume(mut self, volume: impl Into<String>) -> Self {
        self.volumes.push(volume.into());
        self
    }

    /// Add a label
    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    /// Set restart policy
    pub fn with_restart(mut self, policy: impl Into<String>) -> Self {
        self.restart = Some(policy.into());
        self
    }

    /// Add network
    pub fn with_network(mut self, network: impl Into<String>) -> Self {
        self.networks.push(network.into());
        self
    }

    /// Set container name
    pub fn with_container_name(mut self, name: impl Into<String>) -> Self {
        self.container_name = Some(name.into());
        self
    }
}
