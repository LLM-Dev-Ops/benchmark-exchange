//! Benchmark endpoints.

use crate::{
    error::{ApiError, ApiResult},
    extractors::{AuthenticatedUser, Pagination, ValidatedJson},
    responses::{ApiResponse, Created, NoContent, PaginatedResponse},
    state::AppState,
};
use axum::{
    extract::{Path, Query, State},
    routing::{get, post, put},
    Json, Router,
};
use llm_benchmark_application::{
    services::{BenchmarkDto, BenchmarkFilters, BenchmarkVersionDto, Pagination as ServicePagination, ServiceContext},
    validation::{CreateBenchmarkRequest, CreateVersionRequest, StatusTransitionRequest, UpdateBenchmarkRequest},
};
use llm_benchmark_domain::{
    benchmark::{BenchmarkCategory, BenchmarkStatus},
    identifiers::{BenchmarkId, BenchmarkVersionId},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Benchmark list item (summary)
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BenchmarkListItem {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub category: BenchmarkCategory,
    pub status: BenchmarkStatus,
    pub version: Option<String>,
    pub description: String,
    pub submission_count: u64,
    pub created_at: String,
    pub updated_at: String,
}

impl From<BenchmarkDto> for BenchmarkListItem {
    fn from(dto: BenchmarkDto) -> Self {
        Self {
            id: dto.id,
            name: dto.name,
            slug: dto.slug,
            category: dto.category,
            status: dto.status,
            version: dto.current_version,
            description: dto.description,
            submission_count: dto.submission_count,
            created_at: dto.created_at.to_rfc3339(),
            updated_at: dto.updated_at.to_rfc3339(),
        }
    }
}

/// Benchmark detail response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BenchmarkDetail {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub category: BenchmarkCategory,
    pub status: BenchmarkStatus,
    pub version: Option<String>,
    pub description: String,
    pub tags: Vec<String>,
    pub submission_count: u64,
    pub created_at: String,
    pub updated_at: String,
}

impl From<BenchmarkDto> for BenchmarkDetail {
    fn from(dto: BenchmarkDto) -> Self {
        Self {
            id: dto.id,
            name: dto.name,
            slug: dto.slug,
            category: dto.category,
            status: dto.status,
            version: dto.current_version,
            description: dto.description,
            tags: dto.tags,
            submission_count: dto.submission_count,
            created_at: dto.created_at.to_rfc3339(),
            updated_at: dto.updated_at.to_rfc3339(),
        }
    }
}

/// Benchmark version response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BenchmarkVersionResponse {
    pub id: String,
    pub benchmark_id: String,
    pub version: String,
    pub changelog: String,
    pub breaking_changes: bool,
    pub created_at: String,
}

impl From<BenchmarkVersionDto> for BenchmarkVersionResponse {
    fn from(dto: BenchmarkVersionDto) -> Self {
        Self {
            id: dto.id,
            benchmark_id: dto.benchmark_id,
            version: dto.version,
            changelog: dto.changelog,
            breaking_changes: dto.breaking_changes,
            created_at: dto.created_at.to_rfc3339(),
        }
    }
}

/// Create benchmark request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateBenchmarkApiRequest {
    #[validate(length(min = 1, max = 200))]
    pub name: String,

    #[validate(length(min = 1, max = 100))]
    pub slug: String,

    pub category: BenchmarkCategory,

    #[validate(length(min = 1, max = 2000))]
    pub description: String,

    pub version: String,

    #[serde(default)]
    pub tags: Vec<String>,
}

/// Update benchmark request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateBenchmarkApiRequest {
    #[validate(length(min = 1, max = 200))]
    pub name: Option<String>,

    #[validate(length(min = 1, max = 2000))]
    pub description: Option<String>,

    pub tags: Option<Vec<String>>,

    pub long_description: Option<String>,
}

/// Status change request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ChangeStatusRequest {
    pub target_status: BenchmarkStatus,

    #[validate(length(min = 1, max = 1000))]
    pub reason: Option<String>,
}

