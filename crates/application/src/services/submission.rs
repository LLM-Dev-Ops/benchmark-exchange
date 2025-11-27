//! Submission Service
//!
//! Business logic for submission management including creation, verification,
//! scoring, and leaderboard operations.

use super::{
    Authorizer, EventPublisher, PaginatedResult, Pagination, ServiceConfig, ServiceContext,
    ServiceEvent,
};
use crate::scoring::{ScoringEngine, ScoringEngineConfig, ScoringRequest, TestCaseInput};
use crate::validation::{
    CreateSubmissionRequest, LeaderboardQuery, SubmissionQueryFilters, UpdateSubmissionRequest,
    Validatable, VerificationRequest,
};
use crate::{ApplicationError, ApplicationResult};
use async_trait::async_trait;
use llm_benchmark_domain::submission::{
    SubmissionResults, SubmissionVisibility, TestCaseResult, VerificationLevel, VerificationStatus,
};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

/// Submission data transfer object
#[derive(Debug, Clone)]
pub struct SubmissionDto {
    pub id: String,
    pub benchmark_id: String,
    pub benchmark_version_id: String,
    pub model_provider: String,
    pub model_name: String,
    pub model_version: Option<String>,
    pub submitter_id: String,
    pub organization_id: Option<String>,
    pub aggregate_score: f64,
    pub verification_level: VerificationLevel,
    pub visibility: SubmissionVisibility,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Leaderboard entry data transfer object
#[derive(Debug, Clone)]
pub struct LeaderboardEntryDto {
    pub rank: u32,
    pub submission_id: String,
    pub model_provider: String,
    pub model_name: String,
    pub model_version: Option<String>,
    pub aggregate_score: f64,
    pub verification_level: VerificationLevel,
    pub submitter_name: String,
    pub submitted_at: chrono::DateTime<chrono::Utc>,
}

/// Submission repository trait
#[async_trait]
pub trait SubmissionRepositoryPort: Send + Sync {
    async fn create(&self, submission: &CreateSubmissionData) -> Result<String, ApplicationError>;
    async fn get_by_id(&self, id: &str) -> Result<Option<SubmissionDto>, ApplicationError>;
    async fn list(
        &self,
        filters: &SubmissionQueryFilters,
        pagination: &Pagination,
    ) -> Result<(Vec<SubmissionDto>, u64), ApplicationError>;
    async fn update(&self, id: &str, update: &UpdateSubmissionData) -> Result<(), ApplicationError>;
    async fn update_verification(
        &self,
        id: &str,
        verification: &VerificationData,
    ) -> Result<(), ApplicationError>;
    async fn delete(&self, id: &str) -> Result<(), ApplicationError>;
    async fn get_leaderboard(
        &self,
        benchmark_id: &str,
        version_id: Option<&str>,
        limit: u32,
        min_verification: Option<VerificationLevel>,
    ) -> Result<Vec<LeaderboardEntryDto>, ApplicationError>;
    async fn get_user_submissions(
        &self,
        user_id: &str,
        pagination: &Pagination,
    ) -> Result<(Vec<SubmissionDto>, u64), ApplicationError>;
    async fn get_results(&self, id: &str) -> Result<Option<SubmissionResults>, ApplicationError>;
    async fn save_results(&self, id: &str, results: &SubmissionResults) -> Result<(), ApplicationError>;
}

/// Data for creating a submission
#[derive(Debug, Clone)]
pub struct CreateSubmissionData {
    pub benchmark_id: String,
    pub benchmark_version_id: String,
    pub model_provider: String,
    pub model_name: String,
    pub model_version: Option<String>,
    pub submitter_id: String,
    pub organization_id: Option<String>,
    pub aggregate_score: f64,
    pub visibility: SubmissionVisibility,
}

/// Data for updating a submission
#[derive(Debug, Clone)]
pub struct UpdateSubmissionData {
    pub visibility: Option<SubmissionVisibility>,
    pub notes: Option<String>,
}

/// Data for verification
#[derive(Debug, Clone)]
pub struct VerificationData {
    pub level: VerificationLevel,
    pub verified_by: String,
    pub reproduced_score: Option<f64>,
    pub score_variance: Option<f64>,
    pub environment_match: Option<bool>,
    pub notes: Option<String>,
}

/// Submission service implementation
pub struct SubmissionService<R, A, E>
where
    R: SubmissionRepositoryPort,
    A: Authorizer,
    E: EventPublisher,
{
    repository: Arc<R>,
    authorizer: Arc<A>,
    event_publisher: Arc<E>,
    scoring_engine: ScoringEngine,
    config: ServiceConfig,
}

impl<R, A, E> SubmissionService<R, A, E>
where
    R: SubmissionRepositoryPort,
    A: Authorizer,
    E: EventPublisher,
{
    pub fn new(
        repository: Arc<R>,
        authorizer: Arc<A>,
        event_publisher: Arc<E>,
        config: ServiceConfig,
    ) -> Self {
        Self {
            repository,
            authorizer,
            event_publisher,
            scoring_engine: ScoringEngine::new(ScoringEngineConfig::default()),
            config,
        }
    }

    /// Create a new submission
    #[instrument(skip(self, ctx, request), fields(correlation_id = %ctx.correlation_id))]
    pub async fn create(
        &self,
        ctx: &ServiceContext,
        request: CreateSubmissionRequest,
    ) -> ApplicationResult<SubmissionDto> {
        // Validate request
        let validation = request.validate_all();
        validation.ensure_valid()?;

        // Check authorization
        let auth = self
            .authorizer
            .can_create_submission(ctx, &request.benchmark_id)
            .await;
        auth.ensure_allowed()?;

        // Get authenticated user
        let user_id = ctx.require_authenticated()?;

        // Create submission
        let create_data = CreateSubmissionData {
            benchmark_id: request.benchmark_id,
            benchmark_version_id: request.benchmark_version_id,
            model_provider: request.model_provider,
            model_name: request.model_name,
            model_version: request.model_version,
            submitter_id: user_id.to_string(),
            organization_id: ctx.organization_id.clone(),
            aggregate_score: request.results.aggregate_score,
            visibility: request.visibility,
        };

        let id = self.repository.create(&create_data).await?;

        // Store detailed results
        let results = SubmissionResults {
            aggregate_score: request.results.aggregate_score,
            metric_scores: request
                .results
                .metric_scores
                .into_iter()
                .map(|(k, v)| {
                    (
                        k,
                        llm_benchmark_domain::submission::MetricScore {
                            value: v,
                            unit: None,
                            raw_values: None,
                            std_dev: None,
                        },
                    )
                })
                .collect(),
            test_case_results: request
                .results
                .test_case_results
                .into_iter()
                .map(|tc| TestCaseResult {
                    test_case_id: tc.test_case_id,
                    passed: tc.passed,
                    score: tc.score,
                    latency_ms: tc.latency_ms,
                    tokens_generated: tc.tokens_generated,
                    error: None,
                })
                .collect(),
            confidence_interval: None,
            statistical_significance: None,
        };

        self.repository.save_results(&id, &results).await?;

        info!(submission_id = %id, "Submission created");

        // Publish event
        self.event_publisher
            .publish(ServiceEvent::SubmissionCreated {
                submission_id: id.clone(),
            })
            .await?;

        // Fetch and return the created submission
        self.repository
            .get_by_id(&id)
            .await?
            .ok_or_else(|| ApplicationError::Internal("Failed to fetch created submission".to_string()))
    }

    /// Get a submission by ID
    #[instrument(skip(self, ctx), fields(correlation_id = %ctx.correlation_id))]
    pub async fn get_by_id(
        &self,
        ctx: &ServiceContext,
        id: &str,
    ) -> ApplicationResult<Option<SubmissionDto>> {
        let submission = self.repository.get_by_id(id).await?;

        // Check visibility
        if let Some(ref sub) = submission {
            if sub.visibility == SubmissionVisibility::Private {
                // Only owner or admin can see private submissions
                if let Some(user_id) = &ctx.user_id {
                    if user_id != &sub.submitter_id && !ctx.is_admin {
                        return Ok(None);
                    }
                } else {
                    return Ok(None);
                }
            }
        }

        Ok(submission)
    }

    /// Get detailed results for a submission
    #[instrument(skip(self, ctx), fields(correlation_id = %ctx.correlation_id))]
    pub async fn get_results(
        &self,
        ctx: &ServiceContext,
        id: &str,
    ) -> ApplicationResult<Option<SubmissionResults>> {
        // First check if user can view this submission
        let submission = self.get_by_id(ctx, id).await?;
        if submission.is_none() {
            return Ok(None);
        }

        self.repository.get_results(id).await
    }

    /// List submissions with filters
    #[instrument(skip(self, ctx), fields(correlation_id = %ctx.correlation_id))]
    pub async fn list(
        &self,
        ctx: &ServiceContext,
        filters: SubmissionQueryFilters,
        pagination: Pagination,
    ) -> ApplicationResult<PaginatedResult<SubmissionDto>> {
        // Clamp page size
        let pagination = Pagination::new(
            pagination.page.max(1),
            pagination.page_size.min(self.config.max_page_size),
        );

        let (items, total) = self.repository.list(&filters, &pagination).await?;

        // Filter out private submissions not owned by user
        let items: Vec<_> = items
            .into_iter()
            .filter(|sub| {
                if sub.visibility == SubmissionVisibility::Private {
                    ctx.user_id
                        .as_ref()
                        .map(|uid| uid == &sub.submitter_id || ctx.is_admin)
                        .unwrap_or(false)
                } else {
                    true
                }
            })
            .collect();

        Ok(PaginatedResult::new(items, total, &pagination))
    }

    /// Update a submission
    #[instrument(skip(self, ctx, request), fields(correlation_id = %ctx.correlation_id))]
    pub async fn update(
        &self,
        ctx: &ServiceContext,
        id: &str,
        request: UpdateSubmissionRequest,
    ) -> ApplicationResult<SubmissionDto> {
        // Validate request
        let validation = request.validate_all();
        validation.ensure_valid()?;

        // Get existing submission
        let existing = self
            .repository
            .get_by_id(id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound(format!("Submission not found: {}", id)))?;

        // Check ownership
        let user_id = ctx.require_authenticated()?;
        if existing.submitter_id != user_id && !ctx.is_admin {
            return Err(ApplicationError::Forbidden(
                "You can only update your own submissions".to_string(),
            ));
        }

        // Update submission
        let update_data = UpdateSubmissionData {
            visibility: request.visibility,
            notes: request.notes,
        };

        self.repository.update(id, &update_data).await?;

        info!(submission_id = %id, "Submission updated");

        // Fetch and return updated submission
        self.repository
            .get_by_id(id)
            .await?
            .ok_or_else(|| ApplicationError::Internal("Failed to fetch updated submission".to_string()))
    }

    /// Verify a submission
    #[instrument(skip(self, ctx, request), fields(correlation_id = %ctx.correlation_id))]
    pub async fn verify(
        &self,
        ctx: &ServiceContext,
        request: VerificationRequest,
    ) -> ApplicationResult<SubmissionDto> {
        // Validate request
        let validation = request.validate_all();
        validation.ensure_valid()?;

        // Check authorization
        let auth = self
            .authorizer
            .can_verify_submission(ctx, &request.submission_id)
            .await;
        auth.ensure_allowed()?;

        // Get authenticated user
        let user_id = ctx.require_authenticated()?;

        // Get existing submission
        let existing = self
            .repository
            .get_by_id(&request.submission_id)
            .await?
            .ok_or_else(|| {
                ApplicationError::NotFound(format!("Submission not found: {}", request.submission_id))
            })?;

        // Cannot verify own submissions for platform verification
        if request.verification_level == VerificationLevel::PlatformVerified
            || request.verification_level == VerificationLevel::Audited
        {
            if existing.submitter_id == user_id {
                return Err(ApplicationError::Forbidden(
                    "Cannot verify your own submission at this level".to_string(),
                ));
            }
        }

        // Update verification
        let verification_data = VerificationData {
            level: request.verification_level,
            verified_by: user_id.to_string(),
            reproduced_score: request.reproduced_score,
            score_variance: request.score_variance,
            environment_match: request.environment_match,
            notes: request.notes,
        };

        self.repository
            .update_verification(&request.submission_id, &verification_data)
            .await?;

        info!(
            submission_id = %request.submission_id,
            level = ?request.verification_level,
            "Submission verified"
        );

        // Publish event
        self.event_publisher
            .publish(ServiceEvent::SubmissionVerified {
                submission_id: request.submission_id.clone(),
                level: format!("{:?}", request.verification_level),
            })
            .await?;

        // Fetch and return updated submission
        self.repository
            .get_by_id(&request.submission_id)
            .await?
            .ok_or_else(|| ApplicationError::Internal("Failed to fetch verified submission".to_string()))
    }

    /// Get leaderboard for a benchmark
    #[instrument(skip(self, ctx), fields(correlation_id = %ctx.correlation_id))]
    pub async fn get_leaderboard(
        &self,
        ctx: &ServiceContext,
        query: LeaderboardQuery,
    ) -> ApplicationResult<Vec<LeaderboardEntryDto>> {
        // Validate query
        let validation = query.validate_all();
        validation.ensure_valid()?;

        let limit = query.limit.unwrap_or(LeaderboardQuery::DEFAULT_LIMIT);

        self.repository
            .get_leaderboard(
                &query.benchmark_id,
                query.benchmark_version_id.as_deref(),
                limit,
                query.min_verification_level,
            )
            .await
    }

    /// Get submissions by user
    #[instrument(skip(self, ctx), fields(correlation_id = %ctx.correlation_id))]
    pub async fn get_user_submissions(
        &self,
        ctx: &ServiceContext,
        user_id: &str,
        pagination: Pagination,
    ) -> ApplicationResult<PaginatedResult<SubmissionDto>> {
        // Clamp page size
        let pagination = Pagination::new(
            pagination.page.max(1),
            pagination.page_size.min(self.config.max_page_size),
        );

        let (items, total) = self
            .repository
            .get_user_submissions(user_id, &pagination)
            .await?;

        // Filter out private submissions not owned by requesting user
        let is_own = ctx.user_id.as_ref().map(|uid| uid == user_id).unwrap_or(false);
        let items: Vec<_> = items
            .into_iter()
            .filter(|sub| {
                if sub.visibility == SubmissionVisibility::Private {
                    is_own || ctx.is_admin
                } else {
                    true
                }
            })
            .collect();

        Ok(PaginatedResult::new(items, total, &pagination))
    }

    /// Delete a submission
    #[instrument(skip(self, ctx), fields(correlation_id = %ctx.correlation_id))]
    pub async fn delete(&self, ctx: &ServiceContext, id: &str) -> ApplicationResult<()> {
        // Get existing submission
        let existing = self
            .repository
            .get_by_id(id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound(format!("Submission not found: {}", id)))?;

        // Check ownership or admin
        let user_id = ctx.require_authenticated()?;
        if existing.submitter_id != user_id && !ctx.is_admin {
            return Err(ApplicationError::Forbidden(
                "You can only delete your own submissions".to_string(),
            ));
        }

        // Delete submission
        self.repository.delete(id).await?;

        info!(submission_id = %id, "Submission deleted");

        Ok(())
    }

    /// Re-score a submission using the scoring engine
    #[instrument(skip(self, ctx), fields(correlation_id = %ctx.correlation_id))]
    pub async fn rescore(
        &self,
        ctx: &ServiceContext,
        id: &str,
        request: ScoringRequest,
    ) -> ApplicationResult<SubmissionResults> {
        // Check authorization
        ctx.require_admin()?;

        // Get existing submission
        let existing = self
            .repository
            .get_by_id(id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound(format!("Submission not found: {}", id)))?;

        // Score using engine
        let results = self.scoring_engine.score(&request).await?;

        // Save updated results
        self.repository.save_results(id, &results).await?;

        info!(submission_id = %id, new_score = %results.aggregate_score, "Submission re-scored");

        // Publish event
        self.event_publisher
            .publish(ServiceEvent::SubmissionScoreUpdated {
                submission_id: id.to_string(),
            })
            .await?;

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would go here with mock implementations
}
