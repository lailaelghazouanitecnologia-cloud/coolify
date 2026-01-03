//! Team Invitation model
//!
//! Pending team membership invitations.
//! Mirrors Coolify's TeamInvitation model.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Team member role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamRole {
    Owner,
    Admin,
    Member,
    Viewer,
}

impl TeamRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            TeamRole::Owner => "owner",
            TeamRole::Admin => "admin",
            TeamRole::Member => "member",
            TeamRole::Viewer => "viewer",
        }
    }

    pub fn can_manage_team(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin)
    }

    pub fn can_deploy(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin | TeamRole::Member)
    }
}

impl Default for TeamRole {
    fn default() -> Self {
        TeamRole::Member
    }
}

/// Invitation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvitationStatus {
    Pending,
    Accepted,
    Declined,
    Expired,
    Revoked,
}

impl Default for InvitationStatus {
    fn default() -> Self {
        InvitationStatus::Pending
    }
}

/// Team invitation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamInvitation {
    pub id: Uuid,
    pub team_id: Uuid,
    /// Email address of the invitee
    pub email: String,
    /// Role to assign when accepted
    pub role: TeamRole,
    /// Unique invitation token
    pub token: String,
    /// Current status
    pub status: InvitationStatus,
    /// User ID who sent the invitation
    pub invited_by: Uuid,
    /// When the invitation expires
    pub expires_at: DateTime<Utc>,
    /// When the invitation was accepted/declined
    pub responded_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TeamInvitation {
    /// Create a new invitation (expires in 7 days by default)
    pub fn new(team_id: Uuid, email: String, role: TeamRole, invited_by: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            team_id,
            email: email.to_lowercase(),
            role,
            token: Self::generate_token(),
            status: InvitationStatus::Pending,
            invited_by,
            expires_at: now + Duration::days(7),
            responded_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Generate a secure random token
    fn generate_token() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, &bytes)
    }

    /// Check if invitation is still valid
    pub fn is_valid(&self) -> bool {
        self.status == InvitationStatus::Pending && self.expires_at > Utc::now()
    }

    /// Check if invitation is expired
    pub fn is_expired(&self) -> bool {
        self.expires_at <= Utc::now()
    }

    /// Accept the invitation
    pub fn accept(&mut self) {
        self.status = InvitationStatus::Accepted;
        self.responded_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Decline the invitation
    pub fn decline(&mut self) {
        self.status = InvitationStatus::Declined;
        self.responded_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Revoke the invitation
    pub fn revoke(&mut self) {
        self.status = InvitationStatus::Revoked;
        self.updated_at = Utc::now();
    }

    /// Mark as expired
    pub fn mark_expired(&mut self) {
        if self.status == InvitationStatus::Pending {
            self.status = InvitationStatus::Expired;
            self.updated_at = Utc::now();
        }
    }

    /// Get invitation URL
    pub fn get_invite_url(&self, base_url: &str) -> String {
        format!("{}/invitations/{}", base_url, self.token)
    }
}

/// Team member (after invitation is accepted)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    pub id: Uuid,
    pub team_id: Uuid,
    pub user_id: Uuid,
    pub role: TeamRole,
    pub joined_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TeamMember {
    pub fn new(team_id: Uuid, user_id: Uuid, role: TeamRole) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            team_id,
            user_id,
            role,
            joined_at: now,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create owner membership
    pub fn owner(team_id: Uuid, user_id: Uuid) -> Self {
        Self::new(team_id, user_id, TeamRole::Owner)
    }
}
