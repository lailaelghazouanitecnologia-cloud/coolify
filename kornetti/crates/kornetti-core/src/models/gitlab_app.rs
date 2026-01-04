//! GitLab App Model
//!
//! GitLab application configuration for repository access.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// GitLab application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitlabApp {
    pub id: Uuid,
    /// App name/label
    pub name: String,
    /// Application ID
    pub app_id: String,
    /// Application Secret (encrypted)
    pub app_secret: String,
    /// Personal Access Token (encrypted, alternative to OAuth)
    pub personal_access_token: Option<String>,
    /// GitLab instance URL
    pub gitlab_url: String,
    /// Webhook token (encrypted)
    pub webhook_token: Option<String>,
    /// Whether this is self-hosted GitLab
    pub is_self_hosted: bool,
    /// OAuth access token (cached)
    pub access_token: Option<String>,
    /// OAuth refresh token
    pub refresh_token: Option<String>,
    /// Token expiration time
    pub token_expires_at: Option<DateTime<Utc>>,
    /// Owner team
    pub team_id: Uuid,
    /// Whether properly configured
    pub is_configured: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl GitlabApp {
    pub fn new(name: String, app_id: String, app_secret: String, team_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            app_id,
            app_secret,
            personal_access_token: None,
            gitlab_url: "https://gitlab.com".to_string(),
            webhook_token: None,
            is_self_hosted: false,
            access_token: None,
            refresh_token: None,
            token_expires_at: None,
            team_id,
            is_configured: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create for self-hosted GitLab
    pub fn self_hosted(
        name: String,
        app_id: String,
        app_secret: String,
        gitlab_url: String,
        team_id: Uuid,
    ) -> Self {
        let mut app = Self::new(name, app_id, app_secret, team_id);
        app.gitlab_url = gitlab_url;
        app.is_self_hosted = true;
        app
    }

    /// Create using Personal Access Token
    pub fn with_pat(name: String, pat: String, team_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            app_id: String::new(),
            app_secret: String::new(),
            personal_access_token: Some(pat),
            gitlab_url: "https://gitlab.com".to_string(),
            webhook_token: None,
            is_self_hosted: false,
            access_token: None,
            refresh_token: None,
            token_expires_at: None,
            team_id,
            is_configured: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Get API base URL
    pub fn api_url(&self) -> String {
        format!("{}/api/v4", self.gitlab_url.trim_end_matches('/'))
    }

    /// Check if using PAT authentication
    pub fn uses_pat(&self) -> bool {
        self.personal_access_token.is_some()
    }

    /// Get authorization header
    pub fn auth_header(&self) -> Option<String> {
        if let Some(pat) = &self.personal_access_token {
            return Some(format!("Bearer {}", pat));
        }
        if let Some(token) = &self.access_token {
            return Some(format!("Bearer {}", token));
        }
        None
    }

    /// Check if access token is valid
    pub fn has_valid_token(&self) -> bool {
        if self.uses_pat() {
            return true;
        }
        match (&self.access_token, &self.token_expires_at) {
            (Some(_), Some(expires)) => expires > &Utc::now(),
            _ => false,
        }
    }

    /// Update OAuth tokens
    pub fn update_tokens(
        &mut self,
        access_token: String,
        refresh_token: Option<String>,
        expires_at: DateTime<Utc>,
    ) {
        self.access_token = Some(access_token);
        self.refresh_token = refresh_token;
        self.token_expires_at = Some(expires_at);
        self.updated_at = Utc::now();
    }
}

/// GitLab project information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitlabProject {
    pub id: i64,
    pub name: String,
    pub path_with_namespace: String,
    pub web_url: String,
    pub http_url_to_repo: String,
    pub ssh_url_to_repo: String,
    pub default_branch: Option<String>,
    pub visibility: String,
    pub namespace: GitlabNamespace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitlabNamespace {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub kind: String,
}

/// GitLab branch information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitlabBranch {
    pub name: String,
    pub protected: bool,
    pub default: bool,
    pub commit: GitlabCommit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitlabCommit {
    pub id: String,
    pub short_id: String,
    pub message: String,
    pub author_name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gitlab_app_new() {
        let team_id = Uuid::new_v4();
        let app = GitlabApp::new(
            "My GitLab".to_string(),
            "app-id".to_string(),
            "app-secret".to_string(),
            team_id,
        );

        assert_eq!(app.name, "My GitLab");
        assert_eq!(app.gitlab_url, "https://gitlab.com");
        assert!(!app.is_self_hosted);
    }

    #[test]
    fn test_self_hosted() {
        let team_id = Uuid::new_v4();
        let app = GitlabApp::self_hosted(
            "Self Hosted".to_string(),
            "id".to_string(),
            "secret".to_string(),
            "https://gitlab.mycompany.com".to_string(),
            team_id,
        );

        assert!(app.is_self_hosted);
        assert_eq!(app.gitlab_url, "https://gitlab.mycompany.com");
    }

    #[test]
    fn test_api_url() {
        let team_id = Uuid::new_v4();
        let app = GitlabApp::new("Test".to_string(), "id".to_string(), "secret".to_string(), team_id);

        assert_eq!(app.api_url(), "https://gitlab.com/api/v4");
    }

    #[test]
    fn test_with_pat() {
        let team_id = Uuid::new_v4();
        let app = GitlabApp::with_pat("Test".to_string(), "glpat-xxxx".to_string(), team_id);

        assert!(app.uses_pat());
        assert!(app.is_configured);
        assert!(app.has_valid_token());
    }
}
