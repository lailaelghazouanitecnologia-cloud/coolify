//! Job queue backed by Redis

use kornetti_core::{Result, Error};
use redis::AsyncCommands;
use crate::Job;

const QUEUE_KEY: &str = "kornetti:jobs:queue";
const PROCESSING_KEY: &str = "kornetti:jobs:processing";

pub struct JobQueue {
    client: redis::Client,
}

impl JobQueue {
    pub fn new(redis_url: &str) -> Result<Self> {
        let client = redis::Client::open(redis_url)
            .map_err(|e| Error::Database(format!("Failed to connect to Redis: {}", e)))?;
        Ok(Self { client })
    }

    pub async fn push(&self, job: &Job) -> Result<()> {
        let mut conn = self.client.get_multiplexed_async_connection().await
            .map_err(|e| Error::Database(format!("Redis connection failed: {}", e)))?;

        let job_json = serde_json::to_string(job)
            .map_err(|e| Error::Internal(format!("Failed to serialize job: {}", e)))?;

        conn.lpush::<_, _, ()>(QUEUE_KEY, job_json).await
            .map_err(|e| Error::Database(format!("Failed to push job: {}", e)))?;

        Ok(())
    }

    pub async fn pop(&self) -> Result<Option<Job>> {
        let mut conn = self.client.get_multiplexed_async_connection().await
            .map_err(|e| Error::Database(format!("Redis connection failed: {}", e)))?;

        let result: Option<String> = conn.rpoplpush(QUEUE_KEY, PROCESSING_KEY).await
            .map_err(|e| Error::Database(format!("Failed to pop job: {}", e)))?;

        match result {
            Some(job_json) => {
                let job: Job = serde_json::from_str(&job_json)
                    .map_err(|e| Error::Internal(format!("Failed to deserialize job: {}", e)))?;
                Ok(Some(job))
            }
            None => Ok(None),
        }
    }

    pub async fn complete(&self, job: &Job) -> Result<()> {
        let mut conn = self.client.get_multiplexed_async_connection().await
            .map_err(|e| Error::Database(format!("Redis connection failed: {}", e)))?;

        let job_json = serde_json::to_string(job)
            .map_err(|e| Error::Internal(format!("Failed to serialize job: {}", e)))?;

        conn.lrem::<_, _, ()>(PROCESSING_KEY, 1, &job_json).await
            .map_err(|e| Error::Database(format!("Failed to remove job: {}", e)))?;

        Ok(())
    }

    pub async fn len(&self) -> Result<usize> {
        let mut conn = self.client.get_multiplexed_async_connection().await
            .map_err(|e| Error::Database(format!("Redis connection failed: {}", e)))?;

        let len: usize = conn.llen(QUEUE_KEY).await
            .map_err(|e| Error::Database(format!("Failed to get queue length: {}", e)))?;

        Ok(len)
    }
}
