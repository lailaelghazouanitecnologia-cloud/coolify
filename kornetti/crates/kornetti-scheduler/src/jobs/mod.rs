//! Job implementations
//!
//! This module contains all job types that can be executed by the scheduler.

pub mod container_status;
pub mod database_backup;
pub mod deployment;
pub mod docker_cleanup;
pub mod log_cleanup;
pub mod notification;
pub mod server_check;
pub mod server_metrics;

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
