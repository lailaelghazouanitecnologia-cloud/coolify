//! Validate Server Action
//!
//! Validates that a server is properly configured and ready for deployments.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::{info, instrument, warn};

use crate::models::Server;
use super::{OsType, ServerValidation, DOCKER_MINIMUM_VERSION, DOCKER_COMPOSE_MINIMUM_VERSION};
use crate::actions::{Action, ActionError, ActionResult};
use kornetti_ssh::SshClient;

/// Input for server validation
pub struct ValidateServerInput<'a> {
    pub server: &'a Server,
}

/// Action to validate a server's configuration
pub struct ValidateServer {
    ssh: Arc<SshClient>,
}

impl ValidateServer {
    pub fn new(ssh: Arc<SshClient>) -> Self {
        Self { ssh }
    }

    /// Parse Docker version from output
    fn parse_docker_version(output: &str) -> Option<String> {
        // Docker version 24.0.7, build afdd53b
        output.split_whitespace()
            .nth(2)
            .map(|v| v.trim_end_matches(',').to_string())
    }

    /// Parse Docker Compose version from output
    fn parse_compose_version(output: &str) -> Option<String> {
        // Docker Compose version v2.21.0
        output.split_whitespace()
            .last()
            .map(|v| v.trim_start_matches('v').to_string())
    }

    /// Compare semantic versions
    fn version_gte(version: &str, minimum: &str) -> bool {
        let parse = |v: &str| -> Vec<u32> {
            v.split('.')
                .filter_map(|s| s.parse().ok())
                .collect()
        };

        let v1 = parse(version);
        let v2 = parse(minimum);

        for (a, b) in v1.iter().zip(v2.iter()) {
            match a.cmp(b) {
                std::cmp::Ordering::Greater => return true,
                std::cmp::Ordering::Less => return false,
                std::cmp::Ordering::Equal => continue,
            }
        }
        v1.len() >= v2.len()
    }

    /// Parse disk space from df output (in GB)
    fn parse_disk_space(output: &str) -> Option<f64> {
        // df outputs in 1K blocks by default
        output.lines()
            .skip(1)
            .next()
            .and_then(|line| line.split_whitespace().nth(3))
            .and_then(|s| s.parse::<u64>().ok())
            .map(|kb| kb as f64 / 1024.0 / 1024.0)
    }

    /// Parse memory from /proc/meminfo (in GB)
    fn parse_memory(output: &str) -> Option<f64> {
        output.lines()
            .find(|line| line.starts_with("MemTotal:"))
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|s| s.parse::<u64>().ok())
            .map(|kb| kb as f64 / 1024.0 / 1024.0)
    }

    /// Parse CPU count from nproc output
    fn parse_cpu_cores(output: &str) -> Option<u32> {
        output.trim().parse().ok()
    }
}

#[async_trait]
impl Action for ValidateServer {
    type Input = ValidateServerInput<'static>;
    type Output = ServerValidation;

