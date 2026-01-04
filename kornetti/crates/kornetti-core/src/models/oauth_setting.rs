//! OAuth Setting Model
//!
//! OAuth provider configuration for authentication.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// OAuth provider types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum OauthProvider {
    Github,
    Gitlab,
    Google,
    Azure,
    Bitbucket,
    Authentik,
    Clerk,
}

impl OauthProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            OauthProvider::Github => "github",
            OauthProvider::Gitlab => "gitlab",
            OauthProvider::Google => "google",
            OauthProvider::Azure => "azure",
            OauthProvider::Bitbucket => "bitbucket",
            OauthProvider::Authentik => "authentik",
            OauthProvider::Clerk => "clerk",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "github" => Some(OauthProvider::Github),
            "gitlab" => Some(OauthProvider::Gitlab),
            "google" => Some(OauthProvider::Google),
            "azure" => Some(OauthProvider::Azure),
            "bitbucket" => Some(OauthProvider::Bitbucket),
            "authentik" => Some(OauthProvider::Authentik),
            "clerk" => Some(OauthProvider::Clerk),
            _ => None,
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            OauthProvider::Github,
            OauthProvider::Gitlab,
            OauthProvider::Google,
            OauthProvider::Azure,
            OauthProvider::Bitbucket,
            OauthProvider::Authentik,
            OauthProvider::Clerk,
        ]
    }
}

/// OAuth provider settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OauthSetting {
    pub id: Uuid,
    /// Provider type
    pub provider: OauthProvider,
    /// Whether this provider is enabled
    pub enabled: bool,
    /// OAuth Client ID
    pub client_id: Option<String>,
    /// OAuth Client Secret (encrypted)
    pub client_secret: Option<String>,
    /// Custom OAuth authorization URL (for self-hosted providers)
    pub authorize_url: Option<String>,
    /// Custom OAuth token URL
    pub token_url: Option<String>,
    /// Custom OAuth userinfo URL
    pub userinfo_url: Option<String>,
    /// OAuth scopes
    pub scopes: Option<String>,
    /// Whether to allow registration via this provider
    pub allow_registration: bool,
    /// Redirect URL
    pub redirect_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl OauthSetting {
    pub fn new(provider: OauthProvider) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            provider,
            enabled: false,
            client_id: None,
            client_secret: None,
            authorize_url: None,
            token_url: None,
            userinfo_url: None,
            scopes: Self::default_scopes(&provider),
            allow_registration: true,
            redirect_url: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Get default scopes for a provider
    pub fn default_scopes(provider: &OauthProvider) -> Option<String> {
        match provider {
            OauthProvider::Github => Some("read:user user:email".to_string()),
            OauthProvider::Gitlab => Some("read_user".to_string()),
            OauthProvider::Google => Some("openid email profile".to_string()),
            OauthProvider::Azure => Some("openid email profile".to_string()),
            OauthProvider::Bitbucket => Some("account email".to_string()),
            OauthProvider::Authentik => Some("openid email profile".to_string()),
            OauthProvider::Clerk => Some("openid email profile".to_string()),
        }
    }

    /// Check if the provider is properly configured
    pub fn is_configured(&self) -> bool {
        self.enabled && self.client_id.is_some() && self.client_secret.is_some()
    }

    /// Get the authorization URL for this provider
    pub fn get_authorize_url(&self) -> Option<String> {
        if self.authorize_url.is_some() {
            return self.authorize_url.clone();
        }

        match self.provider {
            OauthProvider::Github => Some("https://github.com/login/oauth/authorize".to_string()),
            OauthProvider::Gitlab => Some("https://gitlab.com/oauth/authorize".to_string()),
            OauthProvider::Google => Some("https://accounts.google.com/o/oauth2/v2/auth".to_string()),
            OauthProvider::Azure => None, // Requires tenant ID
            OauthProvider::Bitbucket => Some("https://bitbucket.org/site/oauth2/authorize".to_string()),
            OauthProvider::Authentik => None, // Self-hosted, requires custom URL
            OauthProvider::Clerk => None, // Requires frontend domain
        }
    }

    /// Get the token URL for this provider
    pub fn get_token_url(&self) -> Option<String> {
        if self.token_url.is_some() {
            return self.token_url.clone();
        }

        match self.provider {
            OauthProvider::Github => Some("https://github.com/login/oauth/access_token".to_string()),
            OauthProvider::Gitlab => Some("https://gitlab.com/oauth/token".to_string()),
            OauthProvider::Google => Some("https://oauth2.googleapis.com/token".to_string()),
            OauthProvider::Azure => None,
            OauthProvider::Bitbucket => Some("https://bitbucket.org/site/oauth2/access_token".to_string()),
            OauthProvider::Authentik => None,
            OauthProvider::Clerk => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_provider_from_str() {
        assert_eq!(OauthProvider::from_str("github"), Some(OauthProvider::Github));
        assert_eq!(OauthProvider::from_str("GOOGLE"), Some(OauthProvider::Google));
        assert_eq!(OauthProvider::from_str("invalid"), None);
    }

    #[test]
    fn test_oauth_setting_new() {
        let setting = OauthSetting::new(OauthProvider::Github);

        assert_eq!(setting.provider, OauthProvider::Github);
        assert!(!setting.enabled);
        assert!(setting.scopes.is_some());
    }

    #[test]
    fn test_is_configured() {
        let mut setting = OauthSetting::new(OauthProvider::Github);

        assert!(!setting.is_configured());

        setting.enabled = true;
        setting.client_id = Some("client-id".to_string());
        setting.client_secret = Some("client-secret".to_string());

        assert!(setting.is_configured());
    }
}
