//! Job types and definitions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Job priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum JobPriority {
    /// Critical priority - processed first
    Critical = 4,
    /// High priority
    High = 3,
    /// Normal priority (default)
    Normal = 2,
    /// Low priority - processed last
    Low = 1,
}

impl Default for JobPriority {
    fn default() -> Self {
        Self::Normal
    }
}

impl JobPriority {
    /// Get the queue name for this priority
    pub fn queue_name(&self, prefix: &str) -> String {
        match self {
            Self::Critical => format!("{}:jobs:critical", prefix),
            Self::High => format!("{}:jobs:high", prefix),
            Self::Normal => format!("{}:jobs:normal", prefix),
            Self::Low => format!("{}:jobs:low", prefix),
        }
    }
}

/// Job status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    /// Job is queued and waiting to be processed
    Queued,
    /// Job is currently being processed
    Processing,
    /// Job completed successfully
    Completed,
    /// Job failed
    Failed,
    /// Job was retried
    Retried,
    /// Job is in dead letter queue
    Dead,
}

/// Job type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum JobType {
    /// Verify a submission
    VerifySubmission(VerifySubmissionJob),
    /// Recompute leaderboard
    RecomputeLeaderboard(RecomputeLeaderboardJob),
    /// Sync to registry
    SyncToRegistry(SyncToRegistryJob),
    /// Export to analytics
    ExportToAnalytics(ExportToAnalyticsJob),
    /// Finalize proposal
    FinalizeProposal(FinalizeProposalJob),
    /// Clean up expired data
    CleanupExpiredData(CleanupExpiredDataJob),
    /// Send notification
    SendNotification(SendNotificationJob),
}

/// Job wrapper with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Unique job identifier
    pub id: Uuid,
    /// Job type and payload
    pub job_type: JobType,
    /// Job priority
    pub priority: JobPriority,
    /// Job status
    pub status: JobStatus,
    /// Number of retry attempts
    pub retry_count: u32,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// When the job was created
    pub created_at: DateTime<Utc>,
    /// When the job was scheduled to run
    pub scheduled_at: DateTime<Utc>,
    /// When the job started processing
    pub started_at: Option<DateTime<Utc>>,
    /// When the job completed
    pub completed_at: Option<DateTime<Utc>>,
    /// Error message if failed
    pub error: Option<String>,
}

impl Job {
    /// Create a new job
    pub fn new(job_type: JobType, priority: JobPriority) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            job_type,
            priority,
            status: JobStatus::Queued,
            retry_count: 0,
            max_retries: 3,
            created_at: now,
            scheduled_at: now,
            started_at: None,
            completed_at: None,
            error: None,
        }
    }

    /// Create a delayed job
    pub fn new_delayed(
        job_type: JobType,
        priority: JobPriority,
        delay: chrono::Duration,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            job_type,
            priority,
            status: JobStatus::Queued,
            retry_count: 0,
            max_retries: 3,
            created_at: now,
            scheduled_at: now + delay,
            started_at: None,
            completed_at: None,
            error: None,
        }
    }

    /// Mark job as processing
    pub fn mark_processing(&mut self) {
        self.status = JobStatus::Processing;
        self.started_at = Some(Utc::now());
    }

    /// Mark job as completed
    pub fn mark_completed(&mut self) {
        self.status = JobStatus::Completed;
        self.completed_at = Some(Utc::now());
    }

    /// Mark job as failed
    pub fn mark_failed(&mut self, error: String) {
        self.status = JobStatus::Failed;
        self.completed_at = Some(Utc::now());
        self.error = Some(error);
    }

    /// Check if job should be retried
    pub fn should_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    /// Increment retry count
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
        self.status = JobStatus::Retried;
    }
}

// Job type definitions

/// Verify submission job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifySubmissionJob {
    pub submission_id: Uuid,
    pub benchmark_id: Uuid,
}

/// Recompute leaderboard job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecomputeLeaderboardJob {
    pub benchmark_id: Uuid,
    pub invalidate_cache: bool,
}

/// Sync to registry job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncToRegistryJob {
    pub benchmark_id: Option<Uuid>,
    pub submission_id: Option<Uuid>,
    pub sync_all: bool,
}

/// Export to analytics job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportToAnalyticsJob {
    pub benchmark_id: Uuid,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
}

/// Finalize proposal job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalizeProposalJob {
    pub proposal_id: Uuid,
}

/// Cleanup expired data job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupExpiredDataJob {
    pub cleanup_type: CleanupType,
    pub older_than_days: u32,
}

/// Cleanup type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CleanupType {
    ExpiredSessions,
    OldSubmissions,
    TempFiles,
    ArchivedData,
}

/// Send notification job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendNotificationJob {
    pub recipient: NotificationRecipient,
    pub notification_type: NotificationType,
    pub metadata: serde_json::Value,
}

/// Notification recipient
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationRecipient {
    User(Uuid),
    Email(String),
    Webhook(String),
}

/// Notification type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationType {
    SubmissionVerified,
    SubmissionFailed,
    ProposalFinalized,
    LeaderboardUpdated,
    SystemAlert,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_creation() {
        let job = Job::new(
            JobType::VerifySubmission(VerifySubmissionJob {
                submission_id: Uuid::new_v4(),
                benchmark_id: Uuid::new_v4(),
            }),
            JobPriority::High,
        );

        assert_eq!(job.status, JobStatus::Queued);
        assert_eq!(job.priority, JobPriority::High);
        assert_eq!(job.retry_count, 0);
    }

    #[test]
    fn test_job_lifecycle() {
        let mut job = Job::new(
            JobType::RecomputeLeaderboard(RecomputeLeaderboardJob {
                benchmark_id: Uuid::new_v4(),
                invalidate_cache: true,
            }),
            JobPriority::Normal,
        );

        job.mark_processing();
        assert_eq!(job.status, JobStatus::Processing);
        assert!(job.started_at.is_some());

        job.mark_completed();
        assert_eq!(job.status, JobStatus::Completed);
        assert!(job.completed_at.is_some());
    }

    #[test]
    fn test_job_retry() {
        let mut job = Job::new(
            JobType::SyncToRegistry(SyncToRegistryJob {
                benchmark_id: None,
                submission_id: None,
                sync_all: true,
            }),
            JobPriority::Low,
        );

        assert!(job.should_retry());

        job.increment_retry();
        assert_eq!(job.retry_count, 1);
        assert!(job.should_retry());

        job.increment_retry();
        job.increment_retry();
        assert_eq!(job.retry_count, 3);
        assert!(!job.should_retry());
    }

    #[test]
    fn test_priority_queue_name() {
        assert_eq!(
            JobPriority::Critical.queue_name("test"),
            "test:jobs:critical"
        );
        assert_eq!(JobPriority::High.queue_name("test"), "test:jobs:high");
        assert_eq!(JobPriority::Normal.queue_name("test"), "test:jobs:normal");
        assert_eq!(JobPriority::Low.queue_name("test"), "test:jobs:low");
    }
}
