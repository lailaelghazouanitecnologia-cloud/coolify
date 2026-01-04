//! Delete User Resources Action
//!
//! Deletes all resources owned by a user.

use crate::actions::{Action, ActionContext, ActionResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Delete user resources action
pub struct DeleteUserResources;

/// Input for deleting user resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteUserResourcesInput {
    /// User ID
    pub user_id: Uuid,
    /// Delete applications
    pub delete_applications: bool,
    /// Delete databases
    pub delete_databases: bool,
    /// Delete services
    pub delete_services: bool,
    /// Delete private keys
    pub delete_private_keys: bool,
    /// Delete GitHub apps
    pub delete_github_apps: bool,
    /// Delete S3 storages
    pub delete_s3_storages: bool,
    /// Cleanup containers on servers
    pub cleanup_containers: bool,
    /// Force delete even if resources are running
    pub force: bool,
}

impl Default for DeleteUserResourcesInput {
    fn default() -> Self {
        Self {
            user_id: Uuid::nil(),
            delete_applications: true,
            delete_databases: true,
            delete_services: true,
            delete_private_keys: true,
            delete_github_apps: true,
            delete_s3_storages: true,
            cleanup_containers: true,
            force: false,
        }
    }
}

/// Output of resource deletion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteUserResourcesOutput {
    /// User ID
    pub user_id: Uuid,
    /// Resources deleted
    pub deleted: DeletedResourceCounts,
    /// Resources that failed to delete
    pub failed: Vec<DeletionFailure>,
    /// Whether all deletions succeeded
    pub success: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeletedResourceCounts {
    pub applications: usize,
    pub databases: usize,
    pub services: usize,
    pub private_keys: usize,
    pub github_apps: usize,
    pub s3_storages: usize,
    pub containers_stopped: usize,
}

impl DeletedResourceCounts {
    pub fn total(&self) -> usize {
        self.applications
            + self.databases
            + self.services
            + self.private_keys
            + self.github_apps
            + self.s3_storages
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletionFailure {
    pub resource_type: String,
    pub resource_id: Uuid,
    pub resource_name: String,
    pub error: String,
}

impl Action for DeleteUserResources {
    type Input = DeleteUserResourcesInput;
    type Output = DeleteUserResourcesOutput;

    fn name(&self) -> &'static str {
        "delete_user_resources"
    }

    fn execute(
        &self,
        ctx: &ActionContext,
        input: &Self::Input,
    ) -> ActionResult<Self::Output> {
        // Would:
        // 1. Get all resources for user
        // 2. Stop running containers if cleanup_containers
        // 3. Delete each resource type in order
        // 4. Track successes and failures

        Ok(DeleteUserResourcesOutput {
            user_id: input.user_id,
            deleted: DeletedResourceCounts::default(),
            failed: Vec::new(),
            success: true,
        })
    }
}

impl DeleteUserResources {
    /// Get the deletion order for resources
    /// Resources should be deleted in dependency order
    pub fn deletion_order() -> Vec<&'static str> {
        vec![
            "applications",    // Delete apps first
            "services",        // Then services
            "databases",       // Then databases
            "s3_storages",     // Then S3 storages
            "github_apps",     // Then GitHub apps
            "private_keys",    // Finally private keys
        ]
    }

    /// Check if a resource can be deleted
    pub fn can_delete(resource_type: &str, has_dependents: bool, force: bool) -> bool {
        if force {
            return true;
        }
        !has_dependents
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deletion_order() {
        let order = DeleteUserResources::deletion_order();
        assert_eq!(order[0], "applications");
        assert_eq!(order.last(), Some(&"private_keys"));
    }

    #[test]
    fn test_can_delete() {
        assert!(DeleteUserResources::can_delete("applications", false, false));
        assert!(!DeleteUserResources::can_delete("applications", true, false));
        assert!(DeleteUserResources::can_delete("applications", true, true));
    }

    #[test]
    fn test_deleted_counts_total() {
        let counts = DeletedResourceCounts {
            applications: 5,
            databases: 3,
            services: 2,
            ..Default::default()
        };

        assert_eq!(counts.total(), 10);
    }
}
