//! Install Prerequisites Action
//!
//! Installs required tools on a server based on its OS type.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::{info, instrument, warn};

use crate::models::Server;
use super::{OsType, ValidatePrerequisites, ValidatePrerequisitesInput};
use crate::actions::{Action, ActionError, ActionResult, CommandBuilder};
use crate::ssh_stub::SshClient;

/// Input for installing prerequisites
pub struct InstallPrerequisitesInput<'a> {
    pub server: &'a Server,
}

/// Action to install required tools on a server
pub struct InstallPrerequisites {
    ssh: Arc<dyn SshClient>,
}

impl InstallPrerequisites {
    pub fn new(ssh: Arc<dyn SshClient>) -> Self {
        Self { ssh }
    }

    /// Get install command for required packages
    fn get_install_command(os_type: OsType, packages: &[&str]) -> String {
        let pkg_list = packages.join(" ");

        match os_type {
            OsType::Debian | OsType::Ubuntu => {
                format!(
                    "apt-get update -qq && apt-get install -y -qq {}",
                    pkg_list
                )
            }
            OsType::Rhel | OsType::CentOS | OsType::Fedora => {
                format!("dnf install -y {}", pkg_list)
            }
            OsType::Alpine => {
                format!("apk add --no-cache {}", pkg_list)
            }
            OsType::Suse => {
                format!("zypper install -y --no-confirm {}", pkg_list)
            }
            OsType::Arch => {
                format!("pacman -S --noconfirm --needed {}", pkg_list)
            }
            OsType::Unknown => {
                // Try apt-get as fallback
                format!(
                    "apt-get update -qq && apt-get install -y -qq {} || dnf install -y {} || apk add --no-cache {}",
                    pkg_list, pkg_list, pkg_list
                )
            }
        }
    }

    /// Map tool names to package names for different distros
    fn get_package_name(os_type: OsType, tool: &str) -> String {
        match (os_type, tool) {
            (_, "curl") => "curl".to_string(),
            (_, "wget") => "wget".to_string(),
            (_, "git") => "git".to_string(),
            (_, "jq") => "jq".to_string(),
            (_, "tar") => "tar".to_string(),
            (_, "gzip") => "gzip".to_string(),
            (OsType::Alpine, "rsync") => "rsync".to_string(),
            (_, "rsync") => "rsync".to_string(),
            (OsType::Alpine, "ssh") => "openssh-client".to_string(),
            (_, "ssh") => "openssh-client".to_string(),
            _ => tool.to_string(),
        }
    }
}

#[async_trait]
impl Action for InstallPrerequisites {
    type Input = InstallPrerequisitesInput<'static>;
    type Output = ActionResult;

    fn name(&self) -> &'static str {
        "install_prerequisites"
    }

    #[instrument(skip(self, input), fields(server_id = %input.server.id))]
    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        let server = input.server;
        let start = std::time::Instant::now();

        info!("Installing prerequisites on server {}", server.id);

        let session = self.ssh.connect(server)
            .await
            .map_err(|e| ActionError::ssh_connection_failed(e.to_string()))?;

        // First check what's missing
        let validator = ValidatePrerequisites::new(self.ssh.clone());
        let prereqs = validator.handle(ValidatePrerequisitesInput { server }).await?;

        if prereqs.all_present {
            return Ok(ActionResult::success("All prerequisites already installed")
                .with_duration(start.elapsed()));
        }

        // Detect OS
        let os_output = session.execute("cat /etc/os-release 2>/dev/null || echo 'unknown'")
            .await
            .map_err(|e| ActionError::command_failed("detect os", e.to_string()))?;

        let os_type = OsType::from_os_release(&os_output.stdout);

        // Map missing tools to packages
        let packages: Vec<&str> = prereqs.missing
            .iter()
            .map(|tool| Self::get_package_name(os_type, tool))
            .collect();

        if packages.is_empty() {
            return Ok(ActionResult::success("No packages to install")
                .with_duration(start.elapsed()));
        }

        info!("Installing packages: {:?}", packages);

        let mut builder = CommandBuilder::new();
        builder.echo(&format!("Installing prerequisites: {}", packages.join(", ")));
        builder.add(Self::get_install_command(os_type, &packages));
        builder.echo("Prerequisites installed successfully");

        let commands = builder.build();

        // Execute installation
        for cmd in &commands {
            if cmd.starts_with("echo ") {
                continue;
            }

            let result = session.execute(cmd)
                .await
                .map_err(|e| ActionError::command_failed(cmd, e.to_string()))?;

            if result.exit_code != 0 {
                warn!("Installation command failed: {}", cmd);
                return Err(ActionError::command_failed(cmd, result.stderr));
            }
        }

        // Verify installation
        let final_check = validator.handle(ValidatePrerequisitesInput { server }).await?;

        if !final_check.all_present {
            return Err(ActionError::new(
                "INSTALL_FAILED",
                format!("Failed to install: {:?}", final_check.missing)
            ));
        }

        Ok(ActionResult::success(format!("Installed {} packages", packages.len()))
            .with_duration(start.elapsed())
            .with_commands(commands))
    }
}
