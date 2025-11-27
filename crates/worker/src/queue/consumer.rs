//! Job consumer - fetch and process jobs from Redis

use super::job::{Job, JobPriority, JobStatus};
use crate::config::WorkerConfig;
use crate::metrics::WorkerMetrics;
use crate::workers::JobHandler;
use anyhow::{Context, Result};
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

/// Job consumer for fetching and processing jobs
#[derive(Clone)]
pub struct JobConsumer {
    redis: ConnectionManager,
    prefix: String,
    pool_size: usize,
}

impl JobConsumer {
    /// Create a new job consumer
    pub async fn new(redis_url: &str, pool_size: usize) -> Result<Self> {
        let client = redis::Client::open(redis_url)
            .context("Failed to create Redis client")?;
        let redis = ConnectionManager::new(client)
            .await
            .context("Failed to connect to Redis")?;

        Ok(Self {
            redis,
            prefix: "llm-benchmark".to_string(),
            pool_size,
        })
    }

    /// Start the consumer worker pool
    pub async fn start(
        &self,
        config: WorkerConfig,
        metrics: WorkerMetrics,
    ) -> Result<Vec<JoinHandle<()>>> {
        let semaphore = Arc::new(Semaphore::new(self.pool_size));
        let mut handles = Vec::new();

        info!(pool_size = self.pool_size, "Starting worker pool");

        for worker_id in 0..self.pool_size {
            let consumer = self.clone();
            let config = config.clone();
            let metrics = metrics.clone();
            let semaphore = semaphore.clone();

            let handle = tokio::spawn(async move {
                if let Err(e) = consumer
                    .worker_loop(worker_id, config, metrics, semaphore)
                    .await
                {
                    error!(worker_id, error = %e, "Worker loop error");
                }
            });

            handles.push(handle);
        }

        // Start delayed job processor
        let consumer = self.clone();
        let config = config.clone();
        let handle = tokio::spawn(async move {
            if let Err(e) = consumer.delayed_job_processor(config).await {
                error!(error = %e, "Delayed job processor error");
            }
        });
        handles.push(handle);

        Ok(handles)
    }

