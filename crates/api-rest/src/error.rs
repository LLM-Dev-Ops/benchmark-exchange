//! HTTP error handling and conversion.
//!
//! This module provides error types for the REST API and implements
//! conversion from domain errors to HTTP responses.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use llm_benchmark_application::ApplicationError;
use llm_benchmark_domain::errors::{AppError, AuthorizationError, BenchmarkError, GovernanceError, SubmissionError, ValidationError};
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// API-specific error type
#[derive(Debug, Error)]
pub enum ApiError {
    /// Domain error
    #[error(transparent)]
    Domain(#[from] AppError),

    /// Application layer error
    #[error(transparent)]
    Application(#[from] ApplicationError),

    /// Authentication required
    #[error("Authentication required")]
    Unauthorized,

    /// Invalid JWT token
    #[error("Invalid token: {0}")]
    InvalidToken(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Not found
    #[error("Resource not found")]
    NotFound,

    /// Bad request
    #[error("Bad request: {0}")]
    BadRequest(String),

    /// Conflict (e.g., duplicate resource)
    #[error("Conflict: {0}")]
    Conflict(String),

    /// Forbidden
    #[error("Forbidden: {0}")]
    Forbidden(String),

    /// Internal server error
    #[error("Internal server error")]
    Internal(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    /// Request timeout
    #[error("Request timeout")]
    Timeout,

    /// Payload too large
    #[error("Payload too large")]
    PayloadTooLarge,

    /// Service unavailable
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
}

impl ApiError {
    /// Get the HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::Domain(err) => match err {
                AppError::Authorization(auth_err) => match auth_err {
                    AuthorizationError::AuthenticationRequired => StatusCode::UNAUTHORIZED,
                    AuthorizationError::InvalidCredentials => StatusCode::UNAUTHORIZED,
                    AuthorizationError::TokenExpired => StatusCode::UNAUTHORIZED,
                    _ => StatusCode::FORBIDDEN,
                },
                AppError::Validation(_) => StatusCode::BAD_REQUEST,
                AppError::Benchmark(BenchmarkError::NotFound(_)) => StatusCode::NOT_FOUND,
                AppError::Submission(SubmissionError::NotFound(_)) => StatusCode::NOT_FOUND,
                AppError::Governance(GovernanceError::ProposalNotFound(_)) => StatusCode::NOT_FOUND,
                AppError::Database(_) => StatusCode::SERVICE_UNAVAILABLE,
                AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
                _ => StatusCode::BAD_REQUEST,
            },
            Self::Application(err) => StatusCode::from_u16(err.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Self::Unauthorized | Self::InvalidToken(_) => StatusCode::UNAUTHORIZED,
            Self::Validation(_) | Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::RateLimitExceeded => StatusCode::TOO_MANY_REQUESTS,
            Self::Timeout => StatusCode::REQUEST_TIMEOUT,
            Self::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            Self::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Get error code for API response
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::Domain(err) => err.error_code(),
            Self::Application(err) => err.error_code(),
            Self::Unauthorized | Self::InvalidToken(_) => "UNAUTHORIZED",
            Self::Validation(_) => "VALIDATION_ERROR",
            Self::NotFound => "NOT_FOUND",
            Self::BadRequest(_) => "BAD_REQUEST",
            Self::Conflict(_) => "CONFLICT",
            Self::Forbidden(_) => "FORBIDDEN",
            Self::RateLimitExceeded => "RATE_LIMIT_EXCEEDED",
            Self::Timeout => "TIMEOUT",
            Self::PayloadTooLarge => "PAYLOAD_TOO_LARGE",
            Self::ServiceUnavailable(_) => "SERVICE_UNAVAILABLE",
            Self::Internal(_) => "INTERNAL_ERROR",
        }
    }
}

/// Standardized error response
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error code
    pub error: String,

    /// Human-readable message
    pub message: String,

    /// Optional additional details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,

    /// Request ID for tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl ErrorResponse {
    /// Create a new error response
    pub fn new(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
            details: None,
            request_id: None,
        }
    }

    /// Add details to the error response
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    /// Add request ID to the error response
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }
}

impl fmt::Display for ErrorResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.error, self.message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let error_code = self.error_code();
        let message = self.to_string();

        let body = ErrorResponse::new(error_code, message);

        (status, Json(body)).into_response()
    }
}

// Implement From for common error types
impl From<ValidationError> for ApiError {
    fn from(err: ValidationError) -> Self {
        Self::Domain(AppError::Validation(err))
    }
}

impl From<BenchmarkError> for ApiError {
    fn from(err: BenchmarkError) -> Self {
        Self::Domain(AppError::Benchmark(err))
    }
}

impl From<SubmissionError> for ApiError {
    fn from(err: SubmissionError) -> Self {
        Self::Domain(AppError::Submission(err))
    }
}

impl From<GovernanceError> for ApiError {
    fn from(err: GovernanceError) -> Self {
        Self::Domain(AppError::Governance(err))
    }
}

impl From<AuthorizationError> for ApiError {
    fn from(err: AuthorizationError) -> Self {
        Self::Domain(AppError::Authorization(err))
    }
}

/// Result type for API operations
pub type ApiResult<T> = Result<T, ApiError>;
