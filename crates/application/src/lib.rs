//! Application layer for LLM Benchmark Exchange
//!
//! This crate orchestrates domain logic and coordinates between layers.
//!
//! ## Architecture
//!
//! The application layer sits between the domain and infrastructure layers,
//! providing use case orchestration and business logic coordination.
//!
//! ## Modules
//!
//! - `services` - Business logic services (BenchmarkService, SubmissionService, etc.)
//! - `scoring` - Evaluation and scoring engine
//! - `validation` - Input validation framework
//! - `dto` - Data transfer objects for API layer

pub mod dto;
pub mod scoring;
pub mod services;
pub mod validation;

// Re-export commonly used types
pub use scoring::{
    ScoringEngine, ScoringEngineBuilder, ScoringEngineConfig, ScoringRequest, TestCaseInput,
};
pub use services::{
    AuthorizationResult, Authorizer, DefaultAuthorizer, EventPublisher, NoOpEventPublisher,
    PaginatedResult, Pagination, ServiceConfig, ServiceContext, ServiceEvent,
};
pub use validation::{Validatable, ValidationContext, ValidationResult, ValidationRules};

// Common error types for the application layer
use thiserror::Error;

/// Application-level errors
#[derive(Error, Debug, Clone)]
pub enum ApplicationError {
    /// Resource not found
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Authentication required
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// Permission denied
    #[error("Forbidden: {0}")]
    Forbidden(String),

    /// Invalid input data
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Validation errors
    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    /// Resource conflict (e.g., duplicate)
    #[error("Conflict: {0}")]
    Conflict(String),

    /// Internal server error
    #[error("Internal error: {0}")]
    Internal(String),

    /// External service unavailable
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    /// Request timeout
    #[error("Request timeout: {0}")]
    Timeout(String),
}

impl ApplicationError {
    /// Get HTTP status code for this error
    pub fn http_status(&self) -> u16 {
        match self {
            ApplicationError::NotFound(_) => 404,
            ApplicationError::Unauthorized(_) => 401,
            ApplicationError::Forbidden(_) => 403,
            ApplicationError::InvalidInput(_) => 400,
            ApplicationError::ValidationFailed(_) => 422,
            ApplicationError::Conflict(_) => 409,
            ApplicationError::Internal(_) => 500,
            ApplicationError::ServiceUnavailable(_) => 503,
            ApplicationError::RateLimitExceeded(_) => 429,
            ApplicationError::Timeout(_) => 504,
        }
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ApplicationError::ServiceUnavailable(_)
                | ApplicationError::Timeout(_)
                | ApplicationError::RateLimitExceeded(_)
        )
    }

    /// Get error code for API responses
    pub fn error_code(&self) -> &'static str {
        match self {
            ApplicationError::NotFound(_) => "NOT_FOUND",
            ApplicationError::Unauthorized(_) => "UNAUTHORIZED",
            ApplicationError::Forbidden(_) => "FORBIDDEN",
            ApplicationError::InvalidInput(_) => "INVALID_INPUT",
            ApplicationError::ValidationFailed(_) => "VALIDATION_FAILED",
            ApplicationError::Conflict(_) => "CONFLICT",
            ApplicationError::Internal(_) => "INTERNAL_ERROR",
            ApplicationError::ServiceUnavailable(_) => "SERVICE_UNAVAILABLE",
            ApplicationError::RateLimitExceeded(_) => "RATE_LIMIT_EXCEEDED",
            ApplicationError::Timeout(_) => "TIMEOUT",
        }
    }
}

pub type ApplicationResult<T> = Result<T, ApplicationError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_http_status() {
        assert_eq!(ApplicationError::NotFound("test".to_string()).http_status(), 404);
        assert_eq!(ApplicationError::Unauthorized("test".to_string()).http_status(), 401);
        assert_eq!(ApplicationError::Forbidden("test".to_string()).http_status(), 403);
        assert_eq!(ApplicationError::ValidationFailed("test".to_string()).http_status(), 422);
        assert_eq!(ApplicationError::Conflict("test".to_string()).http_status(), 409);
        assert_eq!(ApplicationError::Internal("test".to_string()).http_status(), 500);
    }

    #[test]
    fn test_error_retryable() {
        assert!(ApplicationError::ServiceUnavailable("test".to_string()).is_retryable());
        assert!(ApplicationError::Timeout("test".to_string()).is_retryable());
        assert!(ApplicationError::RateLimitExceeded("test".to_string()).is_retryable());
        assert!(!ApplicationError::NotFound("test".to_string()).is_retryable());
        assert!(!ApplicationError::Forbidden("test".to_string()).is_retryable());
    }
}
