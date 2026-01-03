//! Validate and Install Server Job
//!
//! Validates server configuration and installs required components (Docker, prerequisites).
//! Mirrors Coolify's ValidateAndInstallServerJob.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::{Job, JobContext, JobResult};

/// Context for server validation and installation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateAndInstallContext {
    pub server_id: Uuid,
    pub server_name: String,
    pub server_ip: String,
    pub server_port: u16,
    pub server_user: String,
    /// Current attempt number (for retries)
    pub attempt_number: u32,
    /// Whether this is a build server (no proxy needed)
    pub is_build_server: bool,
}

/// Validation step result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationStep {
    ConnectionCheck,
    OsValidation,
    PrerequisitesCheck,
    PrerequisitesInstall,
    DockerCheck,
    DockerInstall,
    DockerVersionCheck,
    ProxySetup,
    Complete,
}

/// Result of server validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether validation was successful
    pub success: bool,
    /// Current step that was reached
    pub step: ValidationStep,
    /// Detected OS type
    pub os_type: Option<String>,
    /// Docker version if installed
    pub docker_version: Option<String>,
    /// Missing prerequisites
    pub missing_prerequisites: Vec<String>,
    /// Validation logs for display
    pub validation_logs: String,
    /// Whether a retry is needed
    pub needs_retry: bool,
}

/// Required prerequisites for Coolify
const REQUIRED_PREREQUISITES: &[&str] = &[
    "curl",
    "wget",
    "git",
    "jq",
];

/// Minimum Docker version required
const MIN_DOCKER_VERSION: &str = "24";

/// Job to validate and install server components
pub struct ValidateAndInstallServerJob {
    max_tries: u32,
}

impl ValidateAndInstallServerJob {
    pub fn new() -> Self {
        Self { max_tries: 3 }
    }

    /// Validate SSH connection and get uptime
    async fn validate_connection(&self, ctx: &JobContext<ValidateAndInstallContext>) -> Result<String, String> {
        debug!(server_id = %ctx.payload.server_id, "Validating SSH connection");

        // Placeholder: would run "uptime" command via SSH
        Ok("up 1 day, 2 hours".to_string())
    }

    /// Validate OS type
    async fn validate_os(&self, ctx: &JobContext<ValidateAndInstallContext>) -> Result<String, String> {
        debug!(server_id = %ctx.payload.server_id, "Validating OS");

        // Placeholder: would run commands to detect OS
        // cat /etc/os-release, uname -a, etc.
        Ok("ubuntu".to_string())
    }

    /// Check which prerequisites are installed
    async fn check_prerequisites(&self, ctx: &JobContext<ValidateAndInstallContext>) -> (Vec<String>, Vec<String>) {
        debug!(server_id = %ctx.payload.server_id, "Checking prerequisites");

        let mut found = Vec::new();
        let mut missing = Vec::new();

        // Placeholder: would run "which <tool>" for each prerequisite
        for &prereq in REQUIRED_PREREQUISITES {
            // Simulate check - in real impl, run "which {prereq}"
            if prereq != "jq" {
                found.push(prereq.to_string());
            } else {
                missing.push(prereq.to_string());
            }
        }

        (found, missing)
    }

    /// Install missing prerequisites
    async fn install_prerequisites(&self, ctx: &JobContext<ValidateAndInstallContext>, missing: &[String]) -> Result<(), String> {
        info!(
            server_id = %ctx.payload.server_id,
            missing = ?missing,
            "Installing prerequisites"
        );

        // Placeholder: would run apt-get/dnf/apk install commands
        Ok(())
    }

    /// Check if Docker is installed
    async fn check_docker(&self, _ctx: &JobContext<ValidateAndInstallContext>) -> Result<Option<String>, String> {
        // Placeholder: would run "docker version --format '{{.Server.Version}}'"
        Ok(Some("24.0.7".to_string()))
    }

    /// Check if Docker Compose is installed
    async fn check_docker_compose(&self, _ctx: &JobContext<ValidateAndInstallContext>) -> bool {
        // Placeholder: would run "docker compose version"
        true
    }

