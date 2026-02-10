//! Standardized API response types.
//!
//! This module provides wrapper types for consistent API responses
//! including success responses, paginated results, and error handling.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use llm_benchmark_common::execution::ExecutionResult;
use llm_benchmark_common::pagination::PaginatedResult;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Standard API response wrapper
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiResponse<T> {
    /// Indicates if the request was successful
    pub success: bool,

    /// Response data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,

    /// Optional message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl<T> ApiResponse<T>
where
    T: Serialize,
{
    /// Create a success response with data
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
        }
    }

    /// Create a success response with data and message
    pub fn success_with_message(data: T, message: impl Into<String>) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: Some(message.into()),
        }
    }

    /// Create a response with just a message
    pub fn message(message: impl Into<String>) -> Self {
        Self {
            success: true,
            data: None,
            message: Some(message.into()),
        }
    }
}

impl<T> IntoResponse for ApiResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

/// Paginated API response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginatedResponse<T> {
    /// Items for the current page
    pub items: Vec<T>,

    /// Pagination metadata
    pub pagination: PaginationMeta,
}

/// Pagination metadata
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginationMeta {
    /// Current page number (1-indexed)
    pub page: u32,

    /// Items per page
    pub per_page: u32,

    /// Total number of items
    pub total: u64,

    /// Total number of pages
    pub total_pages: u32,

    /// Whether there is a next page
    pub has_next: bool,

    /// Whether there is a previous page
    pub has_prev: bool,
}

impl<T> From<PaginatedResult<T>> for PaginatedResponse<T> {
    fn from(result: PaginatedResult<T>) -> Self {
        Self {
            items: result.items,
            pagination: PaginationMeta {
                page: result.page,
                per_page: result.per_page,
                total: result.total,
                total_pages: result.total_pages,
                has_next: result.has_next,
                has_prev: result.has_prev,
            },
        }
    }
}

impl<T> IntoResponse for PaginatedResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

/// Created response (HTTP 201)
pub struct Created<T>(pub T);

impl<T> IntoResponse for Created<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        (StatusCode::CREATED, Json(ApiResponse::success(self.0))).into_response()
    }
}

/// No content response (HTTP 204)
pub struct NoContent;

impl IntoResponse for NoContent {
    fn into_response(self) -> Response {
        StatusCode::NO_CONTENT.into_response()
    }
}

/// Accepted response (HTTP 202)
pub struct Accepted<T>(pub T);

impl<T> IntoResponse for Accepted<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        (StatusCode::ACCEPTED, Json(ApiResponse::success(self.0))).into_response()
    }
}

/// API response wrapper that includes execution span data when present.
///
/// Used for agentics-managed requests. The `execution` field contains the
/// repo-level and agent-level spans produced during this invocation.
#[derive(Debug, Serialize, Deserialize)]
pub struct InstrumentedResponse<T> {
    /// The standard API response data.
    #[serde(flatten)]
    pub response: ApiResponse<T>,

    /// Execution span tree (present only for agentics-invoked requests).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<ExecutionResult>,
}

impl<T> InstrumentedResponse<T> {
    /// Create an instrumented response with execution data.
    pub fn new(response: ApiResponse<T>, execution: Option<ExecutionResult>) -> Self {
        Self {
            response,
            execution,
        }
    }
}

impl<T> IntoResponse for InstrumentedResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

/// Paginated API response wrapper that includes execution span data when present.
///
/// Used for agentics-managed requests on paginated endpoints.
#[derive(Debug, Serialize, Deserialize)]
pub struct InstrumentedPaginatedResponse<T> {
    /// The standard paginated response data.
    #[serde(flatten)]
    pub response: PaginatedResponse<T>,

    /// Execution span tree (present only for agentics-invoked requests).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<ExecutionResult>,
}

impl<T> InstrumentedPaginatedResponse<T> {
    /// Create an instrumented paginated response with optional execution data.
    pub fn new(response: PaginatedResponse<T>, execution: Option<ExecutionResult>) -> Self {
        Self {
            response,
            execution,
        }
    }
}

impl<T> IntoResponse for InstrumentedPaginatedResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}
