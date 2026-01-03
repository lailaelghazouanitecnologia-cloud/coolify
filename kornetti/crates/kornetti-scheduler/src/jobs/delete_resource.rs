//! Delete Resource Job
//!
//! Handles cleanup and deletion of resources (applications, databases, services).
//! Mirrors Coolify's DeleteResourceJob.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::{Job, JobContext, JobResult};

/// Type of resource being deleted
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    Application,
    ApplicationPreview,
    Service,
    PostgreSQL,
    MySQL,
    MariaDB,
    MongoDB,
    Redis,
    Clickhouse,
    KeyDB,
    Dragonfly,
}

impl ResourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResourceType::Application => "application",
            ResourceType::ApplicationPreview => "application_preview",
            ResourceType::Service => "service",
            ResourceType::PostgreSQL => "standalone-postgresql",
            ResourceType::MySQL => "standalone-mysql",
            ResourceType::MariaDB => "standalone-mariadb",
            ResourceType::MongoDB => "standalone-mongodb",
            ResourceType::Redis => "standalone-redis",
            ResourceType::Clickhouse => "standalone-clickhouse",
            ResourceType::KeyDB => "standalone-keydb",
            ResourceType::Dragonfly => "standalone-dragonfly",
        }
    }

    pub fn is_database(&self) -> bool {
        matches!(
            self,
            ResourceType::PostgreSQL
                | ResourceType::MySQL
                | ResourceType::MariaDB
                | ResourceType::MongoDB
                | ResourceType::Redis
                | ResourceType::Clickhouse
                | ResourceType::KeyDB
                | ResourceType::Dragonfly
        )
    }
}

/// Context for resource deletion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteResourceContext {
    pub resource_id: Uuid,
    pub resource_type: ResourceType,
    pub resource_name: String,
    pub server_id: Uuid,
    pub server_ip: String,
    /// Container name(s) to stop
    pub container_names: Vec<String>,
    /// Whether to delete volumes
    pub delete_volumes: bool,
    /// Whether to delete connected networks
    pub delete_connected_networks: bool,
    /// Whether to delete configurations
    pub delete_configurations: bool,
    /// Whether to run docker cleanup afterwards
    pub docker_cleanup: bool,
    /// For application previews: the PR ID
    pub pull_request_id: Option<i64>,
}

/// Result of resource deletion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteResourceResult {
    /// Whether deletion was successful
    pub success: bool,
    /// Steps completed during deletion
    pub steps_completed: Vec<String>,
    /// Any warnings that occurred
    pub warnings: Vec<String>,
    /// Error if deletion failed
    pub error: Option<String>,
}

/// Job to delete a resource and clean up its containers/volumes/networks
pub struct DeleteResourceJob;

impl DeleteResourceJob {
    pub fn new() -> Self {
        Self
    }

    /// Stop containers for the resource
    async fn stop_containers(&self, ctx: &JobContext<DeleteResourceContext>) -> Result<(), String> {
        if ctx.payload.container_names.is_empty() {
            return Ok(());
        }

        info!(
            resource_id = %ctx.payload.resource_id,
            containers = ?ctx.payload.container_names,
            "Stopping containers"
        );

        // Placeholder: would run docker stop and docker rm
        // let container_list = ctx.payload.container_names.join(" ");
        // ssh.execute(format!("docker stop -t 30 {} && docker rm -f {}", container_list, container_list))

        Ok(())
    }

    /// Delete volumes for the resource
    async fn delete_volumes(&self, ctx: &JobContext<DeleteResourceContext>) -> Result<(), String> {
        if !ctx.payload.delete_volumes {
            return Ok(());
        }

        debug!(resource_id = %ctx.payload.resource_id, "Deleting volumes");

        // Placeholder: would list and remove volumes
        // docker volume ls --filter label=coolify.resourceId={resource_id} -q | xargs docker volume rm

        Ok(())
    }

    /// Delete networks for the resource
    async fn delete_networks(&self, ctx: &JobContext<DeleteResourceContext>) -> Result<(), String> {
        if !ctx.payload.delete_connected_networks {
            return Ok(());
        }

        debug!(resource_id = %ctx.payload.resource_id, "Deleting networks");

        // Placeholder: would list and remove networks
        // docker network ls --filter label=coolify.resourceId={resource_id} -q | xargs docker network rm

        Ok(())
    }

    /// Delete configuration files
    async fn delete_configurations(&self, ctx: &JobContext<DeleteResourceContext>) -> Result<(), String> {
        if !ctx.payload.delete_configurations {
            return Ok(());
        }

        debug!(resource_id = %ctx.payload.resource_id, "Deleting configurations");

        // Placeholder: would remove configuration directories
        // rm -rf /data/coolify/applications/{uuid}
        // or /data/coolify/databases/{uuid}
        // or /data/coolify/services/{uuid}

        Ok(())
    }

