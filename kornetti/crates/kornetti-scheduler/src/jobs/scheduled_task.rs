//! Scheduled Task Job
//!
//! Executes user-defined scheduled tasks (cron jobs).
//! Mirrors Coolify's ScheduledTaskJob.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use super::{Job, JobContext, JobResult};

/// Type of resource the scheduled task belongs to
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScheduledTaskResourceType {
    Application,
    Service,
}

/// Context for scheduled task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTaskContext {
    /// Unique identifier for the scheduled task
    pub task_id: Uuid,
    /// Name of the task
    pub task_name: String,
    /// Resource type (application or service)
    pub resource_type: ScheduledTaskResourceType,
    /// Resource ID
    pub resource_id: Uuid,
    /// Resource name
    pub resource_name: String,
    /// Server to execute on
    pub server_id: Uuid,
    pub server_ip: String,
    /// Container to execute in
    pub container_name: String,
    /// Command to execute
    pub command: String,
    /// Cron expression for scheduling
    pub cron_expression: String,
    /// Timezone for cron evaluation
    pub timezone: String,
    /// Whether the task is enabled
    pub enabled: bool,
    /// Team ID for notification context
    pub team_id: Uuid,
}

/// Result of scheduled task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTaskResult {
    /// Whether execution was successful
    pub success: bool,
    /// Exit code from command
    pub exit_code: Option<i32>,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// When execution started
    pub started_at: DateTime<Utc>,
    /// When execution finished
    pub finished_at: DateTime<Utc>,
}

/// Execution log entry for a scheduled task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTaskExecution {
    pub id: Uuid,
    pub scheduled_task_id: Uuid,
    pub status: ExecutionStatus,
    pub message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

/// Status of task execution
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Running,
    Success,
    Failed,
}

/// Job to execute scheduled tasks
pub struct ScheduledTaskJob {
    timeout_seconds: u64,
}

impl ScheduledTaskJob {
    pub fn new() -> Self {
        Self {
            timeout_seconds: 3600, // 1 hour default
        }
    }

    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    /// Create an execution log entry
    async fn create_execution_log(&self, ctx: &JobContext<ScheduledTaskContext>) -> ScheduledTaskExecution {
        ScheduledTaskExecution {
            id: Uuid::new_v4(),
            scheduled_task_id: ctx.payload.task_id,
            status: ExecutionStatus::Running,
            message: None,
            started_at: Utc::now(),
            finished_at: None,
        }
    }

    /// Update execution log with result
    async fn update_execution_log(
        &self,
        execution: &mut ScheduledTaskExecution,
        result: &ScheduledTaskResult,
    ) {
        execution.finished_at = Some(result.finished_at);
        execution.status = if result.success {
            ExecutionStatus::Success
        } else {
            ExecutionStatus::Failed
        };
        execution.message = if result.success {
            None
        } else {
            Some(result.stderr.clone())
        };

        // Placeholder: would save to database
        debug!(
            execution_id = %execution.id,
            status = ?execution.status,
            "Updated execution log"
        );
    }

    /// Execute the command in the container
    async fn execute_command(&self, ctx: &JobContext<ScheduledTaskContext>) -> Result<ScheduledTaskResult, String> {
        let started_at = Utc::now();

        info!(
            task_id = %ctx.payload.task_id,
            task_name = %ctx.payload.task_name,
            container = %ctx.payload.container_name,
            command = %ctx.payload.command,
            "Executing scheduled task"
        );

        // Placeholder: would execute via SSH
        // let command = format!(
        //     "docker exec {} sh -c '{}'",
        //     ctx.payload.container_name,
        //     ctx.payload.command.replace("'", "'\\''")
        // );
        // let output = ssh_client.execute_with_timeout(
        //     &ctx.payload.server_ip,
        //     22,
        //     "root",
        //     &command,
        //     Duration::from_secs(self.timeout_seconds)
        // ).await;

        let finished_at = Utc::now();
        let duration_ms = (finished_at - started_at).num_milliseconds() as u64;

        // Placeholder result
        Ok(ScheduledTaskResult {
            success: true,
            exit_code: Some(0),
            stdout: "Task completed successfully".to_string(),
            stderr: String::new(),
            duration_ms,
            started_at,
            finished_at,
        })
    }

    /// Send notification about task execution
    async fn send_notification(&self, ctx: &JobContext<ScheduledTaskContext>, result: &ScheduledTaskResult) {
        if !result.success {
            warn!(
                task_id = %ctx.payload.task_id,
                task_name = %ctx.payload.task_name,
                "Scheduled task failed, would send notification"
            );

            // Placeholder: would dispatch notification job
            // NotificationDispatcher::send(
            //     ctx.payload.team_id,
            //     NotificationType::ScheduledTaskFailed,
            //     format!("Scheduled task '{}' failed", ctx.payload.task_name)
            // )
        }
    }
}

