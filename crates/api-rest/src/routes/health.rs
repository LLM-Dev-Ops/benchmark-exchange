//! Health check endpoints.

use crate::{responses::ApiResponse, state::AppState};
use axum::{extract::State, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Health check response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    /// Service status
    pub status: String,

    /// Service version
    pub version: String,

    /// Service uptime in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime: Option<u64>,
}

/// Readiness check response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReadinessResponse {
    /// Overall readiness status
    pub ready: bool,

    /// Individual component checks
    pub checks: ReadinessChecks,
}

/// Individual readiness checks
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReadinessChecks {
    /// Database connectivity
    pub database: bool,

    /// Cache connectivity
    pub cache: bool,

    /// External services
    pub external_services: bool,
}

/// Health check routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
}

/// Basic health check
///
/// Returns service status and version information.
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    )
)]
async fn health() -> Json<ApiResponse<HealthResponse>> {
    let response = HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime: None,
    };

    Json(ApiResponse::success(response))
}

/// Readiness check
///
/// Returns detailed readiness information including dependencies.
#[utoipa::path(
    get,
    path = "/ready",
    tag = "health",
    responses(
        (status = 200, description = "Service readiness status", body = ReadinessResponse)
    )
)]
async fn ready(State(_state): State<AppState>) -> Json<ApiResponse<ReadinessResponse>> {
    // In production, check actual connectivity to dependencies
    let checks = ReadinessChecks {
        database: true,
        cache: true,
        external_services: true,
    };

    let ready = checks.database && checks.cache && checks.external_services;

    let response = ReadinessResponse { ready, checks };

    Json(ApiResponse::success(response))
}
