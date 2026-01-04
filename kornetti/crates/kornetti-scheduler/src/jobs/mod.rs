//! Job implementations
//!
//! This module contains all job types that can be executed by the scheduler.

pub mod container_status;
pub mod database_backup;
pub mod delete_resource;
pub mod deployment;
pub mod docker_cleanup;
pub mod log_cleanup;
pub mod notification;
pub mod restart_proxy;
pub mod scheduled_task;
pub mod server_check;
pub mod server_connection_check;
pub mod server_metrics;
pub mod validate_and_install_server;
pub mod sentinel;
pub mod pull_request_preview;
pub mod ssl_certificate;
pub mod update_checker;
pub mod cleanup;
pub mod server_storage;
pub mod github_permission;

// Re-export existing jobs
pub use container_status::{
    ContainerStatusJob, ContainerStatusContext, ContainerStatusResult,
    ContainerInfo, ContainerState, HealthStatus,
};
pub use database_backup::{DatabaseBackupJob, DatabaseBackupContext, BackupResult};
pub use deployment::{ApplicationDeploymentJob, DeploymentContext, DeploymentStep};
pub use docker_cleanup::{DockerCleanupJob, CleanupContext, CleanupResult as DockerCleanupResult};
pub use log_cleanup::{
    LogCleanupJob, CleanupConfig, CleanupResult as LogCleanupResult,
    ServerDiskCleanupJob, DiskCleanupResult,
};
pub use notification::{
    SendDiscordNotificationJob, SendTelegramNotificationJob, SendSlackNotificationJob,
    NotificationContext, NotificationDispatcher,
};
pub use server_check::ServerCheckJob;
pub use server_metrics::{
    ServerMetricsJob, ServerMetricsContext, ServerMetrics,
    MetricAlert, AlertSeverity,
};

// Re-export new jobs
pub use delete_resource::{DeleteResourceJob, DeleteResourceContext, DeleteResourceResult, ResourceType};
pub use restart_proxy::{RestartProxyJob, RestartProxyContext, RestartProxyResult, ProxyType};
pub use scheduled_task::{ScheduledTaskJob, ScheduledTaskContext, ScheduledTaskResult, ScheduledTaskManager};
pub use server_connection_check::{ServerConnectionCheckJob, ServerConnectionCheckContext, ConnectionCheckResult};
pub use validate_and_install_server::{ValidateAndInstallServerJob, ValidateAndInstallContext, ValidationResult, ValidationStep};

// Re-export new jobs
pub use sentinel::{
    CheckAndStartSentinelJob, StopSentinelJob, UpdateSentinelJob, ProcessSentinelPushJob,
    SentinelContext, SentinelResult, SentinelPushData, StopSentinelContext,
};
pub use pull_request_preview::{
    PullRequestPreviewJob, CleanupOrphanedPreviewsJob, PostDeploymentStatusJob,
    PullRequestContext, PullRequestAction, PullRequestResult,
    CleanupPreviewsContext, CleanupPreviewsResult,
    DeploymentStatusContext, DeploymentStatusType,
};
pub use ssl_certificate::{
    RegenerateSslCertJob, CheckSslCertificatesJob, UploadSslCertificateJob,
    SslCertContext, SslCertResult, SslProvider,
    CheckCertsContext, CheckCertsResult,
    UploadCertContext,
};

// Re-export update checker jobs
pub use update_checker::{
    CheckForUpdatesJob, CheckForUpdatesPayload, UpdateInfo,
    UpdateKornettiJob, UpdateKornettiPayload,
    PullChangelogJob, PullChangelogPayload,
    PullTemplatesJob, PullTemplatesPayload,
    CheckTraefikVersionJob, CheckTraefikVersionPayload,
    CheckHelperImageJob, CheckHelperImagePayload,
};

// Re-export cleanup jobs
pub use cleanup::{
    CleanupHelperContainersJob, CleanupHelperContainersPayload,
    CleanupInstanceJob, CleanupInstancePayload,
    CleanupOrphanedPreviewsJob as CleanupOrphanedPreviewContainersJob,
    CleanupOrphanedPreviewsPayload,
    CleanupStaleConnectionsJob, CleanupStaleConnectionsPayload,
    ConnectProxyToNetworksJob, ConnectProxyToNetworksPayload,
    VolumeCloneJob, VolumeClonePayload,
};

// Re-export server storage jobs
pub use server_storage::{
    ServerStorageCheckJob, ServerStorageCheckPayload, StorageInfo,
    ServerStorageSaveJob, ServerStorageSavePayload,
    ServerLimitCheckJob, ServerLimitCheckPayload,
    ServerPatchCheckJob, ServerPatchCheckPayload, PatchInfo,
    PushServerUpdateJob, PushServerUpdatePayload, FileUpdate,
    ServerFilesJob, ServerFilesPayload,
};

// Re-export GitHub permission jobs
pub use github_permission::{
    GithubAppPermissionJob, GithubAppPermissionPayload, GithubAppPermissions, PermissionLevel,
    RefreshGithubTokenJob, RefreshGithubTokenPayload,
    SyncGithubReposJob, SyncGithubReposPayload,
    ProcessGithubWebhookJob, ProcessGithubWebhookPayload,
    CreateGithubDeploymentStatusJob, CreateGithubDeploymentStatusPayload,
};

// Job trait and context types used by new-style jobs
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Generic job context that wraps the job payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobContext<T> {
    /// Unique job ID
    pub job_id: Uuid,
    /// Current attempt number (1-indexed)
    pub attempt: u32,
    /// Maximum attempts allowed
    pub max_attempts: u32,
    /// When the job was queued
    pub queued_at: DateTime<Utc>,
    /// Job-specific payload
    pub payload: T,
}

/// Result type for job handlers
pub type JobResult<T> = Result<T, String>;

/// Trait for job handlers
#[async_trait]
pub trait Job: Send + Sync {
    /// The context/input type for this job
    type Context: Send + Sync + Clone;
    /// The result type for this job
    type Result: Send + Sync;

    /// Job name for logging and metrics
    fn name(&self) -> &'static str;

    /// Maximum number of retry attempts
    fn max_tries(&self) -> u32 {
        3
    }

    /// Timeout in seconds
    fn timeout_seconds(&self) -> u64 {
        120
    }

    /// Queue name for this job
    fn queue(&self) -> &'static str {
        "default"
    }

    /// Handle the job
    async fn handle(&self, ctx: JobContext<Self::Context>) -> JobResult<Self::Result>;

    /// Called when all retries are exhausted
    async fn on_failure(&self, ctx: JobContext<Self::Context>, error: String) {
        tracing::error!(
            job_name = self.name(),
            job_id = %ctx.job_id,
            error = %error,
            "Job failed after {} attempts",
            ctx.max_attempts
        );
    }
}
