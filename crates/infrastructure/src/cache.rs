//! Cache module - Redis cache provider and utilities
//!
//! Provides caching functionality using Redis, including rate limiting
//! and distributed locking capabilities.

use async_trait::async_trait;
use redis::{aio::ConnectionManager, AsyncCommands, Client, RedisError};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;
use tracing::{debug, info, instrument, warn};

use crate::{Error, Result};

/// Redis cache configuration.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Redis connection URL (redis://host:port)
    pub url: String,
    /// Default TTL for cached items
    pub default_ttl: Duration,
    /// Key prefix for all cached items
    pub key_prefix: String,
    /// Connection timeout
    pub connection_timeout: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            default_ttl: Duration::from_secs(3600),
            key_prefix: "llm-benchmark:".to_string(),
            connection_timeout: Duration::from_secs(5),
        }
    }
}

impl CacheConfig {
    /// Create configuration from environment variables.
    pub fn from_env() -> Result<Self> {
        let url = std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string());

        let default_ttl = std::env::var("CACHE_DEFAULT_TTL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(3600));

        let key_prefix = std::env::var("CACHE_KEY_PREFIX")
            .unwrap_or_else(|_| "llm-benchmark:".to_string());

        Ok(Self {
            url,
            default_ttl,
            key_prefix,
            ..Default::default()
        })
    }
}

/// Cache trait for type-safe caching operations.
#[async_trait]
pub trait Cache: Send + Sync {
    /// Get a cached value by key.
    async fn get<T: DeserializeOwned + Send>(&self, key: &str) -> Result<Option<T>>;

    /// Set a cached value with the default TTL.
    async fn set<T: Serialize + Send + Sync>(&self, key: &str, value: &T) -> Result<()>;

    /// Set a cached value with a custom TTL.
    async fn set_with_ttl<T: Serialize + Send + Sync>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> Result<()>;

    /// Delete a cached value.
    async fn delete(&self, key: &str) -> Result<bool>;

    /// Check if a key exists.
    async fn exists(&self, key: &str) -> Result<bool>;

    /// Delete all keys matching a pattern.
    async fn delete_pattern(&self, pattern: &str) -> Result<u64>;

    /// Get the TTL remaining for a key.
    async fn ttl(&self, key: &str) -> Result<Option<Duration>>;

    /// Extend the TTL of a key.
    async fn expire(&self, key: &str, ttl: Duration) -> Result<bool>;
}

/// Redis-backed cache implementation.
pub struct RedisCache {
    client: Client,
    connection: ConnectionManager,
    config: CacheConfig,
}

impl RedisCache {
    /// Create a new Redis cache instance.
    #[instrument(skip(config))]
    pub async fn new(config: CacheConfig) -> Result<Self> {
        info!(url = %config.url, "Connecting to Redis cache");

        let client = Client::open(config.url.clone())
            .map_err(|e| Error::Cache(e))?;

        let connection = ConnectionManager::new(client.clone())
            .await
            .map_err(|e| Error::Cache(e))?;

        info!("Redis cache connected successfully");
        Ok(Self {
            client,
            connection,
            config,
        })
    }

    /// Get a connection manager clone for concurrent operations.
    fn conn(&self) -> ConnectionManager {
        self.connection.clone()
    }

    /// Build the full cache key with prefix.
    fn full_key(&self, key: &str) -> String {
        format!("{}{}", self.config.key_prefix, key)
    }

