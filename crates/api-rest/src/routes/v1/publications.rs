//! Publication Agent REST API endpoints
//!
//! This module provides the REST API for the Benchmark Publication Agent.
//! All endpoints are designed for deployment as Google Cloud Edge Functions.
//!
//! ## Agent Classification: BENCHMARK PUBLICATION
//!
//! ## Exposed Operations:
//! - POST /publications - Publish a new benchmark result
//! - POST /publications/validate - Validate without publishing
//! - GET /publications - List publications
//! - GET /publications/:id - Get publication by ID
//! - GET /publications/:id/inspect - Inspect with full metadata
//! - PUT /publications/:id - Update publication
//! - POST /publications/:id/status - Transition status
//!
//! ## Consumed By:
//! - LLM-Registry (MAY reference benchmark metadata)
//! - LLM-Observatory (MAY consume benchmark outputs)
//! - LLM-Orchestrator (MAY consume for planning only)
//! - Governance and audit systems

use crate::{
    error::{ApiError, ApiResult},
    extractors::{AuthenticatedUser, OptionalExecutionContext, Pagination, ValidatedJson, build_service_context},
    responses::{ApiResponse, InstrumentedPaginatedResponse, InstrumentedResponse, PaginatedResponse},
    state::AppState,
};
use axum::{
    extract::{Path, Query, State},
    routing::{get, post, put},
    Router,
};
use chrono::{DateTime, Utc};
use llm_benchmark_application::services::{
    PublicationDto, PublishBenchmarkRequest, ValidateBenchmarkRequest, UpdatePublicationRequest,
    TransitionStatusRequest, PublicationFilters, Pagination as ServicePagination,
    MetricScoreInput, MethodologyInput, DatasetInput, CitationInput,
};
use llm_benchmark_domain::publication::{
    PublicationStatus, ValidationResults, Publication, ConfidenceLevel,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use validator::Validate;

// =============================================================================
// API Request/Response Types
// =============================================================================

/// Publication list item (summary view)
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PublicationListItem {
    pub id: String,
    pub benchmark_id: String,
    pub status: PublicationStatus,
    pub model_provider: String,
    pub model_name: String,
    pub model_version: String,
    pub aggregate_score: f64,
    pub normalized_score: f64,
    pub confidence_level: ConfidenceLevel,
    pub is_latest: bool,
    pub created_at: String,
    pub published_at: Option<String>,
}

impl From<PublicationDto> for PublicationListItem {
    fn from(dto: PublicationDto) -> Self {
        Self {
            id: dto.id,
            benchmark_id: dto.benchmark_id,
            status: dto.status,
            model_provider: dto.model_provider,
            model_name: dto.model_name,
            model_version: dto.model_version,
            aggregate_score: dto.aggregate_score,
            normalized_score: dto.normalized_score,
            confidence_level: dto.confidence_level,
            is_latest: dto.is_latest,
            created_at: dto.created_at.to_rfc3339(),
            published_at: dto.published_at.map(|t| t.to_rfc3339()),
        }
    }
}

/// Publication detail response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PublicationDetail {
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
    pub created_at: String,
    pub updated_at: String,
    pub published_at: Option<String>,
}

impl From<PublicationDto> for PublicationDetail {
    fn from(dto: PublicationDto) -> Self {
        Self {
            id: dto.id,
            benchmark_id: dto.benchmark_id,
            submission_id: dto.submission_id,
            status: dto.status,
            version: dto.version,
            model_provider: dto.model_provider,
            model_name: dto.model_name,
            model_version: dto.model_version,
            aggregate_score: dto.aggregate_score,
            normalized_score: dto.normalized_score,
            confidence_level: dto.confidence_level,
            reproducibility_score: dto.reproducibility_score,
            published_by: dto.published_by,
            organization_id: dto.organization_id,
            tags: dto.tags,
            is_latest: dto.is_latest,
            created_at: dto.created_at.to_rfc3339(),
            updated_at: dto.updated_at.to_rfc3339(),
            published_at: dto.published_at.map(|t| t.to_rfc3339()),
        }
    }
}

