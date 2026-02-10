//! Agentics execution context middleware.
//!
//! Extracts execution context from HTTP headers and creates a repo-level
//! execution span for the request. The execution context is stored in
//! request extensions for downstream extractors and handlers.
//!
//! ## Headers
//!
//! - `X-Parent-Span-Id` (required for agentics-managed calls)
//! - `X-Execution-Id` (optional; generated if absent)

use axum::{
    body::Body,
    http::{Request, Response},
    middleware::Next,
};
use llm_benchmark_common::execution::ExecutionContext;
use uuid::Uuid;

/// Header name for the execution ID from the Core orchestrator.
pub const EXECUTION_ID_HEADER: &str = "x-execution-id";

/// Header name for the parent span ID from the Core-level span.
pub const PARENT_SPAN_ID_HEADER: &str = "x-parent-span-id";

/// Middleware that extracts execution context from HTTP headers.
///
/// If `X-Parent-Span-Id` is present and valid, creates an `ExecutionContext`
/// with a repo-level span and stores it in request extensions.
///
/// If absent, the request proceeds without execution context (non-agentics call).
pub async fn execution_context_middleware(
    mut req: Request<Body>,
    next: Next,
) -> Response<Body> {
    let parent_span_id = req
        .headers()
        .get(PARENT_SPAN_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok());

    if let Some(parent_span_id) = parent_span_id {
        let execution_id = req
            .headers()
            .get(EXECUTION_ID_HEADER)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or_else(Uuid::new_v4);

        let exec_ctx = ExecutionContext::new(execution_id, parent_span_id);
        req.extensions_mut().insert(exec_ctx);
    }

    next.run(req).await
}
