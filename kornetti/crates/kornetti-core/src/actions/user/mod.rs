//! User Actions
//!
//! Actions for user management and cleanup.

mod delete_user_resources;
mod delete_user_servers;
mod delete_user_teams;

pub use delete_user_resources::*;
pub use delete_user_servers::*;
pub use delete_user_teams::*;

use uuid::Uuid;

/// User-related actions collection
pub struct UserActions;

impl UserActions {
    /// Get all resources owned by a user
    pub fn get_user_resources(_user_id: Uuid) -> UserResources {
        // Would query database for all resources
        UserResources::default()
    }
}

/// Summary of resources owned by a user
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct UserResources {
    /// Teams owned by the user
    pub teams: Vec<TeamSummary>,
    /// Servers accessible by the user
    pub servers: Vec<ServerSummary>,
    /// Applications owned by the user
    pub applications: Vec<ResourceSummary>,
    /// Databases owned by the user
    pub databases: Vec<ResourceSummary>,
    /// Services owned by the user
    pub services: Vec<ResourceSummary>,
    /// Private keys owned by the user
    pub private_keys: Vec<ResourceSummary>,
    /// GitHub Apps configured by the user
    pub github_apps: Vec<ResourceSummary>,
    /// S3 Storages configured by the user
    pub s3_storages: Vec<ResourceSummary>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TeamSummary {
    pub id: Uuid,
    pub name: String,
    pub member_count: usize,
    pub is_owner: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServerSummary {
    pub id: Uuid,
    pub name: String,
    pub ip: String,
    pub team_id: Uuid,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourceSummary {
    pub id: Uuid,
    pub name: String,
    pub resource_type: String,
    pub team_id: Uuid,
}

impl UserResources {
    /// Get total count of all resources
    pub fn total_count(&self) -> usize {
        self.teams.len()
            + self.servers.len()
            + self.applications.len()
            + self.databases.len()
            + self.services.len()
            + self.private_keys.len()
            + self.github_apps.len()
            + self.s3_storages.len()
    }

    /// Check if user has any resources
    pub fn has_resources(&self) -> bool {
        self.total_count() > 0
    }
}
