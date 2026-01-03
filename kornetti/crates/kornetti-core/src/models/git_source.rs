//! Git source models for GitHub/GitLab/Bitbucket integration

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// GitHub App installation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubApp {
    pub id: Uuid,
    pub team_id: Uuid,
    pub name: String,
    /// GitHub App ID
    pub app_id: i64,
    /// GitHub App Client ID
    pub client_id: String,
    /// GitHub App Client Secret (encrypted)
    pub client_secret: String,
    /// GitHub App Private Key (encrypted)
    pub private_key: String,
    /// GitHub App Webhook Secret (encrypted)
    pub webhook_secret: Option<String>,
    /// Installation ID for this team's repos
    pub installation_id: Option<i64>,
    /// Custom GitHub URL (for GitHub Enterprise)
    pub custom_url: Option<String>,
    /// Custom API URL (for GitHub Enterprise)
    pub api_url: Option<String>,
    /// Whether this app is public or private
    pub is_public: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl GithubApp {
    pub fn new(team_id: Uuid, name: String, app_id: i64, client_id: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            team_id,
            name,
            app_id,
            client_id,
            client_secret: String::new(),
            private_key: String::new(),
            webhook_secret: None,
            installation_id: None,
            custom_url: None,
            api_url: None,
            is_public: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Get the GitHub URL (or custom for Enterprise)
    pub fn html_url(&self) -> String {
        self.custom_url.clone().unwrap_or_else(|| "https://github.com".to_string())
    }

    /// Get the GitHub API URL
    pub fn api_endpoint(&self) -> String {
        self.api_url.clone().unwrap_or_else(|| "https://api.github.com".to_string())
    }

    /// Check if this is GitHub Enterprise
    pub fn is_enterprise(&self) -> bool {
        self.custom_url.is_some()
    }
}

/// GitLab App/OAuth configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitlabApp {
    pub id: Uuid,
    pub team_id: Uuid,
    pub name: String,
    /// OAuth Application ID
    pub app_id: String,
    /// OAuth Application Secret (encrypted)
    pub app_secret: String,
    /// Deploy Key ID for accessing repos
    pub deploy_key_id: Option<i64>,
    /// Personal access token (encrypted)
    pub access_token: Option<String>,
    /// Refresh token (encrypted)
    pub refresh_token: Option<String>,
    /// Token expiry
    pub token_expires_at: Option<DateTime<Utc>>,
    /// Custom GitLab URL (for self-hosted)
    pub custom_url: Option<String>,
    /// Group ID for group-level access
    pub group_id: Option<i64>,
    /// Whether OAuth is public
    pub is_public: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl GitlabApp {
    pub fn new(team_id: Uuid, name: String, app_id: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            team_id,
            name,
            app_id,
            app_secret: String::new(),
            deploy_key_id: None,
            access_token: None,
            refresh_token: None,
            token_expires_at: None,
            custom_url: None,
            group_id: None,
            is_public: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Get the GitLab URL
    pub fn html_url(&self) -> String {
        self.custom_url.clone().unwrap_or_else(|| "https://gitlab.com".to_string())
    }

    /// Get the GitLab API URL
    pub fn api_endpoint(&self) -> String {
        format!("{}/api/v4", self.html_url())
    }

    /// Check if token needs refresh
    pub fn token_expired(&self) -> bool {
        self.token_expires_at
            .map(|exp| exp < Utc::now())
            .unwrap_or(true)
    }
}

/// Git repository reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitRepository {
    pub provider: GitProvider,
    pub owner: String,
    pub name: String,
    pub branch: String,
    pub commit_sha: Option<String>,
    /// Full clone URL
    pub clone_url: String,
    /// Whether to use deploy key or OAuth
    pub use_deploy_key: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GitProvider {
    Github,
    Gitlab,
    Bitbucket,
    Gitea,
    Custom,
}

impl GitRepository {
    pub fn github(owner: String, name: String, branch: String) -> Self {
        Self {
            provider: GitProvider::Github,
            clone_url: format!("https://github.com/{}/{}.git", owner, name),
            owner,
            name,
            branch,
            commit_sha: None,
            use_deploy_key: false,
        }
    }

    pub fn gitlab(owner: String, name: String, branch: String) -> Self {
        Self {
            provider: GitProvider::Gitlab,
            clone_url: format!("https://gitlab.com/{}/{}.git", owner, name),
            owner,
            name,
            branch,
            commit_sha: None,
            use_deploy_key: false,
        }
    }

    /// Get the web URL for the repository
    pub fn html_url(&self) -> String {
        match self.provider {
            GitProvider::Github => format!("https://github.com/{}/{}", self.owner, self.name),
            GitProvider::Gitlab => format!("https://gitlab.com/{}/{}", self.owner, self.name),
            GitProvider::Bitbucket => format!("https://bitbucket.org/{}/{}", self.owner, self.name),
            _ => self.clone_url.replace(".git", ""),
        }
    }

    /// Get the commit URL
    pub fn commit_url(&self) -> Option<String> {
        self.commit_sha.as_ref().map(|sha| {
            match self.provider {
                GitProvider::Github => format!("{}/commit/{}", self.html_url(), sha),
                GitProvider::Gitlab => format!("{}/-/commit/{}", self.html_url(), sha),
                GitProvider::Bitbucket => format!("{}/commits/{}", self.html_url(), sha),
                _ => format!("{}/commit/{}", self.html_url(), sha),
            }
        })
    }

    /// Get the branch URL
    pub fn branch_url(&self) -> String {
        match self.provider {
            GitProvider::Github => format!("{}/tree/{}", self.html_url(), self.branch),
            GitProvider::Gitlab => format!("{}/-/tree/{}", self.html_url(), self.branch),
            GitProvider::Bitbucket => format!("{}/src/{}", self.html_url(), self.branch),
            _ => format!("{}/tree/{}", self.html_url(), self.branch),
        }
    }
}

/// Webhook payload for git events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub provider: GitProvider,
    pub event_type: WebhookEventType,
    pub repository: GitRepository,
    /// The ref that triggered the event (branch or tag)
    pub ref_name: String,
    /// Commit SHA
    pub commit_sha: String,
    /// Commit message
    pub commit_message: Option<String>,
    /// Author of the commit/PR
    pub author: Option<String>,
    /// Pull request number (if PR event)
    pub pull_request_id: Option<i64>,
    /// Pull request action (opened, closed, synchronized)
    pub pull_request_action: Option<String>,
    /// Base branch for PRs
    pub base_branch: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEventType {
    Push,
    PullRequest,
    Tag,
    Release,
}

impl WebhookPayload {
    /// Check if this webhook should trigger a deployment
    pub fn should_deploy(&self, configured_branch: &str, auto_deploy: bool) -> bool {
        if !auto_deploy {
            return false;
        }

        match self.event_type {
            WebhookEventType::Push => self.ref_name == configured_branch,
            WebhookEventType::PullRequest => {
                self.pull_request_action.as_deref() == Some("opened") ||
                self.pull_request_action.as_deref() == Some("synchronize")
            }
            WebhookEventType::Tag | WebhookEventType::Release => true,
        }
    }
}