    /// Worker loop - continuously fetch and process jobs
    async fn worker_loop(
        &self,
        worker_id: usize,
        config: WorkerConfig,
        metrics: WorkerMetrics,
        semaphore: Arc<Semaphore>,
    ) -> Result<()> {
        let mut redis = self.redis.clone();

        loop {
            // Acquire semaphore permit
            let _permit = semaphore.acquire().await?;

            // Fetch job with priority
            match self.fetch_job(&mut redis, &config).await {
                Ok(Some(mut job)) => {
                    debug!(
                        worker_id,
                        job_id = %job.id,
                        job_type = ?job.job_type,
                        "Processing job"
                    );

                    metrics.increment_jobs_processed();
                    let start = std::time::Instant::now();

                    job.mark_processing();

                    // Process the job
                    let result = self.process_job(&job, &config).await;

                    let duration = start.elapsed();
                    metrics.record_job_duration(duration);

                    match result {
                        Ok(_) => {
                            job.mark_completed();
                            metrics.increment_jobs_succeeded();
                            info!(
                                worker_id,
                                job_id = %job.id,
                                duration_ms = duration.as_millis(),
                                "Job completed successfully"
                            );
                        }
                        Err(e) => {
                            error!(
                                worker_id,
                                job_id = %job.id,
                                error = %e,
                                "Job failed"
                            );

                            if job.should_retry() {
                                job.increment_retry();
                                let backoff = config.retry.calculate_backoff(job.retry_count);
                                warn!(
                                    worker_id,
                                    job_id = %job.id,
                                    retry_count = job.retry_count,
                                    backoff_secs = backoff.as_secs(),
                                    "Retrying job"
                                );

                                // Re-enqueue with delay
                                self.requeue_job(&mut redis, &job, backoff).await?;
                                metrics.increment_jobs_retried();
                            } else {
                                job.mark_failed(e.to_string());
                                // Move to dead letter queue
                                self.move_to_dlq(&mut redis, &job).await?;
                                metrics.increment_jobs_failed();
                            }
                        }
                    }
                }
                Ok(None) => {
                    // No job available, sleep briefly
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Err(e) => {
                    error!(worker_id, error = %e, "Failed to fetch job");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    /// Fetch a job from the queue with priority
    async fn fetch_job(
        &self,
        redis: &mut ConnectionManager,
        config: &WorkerConfig,
    ) -> Result<Option<Job>> {
        // Check queues in priority order
        let queues = vec![
            JobPriority::Critical.queue_name(&self.prefix),
            JobPriority::High.queue_name(&self.prefix),
            JobPriority::Normal.queue_name(&self.prefix),
            JobPriority::Low.queue_name(&self.prefix),
        ];

        // Use BRPOP to block until a job is available
        let result: Option<(String, String)> = redis
            .brpop(&queues, config.queue.blocking_timeout as f64)
            .await
            .ok();

        if let Some((_, job_json)) = result {
            let job: Job = serde_json::from_str(&job_json)
                .context("Failed to deserialize job")?;
            Ok(Some(job))
        } else {
            Ok(None)
        }
    }

    /// Process a job
    async fn process_job(&self, job: &Job, config: &WorkerConfig) -> Result<()> {
        let handler = JobHandler::new(config.clone());
        handler.handle(job).await
    }

    /// Re-enqueue a job with delay
    async fn requeue_job(
        &self,
        redis: &mut ConnectionManager,
        job: &Job,
        delay: Duration,
    ) -> Result<()> {
        let delayed_key = format!("{}:jobs:delayed", self.prefix);
        let score = chrono::Utc::now().timestamp() + delay.as_secs() as i64;
        let job_json = serde_json::to_string(job)
            .context("Failed to serialize job")?;

        redis
            .zadd::<_, _, _, ()>(&delayed_key, &job_json, score)
            .await
            .context("Failed to requeue job")?;

        Ok(())
    }

    /// Move a job to the dead letter queue
    async fn move_to_dlq(&self, redis: &mut ConnectionManager, job: &Job) -> Result<()> {
        let dlq_key = format!("{}:{}", self.prefix, "jobs:dlq");
        let job_json = serde_json::to_string(job)
            .context("Failed to serialize job")?;

        redis
            .lpush::<_, _, ()>(&dlq_key, &job_json)
            .await
            .context("Failed to move job to DLQ")?;

        warn!(job_id = %job.id, "Job moved to dead letter queue");

        Ok(())
    }

    /// Process delayed jobs
    async fn delayed_job_processor(&self, config: WorkerConfig) -> Result<()> {
        let mut redis = self.redis.clone();
        let delayed_key = format!("{}:jobs:delayed", self.prefix);

        loop {
            let now = chrono::Utc::now().timestamp();

            // Get jobs that are ready to be processed
            let jobs: Vec<String> = redis
                .zrangebyscore_limit(&delayed_key, 0, now, 0, 100)
                .await
                .context("Failed to fetch delayed jobs")?;

            for job_json in jobs {
                // Parse job
                let job: Job = match serde_json::from_str(&job_json) {
                    Ok(j) => j,
                    Err(e) => {
                        error!(error = %e, "Failed to parse delayed job");
                        // Remove invalid job
                        redis
                            .zrem::<_, _, ()>(&delayed_key, &job_json)
                            .await
                            .ok();
                        continue;
                    }
                };

                // Remove from delayed set
                redis
                    .zrem::<_, _, ()>(&delayed_key, &job_json)
                    .await
                    .context("Failed to remove delayed job")?;

                // Add to appropriate queue
                let queue_name = job.priority.queue_name(&self.prefix);
                redis
                    .lpush::<_, _, ()>(&queue_name, &job_json)
                    .await
                    .context("Failed to enqueue delayed job")?;

                debug!(job_id = %job.id, "Delayed job enqueued");
            }

            // Sleep before next check
            tokio::time::sleep(Duration::from_secs(config.scheduler.tick_interval)).await;
        }
    }

    /// Get dead letter queue jobs
    pub async fn get_dlq_jobs(&self, limit: usize) -> Result<Vec<Job>> {
        let mut redis = self.redis.clone();
        let dlq_key = format!("{}:{}", self.prefix, "jobs:dlq");

        let jobs_json: Vec<String> = redis
            .lrange(&dlq_key, 0, limit as isize - 1)
            .await
            .context("Failed to fetch DLQ jobs")?;

        let mut jobs = Vec::new();
        for job_json in jobs_json {
            if let Ok(job) = serde_json::from_str(&job_json) {
                jobs.push(job);
            }
        }

        Ok(jobs)
    }

    /// Retry a job from the dead letter queue
    pub async fn retry_dlq_job(&self, job_id: &uuid::Uuid) -> Result<()> {
        let mut redis = self.redis.clone();
        let dlq_key = format!("{}:{}", self.prefix, "jobs:dlq");

        // Get all DLQ jobs
        let jobs_json: Vec<String> = redis
            .lrange(&dlq_key, 0, -1)
            .await
            .context("Failed to fetch DLQ jobs")?;

        for (index, job_json) in jobs_json.iter().enumerate() {
            if let Ok(mut job) = serde_json::from_str::<Job>(job_json) {
                if &job.id == job_id {
                    // Reset job
                    job.status = JobStatus::Queued;
                    job.retry_count = 0;
                    job.error = None;

                    // Remove from DLQ
                    redis
                        .lrem::<_, _, ()>(&dlq_key, 1, job_json)
                        .await
                        .context("Failed to remove job from DLQ")?;

                    // Re-enqueue
                    let queue_name = job.priority.queue_name(&self.prefix);
                    let new_job_json = serde_json::to_string(&job)?;
                    redis
                        .lpush::<_, _, ()>(&queue_name, &new_job_json)
                        .await
                        .context("Failed to re-enqueue job")?;

                    info!(job_id = %job_id, "Job retried from DLQ");
                    return Ok(());
                }
            }
        }

        Err(anyhow::anyhow!("Job not found in DLQ"))
    }
}
