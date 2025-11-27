//! Submission endpoints.

use crate::{
    error::{ApiError, ApiResult},
    extractors::{AuthenticatedUser, Pagination, ValidatedJson},
    responses::{ApiResponse, Created, NoContent, PaginatedResponse},
    state::AppState,
};
use axum::{
    extract::{Path, State},
    routing::{get, patch, post},
    Json, Router,
};
use llm_benchmark_domain::{
    identifiers::{BenchmarkId, SubmissionId},
    submission::{SubmissionVisibility, VerificationLevel},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Submission list item
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SubmissionListItem {
    pub id: SubmissionId,
    pub benchmark_id: BenchmarkId,
    pub benchmark_name: String,
    pub model_name: String,
    pub model_version: String,
    pub score: f64,
    pub verification_level: VerificationLevel,
    pub submitted_by: String,
    pub submitted_at: String,
}

/// Submission detail response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SubmissionDetail {
    pub id: SubmissionId,
    pub benchmark_id: BenchmarkId,
    pub benchmark_name: String,
    pub model_name: String,
    pub model_version: String,
    pub score: f64,
    pub verification_level: VerificationLevel,
    pub visibility: SubmissionVisibility,
    pub submitted_by: String,
    pub submitted_at: String,
    pub metadata: serde_json::Value,
}

/// Create submission request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateSubmissionRequest {
    #[validate(length(min = 1, max = 200))]
    pub model_name: String,

    #[validate(length(min = 1, max = 50))]
    pub model_version: String,

    pub results: serde_json::Value,

    pub metadata: Option<serde_json::Value>,
}

/// Request verification request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RequestVerificationRequest {
    #[validate(length(min = 1, max = 1000))]
    pub notes: Option<String>,
}

/// Update visibility request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateVisibilityRequest {
    pub visibility: SubmissionVisibility,
}

/// Submission routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/benchmarks/:benchmark_id/submissions",
            post(create_submission).get(list_benchmark_submissions),
        )
        .route("/submissions/:id", get(get_submission))
        .route(
            "/submissions/:id/request-verification",
            post(request_verification),
        )
        .route("/submissions/:id/visibility", patch(update_visibility))
}

/// Create submission
///
/// Submit results for a benchmark.
#[utoipa::path(
    post,
    path = "/benchmarks/{benchmark_id}/submissions",
    tag = "submissions",
    params(
        ("benchmark_id" = Uuid, Path, description = "Benchmark ID"),
    ),
    request_body = CreateSubmissionRequest,
    responses(
        (status = 201, description = "Submission created", body = SubmissionDetail),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Benchmark not found"),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
async fn create_submission(
    State(_state): State<AppState>,
    user: AuthenticatedUser,
    Path(benchmark_id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<CreateSubmissionRequest>,
) -> ApiResult<Created<SubmissionDetail>> {
    if !user.can_submit_results() {
        return Err(ApiError::BadRequest(
            "Insufficient permissions to submit results".to_string(),
        ));
    }

    let _benchmark_id = BenchmarkId::from(benchmark_id);

    // In production: Create submission in database
    let submission = SubmissionDetail {
        id: SubmissionId::new(),
        benchmark_id: BenchmarkId::from(benchmark_id),
        benchmark_name: "Example Benchmark".to_string(),
        model_name: req.model_name,
        model_version: req.model_version,
        score: 0.0,
        verification_level: VerificationLevel::Unverified,
        visibility: SubmissionVisibility::Public,
        submitted_by: user.user_id.to_string(),
        submitted_at: chrono::Utc::now().to_rfc3339(),
        metadata: req.metadata.unwrap_or(serde_json::Value::Null),
    };

    Ok(Created(submission))
}

/// Get submission
///
/// Retrieve detailed information about a specific submission.
#[utoipa::path(
    get,
    path = "/submissions/{id}",
    tag = "submissions",
    params(
        ("id" = Uuid, Path, description = "Submission ID"),
    ),
    responses(
        (status = 200, description = "Submission details", body = SubmissionDetail),
        (status = 404, description = "Submission not found"),
    )
)]
async fn get_submission(
    State(_state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<SubmissionDetail>>> {
    let _submission_id = SubmissionId::from(id);

    // In production: Query database
    Err(ApiError::NotFound)
}

/// List benchmark submissions
///
/// List all submissions for a specific benchmark.
#[utoipa::path(
    get,
    path = "/benchmarks/{benchmark_id}/submissions",
    tag = "submissions",
    params(
        ("benchmark_id" = Uuid, Path, description = "Benchmark ID"),
        ("page" = Option<u32>, Query, description = "Page number"),
        ("per_page" = Option<u32>, Query, description = "Items per page"),
    ),
    responses(
        (status = 200, description = "List of submissions", body = PaginatedResponse<SubmissionListItem>),
        (status = 404, description = "Benchmark not found"),
    )
)]
async fn list_benchmark_submissions(
    State(_state): State<AppState>,
    Path(benchmark_id): Path<Uuid>,
    pagination: Pagination,
) -> ApiResult<Json<PaginatedResponse<SubmissionListItem>>> {
    let _benchmark_id = BenchmarkId::from(benchmark_id);

    // In production: Query database
    let items = vec![];
    let total = 0;

    let result = llm_benchmark_common::pagination::PaginatedResult::from_params(
        items,
        &pagination.params,
        total,
    );

    Ok(Json(result.into()))
}

/// Request verification
///
/// Request community verification for a submission.
#[utoipa::path(
    post,
    path = "/submissions/{id}/request-verification",
    tag = "submissions",
    params(
        ("id" = Uuid, Path, description = "Submission ID"),
    ),
    request_body = RequestVerificationRequest,
    responses(
        (status = 200, description = "Verification requested"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Submission not found"),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
async fn request_verification(
    State(_state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<Uuid>,
    ValidatedJson(_req): ValidatedJson<RequestVerificationRequest>,
) -> ApiResult<NoContent> {
    let _submission_id = SubmissionId::from(id);

    // In production: Create verification request
    Err(ApiError::NotFound)
}

/// Update visibility
///
/// Update the visibility setting of a submission.
#[utoipa::path(
    patch,
    path = "/submissions/{id}/visibility",
    tag = "submissions",
    params(
        ("id" = Uuid, Path, description = "Submission ID"),
    ),
    request_body = UpdateVisibilityRequest,
    responses(
        (status = 200, description = "Visibility updated"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Submission not found"),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
async fn update_visibility(
    State(_state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<Uuid>,
    ValidatedJson(_req): ValidatedJson<UpdateVisibilityRequest>,
) -> ApiResult<NoContent> {
    let _submission_id = SubmissionId::from(id);

    // In production: Update visibility in database
    Err(ApiError::NotFound)
}
