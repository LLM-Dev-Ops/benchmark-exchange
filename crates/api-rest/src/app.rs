//! Application builder and configuration.
//!
//! This module provides the main application builder that assembles
//! all routes, middleware, and state into an Axum router.

use crate::{
    config::ApiConfig,
    middleware::{logging_middleware, request_id::request_id_middleware, RateLimitLayer},
    routes,
    state::AppState,
};
use axum::{
    middleware,
    routing::get,
    Router,
};
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// Create the main application router
pub async fn create_app(config: ApiConfig) -> anyhow::Result<Router> {
    // Initialize tracing
    init_tracing(&config)?;

    // Create application state
    let state = AppState::new(config.clone());

    // Build CORS layer
    let cors = build_cors_layer(&config);

    // Build rate limiting layer
    let rate_limit = RateLimitLayer::new();

    // Build the router
    let mut app = Router::new()
        // Health check routes (no auth required)
        .merge(routes::health::routes())
        // API v1 routes
        .nest("/api/v1", routes::v1::routes())
        // Add state
        .with_state(state);

    // Add Swagger UI if enabled
    if config.enable_swagger {
        app = app.merge(swagger_ui(&config));
    }

    let app = app
        // Add middleware layers
        .layer(
            ServiceBuilder::new()
                // Tracing
                .layer(TraceLayer::new_for_http())
                // Compression
                .layer(CompressionLayer::new())
                // CORS
                .layer(cors)
                // Timeout
                .layer(TimeoutLayer::new(Duration::from_secs(
                    config.request_timeout_seconds,
                )))
                // Rate limiting
                .layer(rate_limit)
                // Custom middleware
                .layer(middleware::from_fn(request_id_middleware))
                .layer(middleware::from_fn(logging_middleware)),
        );

    Ok(app)
}

/// Initialize tracing/logging
fn init_tracing(config: &ApiConfig) -> anyhow::Result<()> {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.log_level));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    Ok(())
}

/// Build CORS layer from configuration
fn build_cors_layer(config: &ApiConfig) -> CorsLayer {
    let cors = CorsLayer::new();

    if config.cors_allowed_origins.contains(&"*".to_string()) {
        cors.allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        // In production, parse and validate allowed origins
        cors.allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    }
}

/// Create Swagger UI routes if enabled
fn swagger_ui(config: &ApiConfig) -> SwaggerUi {
    #[derive(OpenApi)]
    #[openapi(
        info(
            title = "LLM Benchmark Exchange API",
            version = "1.0.0",
            description = "REST API for the LLM Benchmark Exchange platform",
            license(name = "MIT"),
        ),
        servers(
            (url = "/api/v1", description = "API v1")
        ),
        tags(
            (name = "health", description = "Health check endpoints"),
            (name = "benchmarks", description = "Benchmark management"),
            (name = "submissions", description = "Result submissions"),
            (name = "leaderboards", description = "Leaderboard queries"),
            (name = "governance", description = "Governance and proposals"),
            (name = "users", description = "User management and authentication"),
        )
    )]
    struct ApiDoc;

    SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi())
}
