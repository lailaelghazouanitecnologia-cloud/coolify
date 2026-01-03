//! Check Updates Action
//!
//! Checks for available system and Docker updates on a server.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use crate::models::Server;
use super::OsType;
use crate::actions::{Action, ActionError};
use kornetti_ssh::SshClient;

/// Update check results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatesAvailable {
    pub system_updates: u32,
    pub security_updates: u32,
    pub docker_update_available: bool,
    pub current_docker_version: Option<String>,
    pub latest_docker_version: Option<String>,
    pub reboot_required: bool,
    pub packages: Vec<PackageUpdate>,
}

/// Individual package update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageUpdate {
    pub name: String,
    pub current_version: String,
    pub new_version: String,
    pub is_security: bool,
}

impl Default for UpdatesAvailable {
    fn default() -> Self {
        Self {
            system_updates: 0,
            security_updates: 0,
            docker_update_available: false,
            current_docker_version: None,
            latest_docker_version: None,
            reboot_required: false,
            packages: Vec::new(),
        }
    }
}

/// Input for update check
pub struct CheckUpdatesInput<'a> {
    pub server: &'a Server,
}

/// Action to check for available updates
pub struct CheckUpdates {
    ssh: Arc<SshClient>,
}

impl CheckUpdates {
    pub fn new(ssh: Arc<SshClient>) -> Self {
        Self { ssh }
    }

    /// Parse apt-get update output for available updates
    fn parse_apt_updates(output: &str) -> (u32, u32, Vec<PackageUpdate>) {
        let mut total = 0u32;
        let mut security = 0u32;
        let mut packages = Vec::new();

        for line in output.lines() {
            if line.contains("upgradable") {
                total += 1;

                // Parse package info: "package/distro version1 arch [upgradable from: version2]"
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let name = parts[0].split('/').next().unwrap_or("").to_string();
                    let new_version = parts[1].to_string();
                    let is_security = line.contains("-security");

                    if is_security {
                        security += 1;
                    }

                    // Extract current version from "[upgradable from: X.X.X]"
                    let current_version = line
                        .split("upgradable from:")
                        .nth(1)
                        .map(|s| s.trim().trim_end_matches(']').to_string())
                        .unwrap_or_default();

                    packages.push(PackageUpdate {
                        name,
                        current_version,
                        new_version,
                        is_security,
                    });
                }
            }
        }

        (total, security, packages)
    }

    /// Parse dnf/yum check-update output
    fn parse_dnf_updates(output: &str) -> (u32, u32, Vec<PackageUpdate>) {
        let mut total = 0u32;
        let security = 0u32; // DNF doesn't easily distinguish security updates
        let mut packages = Vec::new();

        for line in output.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 && !line.starts_with("Last") && !line.starts_with("Obsoleting") {
                let name = parts[0].to_string();
                let new_version = parts[1].to_string();

                packages.push(PackageUpdate {
                    name,
                    current_version: String::new(),
                    new_version,
                    is_security: false,
                });
                total += 1;
            }
        }

        (total, security, packages)
    }
}

#[async_trait]
impl Action for CheckUpdates {
    type Input = CheckUpdatesInput<'static>;
    type Output = UpdatesAvailable;

    fn name(&self) -> &'static str {
        "check_updates"
    }

    #[instrument(skip(self, input), fields(server_id = %input.server.id))]
    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        let server = input.server;

        info!("Checking for updates on server {}", server.id);

        let session = self.ssh.connect(server)
            .await
            .map_err(|e| ActionError::ssh_connection_failed(e.to_string()))?;

        let mut result = UpdatesAvailable::default();

        // Detect OS
        let os_output = session.execute("cat /etc/os-release 2>/dev/null")
            .await
            .map_err(|e| ActionError::command_failed("detect os", e.to_string()))?;

        let os_type = OsType::from_os_release(&os_output.stdout);

        // Check for system updates based on OS
        match os_type {
            OsType::Debian | OsType::Ubuntu => {
                // Update package list first
                let _ = session.execute("apt-get update -qq 2>/dev/null").await;

                let updates = session.execute("apt list --upgradable 2>/dev/null")
                    .await
                    .ok();

                if let Some(output) = updates {
                    let (total, security, packages) = Self::parse_apt_updates(&output.stdout);
                    result.system_updates = total;
                    result.security_updates = security;
                    result.packages = packages;
                }
            }
            OsType::Rhel | OsType::CentOS | OsType::Fedora => {
                let updates = session.execute("dnf check-update 2>/dev/null")
                    .await
                    .ok();

                if let Some(output) = updates {
                    let (total, security, packages) = Self::parse_dnf_updates(&output.stdout);
                    result.system_updates = total;
                    result.security_updates = security;
                    result.packages = packages;
                }
            }
            _ => {
                // Unsupported OS for update checking
            }
        }

        // Check current Docker version
        let docker_version = session.execute("docker --version 2>/dev/null")
            .await
            .ok();

        if let Some(output) = docker_version {
            if output.exit_code == 0 {
                result.current_docker_version = output.stdout
                    .split_whitespace()
                    .nth(2)
                    .map(|v| v.trim_end_matches(',').to_string());
            }
        }

        // Check if reboot is required
        let reboot_check = session.execute("test -f /var/run/reboot-required && echo 'yes'")
            .await
            .ok();

        result.reboot_required = reboot_check
            .map(|r| r.stdout.trim() == "yes")
            .unwrap_or(false);

        info!(
            "Update check complete: {} system updates, {} security updates, reboot_required={}",
            result.system_updates,
            result.security_updates,
            result.reboot_required
        );

        Ok(result)
    }
}