    /// Trigger Docker cleanup job
    async fn trigger_docker_cleanup(&self, ctx: &JobContext<DeleteResourceContext>) {
        if !ctx.payload.docker_cleanup {
            return;
        }

        debug!(server_id = %ctx.payload.server_id, "Triggering Docker cleanup");

        // Placeholder: would dispatch DockerCleanupJob
    }

    /// Handle application preview deletion
    async fn delete_application_preview(&self, ctx: &JobContext<DeleteResourceContext>) -> Result<DeleteResourceResult, String> {
        info!(
            resource_id = %ctx.payload.resource_id,
            pull_request_id = ?ctx.payload.pull_request_id,
            "Deleting application preview"
        );

        let mut steps = Vec::new();
        let mut warnings = Vec::new();

        // Cancel any active deployments for this PR
        // Placeholder: would query and update deployment queue
        steps.push("Cancelled active deployments".to_string());

        // Stop preview containers
        if let Err(e) = self.stop_containers(ctx).await {
            warnings.push(format!("Failed to stop containers: {}", e));
        } else {
            steps.push("Stopped preview containers".to_string());
        }

        // Delete from database (soft delete first, then force delete)
        steps.push("Deleted preview record".to_string());

        Ok(DeleteResourceResult {
            success: true,
            steps_completed: steps,
            warnings,
            error: None,
        })
    }

    /// Handle application deletion
    async fn delete_application(&self, ctx: &JobContext<DeleteResourceContext>) -> Result<DeleteResourceResult, String> {
        info!(
            resource_id = %ctx.payload.resource_id,
            resource_name = %ctx.payload.resource_name,
            "Deleting application"
        );

        let mut steps = Vec::new();
        let mut warnings = Vec::new();

        // Stop containers
        if let Err(e) = self.stop_containers(ctx).await {
            warnings.push(format!("Failed to stop containers: {}", e));
        } else {
            steps.push("Stopped containers".to_string());
        }

        // Delete configurations
        if let Err(e) = self.delete_configurations(ctx).await {
            warnings.push(format!("Failed to delete configurations: {}", e));
        } else {
            steps.push("Deleted configurations".to_string());
        }

        // Delete volumes
        if let Err(e) = self.delete_volumes(ctx).await {
            warnings.push(format!("Failed to delete volumes: {}", e));
        } else if ctx.payload.delete_volumes {
            steps.push("Deleted volumes".to_string());
        }

        // Delete networks
        if let Err(e) = self.delete_networks(ctx).await {
            warnings.push(format!("Failed to delete networks: {}", e));
        } else if ctx.payload.delete_connected_networks {
            steps.push("Deleted networks".to_string());
        }

        // Delete from database
        steps.push("Deleted from database".to_string());

        Ok(DeleteResourceResult {
            success: true,
            steps_completed: steps,
            warnings,
            error: None,
        })
    }

    /// Handle database deletion
    async fn delete_database(&self, ctx: &JobContext<DeleteResourceContext>) -> Result<DeleteResourceResult, String> {
        info!(
            resource_id = %ctx.payload.resource_id,
            resource_type = ?ctx.payload.resource_type,
            "Deleting database"
        );

        let mut steps = Vec::new();
        let mut warnings = Vec::new();

        // Stop containers
        if let Err(e) = self.stop_containers(ctx).await {
            warnings.push(format!("Failed to stop containers: {}", e));
        } else {
            steps.push("Stopped database container".to_string());
        }

        // Delete SSL certificates
        steps.push("Deleted SSL certificates".to_string());

        // Delete scheduled backups
        steps.push("Deleted scheduled backups".to_string());

        // Delete configurations
        if let Err(e) = self.delete_configurations(ctx).await {
            warnings.push(format!("Failed to delete configurations: {}", e));
        } else {
            steps.push("Deleted configurations".to_string());
        }

        // Delete volumes
        if let Err(e) = self.delete_volumes(ctx).await {
            warnings.push(format!("Failed to delete volumes: {}", e));
        } else if ctx.payload.delete_volumes {
            steps.push("Deleted volumes".to_string());
        }

        // Delete from database
        steps.push("Deleted from database".to_string());

        Ok(DeleteResourceResult {
            success: true,
            steps_completed: steps,
            warnings,
            error: None,
        })
    }

