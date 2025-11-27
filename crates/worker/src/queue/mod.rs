//! Job queue implementation

pub mod consumer;
pub mod job;
pub mod producer;

pub use consumer::JobConsumer;
pub use job::{
    CleanupExpiredDataJob, CleanupType, ExportToAnalyticsJob, FinalizeProposalJob, Job,
    JobPriority, JobStatus, JobType, NotificationRecipient, NotificationType,
    RecomputeLeaderboardJob, SendNotificationJob, SyncToRegistryJob, VerifySubmissionJob,
};
pub use producer::JobProducer;

use anyhow::Result;

/// Job queue abstraction
pub struct JobQueue {
    producer: JobProducer,
    consumer: JobConsumer,
}

impl JobQueue {
    /// Create a new job queue
    pub async fn new(redis_url: &str, pool_size: usize) -> Result<Self> {
        let producer = JobProducer::new(redis_url).await?;
        let consumer = JobConsumer::new(redis_url, pool_size).await?;

        Ok(Self { producer, consumer })
    }

    /// Get the producer
    pub fn producer(&self) -> &JobProducer {
        &self.producer
    }

    /// Get mutable producer
    pub fn producer_mut(&mut self) -> &mut JobProducer {
        &mut self.producer
    }

    /// Get the consumer
    pub fn consumer(&self) -> &JobConsumer {
        &self.consumer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_job_queue_creation() {
        let mut queue = JobQueue::new("redis://localhost:6379", 4)
            .await
            .expect("Failed to create job queue");

        // Basic smoke test
        assert!(queue.producer_mut().total_queue_size().await.is_ok());
    }
}
