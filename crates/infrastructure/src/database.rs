//! Database module - PostgreSQL connection pool and utilities
//!
//! Provides connection pool management, health checks, and transaction support
//! for the LLM Benchmark Exchange platform.

use sqlx::{postgres::PgPoolOptions, PgPool, Postgres, Transaction};
use std::time::Duration;
use tracing::{debug, info, instrument, warn};

use crate::{Error, Result};

/// Database configuration for PostgreSQL connections.
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// Database connection URL (postgres://user:pass@host:port/db)
    pub url: String,
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Minimum number of connections to keep open
    pub min_connections: u32,
    /// Timeout for acquiring a connection from the pool
    pub acquire_timeout: Duration,
    /// Maximum time a connection can be idle before being closed
    pub idle_timeout: Duration,
    /// Maximum lifetime of a connection
    pub max_lifetime: Duration,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            max_connections: 100,
            min_connections: 10,
            acquire_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600),
            max_lifetime: Duration::from_secs(1800),
        }
    }
}

impl DatabaseConfig {
    /// Create a new database configuration from environment variables.
    pub fn from_env() -> Result<Self> {
        let url = std::env::var("DATABASE_URL")
            .map_err(|_| Error::Configuration("DATABASE_URL not set".to_string()))?;

        let max_connections = std::env::var("DATABASE_MAX_CONNECTIONS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100);

        let min_connections = std::env::var("DATABASE_MIN_CONNECTIONS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);

        Ok(Self {
            url,
            max_connections,
            min_connections,
            ..Default::default()
        })
    }

    /// Create a test configuration with minimal connections.
    pub fn test_config(url: String) -> Self {
        Self {
            url,
            max_connections: 5,
            min_connections: 1,
            acquire_timeout: Duration::from_secs(5),
            idle_timeout: Duration::from_secs(60),
            max_lifetime: Duration::from_secs(300),
        }
    }
}

/// Database connection pool wrapper with health monitoring.
#[derive(Clone)]
pub struct DatabasePool {
    pool: PgPool,
}

impl DatabasePool {
    /// Create a new database pool with the given configuration.
    #[instrument(skip(config), fields(max_connections = config.max_connections))]
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        info!("Initializing database connection pool");

        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(config.acquire_timeout)
            .idle_timeout(Some(config.idle_timeout))
            .max_lifetime(Some(config.max_lifetime))
            .after_connect(|conn, _meta| {
                Box::pin(async move {
                    // Set session parameters for consistency
                    sqlx::query("SET timezone = 'UTC'")
                        .execute(&mut *conn)
                        .await?;
                    sqlx::query("SET statement_timeout = '30s'")
                        .execute(&mut *conn)
                        .await?;
                    Ok(())
                })
            })
            .connect(&config.url)
            .await
            .map_err(Error::Database)?;

        info!("Database pool initialized successfully");
        Ok(Self { pool })
    }

    /// Get reference to the underlying pool.
    #[inline]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Begin a new database transaction.
    #[instrument(skip(self))]
    pub async fn begin(&self) -> Result<Transaction<'_, Postgres>> {
        debug!("Beginning new transaction");
        self.pool.begin().await.map_err(Error::Database)
    }

    /// Check database health by executing a simple query.
    #[instrument(skip(self))]
    pub async fn health_check(&self) -> Result<HealthStatus> {
        let start = std::time::Instant::now();

        match sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&self.pool)
            .await
        {
            Ok(_) => {
                let latency = start.elapsed();
                debug!(latency_ms = latency.as_millis(), "Health check passed");
                Ok(HealthStatus {
                    healthy: true,
                    latency,
                    pool_size: self.pool.size(),
                    idle_connections: self.pool.num_idle(),
                    error: None,
                })
            }
            Err(e) => {
                warn!(error = %e, "Health check failed");
                Ok(HealthStatus {
                    healthy: false,
                    latency: start.elapsed(),
                    pool_size: self.pool.size(),
                    idle_connections: self.pool.num_idle(),
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Get current pool statistics.
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            size: self.pool.size(),
            idle: self.pool.num_idle(),
        }
    }

    /// Close all connections in the pool.
    pub async fn close(&self) {
        info!("Closing database pool");
        self.pool.close().await;
    }
}

impl std::fmt::Debug for DatabasePool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DatabasePool")
            .field("size", &self.pool.size())
            .field("idle", &self.pool.num_idle())
            .finish()
    }
}

/// Health status for database connections.
#[derive(Debug, Clone)]
pub struct HealthStatus {
    /// Whether the database is healthy
    pub healthy: bool,
    /// Query latency
    pub latency: Duration,
    /// Current pool size
    pub pool_size: u32,
    /// Number of idle connections
    pub idle_connections: usize,
    /// Error message if unhealthy
    pub error: Option<String>,
}

/// Pool statistics.
#[derive(Debug, Clone, Copy)]
pub struct PoolStats {
    /// Current number of connections in the pool
    pub size: u32,
    /// Number of idle connections
    pub idle: usize,
}

/// Extension trait for transaction handling with automatic commit/rollback.
#[async_trait::async_trait]
pub trait TransactionExt {
    /// Commit if result is Ok, rollback if Err.
    async fn commit_or_rollback<T, E>(self, result: std::result::Result<T, E>) -> std::result::Result<T, E>
    where
        T: Send,
        E: From<sqlx::Error> + Send;
}

#[async_trait::async_trait]
impl TransactionExt for Transaction<'_, Postgres> {
    async fn commit_or_rollback<T, E>(self, result: std::result::Result<T, E>) -> std::result::Result<T, E>
    where
        T: Send,
        E: From<sqlx::Error> + Send,
    {
        match result {
            Ok(value) => {
                self.commit().await?;
                Ok(value)
            }
            Err(e) => {
                if let Err(rollback_err) = self.rollback().await {
                    warn!("Failed to rollback transaction: {}", rollback_err);
                }
                Err(e)
            }
        }
    }
}

/// Helper macro for executing queries with consistent error handling.
#[macro_export]
macro_rules! query_one {
    ($pool:expr, $query:expr, $($bind:expr),* $(,)?) => {{
        sqlx::query($query)
            $(.bind($bind))*
            .fetch_one($pool)
            .await
            .map_err($crate::Error::Database)
    }};
}

/// Helper macro for executing queries that may return zero or one row.
#[macro_export]
macro_rules! query_optional {
    ($pool:expr, $query:expr, $($bind:expr),* $(,)?) => {{
        sqlx::query($query)
            $(.bind($bind))*
            .fetch_optional($pool)
            .await
            .map_err($crate::Error::Database)
    }};
}

/// Helper macro for executing queries that return multiple rows.
#[macro_export]
macro_rules! query_all {
    ($pool:expr, $query:expr, $($bind:expr),* $(,)?) => {{
        sqlx::query($query)
            $(.bind($bind))*
            .fetch_all($pool)
            .await
            .map_err($crate::Error::Database)
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DatabaseConfig::default();
        assert_eq!(config.max_connections, 100);
        assert_eq!(config.min_connections, 10);
        assert_eq!(config.acquire_timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_test_config() {
        let config = DatabaseConfig::test_config("postgres://localhost/test".to_string());
        assert_eq!(config.max_connections, 5);
        assert_eq!(config.min_connections, 1);
    }
}
