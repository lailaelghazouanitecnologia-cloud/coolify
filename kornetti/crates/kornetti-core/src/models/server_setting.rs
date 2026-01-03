//! Server Settings model
//!
//! Server-specific configuration and status.
//! Mirrors Coolify's ServerSetting model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Server settings and status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSetting {
    pub id: Uuid,
    pub server_id: Uuid,
    /// Whether the server is reachable via SSH
    pub is_reachable: bool,
    /// Whether Docker is working (server is usable)
    pub is_usable: bool,
    /// Whether the server is force disabled by admin
    pub force_disabled: bool,
    /// Whether to skip server status checks
    pub skip_status_check: bool,
    /// Whether concurrent builds are allowed
    pub concurrent_builds: bool,
    /// Maximum number of concurrent builds
    pub max_concurrent_builds: i32,
    /// Whether to use SSH multiplexing
    pub use_ssh_mux: bool,
    /// Whether server is being validated
    pub is_validating: bool,
    /// Whether server needs revalidation
    pub is_revalidating: bool,
    /// Whether this server acts as a build server
    pub is_build_server: bool,
    /// Whether Docker cleanup is enabled
    pub docker_cleanup_enabled: bool,
    /// Docker cleanup frequency in minutes
    pub docker_cleanup_frequency: i32,
    /// Threshold for Docker cleanup (percentage of disk usage)
    pub docker_cleanup_threshold: i32,
    /// Whether to prune unused volumes during cleanup
    pub docker_prune_volumes: bool,
    /// Custom Docker daemon configuration JSON
    pub docker_daemon_config: Option<String>,
    /// Sentinel container status
    pub sentinel_status: Option<String>,
    /// Last sentinel check timestamp
    pub sentinel_last_check: Option<DateTime<Utc>>,
    /// Wildcard domain for applications
    pub wildcard_domain: Option<String>,
    /// Server timezone
    pub timezone: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ServerSetting {
    pub fn new(server_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            server_id,
            is_reachable: false,
            is_usable: false,
            force_disabled: false,
            skip_status_check: false,
            concurrent_builds: false,
            max_concurrent_builds: 1,
            use_ssh_mux: true,
            is_validating: false,
            is_revalidating: false,
            is_build_server: false,
            docker_cleanup_enabled: true,
            docker_cleanup_frequency: 1440, // 24 hours
            docker_cleanup_threshold: 80,
            docker_prune_volumes: false,
            docker_daemon_config: None,
            sentinel_status: None,
            sentinel_last_check: None,
            wildcard_domain: None,
            timezone: "UTC".to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if server is available for deployments
    pub fn is_available(&self) -> bool {
        self.is_reachable && self.is_usable && !self.force_disabled && !self.is_validating
    }

    /// Check if server can accept new builds
    pub fn can_accept_build(&self, current_builds: i32) -> bool {
        if !self.is_available() {
            return false;
        }
        if !self.concurrent_builds {
            return current_builds == 0;
        }
        current_builds < self.max_concurrent_builds
    }

    /// Mark server as unreachable
    pub fn mark_unreachable(&mut self) {
        self.is_reachable = false;
        self.is_usable = false;
        self.updated_at = Utc::now();
    }

    /// Mark server as reachable and usable
    pub fn mark_usable(&mut self) {
        self.is_reachable = true;
        self.is_usable = true;
        self.updated_at = Utc::now();
    }

    /// Start validation process
    pub fn start_validation(&mut self) {
        self.is_validating = true;
        self.updated_at = Utc::now();
    }

    /// Complete validation process
    pub fn complete_validation(&mut self, success: bool) {
        self.is_validating = false;
        if success {
            self.mark_usable();
        }
        self.updated_at = Utc::now();
    }
}

impl Default for ServerSetting {
    fn default() -> Self {
        Self::new(Uuid::nil())
    }
}
