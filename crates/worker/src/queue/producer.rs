//! Job producer - enqueue jobs to Redis

use super::job::{Job, JobPriority, JobType};
use anyhow::{Context, Result};
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use tracing::{debug, info};

/// Job producer for enqueueing jobs
#[derive(Clone)]
pub struct JobProducer {
    redis: ConnectionManager,
    prefix: String,
}

impl JobProducer {
    /// Create a new job producer
    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = redis::Client::open(redis_url)
            .context("Failed to create Redis client")?;
        let redis = ConnectionManager::new(client)
            .await
            .context("Failed to connect to Redis")?;

        Ok(Self {
            redis,
            prefix: "llm-benchmark".to_string(),
        })
    }

    /// Create a new job producer with custom prefix
    pub async fn with_prefix(redis_url: &str, prefix: String) -> Result<Self> {
        let client = redis::Client::open(redis_url)
            .context("Failed to create Redis client")?;
        let redis = ConnectionManager::new(client)
            .await
            .context("Failed to connect to Redis")?;

        Ok(Self { redis, prefix })
    }

    /// Enqueue a job with default priority (Normal)
    pub async fn enqueue(&mut self, job_type: JobType) -> Result<Job> {
        self.enqueue_with_priority(job_type, JobPriority::Normal)
            .await
    }

    /// Enqueue a job with specific priority
    pub async fn enqueue_with_priority(
        &mut self,
        job_type: JobType,
        priority: JobPriority,
    ) -> Result<Job> {
        let job = Job::new(job_type, priority);
        self.push_job(&job).await?;

        debug!(
            job_id = %job.id,
            priority = ?job.priority,
            "Job enqueued"
        );

        Ok(job)
    }

    /// Enqueue a delayed job
    pub async fn enqueue_delayed(
        &mut self,
        job_type: JobType,
        priority: JobPriority,
        delay: chrono::Duration,
    ) -> Result<Job> {
        let job = Job::new_delayed(job_type, priority, delay);

        // Serialize job
        let job_json = serde_json::to_string(&job)
            .context("Failed to serialize job")?;

        // Add to delayed jobs sorted set with scheduled time as score
        let delayed_key = format!("{}:jobs:delayed", self.prefix);
        let score = job.scheduled_at.timestamp();

        self.redis
            .zadd::<_, _, _, ()>(&delayed_key, &job_json, score)
            .await
            .context("Failed to add delayed job")?;

        debug!(
            job_id = %job.id,
            priority = ?job.priority,
            delay_seconds = delay.num_seconds(),
            "Delayed job enqueued"
        );

        Ok(job)
    }

    /// Enqueue multiple jobs in batch
    pub async fn enqueue_batch(
        &mut self,
        jobs: Vec<(JobType, JobPriority)>,
    ) -> Result<Vec<Job>> {
        let mut created_jobs = Vec::new();

        for (job_type, priority) in jobs {
            let job = Job::new(job_type, priority);
            created_jobs.push(job);
        }

        // Push all jobs in a pipeline for better performance
        let mut pipe = redis::pipe();
        for job in &created_jobs {
            let queue_name = job.priority.queue_name(&self.prefix);
            let job_json = serde_json::to_string(job)
                .context("Failed to serialize job")?;
            pipe.lpush::<_, _>(&queue_name, &job_json);
        }

        pipe.query_async(&mut self.redis)
            .await
            .context("Failed to enqueue batch jobs")?;

        info!(count = created_jobs.len(), "Batch jobs enqueued");

        Ok(created_jobs)
    }

    /// Push a job to the appropriate queue
    async fn push_job(&mut self, job: &Job) -> Result<()> {
        let queue_name = job.priority.queue_name(&self.prefix);
        let job_json = serde_json::to_string(job)
            .context("Failed to serialize job")?;

        self.redis
            .lpush::<_, _, ()>(&queue_name, &job_json)
            .await
            .context("Failed to push job to queue")?;

        Ok(())
    }

    /// Get the number of jobs in a queue
    pub async fn queue_size(&mut self, priority: JobPriority) -> Result<usize> {
        let queue_name = priority.queue_name(&self.prefix);
        let size: usize = self
            .redis
            .llen(&queue_name)
            .await
            .context("Failed to get queue size")?;
        Ok(size)
    }

    /// Get the total number of jobs across all queues
    pub async fn total_queue_size(&mut self) -> Result<usize> {
        let mut total = 0;
        for priority in &[
            JobPriority::Critical,
            JobPriority::High,
            JobPriority::Normal,
            JobPriority::Low,
        ] {
            total += self.queue_size(*priority).await?;
        }
        Ok(total)
    }

    /// Get the number of delayed jobs
    pub async fn delayed_queue_size(&mut self) -> Result<usize> {
        let delayed_key = format!("{}:jobs:delayed", self.prefix);
        let size: usize = self
            .redis
            .zcard(&delayed_key)
            .await
            .context("Failed to get delayed queue size")?;
        Ok(size)
    }

    /// Clear all jobs from a queue (use with caution!)
    pub async fn clear_queue(&mut self, priority: JobPriority) -> Result<()> {
        let queue_name = priority.queue_name(&self.prefix);
        self.redis
            .del::<_, ()>(&queue_name)
            .await
            .context("Failed to clear queue")?;
        info!(priority = ?priority, "Queue cleared");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::job::{RecomputeLeaderboardJob, VerifySubmissionJob};
    use uuid::Uuid;

    // Note: These tests require a running Redis instance
    // They are disabled by default and should be run with --ignored flag

    #[tokio::test]
    #[ignore]
    async fn test_enqueue_job() {
        let mut producer = JobProducer::new("redis://localhost:6379")
            .await
            .expect("Failed to create producer");

        let job_type = JobType::VerifySubmission(VerifySubmissionJob {
            submission_id: Uuid::new_v4(),
            benchmark_id: Uuid::new_v4(),
        });

        let job = producer
            .enqueue(job_type)
            .await
            .expect("Failed to enqueue job");

        assert_eq!(job.priority, JobPriority::Normal);

        // Clean up
        producer.clear_queue(JobPriority::Normal).await.ok();
    }

    #[tokio::test]
    #[ignore]
    async fn test_queue_size() {
        let mut producer = JobProducer::new("redis://localhost:6379")
            .await
            .expect("Failed to create producer");

        // Clear queue first
        producer.clear_queue(JobPriority::High).await.ok();

        let initial_size = producer
            .queue_size(JobPriority::High)
            .await
            .expect("Failed to get queue size");
        assert_eq!(initial_size, 0);

        // Enqueue a job
        let job_type = JobType::RecomputeLeaderboard(RecomputeLeaderboardJob {
            benchmark_id: Uuid::new_v4(),
            invalidate_cache: true,
        });

        producer
            .enqueue_with_priority(job_type, JobPriority::High)
            .await
            .expect("Failed to enqueue job");

        let new_size = producer
            .queue_size(JobPriority::High)
            .await
            .expect("Failed to get queue size");
        assert_eq!(new_size, 1);

        // Clean up
        producer.clear_queue(JobPriority::High).await.ok();
    }
}
