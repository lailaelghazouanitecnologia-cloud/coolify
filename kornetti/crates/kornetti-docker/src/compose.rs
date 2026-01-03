//! Docker Compose utilities

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Docker Compose file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeFile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default)]
    pub services: HashMap<String, ComposeService>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub networks: HashMap<String, ComposeNetwork>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub volumes: HashMap<String, ComposeVolume>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeService {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build: Option<ComposeBuild>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ports: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub volumes: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub environment: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub networks: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restart: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub healthcheck: Option<ComposeHealthcheck>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub labels: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deploy: Option<ComposeDeploy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ComposeBuild {
    Simple(String),
    Extended {
        context: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        dockerfile: Option<String>,
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        args: HashMap<String, String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeHealthcheck {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retries: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_period: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeDeploy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replicas: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ComposeResources>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeResources {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limits: Option<ComposeResourceLimits>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reservations: Option<ComposeResourceLimits>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeResourceLimits {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpus: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComposeNetwork {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComposeVolume {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver: Option<String>,
}

impl ComposeFile {
    /// Parse a Docker Compose file from YAML
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    /// Convert to YAML string
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }
}