/// Validation response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ValidationResponse {
    pub passed: bool,
    pub score: f64,
    pub errors: Vec<ValidationErrorResponse>,
    pub warnings: Vec<ValidationWarningResponse>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ValidationErrorResponse {
    pub code: String,
    pub message: String,
    pub field: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ValidationWarningResponse {
    pub code: String,
    pub message: String,
    pub field: Option<String>,
}

impl From<ValidationResults> for ValidationResponse {
    fn from(results: ValidationResults) -> Self {
        Self {
            passed: results.passed,
            score: results.score,
            errors: results
                .errors
                .into_iter()
                .map(|e| ValidationErrorResponse {
                    code: e.code,
                    message: e.message,
                    field: e.field,
                })
                .collect(),
            warnings: results
                .warnings
                .into_iter()
                .map(|w| ValidationWarningResponse {
                    code: w.code,
                    message: w.message,
                    field: w.field,
                })
                .collect(),
        }
    }
}

// =============================================================================
// API Request Types
// =============================================================================

/// Publish benchmark request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct PublishBenchmarkApiRequest {
    #[validate(length(min = 1))]
    pub benchmark_id: String,

    pub submission_id: Option<String>,

    #[validate(length(min = 1, max = 100))]
    pub model_provider: String,

    #[validate(length(min = 1, max = 200))]
    pub model_name: String,

    #[validate(length(min = 1, max = 100))]
    pub model_version: String,

    #[validate(range(min = 0.0))]
    pub aggregate_score: f64,

    pub metric_scores: HashMap<String, MetricScoreApiInput>,

    pub methodology: MethodologyApiInput,

    pub dataset: DatasetApiInput,

    #[validate(range(min = 1))]
    pub sample_size: u32,

    #[validate(range(min = 0.0))]
    pub variance: f64,

    #[validate(range(min = 1))]
    pub reproduction_count: u32,

    #[serde(default)]
    pub tags: Vec<String>,

    pub citation: Option<CitationApiInput>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct MetricScoreApiInput {
    pub value: f64,
    pub unit: Option<String>,
    #[serde(default = "default_true")]
    pub higher_is_better: bool,
    pub range: Option<(f64, f64)>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct MethodologyApiInput {
    #[validate(length(min = 1, max = 100))]
    pub framework: String,

    #[validate(length(min = 1, max = 100))]
    pub evaluation_method: String,

    pub prompt_template_hash: Option<String>,

    #[validate(length(min = 1, max = 100))]
    pub scoring_method: String,

    #[serde(default)]
    pub normalized: bool,

    pub normalization_method: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct DatasetApiInput {
    #[validate(length(min = 1, max = 200))]
    pub dataset_id: String,

    #[validate(length(min = 1, max = 100))]
    pub dataset_version: String,

    pub subset: Option<String>,

    pub example_count: u32,

    #[validate(length(min = 1, max = 50))]
    pub split: String,

    #[serde(default)]
    pub publicly_available: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CitationApiInput {
    pub doi: Option<String>,
    pub arxiv_id: Option<String>,
    pub bibtex: Option<String>,
    pub plain_text: String,
}

/// Validate benchmark request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ValidateBenchmarkApiRequest {
    #[validate(length(min = 1))]
    pub benchmark_id: String,

    #[validate(length(min = 1, max = 100))]
    pub model_provider: String,

    #[validate(length(min = 1, max = 200))]
    pub model_name: String,

    #[validate(range(min = 0.0))]
    pub aggregate_score: f64,

    pub methodology: MethodologyApiInput,

    pub dataset: DatasetApiInput,
}

/// Update publication request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdatePublicationApiRequest {
    pub tags: Option<Vec<String>>,
    pub citation: Option<CitationApiInput>,
}

/// Status transition request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct TransitionStatusApiRequest {
    pub target_status: PublicationStatus,

    #[validate(length(max = 1000))]
    pub reason: Option<String>,
}

/// Query parameters for listing publications
#[derive(Debug, Deserialize, ToSchema)]
pub struct PublicationListQuery {
    pub benchmark_id: Option<String>,
    pub model_provider: Option<String>,
    pub model_name: Option<String>,
    pub status: Option<PublicationStatus>,
    pub min_confidence: Option<f64>,
    pub tags: Option<String>,
    pub published_after: Option<String>,
    pub published_before: Option<String>,
}

// =============================================================================
// Routes
// =============================================================================

/// Publication routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/publications", get(list_publications).post(publish_benchmark))
        .route("/publications/validate", post(validate_benchmark))
        .route(
            "/publications/:id",
            get(get_publication).put(update_publication),
        )
        .route("/publications/:id/inspect", get(inspect_publication))
        .route("/publications/:id/status", post(transition_status))
}