/// Create version request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateVersionApiRequest {
    #[validate(length(min = 1, max = 50))]
    pub version: String,

    #[validate(length(min = 1, max = 2000))]
    pub changelog: String,

    #[serde(default)]
    pub breaking_changes: bool,

    pub migration_notes: Option<String>,
}

/// Query parameters for listing benchmarks
#[derive(Debug, Deserialize, ToSchema)]
pub struct BenchmarkListQuery {
    pub category: Option<BenchmarkCategory>,
    pub status: Option<BenchmarkStatus>,
    pub search: Option<String>,
    pub tags: Option<String>,
}

/// Benchmark routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/benchmarks", get(list_benchmarks).post(create_benchmark))
        .route("/benchmarks/:id", get(get_benchmark).put(update_benchmark).delete(delete_benchmark))
        .route("/benchmarks/:id/submit-for-review", post(submit_for_review))
        .route("/benchmarks/:id/approve", post(approve_benchmark))
        .route("/benchmarks/:id/reject", post(reject_benchmark))
        .route("/benchmarks/:id/deprecate", post(deprecate_benchmark))
        .route("/benchmarks/:id/versions", get(list_versions).post(create_version))
        .route("/benchmarks/slug/:slug", get(get_benchmark_by_slug))
        .route("/benchmarks/search", get(search_benchmarks))
}

/// Helper to create service context from request
fn create_service_context(user: Option<&AuthenticatedUser>, request_id: &str) -> ServiceContext {
    match user {
        Some(u) => {
            let ctx = ServiceContext::authenticated(u.user_id.to_string(), request_id.to_string());
            if u.is_admin() {
                ctx.with_admin()
            } else {
                ctx
            }
        }
        None => ServiceContext::anonymous(request_id.to_string()),
    }
}

/// List benchmarks
///
/// Returns a paginated list of benchmarks with optional filtering.
#[utoipa::path(
    get,
    path = "/benchmarks",
    tag = "benchmarks",
    params(
        ("page" = Option<u32>, Query, description = "Page number (1-indexed)"),
        ("per_page" = Option<u32>, Query, description = "Items per page"),
        ("sort_by" = Option<String>, Query, description = "Sort field"),
        ("sort_dir" = Option<String>, Query, description = "Sort direction (asc/desc)"),
        ("category" = Option<String>, Query, description = "Filter by category"),
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("search" = Option<String>, Query, description = "Search query"),
    ),
    responses(
        (status = 200, description = "List of benchmarks", body = PaginatedResponse<BenchmarkListItem>)
    )
)]
async fn list_benchmarks(
    State(state): State<AppState>,
    pagination: Pagination,
    Query(query): Query<BenchmarkListQuery>,
) -> ApiResult<Json<PaginatedResponse<BenchmarkListItem>>> {
    let ctx = ServiceContext::anonymous(uuid::Uuid::new_v4().to_string());

    let filters = BenchmarkFilters {
        category: query.category,
        status: query.status,
        search: query.search,
        tags: query.tags.map(|t| t.split(',').map(|s| s.trim().to_string()).collect()),
        maintainer_id: None,
    };

    let service_pagination = ServicePagination::new(
        pagination.params.page,
        pagination.params.per_page,
    );

    let result = state.benchmark_service
        .list(&ctx, filters, service_pagination)
        .await?;

    let items: Vec<BenchmarkListItem> = result.items.into_iter().map(Into::into).collect();

    let paginated = llm_benchmark_common::pagination::PaginatedResult::new(
        items,
        result.page,
        result.page_size,
        result.total,
    );

    Ok(Json(paginated.into()))
}

/// Create benchmark
///
/// Create a new benchmark. Requires contributor role or higher.
#[utoipa::path(
    post,
    path = "/benchmarks",
    tag = "benchmarks",
    request_body = CreateBenchmarkApiRequest,
    responses(
        (status = 201, description = "Benchmark created", body = BenchmarkDetail),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 409, description = "Benchmark with slug already exists"),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
