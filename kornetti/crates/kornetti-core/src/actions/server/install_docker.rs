//! Install Docker Action
//!
//! Installs Docker Engine on a server, handling different OS distributions
//! and configurations.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::{info, instrument, warn};

use crate::models::Server;
use super::{OsType, DOCKER_MINIMUM_VERSION};
use crate::actions::{Action, ActionError, ActionResult, CommandBuilder};
use crate::ssh_stub::SshClient;

/// Input for Docker installation
pub struct InstallDockerInput<'a> {
    pub server: &'a Server,
    /// Force reinstall even if Docker is present
    pub force: bool,
}

/// Action to install Docker Engine on a server
pub struct InstallDocker {
    ssh: Arc<dyn SshClient>,
    docker_version: String,
}

impl InstallDocker {
    pub fn new(ssh: Arc<dyn SshClient>) -> Self {
        Self {
            ssh,
            docker_version: DOCKER_MINIMUM_VERSION.to_string(),
        }
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.docker_version = version.into();
        self
    }

    /// Get install command for Debian/Ubuntu
    fn get_debian_install_command(&self) -> String {
        format!(
            r#"curl --max-time 300 --retry 3 https://releases.rancher.com/install-docker/{version}.sh | sh || \
            curl --max-time 300 --retry 3 https://get.docker.com | sh -s -- --version {version} || \
            (install -m 0755 -d /etc/apt/keyrings && \
            curl -fsSL https://download.docker.com/linux/debian/gpg -o /etc/apt/keyrings/docker.asc && \
            chmod a+r /etc/apt/keyrings/docker.asc && \
            . /etc/os-release && \
            echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/debian ${{VERSION_CODENAME}} stable" > /etc/apt/sources.list.d/docker.list && \
            apt-get update && \
            apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin)"#,
            version = self.docker_version
        )
    }

    /// Get install command for RHEL/CentOS/Fedora
    fn get_rhel_install_command(&self) -> String {
        format!(
            r#"curl https://releases.rancher.com/install-docker/{version}.sh | sh || \
            curl https://get.docker.com | sh -s -- --version {version} || \
            (dnf config-manager --add-repo https://download.docker.com/linux/centos/docker-ce.repo && \
            dnf install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin && \
            systemctl start docker && \
            systemctl enable docker)"#,
            version = self.docker_version
        )
    }

    /// Get install command for SUSE
    fn get_suse_install_command(&self) -> String {
        format!(
            r#"curl https://releases.rancher.com/install-docker/{version}.sh | sh || \
            curl https://get.docker.com | sh -s -- --version {version} || \
            (zypper addrepo https://download.docker.com/linux/sles/docker-ce.repo && \
            zypper refresh && \
            zypper install -y --no-confirm docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin && \
            systemctl start docker && \
            systemctl enable docker)"#,
            version = self.docker_version
        )
    }

    /// Get install command for Arch Linux
    fn get_arch_install_command(&self) -> String {
        "pacman -Syu --noconfirm --needed docker docker-compose && \
         systemctl enable docker.service && \
         systemctl start docker.service".to_string()
    }

    /// Get generic install command
    fn get_generic_install_command(&self) -> String {
        format!(
            "curl --max-time 300 --retry 3 https://releases.rancher.com/install-docker/{version}.sh | sh || \
             curl --max-time 300 --retry 3 https://get.docker.com | sh -s -- --version {version}",
            version = self.docker_version
        )
    }

    /// Get Docker daemon config
    fn get_daemon_config() -> &'static str {
        r#"{
    "log-driver": "json-file",
    "log-opts": {
        "max-size": "10m",
        "max-file": "3"
    }
}"#
    }

    /// Detect OS type from server
    async fn detect_os(&self, server: &Server) -> Result<OsType, ActionError> {
        let session = self.ssh.connect(server)
            .await
            .map_err(|e| ActionError::ssh_connection_failed(e.to_string()))?;

        let output = session.execute("cat /etc/os-release 2>/dev/null || cat /etc/*-release 2>/dev/null || echo 'unknown'")
            .await
            .map_err(|e| ActionError::command_failed("detect os", e.to_string()))?;

        Ok(OsType::from_os_release(&output.stdout))
    }
}

