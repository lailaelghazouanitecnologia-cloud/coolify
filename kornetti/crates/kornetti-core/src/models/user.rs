//! User model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// User account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    /// Hashed password
    #[serde(skip_serializing)]
    pub password: String,
    /// Email verification timestamp
    pub email_verified_at: Option<DateTime<Utc>>,
    /// Two-factor authentication secret
    #[serde(skip_serializing)]
    pub two_factor_secret: Option<String>,
    /// Two-factor recovery codes
    #[serde(skip_serializing)]
    pub two_factor_recovery_codes: Option<String>,
    /// Whether 2FA is confirmed
    pub two_factor_confirmed_at: Option<DateTime<Utc>>,
    /// Force password change on next login
    pub force_password_reset: bool,
    /// Marketing email consent
    pub marketing_emails: bool,
    /// Current team ID
    pub current_team_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn new(name: String, email: String, password: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            email,
            password,
            email_verified_at: None,
            two_factor_secret: None,
            two_factor_recovery_codes: None,
            two_factor_confirmed_at: None,
            force_password_reset: false,
            marketing_emails: false,
            current_team_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if email is verified
    pub fn is_verified(&self) -> bool {
        self.email_verified_at.is_some()
    }

    /// Check if 2FA is enabled
    pub fn has_two_factor(&self) -> bool {
        self.two_factor_confirmed_at.is_some()
    }

    /// Verify the email
    pub fn verify_email(&mut self) {
        self.email_verified_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }
}

/// Personal access token for API authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalAccessToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub team_id: Option<Uuid>,
    pub name: String,
    /// Hashed token value
    #[serde(skip_serializing)]
    pub token: String,
    /// Token abilities (permissions)
    pub abilities: Vec<String>,
    /// Allowed IP addresses (empty = all)
    pub ip_allowlist: Vec<String>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PersonalAccessToken {
    pub fn new(user_id: Uuid, name: String, token: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            team_id: None,
            name,
            token,
            abilities: vec!["*".to_string()], // Full access by default
            ip_allowlist: vec![],
            last_used_at: None,
            expires_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| exp < Utc::now())
            .unwrap_or(false)
    }

    /// Check if IP is allowed
    pub fn is_ip_allowed(&self, ip: &str) -> bool {
        if self.ip_allowlist.is_empty() {
            return true;
        }
        self.ip_allowlist.iter().any(|allowed| allowed == ip)
    }

    /// Check if token has a specific ability
    pub fn has_ability(&self, ability: &str) -> bool {
        self.abilities.contains(&"*".to_string()) ||
        self.abilities.iter().any(|a| a == ability)
    }

    /// Record token usage
    pub fn touch(&mut self) {
        self.last_used_at = Some(Utc::now());
    }
}

/// Team invitation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamInvitation {
    pub id: Uuid,
    pub team_id: Uuid,
    pub email: String,
    pub role: String,
    /// Invitation token
    pub token: String,
    pub invited_by: Uuid,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl TeamInvitation {
    pub fn new(team_id: Uuid, email: String, role: String, invited_by: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            team_id,
            email,
            role,
            token: generate_token(),
            invited_by,
            expires_at: now + chrono::Duration::days(7), // 7 days expiry
            created_at: now,
        }
    }

    /// Check if invitation is expired
    pub fn is_expired(&self) -> bool {
        self.expires_at < Utc::now()
    }
}

fn generate_token() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..64)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}
