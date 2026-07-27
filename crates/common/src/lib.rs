//! Common utilities and shared functionality for the LLM Benchmark Exchange platform.
//!
//! This crate provides foundational utilities used across all services including:
//! - Configuration management
//! - Telemetry and observability
//! - Pagination helpers
//! - Cryptography utilities
//! - DateTime operations
//! - Serialization helpers
//! - Validation utilities
//! - Retry logic with backoff
//!
//! ## Phase 2B Infra Integration
//!
//! As of Phase 2B, this crate integrates with the shared `infra` crates for:
//! - Configuration loading (`infra-config`)
//! - Structured logging and distributed tracing (`infra-otel`)
//! - Error utilities (`infra-errors`)
//! - Retry logic (`infra-retry`)
//!
//! The `infra-integration` feature (enabled by default) uses these centralized modules.
//! The `legacy-local` feature falls back to local implementations (deprecated).

pub mod config;
pub mod crypto;
pub mod datetime;
pub mod execution;
pub mod pagination;
pub mod retry;
pub mod serialization;
pub mod telemetry;
pub mod validation;

// ============================================================================
// Infra Re-exports (Phase 2B Integration)
// ============================================================================

/// Re-exports from the shared `infra` crates.
///
/// Provides environment-based configuration loading, telemetry, error types,
/// and retry utilities.
#[cfg(feature = "infra-integration")]
pub mod infra {
    pub use infra_config as config;
    pub use infra_errors as errors;
    pub use infra_otel as otel;
    pub use infra_retry as retry;
}

// ============================================================================
// Local Re-exports (maintained for compatibility)
// ============================================================================

// Re-export commonly used types
pub use config::{
    AppConfig, ArchitectureConfig, FeatureFlags,
    CacheProvider, StorageProvider, MessagingProvider,
    ValidationMode, AuthorizationMode,
};
pub use crypto::{hash_password, verify_password, generate_token, ChecksumVerifier};
pub use datetime::{now_utc, parse_datetime, format_datetime};
pub use pagination::{PaginationParams, SortParams, SortDirection, PaginatedResult, DateRange};
pub use retry::{RetryConfig, retry_with_backoff, ExponentialBackoff};
pub use telemetry::{init_tracing, create_meter};
pub use validation::{validate_slug, validate_email, validate_url};
pub use execution::{
    ExecutionContext, ExecutionError, ExecutionResult, ExecutionSpan,
    Artifact, SpanStatus, SpanType, AgentSpanGuard,
};

/// Common error type used throughout the crate
pub type Result<T> = std::result::Result<T, anyhow::Error>;

// ============================================================================
// Phase 2B Compatibility Layer
// ============================================================================

/// Facade for Infra retry functionality that wraps the local implementation
/// with Infra-compatible API when `infra-integration` feature is enabled.
#[cfg(feature = "infra-integration")]
pub mod infra_retry {
    //! Retry utilities powered by infra-retry.
    //!
    //! This module provides a compatibility layer that exposes the same API
    //! as the local retry module while delegating to infra-retry internally.

    pub use infra_retry::{
        ExponentialBackoff as InfraExponentialBackoff,
        RetryPolicy as InfraRetryPolicy,
        retry_with_policy as infra_retry_with_policy,
    };

    // Re-export local types for compatibility
    pub use crate::retry::*;
}

/// Facade for Infra telemetry functionality.
///
/// `infra` has no separate logging and tracing crates; both are served by
/// `infra-otel`.
#[cfg(feature = "infra-integration")]
pub mod infra_otel {
    //! Structured logging and distributed tracing powered by infra-otel.

    pub use infra_otel::*;
}
