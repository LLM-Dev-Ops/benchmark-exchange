//! Publication Service - Benchmark Publication Agent
//!
//! This service implements the Benchmark Publication Agent as defined in the constitution.
//!
//! ## Agent Classification: BENCHMARK PUBLICATION
//!
//! ## What this agent MAY do:
//! - Publish benchmark results
//! - Normalize benchmark metrics
//! - Validate benchmark methodology metadata
//! - Emit benchmark comparison artifacts
//! - Surface benchmark confidence and reproducibility signals
//!
//! ## What this agent MUST NOT do (HARD ERRORS):
//! - Execute benchmark workloads
//! - Trigger model execution
//! - Intercept runtime requests
//! - Modify routing or execution behavior
//! - Apply optimizations automatically
//! - Enforce policies or rankings
//!
//! ## Persistence:
//! - ALL persistence via ruvector-service ONLY
//! - NEVER connects directly to Google SQL
//! - NEVER executes SQL
//!
//! ## DecisionEvent:
//! - Emits exactly ONE DecisionEvent per invocation
//! - Persisted to ruvector-service

use super::{
    Authorizer, EventPublisher, PaginatedResult, Pagination, ServiceConfig, ServiceContext,
    ServiceEvent,
};
use crate::{ApplicationError, ApplicationResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use llm_benchmark_domain::identifiers::{BenchmarkId, SubmissionId, UserId};
use llm_benchmark_domain::publication::{
    ConfidenceLevel, DecisionEvent, DecisionOutputs, MethodologyConstraints, MetricValue,
    ModelVersionConstraints, NormalizedMetrics, Publication, PublicationCitation,
    PublicationConfidence, PublicationConstraints, PublicationDecisionType, PublicationEvent,
    PublicationId, PublicationStatus, DatasetScopeConstraints, ValidationError, ValidationResults,
    ValidationWarning,
};
use llm_benchmark_domain::version::SemanticVersion;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;
use llm_benchmark_common::execution::Artifact as ExecArtifact;

// =============================================================================
// Data Transfer Objects
// =============================================================================

/// Publication DTO for API responses
#[derive(Debug, Clone)]
pub struct PublicationDto {
    pub id: String,
    pub benchmark_id: String,
    pub submission_id: Option<String>,
    pub status: PublicationStatus,
    pub version: String,
    pub model_provider: String,
    pub model_name: String,
    pub model_version: String,
    pub aggregate_score: f64,
    pub normalized_score: f64,
    pub confidence_level: ConfidenceLevel,
    pub reproducibility_score: f64,
    pub published_by: String,
    pub organization_id: Option<String>,
    pub tags: Vec<String>,
    pub is_latest: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
}

impl From<Publication> for PublicationDto {
    fn from(p: Publication) -> Self {
        Self {
            id: p.id.to_string(),
            benchmark_id: p.benchmark_id.to_string(),
            submission_id: p.submission_id.map(|s| s.to_string()),
            status: p.status,
            version: p.version.to_string(),
            model_provider: p.model_provider,
            model_name: p.model_name,
            model_version: p.model_version,
            aggregate_score: p.metrics.aggregate_score,
            normalized_score: p.metrics.normalized_score,
            confidence_level: p.confidence.confidence_level(),
            reproducibility_score: p.confidence.reproducibility_score,
            published_by: p.published_by.to_string(),
            organization_id: p.organization_id,
            tags: p.tags,
            is_latest: p.is_latest,
            created_at: p.created_at,
            updated_at: p.updated_at,
            published_at: p.published_at,
        }
    }
}

// =============================================================================
// Request Types
// =============================================================================

/// Request to publish a benchmark result
#[derive(Debug, Clone)]
pub struct PublishBenchmarkRequest {
    pub benchmark_id: String,
    pub submission_id: Option<String>,
    pub model_provider: String,
    pub model_name: String,
    pub model_version: String,
    pub aggregate_score: f64,
    pub metric_scores: HashMap<String, MetricScoreInput>,
    pub methodology: MethodologyInput,
    pub dataset: DatasetInput,
    pub sample_size: u32,
    pub variance: f64,
    pub reproduction_count: u32,
    pub tags: Vec<String>,
    pub citation: Option<CitationInput>,
}

/// Metric score input
#[derive(Debug, Clone)]
pub struct MetricScoreInput {
    pub value: f64,
    pub unit: Option<String>,
    pub higher_is_better: bool,
    pub range: Option<(f64, f64)>,
}

/// Methodology input
#[derive(Debug, Clone)]
pub struct MethodologyInput {
    pub framework: String,
    pub evaluation_method: String,
    pub prompt_template_hash: Option<String>,
    pub scoring_method: String,
    pub normalized: bool,
    pub normalization_method: Option<String>,
}

/// Dataset input
#[derive(Debug, Clone)]
pub struct DatasetInput {
    pub dataset_id: String,
    pub dataset_version: String,
    pub subset: Option<String>,
    pub example_count: u32,
    pub split: String,
    pub publicly_available: bool,
}

/// Citation input
#[derive(Debug, Clone)]
pub struct CitationInput {
    pub doi: Option<String>,
    pub arxiv_id: Option<String>,
    pub bibtex: Option<String>,
    pub plain_text: String,
}

/// Request to validate a benchmark submission
#[derive(Debug, Clone)]
pub struct ValidateBenchmarkRequest {
    pub benchmark_id: String,
    pub model_provider: String,
    pub model_name: String,
    pub aggregate_score: f64,
    pub methodology: MethodologyInput,
    pub dataset: DatasetInput,
}

/// Request to update a publication
#[derive(Debug, Clone)]
pub struct UpdatePublicationRequest {
    pub tags: Option<Vec<String>>,
    pub citation: Option<CitationInput>,
}

/// Request to transition publication status
#[derive(Debug, Clone)]
pub struct TransitionStatusRequest {
    pub target_status: PublicationStatus,
    pub reason: Option<String>,
}

/// Query filters for publications
#[derive(Debug, Clone, Default)]
pub struct PublicationFilters {
    pub benchmark_id: Option<String>,
    pub model_provider: Option<String>,
    pub model_name: Option<String>,
    pub status: Option<PublicationStatus>,
    pub min_confidence: Option<f64>,
    pub tags: Option<Vec<String>>,
    pub published_after: Option<DateTime<Utc>>,
    pub published_before: Option<DateTime<Utc>>,
}

// =============================================================================
// Repository Port (for dependency injection)
// =============================================================================

/// Publication repository port - persistence abstraction
/// This port is implemented by the RuVectorClient adapter
#[async_trait]
pub trait PublicationRepositoryPort: Send + Sync {
    /// Store a new publication
    async fn create(&self, publication: &Publication) -> Result<String, ApplicationError>;

    /// Get publication by ID
    async fn get_by_id(&self, id: &str) -> Result<Option<Publication>, ApplicationError>;

    /// Update a publication
    async fn update(&self, publication: &Publication) -> Result<(), ApplicationError>;

    /// Delete a publication
    async fn delete(&self, id: &str) -> Result<(), ApplicationError>;

    /// List publications with filters
    async fn list(
        &self,
        filters: &PublicationFilters,
        pagination: &Pagination,
    ) -> Result<(Vec<Publication>, u64), ApplicationError>;

    /// Store a DecisionEvent (per constitution: exactly ONE per invocation)
    async fn store_decision_event(&self, event: &DecisionEvent) -> Result<String, ApplicationError>;

    /// Emit telemetry event (LLM-Observatory compatible)
    async fn emit_telemetry(
        &self,
        event_type: &str,
        duration_ms: u64,
        status: &str,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Result<(), ApplicationError>;
}

// =============================================================================
// Publication Authorizer
// =============================================================================

/// Publication-specific authorization
#[async_trait]
pub trait PublicationAuthorizer: Send + Sync {
    async fn can_publish(&self, ctx: &ServiceContext, benchmark_id: &str) -> super::AuthorizationResult;
    async fn can_update(&self, ctx: &ServiceContext, publication_id: &str) -> super::AuthorizationResult;
    async fn can_retract(&self, ctx: &ServiceContext, publication_id: &str) -> super::AuthorizationResult;
    async fn can_validate(&self, ctx: &ServiceContext) -> super::AuthorizationResult;
}

/// Default publication authorizer
pub struct DefaultPublicationAuthorizer;

#[async_trait]
impl PublicationAuthorizer for DefaultPublicationAuthorizer {
    async fn can_publish(&self, ctx: &ServiceContext, _benchmark_id: &str) -> super::AuthorizationResult {
        if ctx.user_id.is_some() {
            super::AuthorizationResult::allow()
        } else {
            super::AuthorizationResult::deny("Authentication required to publish benchmarks")
        }
    }

    async fn can_update(&self, ctx: &ServiceContext, _publication_id: &str) -> super::AuthorizationResult {
        if ctx.user_id.is_some() {
            super::AuthorizationResult::allow()
        } else {
            super::AuthorizationResult::deny("Authentication required to update publications")
        }
    }

    async fn can_retract(&self, ctx: &ServiceContext, _publication_id: &str) -> super::AuthorizationResult {
        if ctx.is_admin {
            super::AuthorizationResult::allow()
        } else {
            super::AuthorizationResult::deny("Admin privileges required to retract publications")
        }
    }

    async fn can_validate(&self, _ctx: &ServiceContext) -> super::AuthorizationResult {
        // Validation is allowed for everyone (read-only operation)
        super::AuthorizationResult::allow()
    }
}

// =============================================================================
// Publication Service Implementation
// =============================================================================

/// Benchmark Publication Agent Service
///
/// This service implements the core logic for the Benchmark Publication Agent.
/// All operations emit exactly ONE DecisionEvent and persist via ruvector-service.
pub struct PublicationService<R, A, E>
where
    R: PublicationRepositoryPort,
    A: PublicationAuthorizer,
    E: EventPublisher,
{
    repository: Arc<R>,
    authorizer: Arc<A>,
    event_publisher: Arc<E>,
    config: ServiceConfig,
}

impl<R, A, E> PublicationService<R, A, E>
where
    R: PublicationRepositoryPort,
    A: PublicationAuthorizer,
    E: EventPublisher,
{
    /// Create a new PublicationService
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
            config,
        }
    }

    /// Compute SHA256 hash of inputs for DecisionEvent
    fn compute_inputs_hash(inputs: &impl serde::Serialize) -> String {
        let json = serde_json::to_string(inputs).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Normalize a metric value to 0.0-1.0 range
    fn normalize_value(value: f64, range: Option<(f64, f64)>, higher_is_better: bool) -> f64 {
        match range {
            Some((min, max)) if max > min => {
                let normalized = (value - min) / (max - min);
                let clamped = normalized.clamp(0.0, 1.0);
                if higher_is_better {
                    clamped
                } else {
                    1.0 - clamped
                }
            }
            _ => {
                // If no range, assume value is already normalized or in 0-100 range
                if value > 1.0 {
                    (value / 100.0).clamp(0.0, 1.0)
                } else {
                    value.clamp(0.0, 1.0)
                }
            }
        }
    }

    /// Calculate reproducibility score from confidence metrics
    fn calculate_reproducibility(variance: f64, sample_size: u32, reproduction_count: u32) -> f64 {
        // Reproducibility factors:
        // 1. Low variance is good
        // 2. High sample size is good
        // 3. Multiple reproductions is good

        let variance_factor = 1.0 / (1.0 + variance);
        let sample_factor = (sample_size as f64).min(10000.0) / 10000.0;
        let reproduction_factor = (reproduction_count as f64).min(10.0) / 10.0;

        // Weighted combination
        let score = 0.4 * variance_factor + 0.3 * sample_factor + 0.3 * reproduction_factor;
        score.clamp(0.0, 1.0)
    }

    // =========================================================================
    // PUBLISH Operation
    // =========================================================================

    /// Publish a benchmark result
    ///
    /// This operation:
    /// 1. Validates the request
    /// 2. Normalizes metrics
    /// 3. Calculates confidence
    /// 4. Creates the publication
    /// 5. Emits exactly ONE DecisionEvent
    /// 6. Emits telemetry
    ///
    /// It does NOT:
    /// - Execute benchmarks
    /// - Invoke models
    /// - Enforce rankings
    #[instrument(skip(self, ctx, request), fields(correlation_id = %ctx.correlation_id))]
    pub async fn publish(
        &self,
        ctx: &ServiceContext,
        request: PublishBenchmarkRequest,
    ) -> ApplicationResult<PublicationDto> {
        let start = std::time::Instant::now();
        let execution_ref = ctx.correlation_id.clone();
        let _guard = ctx.execution_ctx.as_ref().map(|exec| exec.agent_guard("PublicationAgent"));

        // Authorization check
        let auth = self.authorizer.can_publish(ctx, &request.benchmark_id).await;
        auth.ensure_allowed()?;

        // Get user ID
        let user_id_str = ctx.require_authenticated()?;
        let user_id = UserId::from_uuid(
            Uuid::parse_str(user_id_str)
                .map_err(|_| ApplicationError::InvalidInput("Invalid user ID".to_string()))?,
        );

        // Parse benchmark ID
        let benchmark_id = BenchmarkId::from_uuid(
            Uuid::parse_str(&request.benchmark_id)
                .map_err(|_| ApplicationError::InvalidInput("Invalid benchmark ID".to_string()))?,
        );

        // Parse submission ID if provided
        let submission_id = request
            .submission_id
            .as_ref()
            .map(|s| {
                Uuid::parse_str(s)
                    .map(SubmissionId::from_uuid)
                    .map_err(|_| ApplicationError::InvalidInput("Invalid submission ID".to_string()))
            })
            .transpose()?;

        // Compute inputs hash for DecisionEvent
        let inputs_hash = Self::compute_inputs_hash(&request);

        // Normalize metrics
        let mut metric_scores = HashMap::new();
        for (name, input) in &request.metric_scores {
            let normalized = Self::normalize_value(input.value, input.range, input.higher_is_better);
            metric_scores.insert(
                name.clone(),
                MetricValue {
                    value: input.value,
                    normalized,
                    unit: input.unit.clone(),
                    higher_is_better: input.higher_is_better,
                    range: input.range,
                },
            );
        }

        // Calculate overall normalized score
        let normalized_score = if metric_scores.is_empty() {
            Self::normalize_value(request.aggregate_score, None, true)
        } else {
            let sum: f64 = metric_scores.values().map(|m| m.normalized).sum();
            sum / metric_scores.len() as f64
        };

        let metrics = NormalizedMetrics {
            aggregate_score: request.aggregate_score,
            normalized_score,
            metric_scores,
            percentile_rank: None, // Calculated later via batch process
            z_score: None,
        };

        // Calculate confidence
        let reproducibility_score = Self::calculate_reproducibility(
            request.variance,
            request.sample_size,
            request.reproduction_count,
        );

        let confidence = PublicationConfidence {
            reproducibility_score,
            sample_size: request.sample_size,
            variance: request.variance,
            std_dev: Some(request.variance.sqrt()),
            confidence_interval: None,
            reproduction_count: request.reproduction_count,
            coefficient_of_variation: if request.aggregate_score > 0.0 {
                Some(request.variance.sqrt() / request.aggregate_score)
            } else {
                None
            },
        };

        // Build constraints
        let constraints = PublicationConstraints {
            methodology: MethodologyConstraints {
                framework: request.methodology.framework,
                evaluation_method: request.methodology.evaluation_method,
                prompt_template_hash: request.methodology.prompt_template_hash,
                scoring_method: request.methodology.scoring_method,
                normalized: request.methodology.normalized,
                normalization_method: request.methodology.normalization_method,
            },
            dataset_scope: DatasetScopeConstraints {
                dataset_id: request.dataset.dataset_id,
                dataset_version: request.dataset.dataset_version,
                subset: request.dataset.subset,
                example_count: request.dataset.example_count,
                split: request.dataset.split,
                publicly_available: request.dataset.publicly_available,
            },
            model_version: ModelVersionConstraints {
                provider: request.model_provider.clone(),
                model_name: request.model_name.clone(),
                version: request.model_version.clone(),
                parameter_count: None,
                quantization: None,
                context_window: None,
            },
            custom_constraints: HashMap::new(),
        };

        // Build citation
        let citation = request.citation.map(|c| PublicationCitation {
            doi: c.doi,
            arxiv_id: c.arxiv_id,
            bibtex: c.bibtex,
            plain_text: c.plain_text,
        });

        // Create publication
        let now = Utc::now();
        let publication = Publication {
            id: PublicationId::new(),
            benchmark_id,
            submission_id,
            status: PublicationStatus::Draft,
            version: SemanticVersion::new(1, 0, 0),
            model_provider: request.model_provider,
            model_name: request.model_name,
            model_version: request.model_version,
            metrics: metrics.clone(),
            confidence: confidence.clone(),
            constraints: constraints.clone(),
            published_by: user_id,
            organization_id: ctx.organization_id.clone(),
            created_at: now,
            updated_at: now,
            published_at: None,
            citation,
            tags: request.tags,
            is_latest: true,
            previous_version_id: None,
        };

        // Store publication via ruvector-service
        let publication_id = self.repository.create(&publication).await?;

        info!(
            publication_id = %publication_id,
            benchmark_id = %publication.benchmark_id,
            model = %publication.model_name,
            "Publication created"
        );

        // Create DecisionEvent outputs
        let outputs = DecisionOutputs {
            publication_id: Some(publication.id),
            status: "created".to_string(),
            normalized_metrics: Some(metrics),
            validation_results: None,
            metadata: HashMap::new(),
        };

        // Emit DecisionEvent (exactly ONE per invocation - per constitution)
        let decision_event = DecisionEvent::new(
            PublicationDecisionType::BenchmarkPublish,
            inputs_hash,
            outputs,
            confidence,
            constraints,
            execution_ref.clone(),
        );

        let event_id = self.repository.store_decision_event(&decision_event).await?;
        debug!(event_id = %event_id, "DecisionEvent stored");

        // Emit service event
        self.event_publisher
            .publish(ServiceEvent::BenchmarkCreated {
                benchmark_id: publication.benchmark_id.to_string(),
            })
            .await?;

        // Emit telemetry (LLM-Observatory compatible)
        let duration_ms = start.elapsed().as_millis() as u64;
        let mut telemetry_metadata = HashMap::new();
        telemetry_metadata.insert(
            "publication_id".to_string(),
            serde_json::Value::String(publication.id.to_string()),
        );
        telemetry_metadata.insert(
            "benchmark_id".to_string(),
            serde_json::Value::String(publication.benchmark_id.to_string()),
        );

        self.repository
            .emit_telemetry(
                "publication.created",
                duration_ms,
                "success",
                telemetry_metadata,
            )
            .await
            .ok(); // Don't fail on telemetry errors

        if let Some(guard) = _guard {
            guard.attach_artifact(ExecArtifact::new("publication_created", &publication.id.to_string()));
            guard.complete();
        }

        Ok(publication.into())
    }

    // =========================================================================
    // VALIDATE Operation
    // =========================================================================

    /// Validate a benchmark submission without publishing
    ///
    /// This operation performs validation only and does NOT:
    /// - Execute benchmarks
    /// - Invoke models
    /// - Modify any data
    #[instrument(skip(self, ctx, request), fields(correlation_id = %ctx.correlation_id))]
    pub async fn validate(
        &self,
        ctx: &ServiceContext,
        request: ValidateBenchmarkRequest,
    ) -> ApplicationResult<ValidationResults> {
        let start = std::time::Instant::now();
        let execution_ref = ctx.correlation_id.clone();
        let _guard = ctx.execution_ctx.as_ref().map(|exec| exec.agent_guard("PublicationAgent"));
        let benchmark_id_ref = request.benchmark_id.clone();

        // Authorization check
        let auth = self.authorizer.can_validate(ctx).await;
        auth.ensure_allowed()?;

        let inputs_hash = Self::compute_inputs_hash(&request);
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate benchmark ID
        if Uuid::parse_str(&request.benchmark_id).is_err() {
            errors.push(ValidationError {
                code: "INVALID_BENCHMARK_ID".to_string(),
                message: "Benchmark ID is not a valid UUID".to_string(),
                field: Some("benchmark_id".to_string()),
            });
        }

        // Validate score range
        if request.aggregate_score < 0.0 || request.aggregate_score > 100.0 {
            warnings.push(ValidationWarning {
                code: "SCORE_OUT_OF_RANGE".to_string(),
                message: "Aggregate score is outside typical 0-100 range".to_string(),
                field: Some("aggregate_score".to_string()),
            });
        }

        // Validate methodology
        if request.methodology.framework.is_empty() {
            errors.push(ValidationError {
                code: "MISSING_FRAMEWORK".to_string(),
                message: "Evaluation framework must be specified".to_string(),
                field: Some("methodology.framework".to_string()),
            });
        }

        // Validate dataset
        if request.dataset.dataset_id.is_empty() {
            errors.push(ValidationError {
                code: "MISSING_DATASET_ID".to_string(),
                message: "Dataset ID must be specified".to_string(),
                field: Some("dataset.dataset_id".to_string()),
            });
        }

        if request.dataset.example_count == 0 {
            warnings.push(ValidationWarning {
                code: "ZERO_EXAMPLES".to_string(),
                message: "Example count is zero".to_string(),
                field: Some("dataset.example_count".to_string()),
            });
        }

        // Calculate validation score
        let error_penalty = errors.len() as f64 * 0.2;
        let warning_penalty = warnings.len() as f64 * 0.05;
        let score = (1.0 - error_penalty - warning_penalty).max(0.0);

        let results = ValidationResults {
            passed: errors.is_empty(),
            errors,
            warnings,
            score,
        };

        // Build minimal constraints for DecisionEvent
        let constraints = PublicationConstraints {
            methodology: MethodologyConstraints {
                framework: request.methodology.framework,
                evaluation_method: request.methodology.evaluation_method,
                prompt_template_hash: request.methodology.prompt_template_hash,
                scoring_method: request.methodology.scoring_method,
                normalized: request.methodology.normalized,
                normalization_method: request.methodology.normalization_method,
            },
            dataset_scope: DatasetScopeConstraints {
                dataset_id: request.dataset.dataset_id,
                dataset_version: request.dataset.dataset_version,
                subset: request.dataset.subset,
                example_count: request.dataset.example_count,
                split: request.dataset.split,
                publicly_available: request.dataset.publicly_available,
            },
            model_version: ModelVersionConstraints {
                provider: request.model_provider,
                model_name: request.model_name,
                version: "validation-only".to_string(),
                parameter_count: None,
                quantization: None,
                context_window: None,
            },
            custom_constraints: HashMap::new(),
        };

        // Emit DecisionEvent (exactly ONE per invocation)
        let outputs = DecisionOutputs {
            publication_id: None,
            status: if results.passed { "valid" } else { "invalid" }.to_string(),
            normalized_metrics: None,
            validation_results: Some(results.clone()),
            metadata: HashMap::new(),
        };

        let decision_event = DecisionEvent::new(
            PublicationDecisionType::BenchmarkValidate,
            inputs_hash,
            outputs,
            PublicationConfidence::default(),
            constraints,
            execution_ref,
        );

        self.repository.store_decision_event(&decision_event).await?;

        // Emit telemetry
        let duration_ms = start.elapsed().as_millis() as u64;
        let mut telemetry_metadata = HashMap::new();
        telemetry_metadata.insert(
            "validation_passed".to_string(),
            serde_json::Value::Bool(results.passed),
        );
        telemetry_metadata.insert(
            "error_count".to_string(),
            serde_json::Value::Number(results.errors.len().into()),
        );

        self.repository
            .emit_telemetry("publication.validated", duration_ms, "success", telemetry_metadata)
            .await
            .ok();

        if let Some(guard) = _guard {
            guard.attach_artifact(ExecArtifact::new("publication_validated", &benchmark_id_ref));
            guard.complete();
        }

        Ok(results)
    }

    // =========================================================================
    // GET/LIST Operations
    // =========================================================================

    /// Get a publication by ID
    #[instrument(skip(self, ctx), fields(correlation_id = %ctx.correlation_id))]
    pub async fn get_by_id(
        &self,
        ctx: &ServiceContext,
        id: &str,
    ) -> ApplicationResult<Option<PublicationDto>> {
        let _guard = ctx.execution_ctx.as_ref().map(|exec| exec.agent_guard("PublicationAgent"));
        let publication = self.repository.get_by_id(id).await?;
        if let Some(guard) = _guard { guard.complete(); }
        Ok(publication.map(|p| p.into()))
    }

    /// List publications with filters
    #[instrument(skip(self, ctx), fields(correlation_id = %ctx.correlation_id))]
    pub async fn list(
        &self,
        ctx: &ServiceContext,
        filters: PublicationFilters,
        pagination: Pagination,
    ) -> ApplicationResult<PaginatedResult<PublicationDto>> {
        let _guard = ctx.execution_ctx.as_ref().map(|exec| exec.agent_guard("PublicationAgent"));
        let pagination = Pagination::new(
            pagination.page.max(1),
            pagination.page_size.min(self.config.max_page_size),
        );

        let (publications, total) = self.repository.list(&filters, &pagination).await?;
        let items: Vec<PublicationDto> = publications.into_iter().map(|p| p.into()).collect();

        if let Some(guard) = _guard { guard.complete(); }
        Ok(PaginatedResult::new(items, total, &pagination))
    }

    // =========================================================================
    // UPDATE Operation
    // =========================================================================

    /// Update a publication
    #[instrument(skip(self, ctx, request), fields(correlation_id = %ctx.correlation_id))]
    pub async fn update(
        &self,
        ctx: &ServiceContext,
        id: &str,
        request: UpdatePublicationRequest,
    ) -> ApplicationResult<PublicationDto> {
        let start = std::time::Instant::now();
        let execution_ref = ctx.correlation_id.clone();
        let _guard = ctx.execution_ctx.as_ref().map(|exec| exec.agent_guard("PublicationAgent"));

        // Authorization check
        let auth = self.authorizer.can_update(ctx, id).await;
        auth.ensure_allowed()?;

        // Get existing publication
        let mut publication = self
            .repository
            .get_by_id(id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound(format!("Publication not found: {}", id)))?;

        let inputs_hash = Self::compute_inputs_hash(&request);

        // Apply updates
        if let Some(tags) = request.tags {
            publication.tags = tags;
        }

        if let Some(citation_input) = request.citation {
            publication.citation = Some(PublicationCitation {
                doi: citation_input.doi,
                arxiv_id: citation_input.arxiv_id,
                bibtex: citation_input.bibtex,
                plain_text: citation_input.plain_text,
            });
        }

        publication.updated_at = Utc::now();

        // Store update
        self.repository.update(&publication).await?;

        info!(publication_id = %id, "Publication updated");

        // Emit DecisionEvent
        let outputs = DecisionOutputs {
            publication_id: Some(publication.id),
            status: "updated".to_string(),
            normalized_metrics: None,
            validation_results: None,
            metadata: HashMap::new(),
        };

        let decision_event = DecisionEvent::new(
            PublicationDecisionType::BenchmarkUpdate,
            inputs_hash,
            outputs,
            publication.confidence.clone(),
            publication.constraints.clone(),
            execution_ref,
        );

        self.repository.store_decision_event(&decision_event).await?;

        // Emit telemetry
        let duration_ms = start.elapsed().as_millis() as u64;
        let mut telemetry_metadata = HashMap::new();
        telemetry_metadata.insert(
            "publication_id".to_string(),
            serde_json::Value::String(id.to_string()),
        );

        self.repository
            .emit_telemetry("publication.updated", duration_ms, "success", telemetry_metadata)
            .await
            .ok();

        if let Some(guard) = _guard {
            guard.attach_artifact(ExecArtifact::new("publication_updated", id));
            guard.complete();
        }

        Ok(publication.into())
    }

    // =========================================================================
    // STATUS TRANSITION Operation
    // =========================================================================

    /// Transition publication status
    #[instrument(skip(self, ctx, request), fields(correlation_id = %ctx.correlation_id))]
    pub async fn transition_status(
        &self,
        ctx: &ServiceContext,
        id: &str,
        request: TransitionStatusRequest,
    ) -> ApplicationResult<PublicationDto> {
        let start = std::time::Instant::now();
        let execution_ref = ctx.correlation_id.clone();
        let _guard = ctx.execution_ctx.as_ref().map(|exec| exec.agent_guard("PublicationAgent"));

        // Get existing publication
        let mut publication = self
            .repository
            .get_by_id(id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound(format!("Publication not found: {}", id)))?;

        // Validate transition
        if !publication.status.can_transition_to(request.target_status) {
            return Err(ApplicationError::InvalidInput(format!(
                "Cannot transition from {:?} to {:?}",
                publication.status, request.target_status
            )));
        }

        // Authorization for retractions
        if request.target_status == PublicationStatus::Retracted {
            let auth = self.authorizer.can_retract(ctx, id).await;
            auth.ensure_allowed()?;
        }

        let old_status = publication.status;
        publication.status = request.target_status;
        publication.updated_at = Utc::now();

        if request.target_status == PublicationStatus::Published {
            publication.published_at = Some(Utc::now());
        }

        // Store update
        self.repository.update(&publication).await?;

        info!(
            publication_id = %id,
            old_status = ?old_status,
            new_status = ?request.target_status,
            "Publication status transitioned"
        );

        // Determine decision type based on transition
        let decision_type = match request.target_status {
            PublicationStatus::Retracted => PublicationDecisionType::BenchmarkRetract,
            _ => PublicationDecisionType::BenchmarkUpdate,
        };

        // Emit DecisionEvent
        let outputs = DecisionOutputs {
            publication_id: Some(publication.id),
            status: format!("{:?}", request.target_status).to_lowercase(),
            normalized_metrics: None,
            validation_results: None,
            metadata: {
                let mut m = HashMap::new();
                m.insert(
                    "old_status".to_string(),
                    serde_json::Value::String(format!("{:?}", old_status)),
                );
                if let Some(reason) = &request.reason {
                    m.insert("reason".to_string(), serde_json::Value::String(reason.clone()));
                }
                m
            },
        };

        let inputs_hash = Self::compute_inputs_hash(&request);
        let decision_event = DecisionEvent::new(
            decision_type,
            inputs_hash,
            outputs,
            publication.confidence.clone(),
            publication.constraints.clone(),
            execution_ref,
        );

        self.repository.store_decision_event(&decision_event).await?;

        // Emit telemetry
        let duration_ms = start.elapsed().as_millis() as u64;
        let mut telemetry_metadata = HashMap::new();
        telemetry_metadata.insert(
            "publication_id".to_string(),
            serde_json::Value::String(id.to_string()),
        );
        telemetry_metadata.insert(
            "new_status".to_string(),
            serde_json::Value::String(format!("{:?}", request.target_status)),
        );

        self.repository
            .emit_telemetry(
                "publication.status_changed",
                duration_ms,
                "success",
                telemetry_metadata,
            )
            .await
            .ok();

        if let Some(guard) = _guard {
            guard.attach_artifact(ExecArtifact::new("publication_status_changed", id));
            guard.complete();
        }

        Ok(publication.into())
    }

    // =========================================================================
    // INSPECT Operation (read-only)
    // =========================================================================

    /// Inspect publication with full metadata
    /// This is a read-only operation that emits telemetry but no DecisionEvent
    #[instrument(skip(self, ctx), fields(correlation_id = %ctx.correlation_id))]
    pub async fn inspect(
        &self,
        ctx: &ServiceContext,
        id: &str,
    ) -> ApplicationResult<Publication> {
        let start = std::time::Instant::now();
        let _guard = ctx.execution_ctx.as_ref().map(|exec| exec.agent_guard("PublicationAgent"));

        let publication = self
            .repository
            .get_by_id(id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound(format!("Publication not found: {}", id)))?;

        // Emit telemetry
        let duration_ms = start.elapsed().as_millis() as u64;
        let mut telemetry_metadata = HashMap::new();
        telemetry_metadata.insert(
            "publication_id".to_string(),
            serde_json::Value::String(id.to_string()),
        );

        self.repository
            .emit_telemetry("publication.inspected", duration_ms, "success", telemetry_metadata)
            .await
            .ok();

        if let Some(guard) = _guard { guard.complete(); }
        Ok(publication)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_value() {
        // Test with range
        assert!((PublicationService::<(), (), ()>::normalize_value(50.0, Some((0.0, 100.0)), true) - 0.5).abs() < 0.001);
        assert!((PublicationService::<(), (), ()>::normalize_value(50.0, Some((0.0, 100.0)), false) - 0.5).abs() < 0.001);

        // Test without range (percentage)
        assert!((PublicationService::<(), (), ()>::normalize_value(75.0, None, true) - 0.75).abs() < 0.001);

        // Test clamping
        assert_eq!(PublicationService::<(), (), ()>::normalize_value(150.0, Some((0.0, 100.0)), true), 1.0);
    }

    #[test]
    fn test_calculate_reproducibility() {
        let score = PublicationService::<(), (), ()>::calculate_reproducibility(0.01, 10000, 5);
        assert!(score > 0.8); // High sample size, multiple reproductions, low variance

        let score = PublicationService::<(), (), ()>::calculate_reproducibility(1.0, 10, 1);
        assert!(score < 0.5); // Low sample size, single reproduction, high variance
    }
}

// Placeholder types for test compilation
#[cfg(test)]
impl<R, A, E> PublicationService<R, A, E>
where
    R: PublicationRepositoryPort,
    A: PublicationAuthorizer,
    E: EventPublisher,
{
}