// =============================================================================
// Handlers
// =============================================================================

/// List publications
///
/// Returns a paginated list of benchmark publications with optional filtering.
#[utoipa::path(
    get,
    path = "/publications",
    tag = "publications",
    params(
        ("page" = Option<u32>, Query, description = "Page number (1-indexed)"),
        ("per_page" = Option<u32>, Query, description = "Items per page"),
        ("benchmark_id" = Option<String>, Query, description = "Filter by benchmark ID"),
        ("model_provider" = Option<String>, Query, description = "Filter by model provider"),
        ("model_name" = Option<String>, Query, description = "Filter by model name"),
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("min_confidence" = Option<f64>, Query, description = "Minimum confidence score"),
    ),
    responses(
        (status = 200, description = "List of publications", body = PaginatedResponse<PublicationListItem>)
    )
)]
async fn list_publications(
    State(state): State<AppState>,
    pagination: Pagination,
    Query(query): Query<PublicationListQuery>,
    exec: OptionalExecutionContext,
) -> ApiResult<InstrumentedPaginatedResponse<PublicationListItem>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let exec_ctx = exec.0;
    let ctx = build_service_context(None, &request_id, exec_ctx.clone());

    let filters = PublicationFilters {
        benchmark_id: query.benchmark_id,
        model_provider: query.model_provider,
        model_name: query.model_name,
        status: query.status,
        min_confidence: query.min_confidence,
        tags: query.tags.map(|t| t.split(',').map(|s| s.trim().to_string()).collect()),
        published_after: query
            .published_after
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|d| d.with_timezone(&Utc)),
        published_before: query
            .published_before
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|d| d.with_timezone(&Utc)),
    };

    let service_pagination = ServicePagination::new(
        pagination.params.page,
        pagination.params.per_page,
    );

    let result = state
        .publication_service
        .list(&ctx, filters, service_pagination)
        .await?;

    let items: Vec<PublicationListItem> = result.items.into_iter().map(Into::into).collect();

    let paginated = llm_benchmark_common::pagination::PaginatedResult::new(
        items,
        result.page,
        result.page_size,
        result.total,
    );

    let execution = exec_ctx.and_then(|ec| ec.finalize().ok());
    Ok(InstrumentedPaginatedResponse::new(paginated.into(), execution))
}

/// Publish benchmark result
///
/// Publish a new benchmark result. This creates a publication in Draft status.
/// The publication must be validated and transitioned to Published status.
#[utoipa::path(
    post,
    path = "/publications",
    tag = "publications",
    request_body = PublishBenchmarkApiRequest,
    responses(
        (status = 201, description = "Publication created", body = PublicationDetail),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 422, description = "Validation failed"),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