#[async_trait]
impl Action for InstallDocker {
    type Input = InstallDockerInput<'static>;
    type Output = ActionResult;

    fn name(&self) -> &'static str {
        "install_docker"
    }

    #[instrument(skip(self, input), fields(server_id = %input.server.id))]
    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        let server = input.server;
        let start = std::time::Instant::now();

        info!("Starting Docker installation on server {}", server.id);

        // Detect OS type
        let os_type = self.detect_os(server).await?;
        if !os_type.supports_docker_install() {
            return Err(ActionError::unsupported_os(format!("{:?}", os_type)));
        }

        info!("Detected OS type: {:?}", os_type);

        // Build install commands
        let mut builder = CommandBuilder::new();

        builder.echo("Installing Docker Engine...");

        // Add OS-specific install command
        let install_cmd = match os_type {
            OsType::Debian | OsType::Ubuntu => self.get_debian_install_command(),
            OsType::Rhel | OsType::CentOS | OsType::Fedora => self.get_rhel_install_command(),
            OsType::Suse => self.get_suse_install_command(),
            OsType::Arch => self.get_arch_install_command(),
            _ => self.get_generic_install_command(),
        };
        builder.add(install_cmd);

        // Configure Docker daemon
        let config_base64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            Self::get_daemon_config()
        );

        builder.echo("Configuring Docker Engine...");
        builder.add(format!(
            "test -s /etc/docker/daemon.json && cp /etc/docker/daemon.json \"/etc/docker/daemon.json.original-$(date +\"%Y%m%d-%H%M%S\")\""
        ));
        builder.add(format!(
            "test ! -s /etc/docker/daemon.json && echo '{}' | base64 -d | tee /etc/docker/daemon.json > /dev/null",
            config_base64
        ));

        // Restart Docker and create network
        builder.echo("Restarting Docker Engine...");
        builder.add("systemctl enable docker >/dev/null 2>&1 || true");
        builder.add("systemctl restart docker");
        builder.add("docker network create --attachable coolify >/dev/null 2>&1 || true");
        builder.echo("Docker installation complete!");

        let commands = builder.build();

        // Execute commands
        let session = self.ssh.connect(server)
            .await
            .map_err(|e| ActionError::ssh_connection_failed(e.to_string()))?;

        for cmd in &commands {
            if cmd.starts_with("echo '") {
                // Log echo commands
                info!("{}", cmd.trim_start_matches("echo '").trim_end_matches("'"));
            }

            let result = session.execute(cmd)
                .await
                .map_err(|e| ActionError::command_failed(cmd, e.to_string()))?;

            if result.exit_code != 0 && !cmd.contains("|| true") && !cmd.contains("2>/dev/null") {
                warn!("Command failed with exit code {}: {}", result.exit_code, cmd);
            }
        }

        // Verify installation
        let version_output = session.execute("docker --version")
            .await
            .map_err(|e| ActionError::command_failed("docker --version", e.to_string()))?;

        let docker_version = version_output.stdout.trim().to_string();
        info!("Docker installed: {}", docker_version);

        Ok(ActionResult::success(format!("Docker installed: {}", docker_version))
            .with_duration(start.elapsed())
            .with_commands(commands)
            .with_output(serde_json::json!({
                "docker_version": docker_version,
                "os_type": format!("{:?}", os_type),
            })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_os_detection() {
        assert_eq!(OsType::from_os_release("ID=ubuntu"), OsType::Ubuntu);
        assert_eq!(OsType::from_os_release("ID=debian"), OsType::Debian);
        assert_eq!(OsType::from_os_release("ID=centos"), OsType::CentOS);
        assert_eq!(OsType::from_os_release("ID=arch"), OsType::Arch);
        assert_eq!(OsType::from_os_release("ID=unknown123"), OsType::Unknown);
    }

    #[test]
    fn test_package_manager() {
        assert_eq!(OsType::Ubuntu.package_manager(), "apt-get");
        assert_eq!(OsType::CentOS.package_manager(), "dnf");
        assert_eq!(OsType::Arch.package_manager(), "pacman");
    }
}
