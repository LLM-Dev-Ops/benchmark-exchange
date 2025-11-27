//! Error types for the LLM Benchmark Exchange domain.
//!
//! This module defines a comprehensive error hierarchy for all domain operations,
//! providing structured error information with HTTP status codes and error codes
//! for API responses.

use crate::identifiers::*;
use crate::version::VersionParseError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Top-level application error type
///
/// This enum encompasses all possible error types that can occur within the
/// application, providing a unified error handling mechanism.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Benchmark-related errors
    #[error("Benchmark error: {0}")]
    Benchmark(#[from] BenchmarkError),

    /// Submission-related errors
    #[error("Submission error: {0}")]
    Submission(#[from] SubmissionError),

    /// Verification-related errors
    #[error("Verification error: {0}")]
    Verification(#[from] VerificationError),

    /// Governance-related errors
    #[error("Governance error: {0}")]
    Governance(#[from] GovernanceError),

    /// Authorization-related errors
    #[error("Authorization error: {0}")]
    Authorization(#[from] AuthorizationError),

    /// Validation-related errors
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    /// Integration-related errors (external systems)
    #[error("Integration error: {0}")]
    Integration(#[from] IntegrationError),

    /// Database-related errors
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),

    /// Internal server errors
    #[error("Internal error: {0}")]
    Internal(String),
}

impl AppError {
    /// Get the error code for this error
    ///
    /// Error codes are used in API responses for programmatic error handling.
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::Benchmark(_) => "BENCHMARK_ERROR",
            Self::Submission(_) => "SUBMISSION_ERROR",
            Self::Verification(_) => "VERIFICATION_ERROR",
            Self::Governance(_) => "GOVERNANCE_ERROR",
            Self::Authorization(_) => "AUTHORIZATION_ERROR",
            Self::Validation(_) => "VALIDATION_ERROR",
            Self::Integration(_) => "INTEGRATION_ERROR",
            Self::Database(_) => "DATABASE_ERROR",
            Self::Internal(_) => "INTERNAL_ERROR",
        }
    }

    /// Get the HTTP status code for this error
    pub fn http_status(&self) -> u16 {
        match self {
            Self::Authorization(_) => 403,
            Self::Validation(_) => 400,
            Self::Benchmark(BenchmarkError::NotFound(_)) => 404,
            Self::Submission(SubmissionError::NotFound(_)) => 404,
            Self::Governance(GovernanceError::ProposalNotFound(_)) => 404,
            Self::Database(_) => 503,
            Self::Internal(_) => 500,
            _ => 400,
        }
    }

    /// Check if this error is retryable
    ///
    /// Retryable errors are typically transient issues like database
    /// connection failures or external service timeouts.
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Database(_) | Self::Integration(_))
    }
}

/// Benchmark-specific errors
#[derive(Debug, thiserror::Error)]
pub enum BenchmarkError {
    /// Benchmark not found
    #[error("Benchmark not found: {0}")]
    NotFound(BenchmarkId),

    /// Benchmark version not found
    #[error("Benchmark version not found: {0}")]
    VersionNotFound(BenchmarkVersionId),

    /// Invalid benchmark definition
    #[error("Invalid benchmark definition: {0}")]
    InvalidDefinition(String),

    /// Invalid status transition
    #[error("Benchmark status transition not allowed: {from:?} -> {to:?}")]
    InvalidStatusTransition {
        from: crate::benchmark::BenchmarkStatus,
        to: crate::benchmark::BenchmarkStatus,
    },

    /// Duplicate benchmark slug
    #[error("Duplicate benchmark slug: {0}")]
    DuplicateSlug(String),

    /// Dataset validation failed
    #[error("Dataset validation failed: {0}")]
    DatasetValidation(String),

    /// Test case validation failed
    #[error("Test case validation failed: {0}")]
    TestCaseValidation(String),
}

/// Submission-specific errors
#[derive(Debug, thiserror::Error)]
pub enum SubmissionError {
    /// Submission not found
    #[error("Submission not found: {0}")]
    NotFound(SubmissionId),

    /// Benchmark not active for submissions
    #[error("Benchmark not active for submissions")]
    BenchmarkNotActive,

    /// Invalid results format
    #[error("Invalid results format: {0}")]
    InvalidResults(String),

    /// Missing required test cases
    #[error("Missing required test cases: {0:?}")]
    MissingTestCases(Vec<String>),

    /// Score out of valid range
    #[error("Score out of valid range: {score} not in [{min}, {max}]")]
    ScoreOutOfRange { score: f64, min: f64, max: f64 },

    /// Duplicate submission for model version
    #[error("Duplicate submission for model version")]
    DuplicateSubmission,

    /// Rate limit exceeded
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    /// Execution metadata incomplete
    #[error("Execution metadata incomplete: {0}")]
    IncompleteMetadata(String),
}

/// Verification-specific errors
#[derive(Debug, thiserror::Error)]
pub enum VerificationError {
    /// Verification not found
    #[error("Verification not found: {0}")]
    NotFound(VerificationId),

    /// Verification already in progress
    #[error("Verification already in progress")]
    AlreadyInProgress,

    /// Cannot verify submission in current state
    #[error("Cannot verify submission in current state")]
    InvalidSubmissionState,

    /// Reproduction failed
    #[error("Reproduction failed: {0}")]
    ReproductionFailed(String),