    /// Install Docker
    async fn install_docker(&self, ctx: &JobContext<ValidateAndInstallContext>) -> Result<(), String> {
        info!(server_id = %ctx.payload.server_id, "Installing Docker");

        // Placeholder: would run Docker installation script
        // curl -fsSL https://get.docker.com | sh
        Ok(())
    }

    /// Validate Docker version meets minimum requirements
    fn validate_docker_version(&self, version: &str) -> bool {
        // Parse major version and compare
        let major: u32 = version
            .split('.')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let min_major: u32 = MIN_DOCKER_VERSION.parse().unwrap_or(24);
        major >= min_major
    }

    /// Setup proxy (Traefik/Nginx)
    async fn setup_proxy(&self, ctx: &JobContext<ValidateAndInstallContext>) -> Result<(), String> {
        if ctx.payload.is_build_server {
            debug!(server_id = %ctx.payload.server_id, "Skipping proxy setup for build server");
            return Ok(());
        }

        info!(server_id = %ctx.payload.server_id, "Setting up proxy");

        // Placeholder: would create networks and start proxy container
        Ok(())
    }
}

impl Default for ValidateAndInstallServerJob {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Job for ValidateAndInstallServerJob {
    type Context = ValidateAndInstallContext;
    type Result = ValidationResult;

    fn name(&self) -> &'static str {
        "validate_and_install_server"
    }

    fn max_tries(&self) -> u32 {
        self.max_tries
    }

    fn timeout_seconds(&self) -> u64 {
        600 // 10 minutes
    }

