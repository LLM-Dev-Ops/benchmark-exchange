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

pub mod config;
pub mod crypto;
pub mod datetime;
pub mod pagination;
pub mod retry;
pub mod serialization;
pub mod telemetry;
pub mod validation;

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