    fn name(&self) -> &'static str {
        "validate_server"
    }

    #[instrument(skip(self, input), fields(server_id = %input.server.id))]
    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        let server = input.server;
        let start = std::time::Instant::now();

        info!("Validating server {}", server.id);

        let mut validation = ServerValidation::default();

        // Connect to server
        let session = self.ssh.connect(server)
            .await
            .map_err(|e| ActionError::ssh_connection_failed(e.to_string()))?;

        // Check OS type
        let os_output = session.execute("cat /etc/os-release 2>/dev/null || echo 'unknown'")
            .await
            .map_err(|e| ActionError::command_failed("detect os", e.to_string()))?;

        validation.os_type = OsType::from_os_release(&os_output.stdout);
        if validation.os_type == OsType::Unknown {
            validation.errors.push("Unknown operating system".to_string());
        }

        // Extract OS version
        for line in os_output.stdout.lines() {
            if line.starts_with("VERSION_ID=") {
                validation.os_version = Some(
                    line.trim_start_matches("VERSION_ID=")
                        .trim_matches('"')
                        .to_string()
                );
                break;
            }
        }

        // Check root access
        let whoami = session.execute("whoami")
            .await
            .map_err(|e| ActionError::command_failed("whoami", e.to_string()))?;

        validation.has_root_access = whoami.stdout.trim() == "root";
        if !validation.has_root_access {
            // Check sudo access
            let sudo_check = session.execute("sudo -n true 2>/dev/null && echo 'ok'")
                .await
                .ok();
            validation.has_root_access = sudo_check
                .map(|r| r.stdout.trim() == "ok")
                .unwrap_or(false);
        }

        if !validation.has_root_access {
            validation.errors.push("No root or sudo access".to_string());
        }

        // Check Docker
        let docker_version = session.execute("docker --version 2>/dev/null")
            .await
            .ok();

        if let Some(output) = docker_version {
            if output.exit_code == 0 {
                validation.docker_installed = true;
                validation.docker_version = Self::parse_docker_version(&output.stdout);

                // Check if version is sufficient
                if let Some(ref version) = validation.docker_version {
                    if !Self::version_gte(version, DOCKER_MINIMUM_VERSION) {
                        validation.warnings.push(format!(
                            "Docker version {} is below recommended {}",
                            version, DOCKER_MINIMUM_VERSION
                        ));
                    }
                }
            }
        }

        if !validation.docker_installed {
            validation.errors.push("Docker is not installed".to_string());
        }

        // Check Docker is running
        let docker_info = session.execute("docker info 2>/dev/null")
            .await
            .ok();

        validation.docker_running = docker_info
            .map(|r| r.exit_code == 0)
            .unwrap_or(false);

        if validation.docker_installed && !validation.docker_running {
            validation.errors.push("Docker daemon is not running".to_string());
        }

        // Check Docker Compose
        let compose_version = session.execute("docker compose version 2>/dev/null")
            .await
            .ok();

        if let Some(output) = compose_version {
            if output.exit_code == 0 {
                validation.docker_compose_installed = true;

                if let Some(version) = Self::parse_compose_version(&output.stdout) {
                    if !Self::version_gte(&version, DOCKER_COMPOSE_MINIMUM_VERSION) {
                        validation.warnings.push(format!(
                            "Docker Compose version {} is below recommended {}",
                            version, DOCKER_COMPOSE_MINIMUM_VERSION
                        ));
                    }
                }
            }
        }

        if !validation.docker_compose_installed {
            validation.warnings.push("Docker Compose is not installed".to_string());
        }

        // Check disk space
        let disk_output = session.execute("df -k / 2>/dev/null")
            .await
            .ok();

        if let Some(output) = disk_output {
            validation.disk_space_gb = Self::parse_disk_space(&output.stdout);
            if let Some(gb) = validation.disk_space_gb {
                if gb < 10.0 {
                    validation.warnings.push(format!("Low disk space: {:.1} GB available", gb));
                }
            }
        }

        // Check memory
        let memory_output = session.execute("cat /proc/meminfo 2>/dev/null")
            .await
            .ok();

        if let Some(output) = memory_output {
            validation.memory_gb = Self::parse_memory(&output.stdout);
            if let Some(gb) = validation.memory_gb {
                if gb < 1.0 {
                    validation.warnings.push(format!("Low memory: {:.1} GB total", gb));
                }
            }
        }

        // Check CPU cores
        let cpu_output = session.execute("nproc 2>/dev/null")
            .await
            .ok();

        if let Some(output) = cpu_output {
            validation.cpu_cores = Self::parse_cpu_cores(&output.stdout);
        }

        // Determine if valid
        validation.is_valid = validation.errors.is_empty()
            && validation.docker_installed
            && validation.docker_running
            && validation.has_root_access;

        let duration = start.elapsed();
        info!(
            "Server validation complete: valid={}, docker={}, errors={}",
            validation.is_valid,
            validation.docker_installed,
            validation.errors.len()
        );

        Ok(validation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(ValidateServer::version_gte("24.0.7", "24.0"));
        assert!(ValidateServer::version_gte("25.0.0", "24.0"));
        assert!(!ValidateServer::version_gte("23.0.0", "24.0"));
        assert!(ValidateServer::version_gte("24.0", "24.0"));
    }

    #[test]
    fn test_parse_docker_version() {
        assert_eq!(
            ValidateServer::parse_docker_version("Docker version 24.0.7, build afdd53b"),
            Some("24.0.7".to_string())
        );
    }

    #[test]
    fn test_parse_compose_version() {
        assert_eq!(
            ValidateServer::parse_compose_version("Docker Compose version v2.21.0"),
            Some("2.21.0".to_string())
        );
    }
}