impl Default for ScheduledTaskJob {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Job for ScheduledTaskJob {
    type Context = ScheduledTaskContext;
    type Result = ScheduledTaskResult;

    fn name(&self) -> &'static str {
        "scheduled_task"
    }

    fn max_tries(&self) -> u32 {
        1 // Scheduled tasks don't retry by default
    }

    fn timeout_seconds(&self) -> u64 {
        self.timeout_seconds
    }

    async fn handle(&self, ctx: JobContext<Self::Context>) -> JobResult<Self::Result> {
        // Check if task is enabled
        if !ctx.payload.enabled {
            debug!(
                task_id = %ctx.payload.task_id,
                "Scheduled task is disabled, skipping"
            );
            return Ok(ScheduledTaskResult {
                success: true,
                exit_code: None,
                stdout: "Task is disabled".to_string(),
                stderr: String::new(),
                duration_ms: 0,
                started_at: Utc::now(),
                finished_at: Utc::now(),
            });
        }

        info!(
            task_id = %ctx.payload.task_id,
            task_name = %ctx.payload.task_name,
            resource_name = %ctx.payload.resource_name,
            "Starting scheduled task"
        );

        // Create execution log
        let mut execution = self.create_execution_log(&ctx).await;

        // Execute the command
        let result = match self.execute_command(&ctx).await {
            Ok(result) => result,
            Err(e) => {
                error!(
                    task_id = %ctx.payload.task_id,
                    error = %e,
                    "Scheduled task execution failed"
                );
                ScheduledTaskResult {
                    success: false,
                    exit_code: Some(-1),
                    stdout: String::new(),
                    stderr: e,
                    duration_ms: 0,
                    started_at: Utc::now(),
                    finished_at: Utc::now(),
                }
            }
        };

        // Update execution log
        self.update_execution_log(&mut execution, &result).await;

        // Send notification if failed
        self.send_notification(&ctx, &result).await;

        if result.success {
            info!(
                task_id = %ctx.payload.task_id,
                task_name = %ctx.payload.task_name,
                duration_ms = result.duration_ms,
                "Scheduled task completed successfully"
            );
        } else {
            warn!(
                task_id = %ctx.payload.task_id,
                task_name = %ctx.payload.task_name,
                exit_code = ?result.exit_code,
                "Scheduled task failed"
            );
        }

        Ok(result)
    }

    async fn on_failure(&self, ctx: JobContext<Self::Context>, error: String) {
        error!(
            task_id = %ctx.payload.task_id,
            task_name = %ctx.payload.task_name,
            error = %error,
            "Scheduled task job failed"
        );
    }
}

/// Manager for scheduling and dispatching scheduled tasks
pub struct ScheduledTaskManager;

impl ScheduledTaskManager {
    /// Check which scheduled tasks should run now
    pub async fn check_and_dispatch_tasks() {
        debug!("Checking scheduled tasks");

        // Placeholder: would query database for enabled tasks
        // and check if their cron expression matches current time
        // For each matching task, dispatch a ScheduledTaskJob
    }

    /// Parse a cron expression and check if it matches the current time
    pub fn should_run_now(cron_expression: &str, timezone: &str) -> bool {
        // Placeholder: would use a cron parsing library
        // cron::Schedule::from_str(cron_expression)
        //     .map(|schedule| schedule.upcoming(tz).next().is_some())
        //     .unwrap_or(false)
        true // Placeholder
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_context() -> JobContext<ScheduledTaskContext> {
        JobContext {
            job_id: Uuid::new_v4(),
            attempt: 1,
            max_attempts: 1,
            queued_at: Utc::now(),
            payload: ScheduledTaskContext {
                task_id: Uuid::new_v4(),
                task_name: "test-task".to_string(),
                resource_type: ScheduledTaskResourceType::Application,
                resource_id: Uuid::new_v4(),
                resource_name: "test-app".to_string(),
                server_id: Uuid::new_v4(),
                server_ip: "192.168.1.100".to_string(),
                container_name: "test-app-container".to_string(),
                command: "echo 'Hello World'".to_string(),
                cron_expression: "* * * * *".to_string(),
                timezone: "UTC".to_string(),
                enabled: true,
                team_id: Uuid::new_v4(),
            },
        }
    }

    #[tokio::test]
    async fn test_scheduled_task() {
        let job = ScheduledTaskJob::new();
        let ctx = create_test_context();

        let result = job.handle(ctx).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_disabled_task() {
        let job = ScheduledTaskJob::new();
        let mut ctx = create_test_context();
        ctx.payload.enabled = false;

        let result = job.handle(ctx).await.unwrap();
        assert!(result.success);
        assert!(result.stdout.contains("disabled"));
    }
}
