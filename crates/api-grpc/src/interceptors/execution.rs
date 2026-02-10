//! Agentics execution context interceptor for gRPC requests.
//!
//! Extracts execution context from gRPC metadata and creates a repo-level
//! execution span for the request. The execution context is stored in
//! request extensions for downstream service handlers.
//!
//! ## Metadata Keys
//!
//! - `x-parent-span-id` (required for agentics-managed calls)
//! - `x-execution-id` (optional; generated if absent)

use llm_benchmark_common::execution::ExecutionContext;
use tonic::{Request, Status};
use tracing::debug;
use uuid::Uuid;

/// Metadata key for the execution ID from the Core orchestrator.
pub const EXECUTION_ID_KEY: &str = "x-execution-id";

/// Metadata key for the parent span ID from the Core-level span.
pub const PARENT_SPAN_ID_KEY: &str = "x-parent-span-id";

/// Execution context interceptor for gRPC requests.
///
/// If `x-parent-span-id` is present and valid, creates an `ExecutionContext`
/// with a repo-level span and stores it in request extensions.
///
/// If absent, the request proceeds without execution context (non-agentics call).
#[derive(Clone)]
pub struct ExecutionInterceptor;

impl ExecutionInterceptor {
    /// Create a new execution interceptor.
    pub fn new() -> Self {
        Self
    }

    /// Intercept and extract execution context from metadata.
    pub fn intercept<T>(&self, mut req: Request<T>) -> Result<Request<T>, Status> {
        let parent_span_id = req
            .metadata()
            .get(PARENT_SPAN_ID_KEY)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| Uuid::parse_str(s).ok());

        if let Some(parent_span_id) = parent_span_id {
            let execution_id = req
                .metadata()
                .get(EXECUTION_ID_KEY)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| Uuid::parse_str(s).ok())
                .unwrap_or_else(Uuid::new_v4);

            debug!(
                execution_id = %execution_id,
                parent_span_id = %parent_span_id,
                "Creating execution context from gRPC metadata"
            );

            let exec_ctx = ExecutionContext::new(execution_id, parent_span_id);
            req.extensions_mut().insert(exec_ctx);
        }

        Ok(req)
    }
}

impl Default for ExecutionInterceptor {
    fn default() -> Self {
        Self::new()
    }
}
