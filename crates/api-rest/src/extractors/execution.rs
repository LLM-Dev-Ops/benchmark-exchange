//! Agentics execution context extractors.
//!
//! Provides Axum extractors for accessing the execution context set by
//! the execution middleware, plus a helper to build ServiceContext with
//! execution context attached.

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::request::Parts,
};
use llm_benchmark_application::services::ServiceContext;
use llm_benchmark_common::execution::ExecutionContext;

use crate::error::ApiError;
use crate::extractors::auth::AuthenticatedUser;

/// Extractor that optionally provides the execution context.
///
/// Returns `None` if the request was not made with agentics headers.
pub struct OptionalExecutionContext(pub Option<ExecutionContext>);

#[async_trait]
impl<S> FromRequestParts<S> for OptionalExecutionContext
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(OptionalExecutionContext(
            parts.extensions.get::<ExecutionContext>().cloned(),
        ))
    }
}

/// Extractor that REQUIRES an execution context.
///
/// Rejects with 400 Bad Request if `X-Parent-Span-Id` header was not provided.
pub struct RequiredExecutionContext(pub ExecutionContext);

#[async_trait]
impl<S> FromRequestParts<S> for RequiredExecutionContext
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<ExecutionContext>()
            .cloned()
            .map(RequiredExecutionContext)
            .ok_or_else(|| {
                ApiError::BadRequest(
                    "X-Parent-Span-Id header is required for this endpoint".to_string(),
                )
            })
    }
}

/// Build a `ServiceContext` from authentication and execution context.
///
/// Consolidates the duplicated `create_service_context` pattern found in
/// individual route handler modules.
pub fn build_service_context(
    user: Option<&AuthenticatedUser>,
    request_id: &str,
    exec_ctx: Option<ExecutionContext>,
) -> ServiceContext {
    let ctx = match user {
        Some(u) => {
            let ctx =
                ServiceContext::authenticated(u.user_id.to_string(), request_id.to_string());
            if u.is_admin() {
                ctx.with_admin()
            } else {
                ctx
            }
        }
        None => ServiceContext::anonymous(request_id.to_string()),
    };
    match exec_ctx {
        Some(ec) => ctx.with_execution(ec),
        None => ctx,
    }
}
