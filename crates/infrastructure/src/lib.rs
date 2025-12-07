//! Infrastructure layer for LLM Benchmark Exchange
//!
//! This crate provides implementations for:
//! - Database access (PostgreSQL with sqlx)
//! - Repository pattern implementations
//! - Caching (Redis)
//! - Object storage (S3)
//! - Event messaging (Redis pub/sub)
//!
//! ## Architecture
//!
//! The infrastructure layer follows the repository pattern, providing concrete
//! implementations of data access that can be swapped for testing or different
//! storage backends.
//!
//! ## Phase 2B Infra Integration
//!
//! As of Phase 2B, this crate integrates with LLM-Infra modules for:
//! - Caching (`llm-infra-cache`) - Redis and in-memory backends
//! - Rate limiting (`llm-infra-ratelimit`) - Sliding window algorithm
//! - HTTP/gRPC clients (`llm-infra-client`) - With built-in retry and tracing
//! - Core utilities (`llm-infra-core`) - Common infrastructure patterns
//!
//! The `infra-integration` feature (enabled by default) uses these centralized modules.
//! The `legacy-local` feature falls back to local implementations (deprecated).
//!
//! ## Usage
//!
//! ```rust,ignore
//! use llm_benchmark_infrastructure::{
//!     database::{DatabaseConfig, DatabasePool},
//!     repositories::{BenchmarkRepository, PgBenchmarkRepository},
//!     cache::{CacheConfig, RedisCache},
//!     // Phase 2B: Use Infra cache for production
//!     infra::cache::InfraCache,
//! };
//!
//! // Initialize database pool
//! let db_config = DatabaseConfig::from_env()?;
//! let pool = DatabasePool::new(&db_config).await?;
//!
//! // Create repository
//! let benchmark_repo = PgBenchmarkRepository::new(pool.pool().clone());
//! ```

pub mod cache;
pub mod database;
pub mod external_consumers;
pub mod messaging;
pub mod repositories;
pub mod storage;

// ============================================================================
// LLM-Infra Re-exports (Phase 2B Integration)
// ============================================================================

/// Re-exports from LLM-Infra modules for centralized infrastructure.
///
/// These modules provide production-ready implementations for caching,
/// rate limiting, and client abstractions that are shared across the
/// LLM-Dev-Ops ecosystem.
#[cfg(feature = "infra-integration")]
pub mod infra {
    /// Caching layer powered by llm-infra-cache.
    ///
    /// Provides Redis and in-memory cache implementations with:
    /// - Automatic serialization/deserialization
    /// - TTL management
    /// - Cache invalidation patterns
    /// - Distributed locking
    pub mod cache {
        pub use llm_infra_cache::*;
    }

    /// Rate limiting powered by llm-infra-ratelimit.
    ///
    /// Provides production-ready rate limiting with:
    /// - Sliding window algorithm
    /// - Redis backend for distributed rate limiting
    /// - Configurable limits per endpoint/user
    /// - Headers for rate limit status
    pub mod ratelimit {
        pub use llm_infra_ratelimit::*;
    }

    /// HTTP and gRPC client abstractions powered by llm-infra-client.
    ///
    /// Provides client utilities with:
    /// - Automatic retry with backoff
    /// - Distributed tracing propagation
    /// - Circuit breaker pattern
    /// - Request/response logging
    pub mod client {
        pub use llm_infra_client::*;
    }

    /// Core infrastructure utilities powered by llm-infra-core.
    ///
    /// Provides common patterns and utilities shared across modules.
    pub mod core {
        pub use llm_infra_core::*;
    }
}

// Re-export commonly used types
pub use cache::{Cache, CacheConfig, CacheHealthStatus, RateLimitResult, RedisCache};
pub use database::{DatabaseConfig, DatabasePool, HealthStatus, PoolStats, TransactionExt};
pub use messaging::{
    EventMessage, MessagingConfig, MessagingHealthStatus, Publisher, RedisMessaging, Subscriber,
};
pub use repositories::{
    BenchmarkQuery, BenchmarkRecord, BenchmarkRepository, BenchmarkVersionSummary,
    LeaderboardEntry, OrganizationMember, OrganizationQuery, OrganizationRepository,
    PgBenchmarkRepository, PgOrganizationRepository, PgSubmissionRepository, PgUserRepository,
    SubmissionQuery, SubmissionRepository, UserCredentials, UserQuery, UserRepository,
};
pub use storage::{ObjectInfo, ObjectMetadata, S3Storage, Storage, StorageConfig, StorageHealthStatus};

