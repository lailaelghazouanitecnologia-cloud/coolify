//! Server Actions
//!
//! Actions for managing servers, including Docker installation,
//! validation, cleanup, and command execution.

mod install_docker;
mod validate_server;
mod validate_prerequisites;
mod install_prerequisites;
mod cleanup_docker;
mod run_command;
mod check_updates;

pub use install_docker::InstallDocker;
pub use validate_server::ValidateServer;
pub use validate_prerequisites::ValidatePrerequisites;
pub use install_prerequisites::InstallPrerequisites;
pub use cleanup_docker::CleanupDocker;
pub use run_command::RunCommand;
pub use check_updates::CheckUpdates;

use serde::{Deserialize, Serialize};

/// Detected operating system type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OsType {
    Debian,
    Ubuntu,
    Rhel,
    CentOS,
    Fedora,
    Alpine,
    Suse,
    Arch,
    Unknown,
}

impl OsType {
    /// Parse from /etc/os-release content
    pub fn from_os_release(content: &str) -> Self {
        let content_lower = content.to_lowercase();

        if content_lower.contains("ubuntu") {
            OsType::Ubuntu
        } else if content_lower.contains("debian") {
            OsType::Debian
        } else if content_lower.contains("centos") {
            OsType::CentOS
        } else if content_lower.contains("fedora") {
            OsType::Fedora
        } else if content_lower.contains("rhel") || content_lower.contains("red hat") {
            OsType::Rhel
        } else if content_lower.contains("alpine") {
            OsType::Alpine
        } else if content_lower.contains("suse") || content_lower.contains("sles") {
            OsType::Suse
        } else if content_lower.contains("arch") {
            OsType::Arch
        } else {
            OsType::Unknown
        }
    }

    /// Check if this OS type supports automated Docker installation
    pub fn supports_docker_install(&self) -> bool {
        !matches!(self, OsType::Unknown)
    }

    /// Get the package manager for this OS
    pub fn package_manager(&self) -> &'static str {
        match self {
            OsType::Debian | OsType::Ubuntu => "apt-get",
            OsType::Rhel | OsType::CentOS | OsType::Fedora => "dnf",
            OsType::Alpine => "apk",
            OsType::Suse => "zypper",
            OsType::Arch => "pacman",
            OsType::Unknown => "unknown",
        }
    }

    /// Get the OS family (debian, rhel, etc.)
    pub fn family(&self) -> &'static str {
        match self {
            OsType::Debian | OsType::Ubuntu => "debian",
            OsType::Rhel | OsType::CentOS | OsType::Fedora => "rhel",
            OsType::Alpine => "alpine",
            OsType::Suse => "sles",
            OsType::Arch => "arch",
            OsType::Unknown => "unknown",
        }
    }
}

/// Server validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerValidation {
    pub is_valid: bool,
    pub os_type: OsType,
    pub os_version: Option<String>,
    pub docker_installed: bool,
    pub docker_version: Option<String>,
    pub docker_running: bool,
    pub docker_compose_installed: bool,
    pub has_root_access: bool,
    pub disk_space_gb: Option<f64>,
    pub memory_gb: Option<f64>,
    pub cpu_cores: Option<u32>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl Default for ServerValidation {
    fn default() -> Self {
        Self {
            is_valid: false,
            os_type: OsType::Unknown,
            os_version: None,
            docker_installed: false,
            docker_version: None,
            docker_running: false,
            docker_compose_installed: false,
            has_root_access: false,
            disk_space_gb: None,
            memory_gb: None,
            cpu_cores: None,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

/// Docker version requirements
pub const DOCKER_MINIMUM_VERSION: &str = "24.0";
pub const DOCKER_COMPOSE_MINIMUM_VERSION: &str = "2.0";