async fn publish_benchmark(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    ValidatedJson(req): ValidatedJson<PublishBenchmarkApiRequest>,
    exec: OptionalExecutionContext,
) -> ApiResult<(axum::http::StatusCode, InstrumentedResponse<PublicationDetail>)> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let exec_ctx = exec.0;
    let ctx = build_service_context(Some(&user), &request_id, exec_ctx.clone());

    let request = PublishBenchmarkRequest {
        benchmark_id: req.benchmark_id,
        submission_id: req.submission_id,
        model_provider: req.model_provider,
        model_name: req.model_name,
        model_version: req.model_version,
        aggregate_score: req.aggregate_score,
        metric_scores: req
            .metric_scores
            .into_iter()
            .map(|(k, v)| {
                (
                    k,
                    MetricScoreInput {
                        value: v.value,
                        unit: v.unit,
                        higher_is_better: v.higher_is_better,
                        range: v.range,
                    },
                )
            })
            .collect(),
        methodology: MethodologyInput {
            framework: req.methodology.framework,
            evaluation_method: req.methodology.evaluation_method,
            prompt_template_hash: req.methodology.prompt_template_hash,
            scoring_method: req.methodology.scoring_method,
            normalized: req.methodology.normalized,
            normalization_method: req.methodology.normalization_method,
        },
        dataset: DatasetInput {
            dataset_id: req.dataset.dataset_id,
            dataset_version: req.dataset.dataset_version,
            subset: req.dataset.subset,
            example_count: req.dataset.example_count,
            split: req.dataset.split,
            publicly_available: req.dataset.publicly_available,
        },
        sample_size: req.sample_size,
        variance: req.variance,
        reproduction_count: req.reproduction_count,
        tags: req.tags,
        citation: req.citation.map(|c| CitationInput {
            doi: c.doi,
            arxiv_id: c.arxiv_id,
            bibtex: c.bibtex,
            plain_text: c.plain_text,
        }),
    };

    let publication = state.publication_service.publish(&ctx, request).await?;

    let execution = exec_ctx.and_then(|ec| ec.finalize().ok());
    Ok((axum::http::StatusCode::CREATED, InstrumentedResponse::new(
        ApiResponse::success(publication.into()),
        execution,
    )))
}

/// Validate benchmark submission
///
/// Validate a benchmark submission without publishing.
/// Returns validation results including errors and warnings.
#[utoipa::path(
    post,
    path = "/publications/validate",
    tag = "publications",
    request_body = ValidateBenchmarkApiRequest,
    responses(
        (status = 200, description = "Validation results", body = ValidationResponse),
        (status = 400, description = "Invalid request"),
    )
)]
async fn validate_benchmark(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<ValidateBenchmarkApiRequest>,
    exec: OptionalExecutionContext,
) -> ApiResult<InstrumentedResponse<ValidationResponse>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let exec_ctx = exec.0;
    let ctx = build_service_context(None, &request_id, exec_ctx.clone());

    let request = ValidateBenchmarkRequest {
        benchmark_id: req.benchmark_id,
        model_provider: req.model_provider,
        model_name: req.model_name,
        aggregate_score: req.aggregate_score,
        methodology: MethodologyInput {
            framework: req.methodology.framework,
            evaluation_method: req.methodology.evaluation_method,
            prompt_template_hash: req.methodology.prompt_template_hash,
            scoring_method: req.methodology.scoring_method,
            normalized: req.methodology.normalized,
            normalization_method: req.methodology.normalization_method,
        },
        dataset: DatasetInput {
            dataset_id: req.dataset.dataset_id,
            dataset_version: req.dataset.dataset_version,
            subset: req.dataset.subset,
            example_count: req.dataset.example_count,
            split: req.dataset.split,
            publicly_available: req.dataset.publicly_available,
        },
    };

    let results = state.publication_service.validate(&ctx, request).await?;

    let execution = exec_ctx.and_then(|ec| ec.finalize().ok());
    Ok(InstrumentedResponse::new(
        ApiResponse::success(results.into()),
        execution,
    ))
}