// Re-export external consumer types
pub use external_consumers::{
    ExternalConsumerError, ExternalConsumerHealth, ExternalConsumerResult, ServiceHealth,
    // Registry consumer
    RegistryConsumer, RegistryConfig, ModelMetadata, BenchmarkDescriptor, RegistryCorpus,
    // Marketplace consumer
    MarketplaceConsumer, MarketplaceConfig, SharedTestSuite, ShieldFilter, EvaluationTemplate,
    // Observatory consumer
    ObservatoryConsumer, ObservatoryConfig, ExecutionTelemetry, PerformanceMetadata,
    // Test-Bench ingester
    TestBenchIngester, TestBenchConfig, BenchmarkResult, IngestionFormat,
};

// Re-export result and error types
pub type Result<T> = std::result::Result<T, Error>;

/// Infrastructure-level errors
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Database errors from sqlx
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Cache errors from Redis
    #[error("Cache error: {0}")]
    Cache(#[from] redis::RedisError),

    /// Storage errors from S3 operations
    #[error("Storage error: {0}")]
    Storage(String),

    /// Messaging errors
    #[error("Messaging error: {0}")]
    Messaging(String),

    /// Serialization/deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Resource not found errors
    #[error("Not found: {0}")]
    NotFound(String),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Connection errors
    #[error("Connection error: {0}")]
    Connection(String),

    /// Timeout errors
    #[error("Timeout: {0}")]
    Timeout(String),
}

impl Error {
    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Error::Database(_) | Error::Cache(_) | Error::Connection(_) | Error::Timeout(_)
        )
    }

    /// Get HTTP status code for this error
    pub fn http_status(&self) -> u16 {
        match self {
            Error::NotFound(_) => 404,
            Error::Configuration(_) => 400,
            Error::Serialization(_) => 400,
            Error::Database(_) | Error::Cache(_) | Error::Storage(_) | Error::Messaging(_) => 503,
            Error::Connection(_) | Error::Timeout(_) => 503,
        }
    }
}

/// Infrastructure health check result
#[derive(Debug, Clone)]
pub struct InfrastructureHealth {
    /// Overall health status
    pub healthy: bool,
    /// Database health
    pub database: Option<HealthStatus>,
    /// Cache health
    pub cache: Option<CacheHealthStatus>,
    /// Storage health
    pub storage: Option<StorageHealthStatus>,
    /// Messaging health
    pub messaging: Option<MessagingHealthStatus>,
    /// External consumers health
    pub external_consumers: Option<ExternalConsumerHealth>,
}

impl InfrastructureHealth {
    /// Create a new health status
    pub fn new() -> Self {
        Self {
            healthy: true,
            database: None,
            cache: None,
            storage: None,
            messaging: None,
            external_consumers: None,
        }
    }

    /// Set database health
    pub fn with_database(mut self, status: HealthStatus) -> Self {
        if !status.healthy {
            self.healthy = false;
        }
        self.database = Some(status);
        self
    }

    /// Set cache health
    pub fn with_cache(mut self, status: CacheHealthStatus) -> Self {
        if !status.healthy {
            self.healthy = false;
        }
        self.cache = Some(status);
        self
    }

    /// Set storage health
    pub fn with_storage(mut self, status: StorageHealthStatus) -> Self {
        if !status.healthy {
            self.healthy = false;
        }
        self.storage = Some(status);
        self
    }

    /// Set messaging health
    pub fn with_messaging(mut self, status: MessagingHealthStatus) -> Self {
        if !status.healthy {
            self.healthy = false;
        }
        self.messaging = Some(status);
        self
    }

    /// Set external consumers health
    pub fn with_external_consumers(mut self, status: ExternalConsumerHealth) -> Self {
        if !status.healthy {
            self.healthy = false;
        }
        self.external_consumers = Some(status);
        self
    }
}

impl Default for InfrastructureHealth {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_retryable() {
        let db_err = Error::Database(sqlx::Error::PoolTimedOut);
        assert!(db_err.is_retryable());

        let not_found = Error::NotFound("test".to_string());
        assert!(!not_found.is_retryable());
    }

    #[test]
    fn test_error_http_status() {
        let not_found = Error::NotFound("test".to_string());
        assert_eq!(not_found.http_status(), 404);

        let config = Error::Configuration("bad config".to_string());
        assert_eq!(config.http_status(), 400);
    }

    #[test]
    fn test_infrastructure_health() {
        let health = InfrastructureHealth::new();
        assert!(health.healthy);

        let health = health.with_database(HealthStatus {
            healthy: false,
            latency: std::time::Duration::from_millis(100),
            pool_size: 10,
            idle_connections: 5,
            error: Some("connection failed".to_string()),
        });
        assert!(!health.healthy);
    }
}
