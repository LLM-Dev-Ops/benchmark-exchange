//! Worker implementations

pub mod cleanup;
pub mod governance;
pub mod leaderboard;
pub mod notification;
pub mod sync;
pub mod verification;

use crate::config::WorkerConfig;
use crate::queue::job::{Job, JobType};
use anyhow::Result;
use async_trait::async_trait;

/// Worker trait for processing jobs
#[async_trait]
pub trait Worker: Send + Sync {
    /// Process a job
    async fn process(&self, job: &Job) -> Result<()>;

    /// Get the worker name
    fn name(&self) -> &str;
}

/// Job handler that routes jobs to appropriate workers
pub struct JobHandler {
    config: WorkerConfig,
}

impl JobHandler {
    /// Create a new job handler
    pub fn new(config: WorkerConfig) -> Self {
        Self { config }
    }

    /// Handle a job by routing to the appropriate worker
    pub async fn handle(&self, job: &Job) -> Result<()> {
        match &job.job_type {
            JobType::VerifySubmission(_) => {
                let worker = verification::VerificationWorker::new(self.config.clone());
                worker.process(job).await
            }
            JobType::RecomputeLeaderboard(_) => {
                let worker = leaderboard::LeaderboardWorker::new(self.config.clone());
                worker.process(job).await
            }
            JobType::SyncToRegistry(_) | JobType::ExportToAnalytics(_) => {
                let worker = sync::SyncWorker::new(self.config.clone());
                worker.process(job).await
            }
            JobType::FinalizeProposal(_) => {
                let worker = governance::GovernanceWorker::new(self.config.clone());
                worker.process(job).await
            }
            JobType::SendNotification(_) => {
                let worker = notification::NotificationWorker::new(self.config.clone());
                worker.process(job).await
            }
            JobType::CleanupExpiredData(_) => {
                let worker = cleanup::CleanupWorker::new(self.config.clone());
                worker.process(job).await
            }
        }
    }
}
