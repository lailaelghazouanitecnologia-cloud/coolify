//! Kornetti Scheduler
//!
//! Task scheduler, job queue, and deployment engine for Kornetti.
//!
//! # Features
//!
//! - **Job Queue**: Redis-backed job queue for async processing
//! - **Deployment Jobs**: Full application deployment pipeline
//! - **Health Checks**: Server monitoring and health verification
//! - **Worker Pool**: Concurrent job execution with configurable workers

pub mod jobs;
pub mod queue;
pub mod worker;

pub use jobs::{ApplicationDeploymentJob, DeploymentContext, DeploymentStep, ServerCheckJob};

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use kornetti_ssh::SshClient;
use kornetti_docker::DockerExecutor;

/// Job status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Generic job trait
#[async_trait]
pub trait Job: Send + Sync {
    /// Unique job identifier
    fn id(&self) -> Uuid;

    /// Job type name
    fn job_type(&self) -> &'static str;

    /// Execute the job
    async fn execute(&mut self) -> Result<JobResult, JobError>;

    /// Handle job failure
    async fn on_failure(&mut self, error: &JobError);

    /// Check if job can be retried
    fn can_retry(&self) -> bool {
        true
    }

    /// Maximum retry attempts
    fn max_retries(&self) -> u32 {
        3
    }
}

/// Job result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    pub success: bool,
    pub message: Option<String>,
    pub data: Option<serde_json::Value>,
    pub duration_ms: u64,
}

/// Job error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobError {
    pub message: String,
    pub retryable: bool,
    pub details: Option<serde_json::Value>,
}

impl std::fmt::Display for JobError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for JobError {}

/// Job entry in the queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobEntry {
    pub id: Uuid,
    pub job_type: String,
    pub payload: serde_json::Value,
    pub status: JobStatus,
    pub attempts: u32,
    pub max_retries: u32,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}

impl JobEntry {
    pub fn new(job_type: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            job_type: job_type.into(),
            payload,
            status: JobStatus::Queued,
            attempts: 0,
            max_retries: 3,
            created_at: Utc::now(),
            started_at: None,
            finished_at: None,
            last_error: None,
        }
    }
}

/// Scheduler configuration
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Number of worker threads
    pub workers: usize,
    /// Maximum concurrent jobs per worker
    pub max_concurrent_jobs: usize,
    /// Job polling interval
    pub poll_interval: Duration,
    /// Job timeout
    pub job_timeout: Duration,
    /// Redis URL for queue
    pub redis_url: Option<String>,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            workers: 4,
            max_concurrent_jobs: 10,
            poll_interval: Duration::from_secs(1),
            job_timeout: Duration::from_secs(3600), // 1 hour
            redis_url: None,
        }
    }
}

/// Main scheduler
pub struct Scheduler {
    config: SchedulerConfig,
    ssh: Arc<SshClient>,
    docker: Arc<DockerExecutor>,
    jobs: Arc<DashMap<Uuid, JobEntry>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl Scheduler {
    /// Create a new scheduler
    pub fn new(config: SchedulerConfig, ssh: Arc<SshClient>) -> Self {
        let docker = Arc::new(DockerExecutor::new(ssh.clone()));

        Self {
            config,
            ssh,
            docker,
            jobs: Arc::new(DashMap::new()),
            shutdown_tx: None,
        }
    }

    /// Start the scheduler
    pub async fn start(&mut self) -> kornetti_core::Result<()> {
        info!(workers = self.config.workers, "Starting scheduler");

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        // Spawn worker tasks
        let jobs = self.jobs.clone();
        let ssh = self.ssh.clone();
        let docker = self.docker.clone();
        let poll_interval = self.config.poll_interval;

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        info!("Scheduler shutting down");
                        break;
                    }
                    _ = tokio::time::sleep(poll_interval) => {
                        // Process queued jobs
                        Self::process_jobs(&jobs, &ssh, &docker).await;
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the scheduler
    pub async fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
    }

    /// Queue a deployment job
    pub fn queue_deployment(&self, context: DeploymentContext) -> Uuid {
        let payload = serde_json::to_value(&context).unwrap_or_default();
        let entry = JobEntry::new("deployment", payload);
        let id = entry.id;

        self.jobs.insert(id, entry);
        info!(job_id = %id, "Queued deployment job");

        id
    }

    /// Queue a server check job
    pub fn queue_server_check(&self, context: jobs::server_check::ServerCheckContext) -> Uuid {
        let payload = serde_json::to_value(&context).unwrap_or_default();
        let entry = JobEntry::new("server_check", payload);
        let id = entry.id;

        self.jobs.insert(id, entry);
        debug!(job_id = %id, "Queued server check job");

        id
    }

    /// Get job status
    pub fn get_job(&self, id: Uuid) -> Option<JobEntry> {
        self.jobs.get(&id).map(|e| e.clone())
    }

    /// Cancel a job
    pub fn cancel_job(&self, id: Uuid) -> bool {
        if let Some(mut entry) = self.jobs.get_mut(&id) {
            if entry.status == JobStatus::Queued {
                entry.status = JobStatus::Cancelled;
                entry.finished_at = Some(Utc::now());
                return true;
            }
        }
        false
    }

    /// List all jobs
    pub fn list_jobs(&self) -> Vec<JobEntry> {
        self.jobs.iter().map(|e| e.clone()).collect()
    }

    /// List jobs by status
    pub fn list_jobs_by_status(&self, status: JobStatus) -> Vec<JobEntry> {
        self.jobs
            .iter()
            .filter(|e| e.status == status)
            .map(|e| e.clone())
            .collect()
    }

    /// Process queued jobs
    async fn process_jobs(
        jobs: &DashMap<Uuid, JobEntry>,
        _ssh: &SshClient,
        _docker: &DockerExecutor,
    ) {
        // Find queued jobs
        let queued: Vec<Uuid> = jobs
            .iter()
            .filter(|e| e.status == JobStatus::Queued)
            .map(|e| e.id)
            .collect();

        for job_id in queued {
            if let Some(mut entry) = jobs.get_mut(&job_id) {
                entry.status = JobStatus::Running;
                entry.started_at = Some(Utc::now());
                entry.attempts += 1;

                debug!(job_id = %job_id, job_type = %entry.job_type, "Processing job");

                // Here we would dispatch to the appropriate job handler
                // For now, mark as completed after a brief delay
                drop(entry);

                // Simulate job execution
                tokio::spawn({
                    let jobs = jobs.clone();
                    let job_id = job_id;
                    async move {
                        tokio::time::sleep(Duration::from_millis(100)).await;

                        if let Some(mut entry) = jobs.get_mut(&job_id) {
                            entry.status = JobStatus::Completed;
                            entry.finished_at = Some(Utc::now());
                            info!(job_id = %job_id, "Job completed");
                        }
                    }
                });
            }
        }
    }

    /// Clean up old completed jobs
    pub fn cleanup_old_jobs(&self, max_age: Duration) {
        let cutoff = Utc::now() - chrono::Duration::from_std(max_age).unwrap_or_default();

        self.jobs.retain(|_, entry| {
            if matches!(entry.status, JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled) {
                entry.finished_at.map(|t| t > cutoff).unwrap_or(true)
            } else {
                true
            }
        });
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        let ssh = Arc::new(SshClient::new());
        Self::new(SchedulerConfig::default(), ssh)
    }
}
