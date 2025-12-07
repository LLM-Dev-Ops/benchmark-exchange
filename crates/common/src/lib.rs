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
//! As of Phase 2B, this crate integrates with LLM-Infra modules for:
//! - Configuration loading (`llm-infra-config`)
//! - Structured logging (`llm-infra-logging`)
//! - Distributed tracing (`llm-infra-tracing`)
//! - Error utilities (`llm-infra-errors`)
//! - Retry logic (`llm-infra-retry`)
//!
//! The `infra-integration` feature (enabled by default) uses these centralized modules.
//! The `legacy-local` feature falls back to local implementations (deprecated).

pub mod config;
pub mod crypto;
pub mod datetime;
pub mod pagination;
pub mod retry;
pub mod serialization;
pub mod telemetry;
pub mod validation;

// ============================================================================
// LLM-Infra Re-exports (Phase 2B Integration)
// ============================================================================

/// Re-exports from llm-infra-config for centralized configuration management.
///
/// Provides environment-based configuration loading, validation, and hierarchical
/// configuration merging from multiple sources.
#[cfg(feature = "infra-integration")]
pub mod infra {
    pub use llm_infra_config as config;
    pub use llm_infra_logging as logging;
    pub use llm_infra_tracing as tracing;
    pub use llm_infra_errors as errors;
    pub use llm_infra_retry as retry;
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

/// Common error type used throughout the crate
pub type Result<T> = std::result::Result<T, anyhow::Error>;

// ============================================================================
// Phase 2B Compatibility Layer
// ============================================================================

/// Facade for Infra retry functionality that wraps the local implementation
/// with Infra-compatible API when `infra-integration` feature is enabled.
#[cfg(feature = "infra-integration")]
pub mod infra_retry {
    //! Retry utilities powered by llm-infra-retry.
    //!
    //! This module provides a compatibility layer that exposes the same API
    //! as the local retry module while delegating to llm-infra-retry internally.

    pub use llm_infra_retry::{
        RetryConfig as InfraRetryConfig,
        retry_with_backoff as infra_retry_with_backoff,
        ExponentialBackoff as InfraExponentialBackoff,
    };

    // Re-export local types for compatibility
    pub use crate::retry::*;
}

/// Facade for Infra logging functionality.
#[cfg(feature = "infra-integration")]
pub mod infra_logging {
    //! Structured logging powered by llm-infra-logging.

    pub use llm_infra_logging::*;
}

/// Facade for Infra tracing functionality.
#[cfg(feature = "infra-integration")]
pub mod infra_tracing {
    //! Distributed tracing powered by llm-infra-tracing.

    pub use llm_infra_tracing::*;
}
