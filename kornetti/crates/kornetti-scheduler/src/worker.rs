//! Job worker

use async_trait::async_trait;
use kornetti_core::Result;
use crate::{Job, JobQueue, JobType};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{error, info};

#[async_trait]
pub trait JobHandler: Send + Sync {
    async fn handle(&self, job: &Job) -> Result<()>;
}

pub struct Worker {
    queue: JobQueue,
    handlers: std::collections::HashMap<JobType, Arc<dyn JobHandler>>,
    concurrency: usize,
}

impl Worker {
    pub fn new(queue: JobQueue, concurrency: usize) -> Self {
        Self {
            queue,
            handlers: std::collections::HashMap::new(),
            concurrency,
        }
    }

    pub fn register_handler(&mut self, job_type: JobType, handler: Arc<dyn JobHandler>) {
        self.handlers.insert(job_type, handler);
    }

    pub async fn run(&self) -> Result<()> {
        let semaphore = Arc::new(Semaphore::new(self.concurrency));

        loop {
            let permit = semaphore.clone().acquire_owned().await.unwrap();

            if let Some(mut job) = self.queue.pop().await? {
                let handler = self.handlers.get(&job.job_type).cloned();

                tokio::spawn(async move {
                    let _permit = permit;

                    if let Some(handler) = handler {
                        job.mark_started();
                        info!(job_id = %job.id, job_type = ?job.job_type, "Starting job");

                        match handler.handle(&job).await {
                            Ok(()) => {
                                job.mark_completed();
                                info!(job_id = %job.id, "Job completed");
                            }
                            Err(e) => {
                                job.mark_failed(e.to_string());
                                error!(job_id = %job.id, error = %e, "Job failed");
                            }
                        }
                    } else {
                        error!(job_type = ?job.job_type, "No handler registered for job type");
                    }
                });
            } else {
                // No jobs available, wait a bit
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    }
}
