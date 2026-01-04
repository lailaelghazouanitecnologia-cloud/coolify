//! Instance Settings Model
//!
//! Global instance configuration settings.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Global instance settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceSettings {
    pub id: Uuid,
    /// Instance UUID
    pub uuid: String,
    /// Whether registration is enabled
    pub is_registration_enabled: bool,
    /// Whether auto-update is enabled
    pub is_auto_update_enabled: bool,
    /// Whether DNS validation is enabled
    pub is_dns_validation_enabled: bool,
    /// Public FQDN for the instance
    pub fqdn: Option<String>,
    /// Public IPv4 address
    pub public_ipv4: Option<String>,
    /// Public IPv6 address
    pub public_ipv6: Option<String>,
    /// Custom instance name
    pub custom_instance_name: Option<String>,
    /// Instance timezone
    pub instance_timezone: String,
    /// Resale license key
    pub resale_license: Option<String>,
    /// Is this a reseller instance
    pub is_resale_license_active: bool,
    /// Sentinel token for monitoring
    pub sentinel_token: Option<String>,
    /// Is Sentinel enabled
    pub is_sentinel_enabled: bool,
    /// Custom DNS servers
    pub custom_dns_servers: Option<String>,
    /// Dynamic configuration path
    pub dynamic_config_path: Option<String>,
    /// Whether to allow domain wildcards
    pub allow_domain_wildcards: bool,
    /// Default redirect URL
    pub default_redirect_url: Option<String>,
    /// Cleanup interval in hours
    pub cleanup_interval_hours: i32,
    /// Max concurrent builds
    pub max_concurrent_builds: i32,
    /// Generated Fqdn (for internal use)
    pub generated_fqdn: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for InstanceSettings {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            uuid: Uuid::new_v4().to_string(),
            is_registration_enabled: true,
            is_auto_update_enabled: false,
            is_dns_validation_enabled: false,
            fqdn: None,
            public_ipv4: None,
            public_ipv6: None,
            custom_instance_name: None,
            instance_timezone: "UTC".to_string(),
            resale_license: None,
            is_resale_license_active: false,
            sentinel_token: None,
            is_sentinel_enabled: false,
            custom_dns_servers: None,
            dynamic_config_path: None,
            allow_domain_wildcards: false,
            default_redirect_url: None,
            cleanup_interval_hours: 24,
            max_concurrent_builds: 4,
            generated_fqdn: None,
            created_at: now,
            updated_at: now,
        }
    }
}

impl InstanceSettings {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the FQDN or generated FQDN
    pub fn get_fqdn(&self) -> Option<&str> {
        self.fqdn.as_deref().or(self.generated_fqdn.as_deref())
    }

    /// Check if the instance is cloud (has resale license)
    pub fn is_cloud(&self) -> bool {
        self.is_resale_license_active && self.resale_license.is_some()
    }

    /// Get custom DNS servers as a list
    pub fn dns_servers(&self) -> Vec<&str> {
        self.custom_dns_servers
            .as_deref()
            .map(|s| s.split(',').map(|s| s.trim()).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instance_settings_default() {
        let settings = InstanceSettings::default();

        assert!(settings.is_registration_enabled);
        assert!(!settings.is_auto_update_enabled);
        assert_eq!(settings.instance_timezone, "UTC");
        assert_eq!(settings.max_concurrent_builds, 4);
    }

    #[test]
    fn test_get_fqdn() {
        let mut settings = InstanceSettings::default();

        assert!(settings.get_fqdn().is_none());

        settings.fqdn = Some("coolify.example.com".to_string());
        assert_eq!(settings.get_fqdn(), Some("coolify.example.com"));

        settings.fqdn = None;
        settings.generated_fqdn = Some("auto.example.com".to_string());
        assert_eq!(settings.get_fqdn(), Some("auto.example.com"));
    }

    #[test]
    fn test_is_cloud() {
        let mut settings = InstanceSettings::default();

        assert!(!settings.is_cloud());

        settings.is_resale_license_active = true;
        settings.resale_license = Some("license-key".to_string());
        assert!(settings.is_cloud());
    }
}