async fn create_benchmark(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    ValidatedJson(req): ValidatedJson<CreateBenchmarkApiRequest>,
) -> ApiResult<Created<BenchmarkDetail>> {
    if !user.can_propose_benchmarks() {
        return Err(ApiError::Forbidden(
            "Insufficient permissions to create benchmarks".to_string(),
        ));
    }

    let ctx = create_service_context(Some(&user), &uuid::Uuid::new_v4().to_string());

    let request = CreateBenchmarkRequest {
        name: req.name,
        slug: req.slug,
        description: req.description,
        category: req.category,
        tags: req.tags,
        version: req.version,
    };

    let benchmark = state.benchmark_service.create(&ctx, request).await?;

    Ok(Created(benchmark.into()))
}

/// Get benchmark by ID
///
/// Retrieve detailed information about a specific benchmark.
#[utoipa::path(
    get,
    path = "/benchmarks/{id}",
    tag = "benchmarks",
    params(
        ("id" = String, Path, description = "Benchmark ID"),
    ),
    responses(
        (status = 200, description = "Benchmark details", body = BenchmarkDetail),
        (status = 404, description = "Benchmark not found"),
    )
)]
async fn get_benchmark(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiResponse<BenchmarkDetail>>> {
    let ctx = ServiceContext::anonymous(uuid::Uuid::new_v4().to_string());

    let benchmark = state.benchmark_service
        .get_by_id(&ctx, &id)
        .await?
        .ok_or(ApiError::NotFound)?;

    Ok(Json(ApiResponse::success(benchmark.into())))
}

/// Get benchmark by slug
///
/// Retrieve benchmark by its unique slug identifier.
#[utoipa::path(
    get,
    path = "/benchmarks/slug/{slug}",
    tag = "benchmarks",
    params(
        ("slug" = String, Path, description = "Benchmark slug"),
    ),
    responses(
        (status = 200, description = "Benchmark details", body = BenchmarkDetail),
        (status = 404, description = "Benchmark not found"),
    )
)]
async fn get_benchmark_by_slug(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> ApiResult<Json<ApiResponse<BenchmarkDetail>>> {
    let ctx = ServiceContext::anonymous(uuid::Uuid::new_v4().to_string());

    let benchmark = state.benchmark_service
        .get_by_slug(&ctx, &slug)
        .await?
        .ok_or(ApiError::NotFound)?;

    Ok(Json(ApiResponse::success(benchmark.into())))
}

/// Update benchmark
///
/// Update benchmark metadata. Requires appropriate permissions.
#[utoipa::path(
    put,
    path = "/benchmarks/{id}",
    tag = "benchmarks",
    params(
        ("id" = String, Path, description = "Benchmark ID"),
    ),
    request_body = UpdateBenchmarkApiRequest,
    responses(
        (status = 200, description = "Benchmark updated", body = BenchmarkDetail),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Benchmark not found"),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
async fn update_benchmark(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<UpdateBenchmarkApiRequest>,
) -> ApiResult<Json<ApiResponse<BenchmarkDetail>>> {
    let ctx = create_service_context(Some(&user), &uuid::Uuid::new_v4().to_string());

    let request = UpdateBenchmarkRequest {
        name: req.name,
        description: req.description,
        tags: req.tags,
        long_description: req.long_description,
    };

    let benchmark = state.benchmark_service
        .update(&ctx, &id, request)
        .await?;

    Ok(Json(ApiResponse::success(benchmark.into())))
}

/// Delete benchmark
///
/// Delete a benchmark. Requires admin privileges.
#[utoipa::path(
    delete,
    path = "/benchmarks/{id}",
    tag = "benchmarks",
    params(
        ("id" = String, Path, description = "Benchmark ID"),
    ),
    responses(
        (status = 204, description = "Benchmark deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Benchmark not found"),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
async fn delete_benchmark(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<String>,
) -> ApiResult<NoContent> {
    if !user.is_admin() {
        return Err(ApiError::Forbidden(
            "Admin privileges required to delete benchmarks".to_string(),
        ));
    }

    let ctx = create_service_context(Some(&user), &uuid::Uuid::new_v4().to_string());

    state.benchmark_service.delete(&ctx, &id).await?;

    Ok(NoContent)
}

/// Submit benchmark for review
///
/// Submit a draft benchmark for community review.
#[utoipa::path(
    post,
    path = "/benchmarks/{id}/submit-for-review",
    tag = "benchmarks",
    params(
        ("id" = String, Path, description = "Benchmark ID"),
    ),
    responses(
        (status = 200, description = "Benchmark submitted for review", body = BenchmarkDetail),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Benchmark not found"),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
async fn submit_for_review(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiResponse<BenchmarkDetail>>> {
    let ctx = create_service_context(Some(&user), &uuid::Uuid::new_v4().to_string());

    let request = StatusTransitionRequest {
        current_status: BenchmarkStatus::Draft,
        target_status: BenchmarkStatus::UnderReview,
        reason: None,
    };

    let benchmark = state.benchmark_service
        .transition_status(&ctx, &id, request)
        .await?;

    Ok(Json(ApiResponse::success(benchmark.into())))
}

/// Approve benchmark
///
/// Approve a benchmark for active use. Requires reviewer role.
#[utoipa::path(
    post,
    path = "/benchmarks/{id}/approve",
    tag = "benchmarks",
    params(
        ("id" = String, Path, description = "Benchmark ID"),
    ),
    request_body = ChangeStatusRequest,
    responses(
        (status = 200, description = "Benchmark approved", body = BenchmarkDetail),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Benchmark not found"),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
async fn approve_benchmark(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<ChangeStatusRequest>,
) -> ApiResult<Json<ApiResponse<BenchmarkDetail>>> {
    if !user.can_review() {
        return Err(ApiError::Forbidden(
            "Insufficient permissions to approve benchmarks".to_string(),
        ));
    }

    let ctx = create_service_context(Some(&user), &uuid::Uuid::new_v4().to_string());

    let request = StatusTransitionRequest {
        current_status: BenchmarkStatus::UnderReview,
        target_status: BenchmarkStatus::Active,
        reason: req.reason,
    };

    let benchmark = state.benchmark_service
        .transition_status(&ctx, &id, request)
        .await?;

    Ok(Json(ApiResponse::success(benchmark.into())))
}

/// Reject benchmark
///
/// Reject a benchmark under review. Requires reviewer role.
#[utoipa::path(
    post,
    path = "/benchmarks/{id}/reject",
    tag = "benchmarks",
    params(
        ("id" = String, Path, description = "Benchmark ID"),
    ),
    request_body = ChangeStatusRequest,
    responses(
        (status = 200, description = "Benchmark rejected", body = BenchmarkDetail),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Benchmark not found"),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
async fn reject_benchmark(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<ChangeStatusRequest>,
) -> ApiResult<Json<ApiResponse<BenchmarkDetail>>> {
    if !user.can_review() {
        return Err(ApiError::Forbidden(
            "Insufficient permissions to reject benchmarks".to_string(),
        ));
    }

    let ctx = create_service_context(Some(&user), &uuid::Uuid::new_v4().to_string());

    // Rejected means going back to draft status with a reason
    let request = StatusTransitionRequest {
        current_status: BenchmarkStatus::UnderReview,
        target_status: BenchmarkStatus::Draft,
        reason: req.reason,
    };

    let benchmark = state.benchmark_service
        .transition_status(&ctx, &id, request)
        .await?;

    Ok(Json(ApiResponse::success(benchmark.into())))
}

/// Deprecate benchmark
///
/// Mark a benchmark as deprecated. Requires reviewer role.
#[utoipa::path(
    post,
    path = "/benchmarks/{id}/deprecate",
    tag = "benchmarks",
    params(
        ("id" = String, Path, description = "Benchmark ID"),
    ),
    request_body = ChangeStatusRequest,
    responses(
        (status = 200, description = "Benchmark deprecated", body = BenchmarkDetail),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Benchmark not found"),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
async fn deprecate_benchmark(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<ChangeStatusRequest>,
) -> ApiResult<Json<ApiResponse<BenchmarkDetail>>> {
    if !user.can_review() {
        return Err(ApiError::Forbidden(
            "Insufficient permissions to deprecate benchmarks".to_string(),
        ));
    }

    let ctx = create_service_context(Some(&user), &uuid::Uuid::new_v4().to_string());

    let request = StatusTransitionRequest {
        current_status: BenchmarkStatus::Active,
        target_status: BenchmarkStatus::Deprecated,
        reason: req.reason,
    };

    let benchmark = state.benchmark_service
        .transition_status(&ctx, &id, request)
        .await?;

    Ok(Json(ApiResponse::success(benchmark.into())))
}

/// List benchmark versions
///
/// Get all versions for a benchmark.
#[utoipa::path(
    get,
    path = "/benchmarks/{id}/versions",
    tag = "benchmarks",
    params(
        ("id" = String, Path, description = "Benchmark ID"),
    ),
    responses(
        (status = 200, description = "List of versions", body = Vec<BenchmarkVersionResponse>),
        (status = 404, description = "Benchmark not found"),
    )
)]
async fn list_versions(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiResponse<Vec<BenchmarkVersionResponse>>>> {
    let ctx = ServiceContext::anonymous(uuid::Uuid::new_v4().to_string());

    let versions = state.benchmark_service
        .get_versions(&ctx, &id)
        .await?;

    let responses: Vec<BenchmarkVersionResponse> = versions.into_iter().map(Into::into).collect();

    Ok(Json(ApiResponse::success(responses)))
}

/// Create benchmark version
///
/// Create a new version for a benchmark.
#[utoipa::path(
    post,
    path = "/benchmarks/{id}/versions",
    tag = "benchmarks",
    params(
        ("id" = String, Path, description = "Benchmark ID"),
    ),
    request_body = CreateVersionApiRequest,
    responses(
        (status = 201, description = "Version created", body = BenchmarkVersionResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Benchmark not found"),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
async fn create_version(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<CreateVersionApiRequest>,
) -> ApiResult<Created<BenchmarkVersionResponse>> {
    let ctx = create_service_context(Some(&user), &uuid::Uuid::new_v4().to_string());

    let request = CreateVersionRequest {
        version: req.version,
        changelog: req.changelog,
        breaking_changes: req.breaking_changes,
        migration_notes: req.migration_notes,
    };

    let version = state.benchmark_service
        .create_version(&ctx, &id, request)
        .await?;

    Ok(Created(version.into()))
}

/// Search benchmarks
///
/// Search benchmarks by text query.
#[utoipa::path(
    get,
    path = "/benchmarks/search",
    tag = "benchmarks",
    params(
        ("q" = String, Query, description = "Search query"),
        ("page" = Option<u32>, Query, description = "Page number"),
        ("per_page" = Option<u32>, Query, description = "Items per page"),
    ),
    responses(
        (status = 200, description = "Search results", body = PaginatedResponse<BenchmarkListItem>)
    )
)]
async fn search_benchmarks(
    State(state): State<AppState>,
    pagination: Pagination,
    Query(params): Query<SearchQuery>,
) -> ApiResult<Json<PaginatedResponse<BenchmarkListItem>>> {
    let ctx = ServiceContext::anonymous(uuid::Uuid::new_v4().to_string());

    let service_pagination = ServicePagination::new(
        pagination.params.page,
        pagination.params.per_page,
    );

    let result = state.benchmark_service
        .search(&ctx, &params.q, service_pagination)
        .await?;

    let items: Vec<BenchmarkListItem> = result.items.into_iter().map(Into::into).collect();

    let paginated = llm_benchmark_common::pagination::PaginatedResult::new(
        items,
        result.page,
        result.page_size,
        result.total,
    );

    Ok(Json(paginated.into()))
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    q: String,
}
