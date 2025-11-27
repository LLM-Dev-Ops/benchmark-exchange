//! Request logging middleware.

use axum::{
    body::Body,
    http::{Request, Response},
    middleware::Next,
};
use std::time::Instant;
use tracing::{info, warn};

/// Middleware for logging HTTP requests and responses
pub async fn logging_middleware(
    req: Request<Body>,
    next: Next,
) -> Response<Body> {
    let start = Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();
    let request_id = req
        .extensions()
        .get::<String>()
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());

    // Log incoming request
    info!(
        request_id = %request_id,
        method = %method,
        uri = %uri,
        "Incoming request"
    );

    // Process request
    let response = next.run(req).await;

    let duration = start.elapsed();
    let status = response.status();

    // Log response
    if status.is_server_error() {
        warn!(
            request_id = %request_id,
            method = %method,
            uri = %uri,
            status = %status,
            duration_ms = %duration.as_millis(),
            "Request failed"
        );
    } else {
        info!(
            request_id = %request_id,
            method = %method,
            uri = %uri,
            status = %status,
            duration_ms = %duration.as_millis(),
            "Request completed"
        );
    }

    response
}
