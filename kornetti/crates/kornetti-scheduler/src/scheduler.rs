//! Cron-based scheduler

use chrono::{DateTime, Utc, Duration};
use crate::{Job, JobQueue, JobType};
use kornetti_core::Result;
use std::collections::HashMap;
use tracing::info;

pub struct Scheduler {
    queue: JobQueue,
    schedules: Vec<ScheduledJob>,
}

struct ScheduledJob {
    name: String,
    job_type: JobType,
    payload: serde_json::Value,
    interval: Duration,
    last_run: Option<DateTime<Utc>>,
}

impl Scheduler {
    pub fn new(queue: JobQueue) -> Self {
        Self {
            queue,
            schedules: Vec::new(),
        }
    }

    /// Schedule a job to run at a fixed interval
    pub fn schedule(
        &mut self,
        name: &str,
        job_type: JobType,
        payload: serde_json::Value,
        interval: Duration,
    ) {
        self.schedules.push(ScheduledJob {
            name: name.to_string(),
            job_type,
            payload,
            interval,
            last_run: None,
        });
    }

    /// Schedule server health checks (every minute)
    pub fn schedule_server_checks(&mut self) {
        self.schedule(
            "server_checks",
            JobType::ServerCheck,
            serde_json::json!({}),
            Duration::minutes(1),
        );
    }

    /// Schedule Docker cleanup (every hour)
    pub fn schedule_docker_cleanup(&mut self) {
        self.schedule(
            "docker_cleanup",
            JobType::DockerCleanup,
            serde_json::json!({}),
            Duration::hours(1),
        );
    }

    /// Schedule SSL renewal checks (daily)
    pub fn schedule_ssl_renewal(&mut self) {
        self.schedule(
            "ssl_renewal",
            JobType::SslRenewal,
            serde_json::json!({}),
            Duration::days(1),
        );
    }

    /// Run the scheduler loop
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting scheduler with {} scheduled jobs", self.schedules.len());

        loop {
            let now = Utc::now();

            for scheduled in &mut self.schedules {
                let should_run = match scheduled.last_run {
                    None => true,
                    Some(last) => now - last >= scheduled.interval,
                };

                if should_run {
                    info!(name = %scheduled.name, "Running scheduled job");

                    let job = Job::new(scheduled.job_type, scheduled.payload.clone());
                    self.queue.push(&job).await?;
                    scheduled.last_run = Some(now);
                }
            }

            // Check every 10 seconds
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }
    }
}
