//! GitHub App Model
//!
//! GitHub App configuration for repository access and webhooks.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// GitHub App configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubApp {
    pub id: Uuid,
    /// App name/label
    pub name: String,
    /// GitHub App ID
    pub app_id: i64,
    /// Installation ID (set after installation)
    pub installation_id: Option<i64>,
    /// GitHub App client ID
    pub client_id: String,
    /// GitHub App client secret (encrypted)
    pub client_secret: String,
    /// GitHub App private key (PEM format, encrypted)
    pub private_key: String,
    /// Webhook secret (encrypted)
    pub webhook_secret: Option<String>,
    /// Custom HTML URL (for GitHub Enterprise)
    pub html_url: Option<String>,
    /// Custom API URL (for GitHub Enterprise)
    pub api_url: Option<String>,
    /// Whether this is for GitHub Enterprise
    pub is_enterprise: bool,
    /// Organization filter (limit to specific orgs)
    pub organization_filter: Option<String>,
    /// Owner of this GitHub App config
    pub team_id: Uuid,
    /// Whether the app is properly installed
    pub is_installed: bool,
    /// Installation access token (cached)
    pub access_token: Option<String>,
    /// Token expiration time
    pub token_expires_at: Option<DateTime<Utc>>,
    /// Permissions configured on the app
    pub permissions: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl GithubApp {
    pub fn new(
        name: String,
        app_id: i64,
        client_id: String,
        client_secret: String,
        private_key: String,
        team_id: Uuid,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            app_id,
            installation_id: None,
            client_id,
            client_secret,
            private_key,
            webhook_secret: None,
            html_url: None,
            api_url: None,
            is_enterprise: false,
            organization_filter: None,
            team_id,
            is_installed: false,
            access_token: None,
            token_expires_at: None,
            permissions: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Get the HTML URL (default or custom)
    pub fn get_html_url(&self) -> &str {
        self.html_url.as_deref().unwrap_or("https://github.com")
    }

    /// Get the API URL (default or custom)
    pub fn get_api_url(&self) -> &str {
        self.api_url.as_deref().unwrap_or("https://api.github.com")
    }

    /// Check if the access token is valid (not expired)
    pub fn has_valid_token(&self) -> bool {
        match (&self.access_token, &self.token_expires_at) {
            (Some(_), Some(expires)) => expires > &Utc::now(),
            _ => false,
        }
    }

    /// Get the webhook URL for this app
    pub fn webhook_url(&self, base_url: &str) -> String {
        format!("{}/source/github/events/{}", base_url, self.app_id)
    }

    /// Get the installation URL
    pub fn installation_url(&self) -> String {
        format!("{}/apps/{}", self.get_html_url(), self.name.to_lowercase().replace(' ', "-"))
    }

    /// Set installation
    pub fn set_installation(&mut self, installation_id: i64) {
        self.installation_id = Some(installation_id);
        self.is_installed = true;
        self.updated_at = Utc::now();
    }

    /// Update access token
    pub fn update_token(&mut self, token: String, expires_at: DateTime<Utc>) {
        self.access_token = Some(token);
        self.token_expires_at = Some(expires_at);
        self.updated_at = Utc::now();
    }
}

/// GitHub repository information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubRepository {
    pub id: i64,
    pub name: String,
    pub full_name: String,
    pub html_url: String,
    pub clone_url: String,
    pub default_branch: String,
    pub private: bool,
    pub owner: GithubOwner,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubOwner {
    pub login: String,
    pub avatar_url: String,
    #[serde(rename = "type")]
    pub owner_type: String,
}

/// GitHub branch information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubBranch {
    pub name: String,
    pub protected: bool,
    pub commit: GithubCommitRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubCommitRef {
    pub sha: String,
    pub url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_app_new() {
        let team_id = Uuid::new_v4();
        let app = GithubApp::new(
            "My App".to_string(),
            12345,
            "Iv1.abc".to_string(),
            "secret".to_string(),
            "-----BEGIN RSA PRIVATE KEY-----".to_string(),
            team_id,
        );

        assert_eq!(app.name, "My App");
        assert_eq!(app.app_id, 12345);
        assert!(!app.is_installed);
        assert!(!app.is_enterprise);
    }

    #[test]
    fn test_urls() {
        let team_id = Uuid::new_v4();
        let app = GithubApp::new(
            "Test App".to_string(),
            123,
            "client".to_string(),
            "secret".to_string(),
            "key".to_string(),
            team_id,
        );

        assert_eq!(app.get_html_url(), "https://github.com");
        assert_eq!(app.get_api_url(), "https://api.github.com");
    }

    #[test]
    fn test_set_installation() {
        let team_id = Uuid::new_v4();
        let mut app = GithubApp::new(
            "Test".to_string(),
            123,
            "client".to_string(),
            "secret".to_string(),
            "key".to_string(),
            team_id,
        );

        assert!(!app.is_installed);

        app.set_installation(456);

        assert!(app.is_installed);
        assert_eq!(app.installation_id, Some(456));
    }
}
