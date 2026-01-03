//! Application Settings model
//!
//! Application-specific configuration.
//! Mirrors Coolify's ApplicationSetting model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationSetting {
    pub id: Uuid,
    pub application_id: Uuid,
    /// Whether preview deployments are enabled for PRs
    pub is_preview_deployments_enabled: bool,
    /// Whether to auto-deploy on push
    pub is_auto_deploy_enabled: bool,
    /// Whether to force HTTPS
    pub is_force_https_enabled: bool,
    /// Whether application is in debug mode
    pub is_debug_enabled: bool,
    /// Whether Git LFS is enabled
    pub is_git_lfs_enabled: bool,
    /// Whether Git submodules should be initialized
    pub is_git_submodules_enabled: bool,
    /// Whether to preserve repository between builds
    pub is_preserve_repository_enabled: bool,
    /// Whether to enable raw compose deployment
    pub is_raw_compose_deployment_enabled: bool,
    /// Whether to enable container label generation
    pub is_container_label_enabled: bool,
    /// Whether to enable container label escape
    pub is_container_label_escape_enabled: bool,
    /// Whether to connect to predefined networks
    pub is_connect_to_docker_network: bool,
    /// Whether build server is enabled
    pub is_build_server_enabled: bool,
    /// Whether to use static build
    pub is_static: bool,
    /// Custom GPU count for container
    pub gpu_count: Option<i32>,
    /// GPU driver to use
    pub gpu_driver: Option<String>,
    /// GPU device IDs
    pub gpu_device_ids: Option<String>,
    /// Custom healthcheck path
    pub health_check_path: Option<String>,
    /// Healthcheck interval in seconds
    pub health_check_interval: i32,
    /// Healthcheck timeout in seconds
    pub health_check_timeout: i32,
    /// Healthcheck retries
    pub health_check_retries: i32,
    /// Healthcheck start period in seconds
    pub health_check_start_period: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ApplicationSetting {
    pub fn new(application_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            application_id,
            is_preview_deployments_enabled: true,
            is_auto_deploy_enabled: true,
            is_force_https_enabled: true,
            is_debug_enabled: false,
            is_git_lfs_enabled: false,
            is_git_submodules_enabled: false,
            is_preserve_repository_enabled: false,
            is_raw_compose_deployment_enabled: false,
            is_container_label_enabled: true,
            is_container_label_escape_enabled: true,
            is_connect_to_docker_network: false,
            is_build_server_enabled: false,
            is_static: false,
            gpu_count: None,
            gpu_driver: None,
            gpu_device_ids: None,
            health_check_path: Some("/".to_string()),
            health_check_interval: 30,
            health_check_timeout: 30,
            health_check_retries: 3,
            health_check_start_period: 30,
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if GPU is configured
    pub fn has_gpu(&self) -> bool {
        self.gpu_count.map(|c| c > 0).unwrap_or(false)
    }

    /// Get healthcheck configuration for Docker
    pub fn healthcheck_config(&self) -> HealthcheckConfig {
        HealthcheckConfig {
            path: self.health_check_path.clone().unwrap_or_else(|| "/".to_string()),
            interval: self.health_check_interval,
            timeout: self.health_check_timeout,
            retries: self.health_check_retries,
            start_period: self.health_check_start_period,
        }
    }
}

/// Healthcheck configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthcheckConfig {
    pub path: String,
    pub interval: i32,
    pub timeout: i32,
    pub retries: i32,
    pub start_period: i32,
}

impl HealthcheckConfig {
    /// Generate Docker healthcheck command
    pub fn to_docker_command(&self, port: u16) -> String {
        format!(
            "curl -sf http://localhost:{}{} || exit 1",
            port, self.path
        )
    }
}

impl Default for ApplicationSetting {
    fn default() -> Self {
        Self::new(Uuid::nil())
    }
}
