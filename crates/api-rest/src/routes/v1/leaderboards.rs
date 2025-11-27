//! Leaderboard endpoints.

use crate::{
    error::{ApiError, ApiResult},
    extractors::Pagination,
    responses::{ApiResponse, PaginatedResponse},
    state::AppState,
};
use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use llm_benchmark_domain::{
    benchmark::BenchmarkCategory,
    identifiers::{BenchmarkId, ModelId},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Leaderboard entry
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LeaderboardEntry {
    pub rank: u32,
    pub model_name: String,
    pub model_version: String,
    pub score: f64,
    pub verification_status: String,
    pub submission_date: String,
    pub submitted_by: String,
}

/// Model comparison result
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ModelComparison {
    pub models: Vec<ModelComparisonEntry>,
    pub benchmarks: Vec<BenchmarkComparison>,
}

/// Model entry in comparison
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ModelComparisonEntry {
    pub model_id: ModelId,
    pub name: String,
    pub version: String,
}

/// Benchmark comparison data
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BenchmarkComparison {
    pub benchmark_id: BenchmarkId,
    pub benchmark_name: String,
    pub scores: Vec<Option<f64>>,
}

/// Model history entry
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ModelHistoryEntry {
    pub benchmark_name: String,
    pub score: f64,
    pub submitted_at: String,
    pub version: String,
}

/// Model comparison query
#[derive(Debug, Deserialize, ToSchema)]
pub struct CompareModelsQuery {
    pub models: String, // Comma-separated model IDs
}

/// Leaderboard routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/benchmarks/:id/leaderboard",
            get(get_benchmark_leaderboard),
        )
        .route(
            "/categories/:category/leaderboard",
            get(get_category_leaderboard),
        )
        .route("/models/compare", get(compare_models))
        .route("/models/:id/history", get(get_model_history))
}

/// Get benchmark leaderboard
///
/// Retrieve the leaderboard for a specific benchmark.
#[utoipa::path(
    get,
    path = "/benchmarks/{id}/leaderboard",
    tag = "leaderboards",
    params(
        ("id" = Uuid, Path, description = "Benchmark ID"),
        ("page" = Option<u32>, Query, description = "Page number"),
        ("per_page" = Option<u32>, Query, description = "Items per page"),
    ),
    responses(
        (status = 200, description = "Benchmark leaderboard", body = PaginatedResponse<LeaderboardEntry>),
        (status = 404, description = "Benchmark not found"),
    )
)]
async fn get_benchmark_leaderboard(
    State(_state): State<AppState>,
    Path(id): Path<Uuid>,
    pagination: Pagination,
) -> ApiResult<Json<PaginatedResponse<LeaderboardEntry>>> {
    let _benchmark_id = BenchmarkId::from(id);

    // In production: Query database for leaderboard entries
    let items = vec![];
    let total = 0;

    let result = llm_benchmark_common::pagination::PaginatedResult::from_params(
        items,
        &pagination.params,
        total,
    );

    Ok(Json(result.into()))
}

/// Get category leaderboard
///
/// Retrieve aggregated leaderboard for a benchmark category.
#[utoipa::path(
    get,
    path = "/categories/{category}/leaderboard",
    tag = "leaderboards",
    params(
        ("category" = String, Path, description = "Benchmark category"),
        ("page" = Option<u32>, Query, description = "Page number"),
        ("per_page" = Option<u32>, Query, description = "Items per page"),
    ),
    responses(
        (status = 200, description = "Category leaderboard", body = PaginatedResponse<LeaderboardEntry>),
        (status = 400, description = "Invalid category"),
    )
)]
async fn get_category_leaderboard(
    State(_state): State<AppState>,
    Path(_category): Path<String>,
    pagination: Pagination,
) -> ApiResult<Json<PaginatedResponse<LeaderboardEntry>>> {
    // In production: Parse category and query database
    let items = vec![];
    let total = 0;

    let result = llm_benchmark_common::pagination::PaginatedResult::from_params(
        items,
        &pagination.params,
        total,
    );

    Ok(Json(result.into()))
}

/// Compare models
///
/// Compare performance of multiple models across benchmarks.
#[utoipa::path(
    get,
    path = "/models/compare",
    tag = "leaderboards",
    params(
        ("models" = String, Query, description = "Comma-separated model IDs"),
    ),
    responses(
        (status = 200, description = "Model comparison", body = ModelComparison),
        (status = 400, description = "Invalid request"),
    )
)]
async fn compare_models(
    State(_state): State<AppState>,
    Query(_query): Query<CompareModelsQuery>,
) -> ApiResult<Json<ApiResponse<ModelComparison>>> {
    // In production: Parse model IDs and fetch comparison data
    let comparison = ModelComparison {
        models: vec![],
        benchmarks: vec![],
    };

    Ok(Json(ApiResponse::success(comparison)))
}

/// Get model history
///
/// Retrieve submission history for a specific model.
#[utoipa::path(
    get,
    path = "/models/{id}/history",
    tag = "leaderboards",
    params(
        ("id" = Uuid, Path, description = "Model ID"),
        ("page" = Option<u32>, Query, description = "Page number"),
        ("per_page" = Option<u32>, Query, description = "Items per page"),
    ),
    responses(
        (status = 200, description = "Model history", body = PaginatedResponse<ModelHistoryEntry>),
        (status = 404, description = "Model not found"),
    )
)]
async fn get_model_history(
    State(_state): State<AppState>,
    Path(id): Path<Uuid>,
    pagination: Pagination,
) -> ApiResult<Json<PaginatedResponse<ModelHistoryEntry>>> {
    let _model_id = ModelId::from(id);

    // In production: Query submission history
    let items = vec![];
    let total = 0;

    let result = llm_benchmark_common::pagination::PaginatedResult::from_params(
        items,
        &pagination.params,
        total,
    );

    Ok(Json(result.into()))
}
