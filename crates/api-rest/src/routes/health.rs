//! Health check endpoints.
//!
//! Phase 7 Hardening: Health probes MUST verify RuVector connectivity.
//! Per Phase 7: If RuVector is unavailable, probes MUST fail.

use crate::{responses::ApiResponse, state::AppState};
use axum::{
    extract::State,
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};
use utoipa::ToSchema;

// =============================================================================
// Phase 7 Performance Budgets
// =============================================================================

/// Maximum tokens allowed per run (Phase 7)
pub const MAX_TOKENS: u32 = 2500;

/// Maximum latency in milliseconds per run (Phase 7)
pub const MAX_LATENCY_MS: u64 = 5000;

/// Maximum external service calls per run (Phase 7)
pub const MAX_CALLS_PER_RUN: u32 = 5;

// =============================================================================
// Response Types
// =============================================================================

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

    /// Agent identity (Phase 7)
    pub agent: AgentInfo,

    /// RuVector status (Phase 7 - required)
    pub ruvector: RuVectorStatus,
}

/// Agent identity information (Phase 7)
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AgentInfo {
    /// Agent name
    pub name: String,
    /// Agent domain
    pub domain: String,
    /// Agent phase
    pub phase: String,
    /// Agent layer
    pub layer: String,
    /// Agent version
    pub version: String,
}

impl Default for AgentInfo {
    fn default() -> Self {
        Self {
            name: std::env::var("AGENT_NAME")
                .unwrap_or_else(|_| "benchmark-publication-agent".to_string()),
            domain: std::env::var("AGENT_DOMAIN")
                .unwrap_or_else(|_| "benchmark".to_string()),
            phase: std::env::var("AGENT_PHASE")
                .unwrap_or_else(|_| "phase7".to_string()),
            layer: std::env::var("AGENT_LAYER")
                .unwrap_or_else(|_| "layer2".to_string()),
            version: std::env::var("AGENT_VERSION")
                .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string()),
        }
    }
}

/// RuVector status (Phase 7)
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RuVectorStatus {
    /// Whether RuVector is required (always true for Phase 7)
    pub required: bool,
    /// Whether RuVector is connected
    pub connected: bool,
    /// Last check latency in ms
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    /// Error message if not connected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Readiness check response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReadinessResponse {
    /// Overall readiness status
    pub ready: bool,

    /// Individual component checks
    pub checks: ReadinessChecks,

    /// Phase 7 performance budgets
    pub budgets: PerformanceBudgets,
}

/// Individual readiness checks
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReadinessChecks {
    /// RuVector connectivity (Phase 7 REQUIRED)
    pub ruvector: bool,

    /// Cache connectivity
    pub cache: bool,

    /// Agent identity configured
    pub agent_identity: bool,
}

/// Phase 7 performance budgets
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PerformanceBudgets {
    /// Maximum tokens per run
    pub max_tokens: u32,
    /// Maximum latency per run (ms)
    pub max_latency_ms: u64,
    /// Maximum external calls per run
    pub max_calls_per_run: u32,
}

impl Default for PerformanceBudgets {
    fn default() -> Self {
        Self {
            max_tokens: MAX_TOKENS,
            max_latency_ms: MAX_LATENCY_MS,
            max_calls_per_run: MAX_CALLS_PER_RUN,
        }
    }
}

/// Liveness check response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LivenessResponse {
    /// Whether the service is alive
    pub alive: bool,
    /// Service uptime
    pub uptime_seconds: u64,
}

// =============================================================================
// Routes
// =============================================================================

/// Health check routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/live", get(live))
}

// =============================================================================
// Handlers
// =============================================================================