    /// Score variance too high
    #[error("Score variance too high: {variance} > {threshold}")]
    HighScoreVariance { variance: f64, threshold: f64 },

    /// Environment mismatch
    #[error("Environment mismatch: {0}")]
    EnvironmentMismatch(String),
}

/// Governance-specific errors
#[derive(Debug, thiserror::Error)]
pub enum GovernanceError {
    /// Proposal not found
    #[error("Proposal not found: {0}")]
    ProposalNotFound(ProposalId),

    /// Voting period not active
    #[error("Voting period not active")]
    VotingNotActive,

    /// Already voted on this proposal
    #[error("Already voted on this proposal")]
    AlreadyVoted,

    /// Insufficient voting power
    #[error("Insufficient voting power")]
    InsufficientVotingPower,

    /// Proposal cannot be modified in current state
    #[error("Proposal cannot be modified in current state")]
    ProposalNotModifiable,

    /// Quorum not reached
    #[error("Quorum not reached")]
    QuorumNotReached,
}

/// Authorization errors
#[derive(Debug, thiserror::Error)]
pub enum AuthorizationError {
    /// Authentication required
    #[error("Authentication required")]
    AuthenticationRequired,

    /// Invalid credentials
    #[error("Invalid credentials")]
    InvalidCredentials,

    /// Token expired
    #[error("Token expired")]
    TokenExpired,

    /// Insufficient permissions for action
    #[error("Insufficient permissions for action: {action}")]
    InsufficientPermissions { action: String },

    /// Resource access denied
    #[error("Resource access denied")]
    AccessDenied,

    /// Account suspended
    #[error("Account suspended")]
    AccountSuspended,
}

/// Validation errors
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    /// Field validation failed
    #[error("Field validation failed: {field} - {message}")]
    FieldValidation { field: String, message: String },

    /// Multiple validation errors
    #[error("Multiple validation errors: {0:?}")]
    Multiple(Vec<String>),

    /// Version parse error
    #[error("Version parse error: {0}")]
    VersionParse(#[from] VersionParseError),

    /// Invalid URL
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// Invalid checksum
    #[error("Invalid checksum: {0}")]
    InvalidChecksum(String),
}

/// Integration errors (LLM DevOps modules and external services)
#[derive(Debug, thiserror::Error)]
pub enum IntegrationError {
    /// LLM-Test-Bench connection failed
    #[error("LLM-Test-Bench connection failed: {0}")]
    TestBenchConnection(String),

    /// LLM-Registry sync failed
    #[error("LLM-Registry sync failed: {0}")]
    RegistrySync(String),

    /// LLM-Analytics-Hub export failed
    #[error("LLM-Analytics-Hub export failed: {0}")]
    AnalyticsExport(String),

    /// External API error
    #[error("External API error: {service} - {message}")]
    ExternalApi { service: String, message: String },

    /// Timeout waiting for service
    #[error("Timeout waiting for {service}: {timeout_ms}ms")]
    Timeout { service: String, timeout_ms: u64 },
}

/// Database errors
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    /// Connection pool exhausted
    #[error("Connection pool exhausted")]
    PoolExhausted,

    /// Query execution failed
    #[error("Query execution failed: {0}")]
    QueryFailed(String),

    /// Transaction failed
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    /// Constraint violation
    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Standardized API error response
///
/// This structure is returned in API responses to provide
/// consistent error information to clients.
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error details
    pub error: ErrorDetail,

    /// Unique request identifier for tracing
    pub request_id: String,

    /// Timestamp when the error occurred
    pub timestamp: DateTime<Utc>,
}

/// Detailed error information
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorDetail {
    /// Machine-readable error code
    pub code: String,

    /// Human-readable error message
    pub message: String,

    /// Additional error details (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,

    /// Link to documentation for this error (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help_url: Option<String>,
}

impl From<AppError> for ErrorResponse {
    fn from(error: AppError) -> Self {
        Self {
            error: ErrorDetail {
                code: error.error_code().to_string(),
                message: error.to_string(),
                details: None,
                help_url: None,
            },
            request_id: String::new(), // Set by middleware
            timestamp: Utc::now(),
        }
    }
}

/// Application-wide result type
pub type AppResult<T> = Result<T, AppError>;

/// Service-specific result types
pub type BenchmarkResult<T> = Result<T, BenchmarkError>;
pub type SubmissionResult<T> = Result<T, SubmissionError>;
pub type VerificationResult<T> = Result<T, VerificationError>;
pub type GovernanceResult<T> = Result<T, GovernanceError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        let err = AppError::Benchmark(BenchmarkError::NotFound(BenchmarkId::new()));
        assert_eq!(err.error_code(), "BENCHMARK_ERROR");
        assert_eq!(err.http_status(), 404);

        let err = AppError::Authorization(AuthorizationError::AuthenticationRequired);
        assert_eq!(err.error_code(), "AUTHORIZATION_ERROR");
        assert_eq!(err.http_status(), 403);
    }

    #[test]
    fn test_retryable() {
        let err = AppError::Database(DatabaseError::PoolExhausted);
        assert!(err.is_retryable());

        let err = AppError::Validation(ValidationError::InvalidUrl("bad".to_string()));
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_error_response_serialization() {
        let err = AppError::Benchmark(BenchmarkError::NotFound(BenchmarkId::new()));
        let response = ErrorResponse::from(err);

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("BENCHMARK_ERROR"));
    }
}
