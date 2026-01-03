//! Load Compose File Action
//!
//! Loads and parses a Docker Compose file from an application's repository.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use crate::models::{Server, Application};
use crate::actions::{Action, ActionError};
use crate::ssh_stub::SshClient;

/// Parsed compose file information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeFile {
    /// Raw YAML content
    pub content: String,
    /// Parsed services
    pub services: Vec<ComposeService>,
    /// Networks defined
    pub networks: Vec<String>,
    /// Volumes defined
    pub volumes: Vec<String>,
    /// File path
    pub path: String,
}

/// Compose service definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeService {
    pub name: String,
    pub image: Option<String>,
    pub build: Option<String>,
    pub ports: Vec<String>,
    pub environment: Vec<String>,
    pub volumes: Vec<String>,
    pub depends_on: Vec<String>,
    pub restart: Option<String>,
}

pub struct LoadComposeFileInput<'a> {
    pub server: &'a Server,
    pub application: &'a Application,
    /// Directory containing the compose file
    pub working_dir: String,
    /// Compose file name (default: docker-compose.yml)
    pub filename: Option<String>,
}

pub struct LoadComposeFile {
    ssh: Arc<dyn SshClient>,
}

impl LoadComposeFile {
    pub fn new(ssh: Arc<dyn SshClient>) -> Self {
        Self { ssh }
    }

    /// Parse compose YAML content
    fn parse_compose(content: &str, path: &str) -> Result<ComposeFile, ActionError> {
        let yaml: serde_yaml::Value = serde_yaml::from_str(content)
            .map_err(|e| ActionError::configuration_error(format!("Invalid YAML: {}", e)))?;

        let mut services = Vec::new();
        let mut networks = Vec::new();
        let mut volumes = Vec::new();

        // Parse services
        if let Some(serde_yaml::Value::Mapping(svc_map)) = yaml.get("services") {
            for (name, config) in svc_map {
                let name = name.as_str().unwrap_or("").to_string();

                let service = ComposeService {
                    name,
                    image: config.get("image")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    build: config.get("build")
                        .and_then(|v| {
                            if v.is_string() {
                                v.as_str().map(|s| s.to_string())
                            } else if let Some(ctx) = v.get("context") {
                                ctx.as_str().map(|s| s.to_string())
                            } else {
                                Some(".".to_string())
                            }
                        }),
                    ports: Self::parse_string_list(config.get("ports")),
                    environment: Self::parse_env_list(config.get("environment")),
                    volumes: Self::parse_string_list(config.get("volumes")),
                    depends_on: Self::parse_depends_on(config.get("depends_on")),
                    restart: config.get("restart")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                };

                services.push(service);
            }
        }

        // Parse networks
        if let Some(serde_yaml::Value::Mapping(net_map)) = yaml.get("networks") {
            for (name, _) in net_map {
                if let Some(name) = name.as_str() {
                    networks.push(name.to_string());
                }
            }
        }

        // Parse volumes
        if let Some(serde_yaml::Value::Mapping(vol_map)) = yaml.get("volumes") {
            for (name, _) in vol_map {
                if let Some(name) = name.as_str() {
                    volumes.push(name.to_string());
                }
            }
        }

        Ok(ComposeFile {
            content: content.to_string(),
            services,
            networks,
            volumes,
            path: path.to_string(),
        })
    }

    fn parse_string_list(value: Option<&serde_yaml::Value>) -> Vec<String> {
        value
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn parse_env_list(value: Option<&serde_yaml::Value>) -> Vec<String> {
        match value {
            Some(serde_yaml::Value::Sequence(seq)) => {
                seq.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            }
            Some(serde_yaml::Value::Mapping(map)) => {
                map.iter()
                    .filter_map(|(k, v)| {
                        let key = k.as_str()?;
                        let val = v.as_str().unwrap_or("");
                        Some(format!("{}={}", key, val))
                    })
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    fn parse_depends_on(value: Option<&serde_yaml::Value>) -> Vec<String> {
        match value {
            Some(serde_yaml::Value::Sequence(seq)) => {
                seq.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            }
            Some(serde_yaml::Value::Mapping(map)) => {
                map.iter()
                    .filter_map(|(k, _)| k.as_str().map(|s| s.to_string()))
                    .collect()
            }
            _ => Vec::new(),
        }
    }
}

#[async_trait]
impl Action for LoadComposeFile {
    type Input = LoadComposeFileInput<'static>;
    type Output = ComposeFile;

    fn name(&self) -> &'static str {
        "load_compose_file"
    }

    #[instrument(skip(self, input), fields(application_id = %input.application.id))]
    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        let server = input.server;

        info!("Loading compose file for application {}", input.application.id);

        let session = self.ssh.connect(server)
            .await
            .map_err(|e| ActionError::ssh_connection_failed(e.to_string()))?;

        // Find compose file
        let filename = input.filename.as_deref().unwrap_or("docker-compose.yml");
        let compose_path = format!("{}/{}", input.working_dir, filename);

        // Try alternative names if primary doesn't exist
        let alt_names = ["docker-compose.yml", "docker-compose.yaml", "compose.yml", "compose.yaml"];

        let mut content = None;
        let mut found_path = compose_path.clone();

        for name in &alt_names {
            let path = format!("{}/{}", input.working_dir, name);
            let result = session.execute(&format!("cat {} 2>/dev/null", path))
                .await
                .ok();

            if let Some(output) = result {
                if output.exit_code == 0 && !output.stdout.is_empty() {
                    content = Some(output.stdout);
                    found_path = path;
                    break;
                }
            }
        }

        let content = content.ok_or_else(|| {
            ActionError::configuration_error(format!(
                "No compose file found in {}",
                input.working_dir
            ))
        })?;

        Self::parse_compose(&content, &found_path)
    }
}