/// Basic health check
///
/// Returns service status and version information.
/// Per Phase 7: MUST verify RuVector is available.
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse),
        (status = 503, description = "Service unhealthy - RuVector unavailable")
    )
)]
async fn health() -> Result<Json<ApiResponse<HealthResponse>>, (StatusCode, Json<ApiResponse<HealthResponse>>)> {
    let agent = AgentInfo::default();

    // Phase 7: Check RuVector connectivity
    let ruvector_status = check_ruvector_health().await;

    // Per Phase 7: If RuVector is unavailable, health check MUST fail
    if !ruvector_status.connected {
        error!(
            event = "agent_abort",
            agent_name = %agent.name,
            reason = "ruvector_unavailable",
            error = ?ruvector_status.error,
            "Health check failed: RuVector unavailable"
        );

        let response = HealthResponse {
            status: "unhealthy".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime: None,
            agent,
            ruvector: ruvector_status,
        };

        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse::success(response)),
        ));
    }

    info!(
        event = "agent_started",
        agent_name = %agent.name,
        agent_version = %agent.version,
        phase = %agent.phase,
        layer = %agent.layer,
        ruvector = true,
        "Health check passed"
    );

    let response = HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime: None,
        agent,
        ruvector: ruvector_status,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// Readiness check
///
/// Returns detailed readiness information including dependencies.
/// Per Phase 7: MUST verify RuVector is available.
#[utoipa::path(
    get,
    path = "/ready",
    tag = "health",
    responses(
        (status = 200, description = "Service is ready", body = ReadinessResponse),
        (status = 503, description = "Service not ready")
    )
)]
async fn ready(State(_state): State<AppState>) -> Result<Json<ApiResponse<ReadinessResponse>>, (StatusCode, Json<ApiResponse<ReadinessResponse>>)> {
    // Phase 7: Check RuVector connectivity
    let ruvector_status = check_ruvector_health().await;
    let ruvector_ready = ruvector_status.connected;

    // Check agent identity is configured
    let agent_identity_configured = std::env::var("AGENT_NAME").is_ok()
        || std::env::var("AGENT_DOMAIN").is_ok();

    let checks = ReadinessChecks {
        ruvector: ruvector_ready,
        cache: true, // Cache is optional
        agent_identity: agent_identity_configured,
    };

    // Per Phase 7: RuVector is REQUIRED - if not ready, fail
    let ready = checks.ruvector;

    if !ready {
        warn!(
            event = "readiness_failed",
            ruvector = ruvector_ready,
            agent_identity = agent_identity_configured,
            "Readiness check failed"
        );

        let response = ReadinessResponse {
            ready,
            checks,
            budgets: PerformanceBudgets::default(),
        };

        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse::success(response)),
        ));
    }

    let response = ReadinessResponse {
        ready,
        checks,
        budgets: PerformanceBudgets::default(),
    };

    Ok(Json(ApiResponse::success(response)))
}

/// Liveness check
///
/// Simple liveness probe - just checks if the service is running.
/// Does NOT check dependencies (that's what readiness is for).
#[utoipa::path(
    get,
    path = "/live",
    tag = "health",
    responses(
        (status = 200, description = "Service is alive", body = LivenessResponse)
    )
)]
async fn live() -> Json<ApiResponse<LivenessResponse>> {
    // Liveness is simple - if we can respond, we're alive
    // Uptime tracking would require storing start time in state
    let response = LivenessResponse {
        alive: true,
        uptime_seconds: 0, // Would need to track actual uptime
    };

    Json(ApiResponse::success(response))
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Check RuVector health status
async fn check_ruvector_health() -> RuVectorStatus {
    use llm_benchmark_infrastructure::external_consumers::{HttpRuVectorClient, RuVectorConfig};

    // Get RuVector URL from environment
    let ruvector_url = std::env::var("RUVECTOR_SERVICE_URL");

    if ruvector_url.is_err() {
        return RuVectorStatus {
            required: true,
            connected: false,
            latency_ms: None,
            error: Some("RUVECTOR_SERVICE_URL not configured".to_string()),
        };
    }

    // Create a temporary client to check health
    let config = RuVectorConfig::default();
    let client = match HttpRuVectorClient::new(config) {
        Ok(c) => c,
        Err(e) => {
            return RuVectorStatus {
                required: true,
                connected: false,
                latency_ms: None,
                error: Some(format!("Failed to create RuVector client: {}", e)),
            };
        }
    };

    // Perform health check
    match client.health_check().await {
        Ok(health) => RuVectorStatus {
            required: true,
            connected: health.healthy,
            latency_ms: Some(health.latency_ms),
            error: health.error,
        },
        Err(e) => RuVectorStatus {
            required: true,
            connected: false,
            latency_ms: None,
            error: Some(format!("Health check failed: {}", e)),
        },
    }
}