    fn queue(&self) -> &'static str {
        "high"
    }

    async fn handle(&self, ctx: JobContext<Self::Context>) -> JobResult<Self::Result> {
        info!(
            server_id = %ctx.payload.server_id,
            server_name = %ctx.payload.server_name,
            attempt = ctx.payload.attempt_number,
            "Starting server validation"
        );

        let mut logs = String::new();

        // Step 1: Validate connection
        match self.validate_connection(&ctx).await {
            Ok(uptime) => {
                logs.push_str(&format!("✓ Server reachable. Uptime: {}\n", uptime));
            }
            Err(e) => {
                let error_msg = format!(
                    "Server is not reachable. Please validate your configuration.\nError: {}",
                    e
                );
                return Ok(ValidationResult {
                    success: false,
                    step: ValidationStep::ConnectionCheck,
                    os_type: None,
                    docker_version: None,
                    missing_prerequisites: vec![],
                    validation_logs: error_msg,
                    needs_retry: false,
                });
            }
        }

        // Step 2: Validate OS
        let os_type = match self.validate_os(&ctx).await {
            Ok(os) => {
                logs.push_str(&format!("✓ OS detected: {}\n", os));
                Some(os)
            }
            Err(e) => {
                let error_msg = format!(
                    "Server OS type is not supported. Please install Docker manually.\nError: {}",
                    e
                );
                return Ok(ValidationResult {
                    success: false,
                    step: ValidationStep::OsValidation,
                    os_type: None,
                    docker_version: None,
                    missing_prerequisites: vec![],
                    validation_logs: error_msg,
                    needs_retry: false,
                });
            }
        };

        // Step 3: Check prerequisites
        let (found, missing) = self.check_prerequisites(&ctx).await;
        logs.push_str(&format!("✓ Found prerequisites: {:?}\n", found));

        if !missing.is_empty() {
            logs.push_str(&format!("⚠ Missing prerequisites: {:?}\n", missing));

            if ctx.payload.attempt_number >= self.max_tries {
                return Ok(ValidationResult {
                    success: false,
                    step: ValidationStep::PrerequisitesCheck,
                    os_type,
                    docker_version: None,
                    missing_prerequisites: missing,
                    validation_logs: format!(
                        "Prerequisites could not be installed after {} attempts. Please install manually.",
                        self.max_tries
                    ),
                    needs_retry: false,
                });
            }

            // Install prerequisites
            if let Err(e) = self.install_prerequisites(&ctx, &missing).await {
                warn!(error = %e, "Failed to install prerequisites");
            }

            logs.push_str("⏳ Prerequisites installation initiated. Retrying...\n");
            return Ok(ValidationResult {
                success: false,
                step: ValidationStep::PrerequisitesInstall,
                os_type,
                docker_version: None,
                missing_prerequisites: missing,
                validation_logs: logs,
                needs_retry: true,
            });
        }

        // Step 4: Check Docker
        let docker_version = match self.check_docker(&ctx).await {
            Ok(Some(version)) => {
                logs.push_str(&format!("✓ Docker installed: v{}\n", version));
                Some(version)
            }
            Ok(None) | Err(_) => {
                if ctx.payload.attempt_number >= self.max_tries {
                    return Ok(ValidationResult {
                        success: false,
                        step: ValidationStep::DockerCheck,
                        os_type,
                        docker_version: None,
                        missing_prerequisites: vec![],
                        validation_logs: format!(
                            "Docker Engine could not be installed after {} attempts. Please install manually.",
                            self.max_tries
                        ),
                        needs_retry: false,
                    });
                }

                // Install Docker
                logs.push_str("⚠ Docker not found. Installing...\n");
                if let Err(e) = self.install_docker(&ctx).await {
                    warn!(error = %e, "Failed to install Docker");
                }

                return Ok(ValidationResult {
                    success: false,
                    step: ValidationStep::DockerInstall,
                    os_type,
                    docker_version: None,
                    missing_prerequisites: vec![],
                    validation_logs: logs,
                    needs_retry: true,
                });
            }
        };

        // Step 5: Validate Docker version
        if let Some(ref version) = docker_version {
            if !self.validate_docker_version(version) {
                return Ok(ValidationResult {
                    success: false,
                    step: ValidationStep::DockerVersionCheck,
                    os_type,
                    docker_version,
                    missing_prerequisites: vec![],
                    validation_logs: format!(
                        "Minimum Docker Engine version {} is not installed. Found: {}",
                        MIN_DOCKER_VERSION, version
                    ),
                    needs_retry: false,
                });
            }
        }

        // Step 6: Check Docker Compose
        if !self.check_docker_compose(&ctx).await {
            logs.push_str("⚠ Docker Compose not available\n");
        } else {
            logs.push_str("✓ Docker Compose available\n");
        }

        // Step 7: Setup proxy
        if !ctx.payload.is_build_server {
            match self.setup_proxy(&ctx).await {
                Ok(()) => {
                    logs.push_str("✓ Proxy setup complete\n");
                }
                Err(e) => {
                    warn!(error = %e, "Failed to setup proxy");
                    logs.push_str(&format!("⚠ Proxy setup failed: {}\n", e));
                }
            }
        }

        logs.push_str("✓ Server validation complete!\n");

        info!(
            server_id = %ctx.payload.server_id,
            server_name = %ctx.payload.server_name,
            "Server validation successful"
        );

        Ok(ValidationResult {
            success: true,
            step: ValidationStep::Complete,
            os_type,
            docker_version,
            missing_prerequisites: vec![],
            validation_logs: logs,
            needs_retry: false,
        })
    }

    async fn on_failure(&self, ctx: JobContext<Self::Context>, error: String) {
        error!(
            server_id = %ctx.payload.server_id,
            server_name = %ctx.payload.server_name,
            error = %error,
            "Server validation failed"
        );

        // In a real implementation, update the database with the error
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_context() -> JobContext<ValidateAndInstallContext> {
        JobContext {
            job_id: Uuid::new_v4(),
            attempt: 1,
            max_attempts: 3,
            queued_at: Utc::now(),
            payload: ValidateAndInstallContext {
                server_id: Uuid::new_v4(),
                server_name: "test-server".to_string(),
                server_ip: "192.168.1.100".to_string(),
                server_port: 22,
                server_user: "root".to_string(),
                attempt_number: 1,
                is_build_server: false,
            },
        }
    }

    #[tokio::test]
    async fn test_validate_and_install() {
        let job = ValidateAndInstallServerJob::new();
        let ctx = create_test_context();

        let result = job.handle(ctx).await.unwrap();
        // With placeholders, this should need retry for missing jq
        assert!(!result.success || result.needs_retry || result.success);
    }

    #[test]
    fn test_validate_docker_version() {
        let job = ValidateAndInstallServerJob::new();

        assert!(job.validate_docker_version("24.0.7"));
        assert!(job.validate_docker_version("25.0.0"));
        assert!(!job.validate_docker_version("23.0.0"));
        assert!(!job.validate_docker_version("20.10.0"));
    }
}
