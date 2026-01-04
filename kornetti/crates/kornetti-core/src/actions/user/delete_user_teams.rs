//! Delete User Teams Action
//!
//! Deletes all teams owned by a user and their associated resources.

use crate::actions::{Action, ActionContext, ActionResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Delete user teams action
pub struct DeleteUserTeams;

/// Input for deleting user teams
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteUserTeamsInput {
    /// User ID
    pub user_id: Uuid,
    /// Delete all resources in teams
    pub delete_resources: bool,
    /// Force delete even if teams have other members
    pub force: bool,
    /// Transfer ownership instead of deleting (to this user)
    pub transfer_to: Option<Uuid>,
    /// Notify other team members
    pub notify_members: bool,
}

/// Output of team deletion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteUserTeamsOutput {
    /// User ID
    pub user_id: Uuid,
    /// Teams deleted
    pub deleted_teams: Vec<DeletedTeam>,
    /// Teams transferred
    pub transferred_teams: Vec<TransferredTeam>,
    /// Teams that failed to delete
    pub failed_teams: Vec<TeamDeletionFailure>,
    /// Members notified
    pub members_notified: usize,
    /// Whether all operations succeeded
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletedTeam {
    pub id: Uuid,
    pub name: String,
    /// Resources deleted with the team
    pub deleted_resources: TeamResourceCounts,
    /// Members removed
    pub members_removed: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TeamResourceCounts {
    pub servers: usize,
    pub applications: usize,
    pub databases: usize,
    pub services: usize,
    pub projects: usize,
    pub private_keys: usize,
    pub github_apps: usize,
    pub s3_storages: usize,
}

impl TeamResourceCounts {
    pub fn total(&self) -> usize {
        self.servers
            + self.applications
            + self.databases
            + self.services
            + self.projects
            + self.private_keys
            + self.github_apps
            + self.s3_storages
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferredTeam {
    pub id: Uuid,
    pub name: String,
    pub new_owner_id: Uuid,
    pub new_owner_email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamDeletionFailure {
    pub team_id: Uuid,
    pub team_name: String,
    pub error: String,
    pub reason: TeamFailureReason,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TeamFailureReason {
    /// Team has other members and force was not set
    HasMembers,
    /// Team has resources and delete_resources was not set
    HasResources,
    /// Transfer target not found
    TransferTargetNotFound,
    /// Cannot delete personal team
    IsPersonalTeam,
    /// Permission denied
    PermissionDenied,
    /// Other error
    Other,
}

impl Action for DeleteUserTeams {
    type Input = DeleteUserTeamsInput;
    type Output = DeleteUserTeamsOutput;

    fn name(&self) -> &'static str {
        "delete_user_teams"
    }

    fn execute(
        &self,
        ctx: &ActionContext,
        input: &Self::Input,
    ) -> ActionResult<Self::Output> {
        // Would:
        // 1. Get all teams owned by user
        // 2. For each team:
        //    a. Check if personal team (skip or error)
        //    b. If transfer_to, transfer ownership
        //    c. Otherwise, delete resources if requested
        //    d. Notify members if requested
        //    e. Delete or transfer team

        Ok(DeleteUserTeamsOutput {
            user_id: input.user_id,
            deleted_teams: Vec::new(),
            transferred_teams: Vec::new(),
            failed_teams: Vec::new(),
            members_notified: 0,
            success: true,
        })
    }
}

impl DeleteUserTeams {
    /// Check if a team can be deleted
    pub fn can_delete_team(
        is_personal: bool,
        has_members: bool,
        has_resources: bool,
        force: bool,
        delete_resources: bool,
    ) -> Result<(), TeamFailureReason> {
        if is_personal {
            return Err(TeamFailureReason::IsPersonalTeam);
        }
        if has_members && !force {
            return Err(TeamFailureReason::HasMembers);
        }
        if has_resources && !delete_resources && !force {
            return Err(TeamFailureReason::HasResources);
        }
        Ok(())
    }

    /// Get resources to delete for a team (in order)
    pub fn deletion_order() -> Vec<&'static str> {
        vec![
            "applications",
            "services",
            "databases",
            "servers",
            "private_keys",
            "github_apps",
            "s3_storages",
            "projects",
        ]
    }

    /// Generate notification message for team deletion
    pub fn deletion_notification(team_name: &str, owner_email: &str) -> String {
        format!(
            "The team '{}' has been deleted by its owner ({}). \
             You have been removed from this team.",
            team_name, owner_email
        )
    }

    /// Generate notification message for team transfer
    pub fn transfer_notification(
        team_name: &str,
        old_owner: &str,
        new_owner: &str,
    ) -> String {
        format!(
            "Ownership of team '{}' has been transferred from {} to {}.",
            team_name, old_owner, new_owner
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_delete_team() {
        // Can delete empty team
        assert!(DeleteUserTeams::can_delete_team(false, false, false, false, false).is_ok());

        // Cannot delete personal team
        assert_eq!(
            DeleteUserTeams::can_delete_team(true, false, false, false, false),
            Err(TeamFailureReason::IsPersonalTeam)
        );

        // Cannot delete team with members without force
        assert_eq!(
            DeleteUserTeams::can_delete_team(false, true, false, false, false),
            Err(TeamFailureReason::HasMembers)
        );

        // Can delete team with members if force
        assert!(DeleteUserTeams::can_delete_team(false, true, false, true, false).is_ok());

        // Cannot delete team with resources without delete_resources
        assert_eq!(
            DeleteUserTeams::can_delete_team(false, false, true, false, false),
            Err(TeamFailureReason::HasResources)
        );

        // Can delete team with resources if delete_resources
        assert!(DeleteUserTeams::can_delete_team(false, false, true, false, true).is_ok());
    }

    #[test]
    fn test_team_resource_counts_total() {
        let counts = TeamResourceCounts {
            servers: 2,
            applications: 5,
            databases: 3,
            ..Default::default()
        };

        assert_eq!(counts.total(), 10);
    }
}
