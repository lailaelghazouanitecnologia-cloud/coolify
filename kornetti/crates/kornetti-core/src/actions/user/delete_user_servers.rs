//! Delete User Servers Action
//!
//! Deletes all servers accessible by a user (that they own through teams).

use crate::actions::{Action, ActionContext, ActionResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Delete user servers action
pub struct DeleteUserServers;

/// Input for deleting user servers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteUserServersInput {
    /// User ID
    pub user_id: Uuid,
    /// Only delete servers from teams the user owns
    pub owned_teams_only: bool,
    /// Force delete even if servers have resources
    pub force: bool,
    /// Stop all containers on servers before deletion
    pub stop_containers: bool,
    /// Revoke SSH keys from servers
    pub revoke_ssh_keys: bool,
}

/// Output of server deletion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteUserServersOutput {
    /// User ID
    pub user_id: Uuid,
    /// Servers deleted
    pub deleted_servers: Vec<DeletedServer>,
    /// Servers that failed to delete
    pub failed_servers: Vec<ServerDeletionFailure>,
    /// Whether all deletions succeeded
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletedServer {
    pub id: Uuid,
    pub name: String,
    pub ip: String,
    pub team_id: Uuid,
    /// Resources that were cleaned up
    pub cleaned_up_resources: CleanedUpResources,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CleanedUpResources {
    pub containers_stopped: usize,
    pub applications_deleted: usize,
    pub databases_deleted: usize,
    pub services_deleted: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerDeletionFailure {
    pub server_id: Uuid,
    pub server_name: String,
    pub error: String,
    pub reason: DeletionFailureReason,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeletionFailureReason {
    /// Server has resources and force was not set
    HasResources,
    /// SSH connection failed
    ConnectionFailed,
    /// Permission denied
    PermissionDenied,
    /// Server is localhost (cannot be deleted)
    IsLocalhost,
    /// Other error
    Other,
}

impl Action for DeleteUserServers {
    type Input = DeleteUserServersInput;
    type Output = DeleteUserServersOutput;

    fn name(&self) -> &'static str {
        "delete_user_servers"
    }

    fn execute(
        &self,
        ctx: &ActionContext,
        input: &Self::Input,
    ) -> ActionResult<Self::Output> {
        // Would:
        // 1. Get all teams owned by user (if owned_teams_only)
        // 2. Get all servers from those teams
        // 3. For each server:
        //    a. Check if localhost (skip)
        //    b. Stop containers if requested
        //    c. Delete resources on server
        //    d. Revoke SSH keys if requested
        //    e. Delete server record

        Ok(DeleteUserServersOutput {
            user_id: input.user_id,
            deleted_servers: Vec::new(),
            failed_servers: Vec::new(),
            success: true,
        })
    }
}

impl DeleteUserServers {
    /// Check if a server can be deleted
    pub fn can_delete_server(
        is_localhost: bool,
        has_resources: bool,
        force: bool,
    ) -> Result<(), DeletionFailureReason> {
        if is_localhost {
            return Err(DeletionFailureReason::IsLocalhost);
        }
        if has_resources && !force {
            return Err(DeletionFailureReason::HasResources);
        }
        Ok(())
    }

    /// Get SSH revocation command
    pub fn ssh_revoke_command(public_key: &str) -> String {
        // Remove the public key from authorized_keys
        format!(
            "sed -i '/{}/d' ~/.ssh/authorized_keys",
            public_key.replace('/', "\\/")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_delete_server() {
        assert!(DeleteUserServers::can_delete_server(false, false, false).is_ok());
        assert_eq!(
            DeleteUserServers::can_delete_server(true, false, false),
            Err(DeletionFailureReason::IsLocalhost)
        );
        assert_eq!(
            DeleteUserServers::can_delete_server(false, true, false),
            Err(DeletionFailureReason::HasResources)
        );
        assert!(DeleteUserServers::can_delete_server(false, true, true).is_ok());
    }
}
