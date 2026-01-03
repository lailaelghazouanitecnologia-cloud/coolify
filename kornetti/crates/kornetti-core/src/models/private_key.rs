//! Private key model for SSH authentication

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// SSH private key for server authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivateKey {
    pub id: Uuid,
    pub team_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    /// The actual private key content (encrypted at rest)
    pub private_key: String,
    /// Whether this is the default key for the team
    pub is_git_related: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PrivateKey {
    pub fn new(team_id: Uuid, name: String, private_key: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            team_id,
            name,
            description: None,
            private_key,
            is_git_related: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if the key is an RSA key
    pub fn is_rsa(&self) -> bool {
        self.private_key.contains("RSA PRIVATE KEY")
    }

    /// Check if the key is an Ed25519 key
    pub fn is_ed25519(&self) -> bool {
        self.private_key.contains("OPENSSH PRIVATE KEY")
    }

    /// Get the public key from the private key (would need ssh-keygen)
    pub fn fingerprint(&self) -> Option<String> {
        // In real implementation, compute fingerprint using ssh-keygen or rust crypto
        None
    }
}
