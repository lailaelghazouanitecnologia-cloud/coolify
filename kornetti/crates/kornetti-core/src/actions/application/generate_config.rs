//! Generate Config Action
//!
//! Generates Docker Compose configuration for an application.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::{info, instrument};

use crate::models::Application;
use super::{DeploymentConfig, ResourceLimits, HealthCheckConfig, generate_traefik_labels, generate_application_compose};
use crate::actions::{Action, ActionError, ActionResult};

/// Input for generating application configuration
pub struct GenerateConfigInput<'a> {
    pub application: &'a Application,
    /// Container port to expose
    pub container_port: u16,
    /// Enable HTTPS
    pub https: bool,
    /// Force HTTPS redirect
    pub force_https: bool,
    /// Custom health check
    pub health_check: Option<HealthCheckConfig>,
}

/// Generated configuration
pub struct GeneratedConfig {
    pub compose_yaml: String,
    pub labels: Vec<(String, String)>,
    pub config: DeploymentConfig,
}

pub struct GenerateConfig;

impl GenerateConfig {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Action for GenerateConfig {
    type Input = GenerateConfigInput<'static>;
    type Output = GeneratedConfig;

    fn name(&self) -> &'static str {
        "generate_config"
    }

    #[instrument(skip(self, input), fields(application_id = %input.application.id))]
    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        let app = input.application;

        info!("Generating configuration for application {}", app.id);

        let container_name = app.uuid.to_string();

        // Collect domains
        let domains: Vec<String> = app.fqdn
            .as_ref()
            .map(|f| {
                f.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        // Generate Traefik labels
        let labels = generate_traefik_labels(
            &app.uuid.to_string(),
            &domains,
            input.container_port,
            input.https,
            input.force_https,
        );

        // Build deployment config
        let config = DeploymentConfig {
            container_name: container_name.clone(),
            image: format!("{}:{}", container_name, "latest"),
            network: "coolify".to_string(),
            env_vars: app.environment_variables.clone().unwrap_or_default(),
            ports: app.port_mappings.clone(),
            volumes: app.persistent_storages.clone(),
            labels: labels.clone(),
            resource_limits: ResourceLimits {
                memory: app.limits_memory.clone(),
                cpus: app.limits_cpus,
            },
            health_check: input.health_check.or_else(|| {
                // Generate default health check based on port
                Some(HealthCheckConfig {
                    test: vec![
                        "CMD-SHELL".to_string(),
                        format!("curl -f http://localhost:{}/ || exit 1", input.container_port),
                    ],
                    interval: "10s".to_string(),
                    timeout: "5s".to_string(),
                    retries: 3,
                    start_period: "30s".to_string(),
                })
            }),
            replicas: 1,
        };

        // Generate compose YAML
        let compose_yaml = generate_application_compose(&config);

        Ok(GeneratedConfig {
            compose_yaml,
            labels,
            config,
        })
    }
}
