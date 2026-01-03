//! Service Component models
//!
//! Models for Docker Compose service components.
//! Mirrors Coolify's ServiceDatabase and ServiceApplication models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Database component within a Docker Compose service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDatabase {
    pub id: Uuid,
    pub service_id: Uuid,
    /// Name of the database container
    pub name: String,
    /// Human-readable description
    pub description: Option<String>,
    /// Docker image to use
    pub image: String,
    /// Exclude from service proxy
    pub exclude_from_status: bool,
    /// Public port mapping (if exposed)
    pub public_port: Option<u16>,
    /// Is the database publicly accessible
    pub is_public: bool,
    /// Current container status
    pub status: ComponentStatus,
    /// Resource limits
    pub limits_memory: Option<String>,
    pub limits_cpus: Option<String>,
    /// Healthcheck configuration
    pub healthcheck_enabled: bool,
    pub healthcheck_command: Option<String>,
    pub healthcheck_interval: Option<u32>,
    pub healthcheck_timeout: Option<u32>,
    pub healthcheck_retries: Option<u32>,
    pub healthcheck_start_period: Option<u32>,
    /// File storage configuration from compose
    pub file_storages: Vec<FileStorage>,
    /// Volumes from compose
    pub volumes: Vec<VolumeMount>,
    /// Environment variables
    pub environment_variables: Vec<EnvVariable>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Application component within a Docker Compose service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceApplication {
    pub id: Uuid,
    pub service_id: Uuid,
    /// Name of the application container
    pub name: String,
    /// Human-readable description
    pub description: Option<String>,
    /// FQDN for the application (if web-accessible)
    pub fqdn: Option<String>,
    /// Docker image to use
    pub image: String,
    /// Exclude from service proxy
    pub exclude_from_status: bool,
    /// Required FQDN for routing
    pub required_fqdn: bool,
    /// Current container status
    pub status: ComponentStatus,
    /// Is this a database component? (for backward compat)
    pub is_database: bool,
    /// Connect to predefined network
    pub connect_to_docker_network: bool,
    /// Resource limits
    pub limits_memory: Option<String>,
    pub limits_cpus: Option<String>,
    /// Healthcheck configuration
    pub healthcheck_enabled: bool,
    pub healthcheck_command: Option<String>,
    pub healthcheck_interval: Option<u32>,
    pub healthcheck_timeout: Option<u32>,
    pub healthcheck_retries: Option<u32>,
    pub healthcheck_start_period: Option<u32>,
    /// File storage configuration from compose
    pub file_storages: Vec<FileStorage>,
    /// Volumes from compose
    pub volumes: Vec<VolumeMount>,
    /// Environment variables
    pub environment_variables: Vec<EnvVariable>,
    /// Port mappings
    pub ports: Vec<PortMapping>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Component status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComponentStatus {
    Running,
    Stopped,
    Starting,
    Stopping,
    Restarting,
    Error,
    Unknown,
}

impl Default for ComponentStatus {
    fn default() -> Self {
        ComponentStatus::Unknown
    }
}

/// File storage from Docker Compose
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStorage {
    pub id: Uuid,
    /// Path on the host filesystem
    pub fs_path: String,
    /// Path inside the container
    pub mount_path: String,
    /// File content (for config files)
    pub content: Option<String>,
    /// Is this a directory?
    pub is_directory: bool,
}

/// Volume mount configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    pub id: Uuid,
    /// Volume name (for named volumes) or host path
    pub name: String,
    /// Path inside the container
    pub mount_path: String,
    /// Read-only mount
    pub read_only: bool,
}

/// Environment variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVariable {
    pub id: Uuid,
    pub key: String,
    pub value: String,
    /// Whether to show in build logs
    pub is_build_time: bool,
    /// Whether value is a secret
    pub is_secret: bool,
    /// Real value (for UI display, might be different)
    pub real_value: Option<String>,
}

/// Port mapping for service applications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    pub id: Uuid,
    /// Host port
    pub host: u16,
    /// Container port
    pub container: u16,
    /// Protocol
    pub protocol: PortProtocol,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PortProtocol {
    Tcp,
    Udp,
}