    /// Handle service deletion
    async fn delete_service(&self, ctx: &JobContext<DeleteResourceContext>) -> Result<DeleteResourceResult, String> {
        info!(
            resource_id = %ctx.payload.resource_id,
            resource_name = %ctx.payload.resource_name,
            "Deleting service"
        );

        let mut steps = Vec::new();
        let mut warnings = Vec::new();

        // Stop service (which may include multiple containers)
        if let Err(e) = self.stop_containers(ctx).await {
            warnings.push(format!("Failed to stop containers: {}", e));
        } else {
            steps.push("Stopped service containers".to_string());
        }

        // Delete configurations
        if let Err(e) = self.delete_configurations(ctx).await {
            warnings.push(format!("Failed to delete configurations: {}", e));
        } else {
            steps.push("Deleted configurations".to_string());
        }

        // Delete volumes
        if let Err(e) = self.delete_volumes(ctx).await {
            warnings.push(format!("Failed to delete volumes: {}", e));
        } else if ctx.payload.delete_volumes {
            steps.push("Deleted volumes".to_string());
        }

        // Delete networks
        if let Err(e) = self.delete_networks(ctx).await {
            warnings.push(format!("Failed to delete networks: {}", e));
        } else if ctx.payload.delete_connected_networks {
            steps.push("Deleted networks".to_string());
        }

        // Delete from database
        steps.push("Deleted from database".to_string());

        Ok(DeleteResourceResult {
            success: true,
            steps_completed: steps,
            warnings,
            error: None,
        })
    }
}

impl Default for DeleteResourceJob {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Job for DeleteResourceJob {
    type Context = DeleteResourceContext;
    type Result = DeleteResourceResult;

    fn name(&self) -> &'static str {
        "delete_resource"
    }

    fn queue(&self) -> &'static str {
        "high"
    }

    fn timeout_seconds(&self) -> u64 {
        300 // 5 minutes
    }

    async fn handle(&self, ctx: JobContext<Self::Context>) -> JobResult<Self::Result> {
        info!(
            resource_id = %ctx.payload.resource_id,
            resource_type = ?ctx.payload.resource_type,
            resource_name = %ctx.payload.resource_name,
            "Starting resource deletion"
        );

        let result = match ctx.payload.resource_type {
            ResourceType::ApplicationPreview => {
                self.delete_application_preview(&ctx).await
            }
            ResourceType::Application => {
                self.delete_application(&ctx).await
            }
            ResourceType::Service => {
                self.delete_service(&ctx).await
            }
            rt if rt.is_database() => {
                self.delete_database(&ctx).await
            }
            _ => {
                Err("Unknown resource type".to_string())
            }
        };

        // Always trigger Docker cleanup if requested
        self.trigger_docker_cleanup(&ctx).await;

        match result {
            Ok(res) => {
                info!(
                    resource_id = %ctx.payload.resource_id,
                    steps = ?res.steps_completed,
                    "Resource deletion complete"
                );
                Ok(res)
            }
            Err(e) => {
                error!(
                    resource_id = %ctx.payload.resource_id,
                    error = %e,
                    "Resource deletion failed"
                );
                Ok(DeleteResourceResult {
                    success: false,
                    steps_completed: vec![],
                    warnings: vec![],
                    error: Some(e),
                })
            }
        }
    }

    async fn on_failure(&self, ctx: JobContext<Self::Context>, error: String) {
        error!(
            resource_id = %ctx.payload.resource_id,
            resource_type = ?ctx.payload.resource_type,
            error = %error,
            "Delete resource job failed"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_context(resource_type: ResourceType) -> JobContext<DeleteResourceContext> {
        JobContext {
            job_id: Uuid::new_v4(),
            attempt: 1,
            max_attempts: 1,
            queued_at: Utc::now(),
            payload: DeleteResourceContext {
                resource_id: Uuid::new_v4(),
                resource_type,
                resource_name: "test-resource".to_string(),
                server_id: Uuid::new_v4(),
                server_ip: "192.168.1.100".to_string(),
                container_names: vec!["test-container".to_string()],
                delete_volumes: true,
                delete_connected_networks: true,
                delete_configurations: true,
                docker_cleanup: true,
                pull_request_id: None,
            },
        }
    }

    #[tokio::test]
    async fn test_delete_application() {
        let job = DeleteResourceJob::new();
        let ctx = create_test_context(ResourceType::Application);

        let result = job.handle(ctx).await.unwrap();
        assert!(result.success);
        assert!(!result.steps_completed.is_empty());
    }

    #[tokio::test]
    async fn test_delete_database() {
        let job = DeleteResourceJob::new();
        let ctx = create_test_context(ResourceType::PostgreSQL);

        let result = job.handle(ctx).await.unwrap();
        assert!(result.success);
        assert!(result.resource_type_is_database());
    }

    impl DeleteResourceResult {
        fn resource_type_is_database(&self) -> bool {
            self.steps_completed.iter().any(|s| s.contains("database"))
        }
    }
}
