//! Team model for multi-tenancy

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub personal: bool,
    pub settings: TeamSettings,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamSettings {
    pub concurrent_builds: u32,
    pub smtp_enabled: bool,
    pub discord_enabled: bool,
    pub telegram_enabled: bool,
    pub resend_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    pub id: Uuid,
    pub team_id: Uuid,
    pub user_id: Uuid,
    pub role: TeamRole,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TeamRole {
    Owner,
    Admin,
    Member,
    Viewer,
}

impl TeamRole {
    pub fn can_manage_team(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin)
    }

    pub fn can_deploy(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin | TeamRole::Member)
    }

    pub fn can_view(&self) -> bool {
        true
    }
}

impl Default for TeamSettings {
    fn default() -> Self {
        Self {
            concurrent_builds: 2,
            smtp_enabled: false,
            discord_enabled: false,
            telegram_enabled: false,
            resend_enabled: false,
        }
    }
}
