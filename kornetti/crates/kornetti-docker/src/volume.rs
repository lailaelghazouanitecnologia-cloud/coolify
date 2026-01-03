//! Docker volume management

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Volume configuration
#[derive(Debug, Clone)]
pub struct VolumeConfig {
    pub name: String,
    pub driver: VolumeDriver,
    pub driver_opts: HashMap<String, String>,
    pub labels: HashMap<String, String>,
}

impl Default for VolumeConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            driver: VolumeDriver::Local,
            driver_opts: HashMap::new(),
            labels: HashMap::new(),
        }
    }
}

impl VolumeConfig {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn driver(mut self, driver: VolumeDriver) -> Self {
        self.driver = driver;
        self
    }

    pub fn driver_opt(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.driver_opts.insert(key.into(), value.into());
        self
    }

    pub fn label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    /// Generate docker volume create command
    pub fn to_create_command(&self) -> String {
        let mut cmd = vec!["docker", "volume", "create"];

        cmd.push("--driver");
        cmd.push(match self.driver {
            VolumeDriver::Local => "local",
            VolumeDriver::Nfs => "local", // NFS uses local driver with options
            VolumeDriver::Custom(ref s) => s.as_str(),
        });

        for (key, value) in &self.driver_opts {
            cmd.push("--opt");
            cmd.push(&format!("{}={}", key, value));
        }

        for (key, value) in &self.labels {
            cmd.push("--label");
            cmd.push(&format!("{}={}", key, value));
        }

        cmd.push(&self.name);

        cmd.join(" ")
    }
}

#[derive(Debug, Clone, Default)]
pub enum VolumeDriver {
    #[default]
    Local,
    Nfs,
    Custom(String),
}

/// Volume information from Docker
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct VolumeInfo {
    pub name: String,
    pub driver: String,
    pub mountpoint: String,
    pub scope: String,
    #[serde(default)]
    pub labels: HashMap<String, String>,
    #[serde(default)]
    pub options: Option<HashMap<String, String>>,
}

/// Parse volume list output from docker volume ls
pub fn parse_volume_list(output: &str) -> Vec<VolumeListEntry> {
    output
        .lines()
        .skip(1) // Skip header
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                Some(VolumeListEntry {
                    driver: parts[0].to_string(),
                    name: parts[1].to_string(),
                })
            } else {
                None
            }
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct VolumeListEntry {
    pub driver: String,
    pub name: String,
}

/// Volume mount types
#[derive(Debug, Clone)]
pub enum VolumeMount {
    /// Named volume
    Named {
        name: String,
        target: String,
        read_only: bool,
    },
    /// Bind mount from host
    Bind {
        source: String,
        target: String,
        read_only: bool,
    },
    /// Tmpfs mount
    Tmpfs {
        target: String,
        size: Option<u64>,
    },
}

impl VolumeMount {
    pub fn named(name: impl Into<String>, target: impl Into<String>) -> Self {
        Self::Named {
            name: name.into(),
            target: target.into(),
            read_only: false,
        }
    }

    pub fn bind(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self::Bind {
            source: source.into(),
            target: target.into(),
            read_only: false,
        }
    }

    pub fn tmpfs(target: impl Into<String>) -> Self {
        Self::Tmpfs {
            target: target.into(),
            size: None,
        }
    }

    pub fn read_only(mut self) -> Self {
        match &mut self {
            VolumeMount::Named { read_only, .. } | VolumeMount::Bind { read_only, .. } => {
                *read_only = true;
            }
            _ => {}
        }
        self
    }

    /// Convert to docker -v argument
    pub fn to_volume_arg(&self) -> String {
        match self {
            VolumeMount::Named { name, target, read_only } => {
                if *read_only {
                    format!("{}:{}:ro", name, target)
                } else {
                    format!("{}:{}", name, target)
                }
            }
            VolumeMount::Bind { source, target, read_only } => {
                if *read_only {
                    format!("{}:{}:ro", source, target)
                } else {
                    format!("{}:{}", source, target)
                }
            }
            VolumeMount::Tmpfs { target, size } => {
                if let Some(size) = size {
                    format!("type=tmpfs,destination={},tmpfs-size={}", target, size)
                } else {
                    format!("type=tmpfs,destination={}", target)
                }
            }
        }
    }
}
