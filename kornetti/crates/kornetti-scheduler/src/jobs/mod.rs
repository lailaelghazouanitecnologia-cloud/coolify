//! Job implementations
//!
//! This module contains all job types that can be executed by the scheduler.

pub mod database_backup;
pub mod deployment;
pub mod docker_cleanup;
pub mod notification;
pub mod server_check;

pub use database_backup::{DatabaseBackupJob, DatabaseBackupContext, BackupResult};
pub use deployment::{ApplicationDeploymentJob, DeploymentContext, DeploymentStep};
pub use docker_cleanup::{DockerCleanupJob, CleanupContext, CleanupResult};
pub use notification::{
    SendDiscordNotificationJob, SendTelegramNotificationJob, SendSlackNotificationJob,
    NotificationContext, NotificationDispatcher,
};
pub use server_check::ServerCheckJob;
