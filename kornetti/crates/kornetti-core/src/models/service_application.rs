//! Service Application Model
//!
//! Applications deployed as part of a Docker Compose service.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Application deployed within a service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceApplication {
    pub id: Uuid,
    /// UUID for external references
    pub uuid: String,
    /// Application name (from compose)
    pub name: String,
    /// Human-readable description
    pub description: Option<String>,
    /// Parent service ID
    pub service_id: Uuid,
    /// Docker image
    pub image: String,
    /// Container command override
    pub command: Option<String>,
    /// Entrypoint override
    pub entrypoint: Option<String>,
    /// FQDN for external access
    pub fqdn: Option<String>,
    /// Port mappings
    pub ports: Option<String>,
    /// Internal port for proxy
    pub internal_port: Option<u16>,
    /// Whether to expose via proxy
    pub is_public: bool,
    /// Whether to force HTTPS
    pub https_only: bool,
    /// Whether to strip www prefix
    pub www_redirect: bool,
    /// Health check configuration
    pub health_check: Option<HealthCheckConfig>,
    /// Resource limits
    pub limits: Option<ResourceLimits>,
    /// Labels to add to container
    pub labels: Option<serde_json::Value>,
    /// Current status
    pub status: ServiceAppStatus,
    /// Exclude from status checks
    pub exclude_from_status: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ServiceAppStatus {
    Running,
    Stopped,
    Starting,
    Stopping,
    Restarting,
    Error,
    Unknown,
}

impl Default for ServiceAppStatus {
    fn default() -> Self {
        ServiceAppStatus::Unknown
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    pub enabled: bool,
    pub command: Option<String>,
    pub interval: Option<String>,
    pub timeout: Option<String>,
    pub retries: Option<u32>,
    pub start_period: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub memory: Option<String>,
    pub memory_reservation: Option<String>,
    pub memory_swap: Option<String>,
    pub cpus: Option<String>,
    pub cpu_shares: Option<i32>,
}

impl ServiceApplication {
    pub fn new(name: String, service_id: Uuid, image: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            uuid: Uuid::new_v4().to_string(),
            name,
            description: None,
            service_id,
            image,
            command: None,
            entrypoint: None,
            fqdn: None,
            ports: None,
            internal_port: None,
            is_public: false,
            https_only: true,
            www_redirect: true,
            health_check: None,
            limits: None,
            labels: None,
            status: ServiceAppStatus::Unknown,
            exclude_from_status: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Get container name for this application
    pub fn container_name(&self, service_uuid: &str) -> String {
        format!("{}-{}", service_uuid, self.name)
    }

    /// Parse port mappings
    pub fn port_mappings(&self) -> Vec<PortMapping> {
        self.ports
            .as_ref()
            .map(|p| {
                p.split(',')
                    .filter_map(|s| PortMapping::parse(s.trim()))
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone)]
pub struct PortMapping {
    pub host_port: Option<u16>,
    pub container_port: u16,
    pub protocol: String,
}

impl PortMapping {
    pub fn parse(s: &str) -> Option<Self> {
        // Parse formats like "80", "8080:80", "8080:80/tcp"
        let (port_part, protocol) = if let Some(slash) = s.rfind('/') {
            (&s[..slash], &s[slash + 1..])
        } else {
            (s, "tcp")
        };

        if let Some(colon) = port_part.find(':') {
            let host = port_part[..colon].parse().ok()?;
            let container = port_part[colon + 1..].parse().ok()?;
            Some(PortMapping {
                host_port: Some(host),
                container_port: container,
                protocol: protocol.to_string(),
            })
        } else {
            let container = port_part.parse().ok()?;
            Some(PortMapping {
                host_port: None,
                container_port: container,
                protocol: protocol.to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_application_new() {
        let service_id = Uuid::new_v4();
        let app = ServiceApplication::new(
            "web".to_string(),
            service_id,
            "nginx:latest".to_string(),
        );

        assert_eq!(app.name, "web");
        assert_eq!(app.image, "nginx:latest");
        assert!(!app.is_public);
    }

    #[test]
    fn test_port_mapping_parse() {
        let pm = PortMapping::parse("8080:80").unwrap();
        assert_eq!(pm.host_port, Some(8080));
        assert_eq!(pm.container_port, 80);

        let pm = PortMapping::parse("443/tcp").unwrap();
        assert_eq!(pm.host_port, None);
        assert_eq!(pm.container_port, 443);
        assert_eq!(pm.protocol, "tcp");
    }
}
