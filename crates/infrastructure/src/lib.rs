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
//! ## Usage
//!
//! ```rust,ignore
//! use llm_benchmark_infrastructure::{
//!     database::{DatabaseConfig, DatabasePool},
//!     repositories::{BenchmarkRepository, PgBenchmarkRepository},
//!     cache::{CacheConfig, RedisCache},
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
pub mod messaging;
pub mod repositories;
pub mod storage;

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