/// Get publication by ID
///
/// Retrieve detailed information about a specific publication.
#[utoipa::path(
    get,
    path = "/publications/{id}",
    tag = "publications",
    params(
        ("id" = String, Path, description = "Publication ID"),
    ),
    responses(
        (status = 200, description = "Publication details", body = PublicationDetail),
        (status = 404, description = "Publication not found"),
    )
)]
async fn get_publication(
    State(state): State<AppState>,
    Path(id): Path<String>,
    exec: OptionalExecutionContext,
) -> ApiResult<InstrumentedResponse<PublicationDetail>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let exec_ctx = exec.0;
    let ctx = build_service_context(None, &request_id, exec_ctx.clone());

    let publication = state
        .publication_service
        .get_by_id(&ctx, &id)
        .await?
        .ok_or(ApiError::NotFound)?;

    let execution = exec_ctx.and_then(|ec| ec.finalize().ok());
    Ok(InstrumentedResponse::new(
        ApiResponse::success(publication.into()),
        execution,
    ))
}

/// Inspect publication with full metadata
///
/// Retrieve full publication details including all metadata, constraints, and confidence metrics.
#[utoipa::path(
    get,
    path = "/publications/{id}/inspect",
    tag = "publications",
    params(
        ("id" = String, Path, description = "Publication ID"),
    ),
    responses(
        (status = 200, description = "Full publication metadata"),
        (status = 404, description = "Publication not found"),
    )
)]
async fn inspect_publication(
    State(state): State<AppState>,
    Path(id): Path<String>,
    exec: OptionalExecutionContext,
) -> ApiResult<InstrumentedResponse<Publication>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let exec_ctx = exec.0;
    let ctx = build_service_context(None, &request_id, exec_ctx.clone());

    let publication = state.publication_service.inspect(&ctx, &id).await?;

    let execution = exec_ctx.and_then(|ec| ec.finalize().ok());
    Ok(InstrumentedResponse::new(
        ApiResponse::success(publication),
        execution,
    ))
}

/// Update publication
///
/// Update publication metadata (tags, citation).
#[utoipa::path(
    put,
    path = "/publications/{id}",
    tag = "publications",
    params(
        ("id" = String, Path, description = "Publication ID"),
    ),
    request_body = UpdatePublicationApiRequest,
    responses(
        (status = 200, description = "Publication updated", body = PublicationDetail),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Publication not found"),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
async fn update_publication(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<UpdatePublicationApiRequest>,
    exec: OptionalExecutionContext,
) -> ApiResult<InstrumentedResponse<PublicationDetail>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let exec_ctx = exec.0;
    let ctx = build_service_context(Some(&user), &request_id, exec_ctx.clone());

    let request = UpdatePublicationRequest {
        tags: req.tags,
        citation: req.citation.map(|c| CitationInput {
            doi: c.doi,
            arxiv_id: c.arxiv_id,
            bibtex: c.bibtex,
            plain_text: c.plain_text,
        }),
    };

    let publication = state.publication_service.update(&ctx, &id, request).await?;

    let execution = exec_ctx.and_then(|ec| ec.finalize().ok());
    Ok(InstrumentedResponse::new(
        ApiResponse::success(publication.into()),
        execution,
    ))
}

/// Transition publication status
///
/// Change the publication status (e.g., Draft -> PendingValidation -> Published).
#[utoipa::path(
    post,
    path = "/publications/{id}/status",
    tag = "publications",
    params(
        ("id" = String, Path, description = "Publication ID"),
    ),
    request_body = TransitionStatusApiRequest,
    responses(
        (status = 200, description = "Status transitioned", body = PublicationDetail),
        (status = 400, description = "Invalid transition"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Publication not found"),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
async fn transition_status(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<TransitionStatusApiRequest>,
    exec: OptionalExecutionContext,
) -> ApiResult<InstrumentedResponse<PublicationDetail>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let exec_ctx = exec.0;
    let ctx = build_service_context(Some(&user), &request_id, exec_ctx.clone());

    let request = TransitionStatusRequest {
        target_status: req.target_status,
        reason: req.reason,
    };

    let publication = state
        .publication_service
        .transition_status(&ctx, &id, request)
        .await?;

    let execution = exec_ctx.and_then(|ec| ec.finalize().ok());
    Ok(InstrumentedResponse::new(
        ApiResponse::success(publication.into()),
        execution,
    ))
}