impl Default for PortProtocol {
    fn default() -> Self {
        PortProtocol::Tcp
    }
}

impl ServiceDatabase {
    pub fn new(service_id: Uuid, name: String, image: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            service_id,
            name,
            description: None,
            image,
            exclude_from_status: false,
            public_port: None,
            is_public: false,
            status: ComponentStatus::Unknown,
            limits_memory: None,
            limits_cpus: None,
            healthcheck_enabled: true,
            healthcheck_command: None,
            healthcheck_interval: None,
            healthcheck_timeout: None,
            healthcheck_retries: None,
            healthcheck_start_period: None,
            file_storages: Vec::new(),
            volumes: Vec::new(),
            environment_variables: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Get the container name for this database
    pub fn container_name(&self, service_uuid: &str) -> String {
        format!("{}-{}", service_uuid, self.name)
    }
}

impl ServiceApplication {
    pub fn new(service_id: Uuid, name: String, image: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            service_id,
            name,
            description: None,
            fqdn: None,
            image,
            exclude_from_status: false,
            required_fqdn: false,
            status: ComponentStatus::Unknown,
            is_database: false,
            connect_to_docker_network: true,
            limits_memory: None,
            limits_cpus: None,
            healthcheck_enabled: true,
            healthcheck_command: None,
            healthcheck_interval: None,
            healthcheck_timeout: None,
            healthcheck_retries: None,
            healthcheck_start_period: None,
            file_storages: Vec::new(),
            volumes: Vec::new(),
            environment_variables: Vec::new(),
            ports: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Get the container name for this application
    pub fn container_name(&self, service_uuid: &str) -> String {
        format!("{}-{}", service_uuid, self.name)
    }

    /// Parse FQDN to get domains
    pub fn domains(&self) -> Vec<String> {
        self.fqdn
            .as_ref()
            .map(|f| f.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default()
    }
}

/// Cloud provider token for server provisioning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudProviderToken {
    pub id: Uuid,
    pub team_id: Uuid,
    pub name: String,
    pub provider: CloudProviderType,
    /// Encrypted API token
    pub token: String,
    /// Whether token has been validated
    pub is_valid: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CloudProviderType {
    Hetzner,
    Vultr,
    DigitalOcean,
    Aws,
    Linode,
    Gcp,
    Azure,
}

impl CloudProviderToken {
    pub fn new(team_id: Uuid, name: String, provider: CloudProviderType, token: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            team_id,
            name,
            provider,
            token,
            is_valid: false,
            created_at: now,
            updated_at: now,
        }
    }
}

/// OAuth settings for authentication providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OauthSetting {
    pub id: Uuid,
    pub provider: OauthProvider,
    pub enabled: bool,
    pub client_id: String,
    /// Encrypted client secret
    pub client_secret: String,
    /// Custom redirect URI (optional)
    pub redirect_uri: Option<String>,
    /// Additional scopes
    pub scopes: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OauthProvider {
    Github,
    Gitlab,
    Google,
    Azure,
    Bitbucket,
}

impl OauthSetting {
    pub fn new(provider: OauthProvider, client_id: String, client_secret: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            provider,
            enabled: true,
            client_id,
            client_secret,
            redirect_uri: None,
            scopes: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Get the authorization URL for this provider
    pub fn auth_url(&self, state: &str) -> String {
        match self.provider {
            OauthProvider::Github => {
                format!(
                    "https://github.com/login/oauth/authorize?client_id={}&state={}&scope={}",
                    self.client_id,
                    state,
                    self.scopes.join(" ")
                )
            }
            OauthProvider::Gitlab => {
                format!(
                    "https://gitlab.com/oauth/authorize?client_id={}&state={}&redirect_uri={}&response_type=code&scope={}",
                    self.client_id,
                    state,
                    self.redirect_uri.as_deref().unwrap_or(""),
                    self.scopes.join(" ")
                )
            }
            OauthProvider::Google => {
                format!(
                    "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&state={}&redirect_uri={}&response_type=code&scope={}",
                    self.client_id,
                    state,
                    self.redirect_uri.as_deref().unwrap_or(""),
                    self.scopes.join(" ")
                )
            }
            _ => String::new(),
        }
    }
}
