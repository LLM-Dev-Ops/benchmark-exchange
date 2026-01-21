use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Clone)]
struct AppState {
    service_name: String,
    service_version: String,
    start_time: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    service: String,
    version: String,
    uptime_seconds: i64,
}

#[derive(Serialize)]
struct PublicationListResponse {
    publications: Vec<Publication>,
    total: usize,
}

#[derive(Serialize, Deserialize)]
struct Publication {
    id: String,
    benchmark_id: String,
    status: String,
    created_at: String,
}

async fn health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let uptime = chrono::Utc::now() - state.start_time;
    Json(HealthResponse {
        status: "healthy".to_string(),
        service: state.service_name.clone(),
        version: state.service_version.clone(),
        uptime_seconds: uptime.num_seconds(),
    })
}

async fn live() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "live"})))
}

async fn ready() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "ready"})))
}

async fn list_publications() -> impl IntoResponse {
    Json(PublicationListResponse {
        publications: vec![],
        total: 0,
    })
}

async fn validate_publication() -> impl IntoResponse {
    Json(serde_json::json!({
        "valid": true,
        "message": "Validation endpoint operational"
    }))
}

async fn openapi() -> impl IntoResponse {
    Json(serde_json::json!({
        "openapi": "3.0.0",
        "info": {
            "title": "LLM Benchmark Gateway API",
            "version": "0.1.0"
        },
        "paths": {}
    }))
}

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .json()
        .init();

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .unwrap_or(8080);

    let state = Arc::new(AppState {
        service_name: std::env::var("SERVICE_NAME").unwrap_or_else(|_| "llm-benchmark-gateway".to_string()),
        service_version: std::env::var("SERVICE_VERSION").unwrap_or_else(|_| "0.1.0".to_string()),
        start_time: chrono::Utc::now(),
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/live", get(live))
        .route("/ready", get(ready))
        .route("/api/v1/publications", get(list_publications))
        .route("/api/v1/publications/validate", axum::routing::post(validate_publication))
        .route("/api/v1/openapi.json", get(openapi))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("LLM Benchmark Gateway starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
