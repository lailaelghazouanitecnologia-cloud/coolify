//! Docker network management

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Network configuration
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub name: String,
    pub driver: NetworkDriver,
    pub attachable: bool,
    pub internal: bool,
    pub ipam: Option<IpamConfig>,
    pub labels: HashMap<String, String>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            driver: NetworkDriver::Bridge,
            attachable: true,
            internal: false,
            ipam: None,
            labels: HashMap::new(),
        }
    }
}

impl NetworkConfig {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn driver(mut self, driver: NetworkDriver) -> Self {
        self.driver = driver;
        self
    }

    pub fn internal(mut self) -> Self {
        self.internal = true;
        self
    }

    pub fn label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    /// Generate docker network create command
    pub fn to_create_command(&self) -> String {
        let mut cmd = vec!["docker", "network", "create"];

        cmd.push("--driver");
        cmd.push(match self.driver {
            NetworkDriver::Bridge => "bridge",
            NetworkDriver::Overlay => "overlay",
            NetworkDriver::Host => "host",
            NetworkDriver::None => "none",
            NetworkDriver::Macvlan => "macvlan",
        });

        if self.attachable {
            cmd.push("--attachable");
        }

        if self.internal {
            cmd.push("--internal");
        }

        for (key, value) in &self.labels {
            cmd.push("--label");
            cmd.push(&format!("{}={}", key, value));
        }

        cmd.push(&self.name);

        cmd.join(" ")
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum NetworkDriver {
    #[default]
    Bridge,
    Overlay,
    Host,
    None,
    Macvlan,
}

#[derive(Debug, Clone)]
pub struct IpamConfig {
    pub driver: String,
    pub subnet: Option<String>,
    pub gateway: Option<String>,
}

/// Network information from Docker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub scope: String,
    pub internal: bool,
    pub attachable: bool,
    pub containers: HashMap<String, NetworkContainer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkContainer {
    pub name: String,
    #[serde(rename = "IPv4Address")]
    pub ipv4_address: String,
    #[serde(rename = "IPv6Address")]
    pub ipv6_address: String,
}

/// Parse network list output from docker network ls
pub fn parse_network_list(output: &str) -> Vec<NetworkListEntry> {
    output
        .lines()
        .skip(1) // Skip header
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                Some(NetworkListEntry {
                    id: parts[0].to_string(),
                    name: parts[1].to_string(),
                    driver: parts[2].to_string(),
                    scope: parts[3].to_string(),
                })
            } else {
                None
            }
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct NetworkListEntry {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub scope: String,
}