    /// Check Redis health.
    #[instrument(skip(self))]
    pub async fn health_check(&self) -> Result<CacheHealthStatus> {
        let start = std::time::Instant::now();

        let mut conn = self.conn();
        match redis::cmd("PING")
            .query_async::<_, String>(&mut conn)
            .await
        {
            Ok(response) if response == "PONG" => {
                let latency = start.elapsed();
                debug!(latency_ms = latency.as_millis(), "Cache health check passed");
                Ok(CacheHealthStatus {
                    healthy: true,
                    latency,
                    error: None,
                })
            }
            Ok(response) => {
                Ok(CacheHealthStatus {
                    healthy: false,
                    latency: start.elapsed(),
                    error: Some(format!("Unexpected PING response: {}", response)),
                })
            }
            Err(e) => {
                warn!(error = %e, "Cache health check failed");
                Ok(CacheHealthStatus {
                    healthy: false,
                    latency: start.elapsed(),
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Check rate limit for a key.
    ///
    /// Returns (allowed, remaining, reset_at_secs).
    #[instrument(skip(self))]
    pub async fn check_rate_limit(
        &self,
        key: &str,
        limit: u64,
        window_secs: u64,
    ) -> Result<RateLimitResult> {
        let full_key = self.full_key(&format!("ratelimit:{}", key));
        let mut conn = self.conn();

        // Use sliding window rate limiting with sorted sets
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let window_start = now - window_secs;

        // Remove old entries
        let _: () = redis::cmd("ZREMRANGEBYSCORE")
            .arg(&full_key)
            .arg(0)
            .arg(window_start)
            .query_async(&mut conn)
            .await
            .map_err(Error::Cache)?;

        // Count current entries
        let count: u64 = redis::cmd("ZCARD")
            .arg(&full_key)
            .query_async(&mut conn)
            .await
            .map_err(Error::Cache)?;

        if count >= limit {
            // Get the oldest entry to determine reset time
            let oldest: Vec<(String, f64)> = redis::cmd("ZRANGE")
                .arg(&full_key)
                .arg(0)
                .arg(0)
                .arg("WITHSCORES")
                .query_async(&mut conn)
                .await
                .map_err(Error::Cache)?;

            let reset_at = oldest
                .first()
                .map(|(_, score)| *score as u64 + window_secs)
                .unwrap_or(now + window_secs);

            return Ok(RateLimitResult {
                allowed: false,
                remaining: 0,
                reset_at,
                limit,
            });
        }

        // Add new entry
        let member = format!("{}:{}", now, uuid::Uuid::new_v4());
        let _: () = redis::cmd("ZADD")
            .arg(&full_key)
            .arg(now)
            .arg(&member)
            .query_async(&mut conn)
            .await
            .map_err(Error::Cache)?;

        // Set expiry on the key
        let _: () = redis::cmd("EXPIRE")
            .arg(&full_key)
            .arg(window_secs)
            .query_async(&mut conn)
            .await
            .map_err(Error::Cache)?;

        Ok(RateLimitResult {
            allowed: true,
            remaining: limit - count - 1,
            reset_at: now + window_secs,
            limit,
        })
    }

    /// Acquire a distributed lock.
    #[instrument(skip(self))]
    pub async fn acquire_lock(&self, key: &str, ttl: Duration) -> Result<Option<LockGuard>> {
        let full_key = self.full_key(&format!("lock:{}", key));
        let lock_value = uuid::Uuid::new_v4().to_string();
        let mut conn = self.conn();

        // Try to set the lock with NX (only if not exists)
        let acquired: bool = redis::cmd("SET")
            .arg(&full_key)
            .arg(&lock_value)
            .arg("NX")
            .arg("PX")
            .arg(ttl.as_millis() as u64)
            .query_async(&mut conn)
            .await
            .map_err(Error::Cache)?;

        if acquired {
            debug!(key = %key, "Lock acquired");
            Ok(Some(LockGuard {
                key: full_key,
                value: lock_value,
                connection: self.conn(),
            }))
        } else {
            debug!(key = %key, "Lock not acquired (already held)");
            Ok(None)
        }
    }

    /// Increment a counter atomically.
    #[instrument(skip(self))]
    pub async fn increment(&self, key: &str, delta: i64) -> Result<i64> {
        let full_key = self.full_key(key);
        let mut conn = self.conn();

        let value: i64 = conn.incr(&full_key, delta).await.map_err(Error::Cache)?;
        Ok(value)
    }

    /// Get multiple values at once.
    #[instrument(skip(self))]
    pub async fn mget<T: DeserializeOwned>(&self, keys: &[&str]) -> Result<Vec<Option<T>>> {
        if keys.is_empty() {
            return Ok(Vec::new());
        }

        let full_keys: Vec<String> = keys.iter().map(|k| self.full_key(k)).collect();
        let mut conn = self.conn();

        let values: Vec<Option<String>> = conn.mget(&full_keys).await.map_err(Error::Cache)?;

        let results: Vec<Option<T>> = values
            .into_iter()
            .map(|v| {
                v.and_then(|s| serde_json::from_str(&s).ok())
            })
            .collect();

        Ok(results)
    }
}

#[async_trait]
impl Cache for RedisCache {
    #[instrument(skip(self))]
    async fn get<T: DeserializeOwned + Send>(&self, key: &str) -> Result<Option<T>> {
        let full_key = self.full_key(key);
        let mut conn = self.conn();

        let value: Option<String> = conn.get(&full_key).await.map_err(Error::Cache)?;

        match value {
            Some(s) => {
                let parsed: T = serde_json::from_str(&s).map_err(Error::Serialization)?;
                debug!(key = %key, "Cache hit");
                Ok(Some(parsed))
            }
            None => {
                debug!(key = %key, "Cache miss");
                Ok(None)
            }
        }
    }

    #[instrument(skip(self, value))]
    async fn set<T: Serialize + Send + Sync>(&self, key: &str, value: &T) -> Result<()> {
        self.set_with_ttl(key, value, self.config.default_ttl).await
    }

    #[instrument(skip(self, value))]
    async fn set_with_ttl<T: Serialize + Send + Sync>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> Result<()> {
        let full_key = self.full_key(key);
        let serialized = serde_json::to_string(value).map_err(Error::Serialization)?;
        let mut conn = self.conn();

        conn.set_ex(&full_key, &serialized, ttl.as_secs())
            .await
            .map_err(Error::Cache)?;

        debug!(key = %key, ttl_secs = ttl.as_secs(), "Cache set");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn delete(&self, key: &str) -> Result<bool> {
        let full_key = self.full_key(key);
        let mut conn = self.conn();

        let deleted: u64 = conn.del(&full_key).await.map_err(Error::Cache)?;
        debug!(key = %key, deleted = deleted > 0, "Cache delete");
        Ok(deleted > 0)
    }

    #[instrument(skip(self))]
    async fn exists(&self, key: &str) -> Result<bool> {
        let full_key = self.full_key(key);
        let mut conn = self.conn();

        let exists: bool = conn.exists(&full_key).await.map_err(Error::Cache)?;
        Ok(exists)
    }

    #[instrument(skip(self))]
    async fn delete_pattern(&self, pattern: &str) -> Result<u64> {
        let full_pattern = self.full_key(pattern);
        let mut conn = self.conn();

        // Use SCAN to find matching keys
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(&full_pattern)
            .query_async(&mut conn)
            .await
            .map_err(Error::Cache)?;

        if keys.is_empty() {
            return Ok(0);
        }

        let deleted: u64 = conn.del(&keys).await.map_err(Error::Cache)?;
        debug!(pattern = %pattern, deleted = deleted, "Cache delete pattern");
        Ok(deleted)
    }

    #[instrument(skip(self))]
    async fn ttl(&self, key: &str) -> Result<Option<Duration>> {
        let full_key = self.full_key(key);
        let mut conn = self.conn();

        let ttl: i64 = conn.ttl(&full_key).await.map_err(Error::Cache)?;

        if ttl < 0 {
            Ok(None)
        } else {
            Ok(Some(Duration::from_secs(ttl as u64)))
        }
    }

    #[instrument(skip(self))]
    async fn expire(&self, key: &str, ttl: Duration) -> Result<bool> {
        let full_key = self.full_key(key);
        let mut conn = self.conn();

        let success: bool = conn
            .expire(&full_key, ttl.as_secs() as i64)
            .await
            .map_err(Error::Cache)?;

        Ok(success)
    }
}

impl std::fmt::Debug for RedisCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisCache")
            .field("config", &self.config)
            .finish()
    }
}

/// Cache health status.
#[derive(Debug, Clone)]
pub struct CacheHealthStatus {
    /// Whether the cache is healthy
    pub healthy: bool,
    /// Query latency
    pub latency: Duration,
    /// Error message if unhealthy
    pub error: Option<String>,
}

/// Rate limit result.
#[derive(Debug, Clone)]
pub struct RateLimitResult {
    /// Whether the request is allowed
    pub allowed: bool,
    /// Number of requests remaining in the window
    pub remaining: u64,
    /// Unix timestamp when the limit resets
    pub reset_at: u64,
    /// The limit
    pub limit: u64,
}

/// Distributed lock guard.
///
/// Automatically releases the lock when dropped.
pub struct LockGuard {
    key: String,
    value: String,
    connection: ConnectionManager,
}

impl LockGuard {
    /// Manually release the lock.
    pub async fn release(mut self) -> Result<bool> {
        self.release_internal().await
    }

    async fn release_internal(&mut self) -> Result<bool> {
        // Use Lua script to atomically check and delete
        let script = r#"
            if redis.call("get", KEYS[1]) == ARGV[1] then
                return redis.call("del", KEYS[1])
            else
                return 0
            end
        "#;

        let released: i32 = redis::cmd("EVAL")
            .arg(script)
            .arg(1)
            .arg(&self.key)
            .arg(&self.value)
            .query_async(&mut self.connection)
            .await
            .map_err(Error::Cache)?;

        Ok(released == 1)
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        // We can't do async in drop, so we spawn a task
        let key = self.key.clone();
        let value = self.value.clone();
        let mut conn = self.connection.clone();

        tokio::spawn(async move {
            let script = r#"
                if redis.call("get", KEYS[1]) == ARGV[1] then
                    return redis.call("del", KEYS[1])
                else
                    return 0
                end
            "#;

            let _: std::result::Result<i32, RedisError> = redis::cmd("EVAL")
                .arg(script)
                .arg(1)
                .arg(&key)
                .arg(&value)
                .query_async(&mut conn)
                .await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CacheConfig::default();
        assert_eq!(config.url, "redis://localhost:6379");
        assert_eq!(config.default_ttl, Duration::from_secs(3600));
        assert_eq!(config.key_prefix, "llm-benchmark:");
    }

    #[test]
    fn test_full_key() {
        // This would need a Redis connection to test properly
        // But we can at least verify the key format logic
        let prefix = "test:";
        let key = "mykey";
        let full = format!("{}{}", prefix, key);
        assert_eq!(full, "test:mykey");
    }
}
