//! Application Actions
//!
//! Actions for managing deployed applications.

mod stop_application;
mod generate_config;
mod load_compose;
mod deploy_application;
mod restart_application;
mod nixpacks_build;
mod dockerfile_generate;

pub use stop_application::StopApplication;
pub use generate_config::GenerateConfig;
pub use load_compose::LoadComposeFile;
pub use deploy_application::{DeployApplication, DeployApplicationInput, DeployApplicationOutput};
pub use restart_application::{RestartApplication, RestartApplicationInput, RestartApplicationOutput};
pub use nixpacks_build::{
    NixpacksBuild, NixpacksBuildInput, NixpacksBuildOutput, NixpacksAppType,
    NixpacksPlan, NixpacksPhases, NixpacksStartPhase,
};
pub use dockerfile_generate::{
    DockerfileGenerate, DockerfileGenerateInput, DockerfileGenerateOutput, BuildPackType,
};

use serde::{Deserialize, Serialize};

/// Application deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentConfig {
    /// Container name
    pub container_name: String,
    /// Docker image
    pub image: String,
    /// Docker network
    pub network: String,
    /// Environment variables
    pub env_vars: Vec<(String, String)>,
    /// Port mappings
    pub ports: Vec<(u16, u16)>,
    /// Volume mounts
    pub volumes: Vec<(String, String)>,
    /// Traefik labels for routing
    pub labels: Vec<(String, String)>,
    /// Resource limits
    pub resource_limits: ResourceLimits,
    /// Health check configuration
    pub health_check: Option<HealthCheckConfig>,
    /// Number of replicas
    pub replicas: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub memory: Option<String>,
    pub cpus: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    pub test: Vec<String>,
    pub interval: String,
    pub timeout: String,
    pub retries: u32,
    pub start_period: String,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            test: vec!["CMD-SHELL".to_string(), "curl -f http://localhost/ || exit 1".to_string()],
            interval: "10s".to_string(),
            timeout: "5s".to_string(),
            retries: 3,
            start_period: "30s".to_string(),
        }
    }
}

/// Generate Traefik labels for an application
pub fn generate_traefik_labels(
    uuid: &str,
    domains: &[String],
    container_port: u16,
    https: bool,
    force_https: bool,
) -> Vec<(String, String)> {
    let mut labels = vec![
        ("traefik.enable".to_string(), "true".to_string()),
        ("coolify.managed".to_string(), "true".to_string()),
        ("coolify.applicationId".to_string(), uuid.to_string()),
    ];

    if domains.is_empty() {
        return labels;
    }

    // Generate host rule
    let hosts: Vec<String> = domains.iter()
        .map(|d| format!("Host(`{}`)", d))
        .collect();
    let rule = hosts.join(" || ");

    // HTTP router
    labels.push((
        format!("traefik.http.routers.{}-http.rule", uuid),
        rule.clone(),
    ));
    labels.push((
        format!("traefik.http.routers.{}-http.entrypoints", uuid),
        "web".to_string(),
    ));

    if force_https {
        labels.push((
            format!("traefik.http.routers.{}-http.middlewares", uuid),
            format!("{}-redirect", uuid),
        ));
        labels.push((
            format!("traefik.http.middlewares.{}-redirect.redirectscheme.scheme", uuid),
            "https".to_string(),
        ));
    } else {
        labels.push((
            format!("traefik.http.routers.{}-http.service", uuid),
            format!("{}-service", uuid),
        ));
    }

    // HTTPS router
    if https {
        labels.push((
            format!("traefik.http.routers.{}-https.rule", uuid),
            rule,
        ));
        labels.push((
            format!("traefik.http.routers.{}-https.entrypoints", uuid),
            "websecure".to_string(),
        ));
        labels.push((
            format!("traefik.http.routers.{}-https.tls", uuid),
            "true".to_string(),
        ));
        labels.push((
            format!("traefik.http.routers.{}-https.tls.certresolver", uuid),
            "letsencrypt".to_string(),
        ));
        labels.push((
            format!("traefik.http.routers.{}-https.service", uuid),
            format!("{}-service", uuid),
        ));
    }

    // Service
    labels.push((
        format!("traefik.http.services.{}-service.loadbalancer.server.port", uuid),
        container_port.to_string(),
    ));

    labels
}

/// Generate Docker Compose YAML for an application
pub fn generate_application_compose(config: &DeploymentConfig) -> String {
    let labels: Vec<String> = config.labels.iter()
        .map(|(k, v)| format!("      - \"{}={}\"", k, v))
        .collect();

    let ports: Vec<String> = config.ports.iter()
        .map(|(h, c)| format!("      - \"{}:{}\"", h, c))
        .collect();

    let volumes: Vec<String> = config.volumes.iter()
        .map(|(h, c)| format!("      - \"{}:{}\"", h, c))
        .collect();

    let env_vars: Vec<String> = config.env_vars.iter()
        .map(|(k, v)| format!("      - \"{}={}\"", k, v))
        .collect();

    let mut yaml = format!(r#"services:
  {}:
    image: {}
    container_name: {}
    restart: unless-stopped
    networks:
      - {}
"#,
        config.container_name,
        config.image,
        config.container_name,
        config.network
    );

    if !labels.is_empty() {
        yaml.push_str("    labels:\n");
        yaml.push_str(&labels.join("\n"));
        yaml.push_str("\n");
    }

    if !ports.is_empty() {
        yaml.push_str("    ports:\n");
        yaml.push_str(&ports.join("\n"));
        yaml.push_str("\n");
    }

    if !volumes.is_empty() {
        yaml.push_str("    volumes:\n");
        yaml.push_str(&volumes.join("\n"));
        yaml.push_str("\n");
    }

    if !env_vars.is_empty() {
        yaml.push_str("    environment:\n");
        yaml.push_str(&env_vars.join("\n"));
        yaml.push_str("\n");
    }

    if let Some(ref hc) = config.health_check {
        yaml.push_str(&format!(r#"    healthcheck:
      test: {}
      interval: {}
      timeout: {}
      retries: {}
      start_period: {}
"#,
            serde_json::to_string(&hc.test).unwrap_or_default(),
            hc.interval,
            hc.timeout,
            hc.retries,
            hc.start_period
        ));
    }

    if let Some(ref mem) = config.resource_limits.memory {
        yaml.push_str(&format!("    mem_limit: {}\n", mem));
    }

    if let Some(cpus) = config.resource_limits.cpus {
        yaml.push_str(&format!("    cpus: {}\n", cpus));
    }

    yaml.push_str(&format!(r#"
networks:
  {}:
    external: true
"#, config.network));

    yaml
}
