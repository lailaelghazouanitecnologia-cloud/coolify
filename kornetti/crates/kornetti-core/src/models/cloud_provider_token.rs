//! Cloud Provider Token Model
//!
//! API tokens for cloud providers like Hetzner, DigitalOcean, etc.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Cloud provider types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CloudProvider {
    Hetzner,
    DigitalOcean,
    Linode,
    Vultr,
    Aws,
    Gcp,
    Azure,
}

impl CloudProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            CloudProvider::Hetzner => "hetzner",
            CloudProvider::DigitalOcean => "digitalocean",
            CloudProvider::Linode => "linode",
            CloudProvider::Vultr => "vultr",
            CloudProvider::Aws => "aws",
            CloudProvider::Gcp => "gcp",
            CloudProvider::Azure => "azure",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "hetzner" => Some(CloudProvider::Hetzner),
            "digitalocean" => Some(CloudProvider::DigitalOcean),
            "linode" => Some(CloudProvider::Linode),
            "vultr" => Some(CloudProvider::Vultr),
            "aws" => Some(CloudProvider::Aws),
            "gcp" => Some(CloudProvider::Gcp),
            "azure" => Some(CloudProvider::Azure),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            CloudProvider::Hetzner => "Hetzner Cloud",
            CloudProvider::DigitalOcean => "DigitalOcean",
            CloudProvider::Linode => "Linode",
            CloudProvider::Vultr => "Vultr",
            CloudProvider::Aws => "Amazon Web Services",
            CloudProvider::Gcp => "Google Cloud Platform",
            CloudProvider::Azure => "Microsoft Azure",
        }
    }

    pub fn api_base_url(&self) -> Option<&'static str> {
        match self {
            CloudProvider::Hetzner => Some("https://api.hetzner.cloud/v1"),
            CloudProvider::DigitalOcean => Some("https://api.digitalocean.com/v2"),
            CloudProvider::Linode => Some("https://api.linode.com/v4"),
            CloudProvider::Vultr => Some("https://api.vultr.com/v2"),
            CloudProvider::Aws => None, // Uses regional endpoints
            CloudProvider::Gcp => None, // Uses service-specific endpoints
            CloudProvider::Azure => None, // Uses resource-specific endpoints
        }
    }
}

/// Cloud provider API token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudProviderToken {
    pub id: Uuid,
    /// Token name/label
    pub name: String,
    /// Provider type
    pub provider: CloudProvider,
    /// API token (encrypted)
    pub token: String,
    /// Associated team ID
    pub team_id: Uuid,
    /// Whether the token is valid
    pub is_valid: bool,
    /// Last validation time
    pub last_validated_at: Option<DateTime<Utc>>,
    /// Validation error message
    pub validation_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CloudProviderToken {
    pub fn new(name: String, provider: CloudProvider, token: String, team_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            provider,
            token,
            team_id,
            is_valid: false,
            last_validated_at: None,
            validation_error: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Mark the token as valid
    pub fn mark_valid(&mut self) {
        self.is_valid = true;
        self.last_validated_at = Some(Utc::now());
        self.validation_error = None;
        self.updated_at = Utc::now();
    }

    /// Mark the token as invalid with an error
    pub fn mark_invalid(&mut self, error: String) {
        self.is_valid = false;
        self.last_validated_at = Some(Utc::now());
        self.validation_error = Some(error);
        self.updated_at = Utc::now();
    }

    /// Get authorization header value
    pub fn auth_header(&self) -> String {
        match self.provider {
            CloudProvider::Hetzner => format!("Bearer {}", self.token),
            CloudProvider::DigitalOcean => format!("Bearer {}", self.token),
            CloudProvider::Linode => format!("Bearer {}", self.token),
            CloudProvider::Vultr => format!("Bearer {}", self.token),
            CloudProvider::Aws => self.token.clone(), // AWS uses different auth
            CloudProvider::Gcp => format!("Bearer {}", self.token),
            CloudProvider::Azure => format!("Bearer {}", self.token),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloud_provider_from_str() {
        assert_eq!(CloudProvider::from_str("hetzner"), Some(CloudProvider::Hetzner));
        assert_eq!(CloudProvider::from_str("DIGITALOCEAN"), Some(CloudProvider::DigitalOcean));
        assert_eq!(CloudProvider::from_str("invalid"), None);
    }

    #[test]
    fn test_cloud_provider_token_new() {
        let team_id = Uuid::new_v4();
        let token = CloudProviderToken::new(
            "My Token".to_string(),
            CloudProvider::Hetzner,
            "secret-token".to_string(),
            team_id,
        );

        assert_eq!(token.name, "My Token");
        assert_eq!(token.provider, CloudProvider::Hetzner);
        assert!(!token.is_valid);
    }

    #[test]
    fn test_mark_valid_invalid() {
        let team_id = Uuid::new_v4();
        let mut token = CloudProviderToken::new(
            "Test".to_string(),
            CloudProvider::Hetzner,
            "token".to_string(),
            team_id,
        );

        token.mark_valid();
        assert!(token.is_valid);
        assert!(token.last_validated_at.is_some());

        token.mark_invalid("Invalid token".to_string());
        assert!(!token.is_valid);
        assert_eq!(token.validation_error, Some("Invalid token".to_string()));
    }
}
