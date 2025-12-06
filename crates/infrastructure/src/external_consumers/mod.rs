//! External Consumer Adapters for LLM-Dev-Ops Ecosystem
//!
//! This module provides thin runtime adapters for consuming data from:
//! - LLM-Registry: Model metadata, benchmark descriptors, registry-linked corpora
//! - LLM-Marketplace: Shared test suites, shield filters, evaluation templates
//! - LLM-Observatory: Telemetry, benchmark execution statistics, performance metadata
//! - LLM-Test-Bench: Runtime file-based ingestion (no compile-time dependency)
//!
//! All adapters are additive and do not modify existing exchange logic or public APIs.

pub mod registry;
pub mod marketplace;
pub mod observatory;
pub mod testbench;

// Re-export adapter types
pub use registry::{
    RegistryConsumer, RegistryConfig, ModelMetadata, BenchmarkDescriptor, RegistryCorpus,
};
pub use marketplace::{
    MarketplaceConsumer, MarketplaceConfig, SharedTestSuite, ShieldFilter, EvaluationTemplate,
};
pub use observatory::{
    ObservatoryConsumer, ObservatoryConfig, ExecutionTelemetry, PerformanceMetadata,
};
pub use testbench::{
    TestBenchIngester, TestBenchConfig, BenchmarkResult, IngestionFormat,
};

use thiserror::Error;

/// Errors from external consumer operations
#[derive(Error, Debug, Clone)]
pub enum ExternalConsumerError {
    /// Connection failed to external service
    #[error("Connection failed to {service}: {message}")]
    ConnectionFailed { service: String, message: String },

    /// Service unavailable
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    /// Invalid response from external service
    #[error("Invalid response from {service}: {message}")]
    InvalidResponse { service: String, message: String },

    /// Resource not found
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Parse error for file-based ingestion
    #[error("Parse error: {0}")]
    ParseError(String),

    /// IO error for file operations
    #[error("IO error: {0}")]
    IoError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded for {service}")]
    RateLimitExceeded { service: String },

    /// Timeout
    #[error("Timeout waiting for {service}")]
    Timeout { service: String },
}

impl ExternalConsumerError {
    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ExternalConsumerError::ConnectionFailed { .. }
                | ExternalConsumerError::ServiceUnavailable(_)
                | ExternalConsumerError::RateLimitExceeded { .. }
                | ExternalConsumerError::Timeout { .. }
        )
    }
}

/// Result type for external consumer operations
pub type ExternalConsumerResult<T> = Result<T, ExternalConsumerError>;

/// Health status for external consumers
#[derive(Debug, Clone)]
pub struct ExternalConsumerHealth {
    /// Registry connection status
    pub registry: Option<ServiceHealth>,
    /// Marketplace connection status
    pub marketplace: Option<ServiceHealth>,
    /// Observatory connection status
    pub observatory: Option<ServiceHealth>,
    /// Overall health
    pub healthy: bool,
}

/// Individual service health
#[derive(Debug, Clone)]
pub struct ServiceHealth {
    /// Whether the service is healthy
    pub healthy: bool,
    /// Latency to service
    pub latency_ms: u64,
    /// Error message if unhealthy
    pub error: Option<String>,
}

impl Default for ExternalConsumerHealth {
    fn default() -> Self {
        Self {
            registry: None,
            marketplace: None,
            observatory: None,
            healthy: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_retryable() {
        let conn_err = ExternalConsumerError::ConnectionFailed {
            service: "registry".to_string(),
            message: "timeout".to_string(),
        };
        assert!(conn_err.is_retryable());

        let not_found = ExternalConsumerError::NotFound("resource".to_string());
        assert!(!not_found.is_retryable());
    }
}
