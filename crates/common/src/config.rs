//! Configuration management for the application.
//!
//! This module provides a centralized configuration system that loads settings
//! from environment variables and configuration files.
//!
//! ## Architecture Options
//!
//! End-users can select their preferred options for each use case:
//!
//! - **Cache Provider**: InMemory, Redis, Memcached, or None
//! - **Storage Provider**: S3, GCS, Azure Blob, or Local Filesystem
//! - **Messaging Provider**: InMemory, Redis Pub/Sub, Redis Streams, Kafka, AWS SQS/SNS, GCP Pub/Sub
//! - **Validation Mode**: Lenient, Standard, or Strict
//! - **Authorization Mode**: None, RBAC, ABAC, or Policy-based
//!
//! ## Example Configuration
//!
//! ```toml
//! [architecture]
//! cache_provider = "redis"
//! storage_provider = "s3"
//! messaging_provider = "redis_streams"
//! validation_mode = "strict"
//! authorization_mode = "rbac"
//!
//! [features]
//! rate_limiting = true
//! caching = true
//! event_sourcing = false
//! audit_logging = true
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub s3: S3Config,
    pub auth: AuthConfig,
    pub telemetry: TelemetryConfig,
    /// Architecture options for flexible component selection
    #[serde(default)]
    pub architecture: ArchitectureConfig,
    /// Feature flags for toggling functionality
    #[serde(default)]
    pub features: FeatureFlags,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Host to bind to (e.g., "0.0.0.0")
    #[serde(default = "default_host")]
    pub host: String,

    /// Port to listen on
    #[serde(default = "default_port")]
    pub port: u16,

    /// Number of worker threads
    #[serde(default = "default_workers")]
    pub workers: usize,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database connection URL
    pub url: String,

    /// Connection pool size
    #[serde(default = "default_pool_size")]
    pub pool_size: u32,

    /// Connection timeout in seconds
    #[serde(default = "default_db_timeout")]
    pub timeout_seconds: u64,
}

/// Redis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis connection URL
    pub url: String,

    /// Connection pool size
    #[serde(default = "default_redis_pool_size")]
    pub pool_size: u32,
}

/// S3 configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    /// S3 bucket name
    pub bucket: String,

    /// AWS region
    pub region: String,

    /// Custom S3 endpoint (for S3-compatible services)
    pub endpoint: Option<String>,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// JWT secret key for signing tokens
    pub jwt_secret: String,

    /// Token expiry duration in seconds
    #[serde(default = "default_token_expiry")]
    pub token_expiry_seconds: u64,
}

/// Telemetry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// OpenTelemetry collector endpoint
    pub otlp_endpoint: Option<String>,

    /// Service name for tracing
    #[serde(default = "default_service_name")]
    pub service_name: String,

    /// Enable JSON logging format
    #[serde(default = "default_json_logging")]
    pub json_logging: bool,

    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

// Default value functions
fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_workers() -> usize {
    num_cpus::get()
}

fn default_pool_size() -> u32 {
    10
}

fn default_db_timeout() -> u64 {
    30
}

fn default_redis_pool_size() -> u32 {
    5
}

fn default_token_expiry() -> u64 {
    3600 // 1 hour
}

fn default_service_name() -> String {
    "llm-benchmark-exchange".to_string()
}

fn default_json_logging() -> bool {
    false
}

fn default_log_level() -> String {
    "info".to_string()
}

impl AppConfig {
    /// Load configuration from environment variables and configuration files.
    ///
    /// The configuration is loaded in the following order (later sources override earlier ones):
    /// 1. Default values
    /// 2. config/default.toml (if exists)
    /// 3. config/{environment}.toml (if exists, where environment is from APP_ENV)
    /// 4. Environment variables (prefixed with APP_)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use common::config::AppConfig;
    ///
    /// let config = AppConfig::load().expect("Failed to load configuration");
    /// println!("Server will run on {}:{}", config.server.host, config.server.port);
    /// ```
    pub fn load() -> Result<Self> {
        let env = std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());

        let config = config::Config::builder()
            // Start with default configuration file
            .add_source(
                config::File::with_name("config/default")
                    .required(false)
            )
            // Add environment-specific configuration
            .add_source(
                config::File::with_name(&format!("config/{}", env))
                    .required(false)
            )
            // Add environment variables (prefix: APP_)
            // Example: APP_SERVER__PORT=3000
            .add_source(
                config::Environment::with_prefix("APP")
                    .separator("__")
                    .try_parsing(true)
            )
            .build()
            .context("Failed to build configuration")?;

        let app_config: AppConfig = config
            .try_deserialize()
            .context("Failed to deserialize configuration")?;

        // Validate the configuration
        app_config.validate()?;

        Ok(app_config)
    }

    /// Validate the configuration
    fn validate(&self) -> Result<()> {
        // Validate server config
        if self.server.port == 0 {
            anyhow::bail!("Server port must be greater than 0");
        }

        if self.server.workers == 0 {
            anyhow::bail!("Number of workers must be greater than 0");
        }

        // Validate database config
        if self.database.url.is_empty() {
            anyhow::bail!("Database URL is required");
        }

        if self.database.pool_size == 0 {
            anyhow::bail!("Database pool size must be greater than 0");
        }

        if self.database.timeout_seconds == 0 {
            anyhow::bail!("Database timeout must be greater than 0");
        }

        // Validate Redis config
        if self.redis.url.is_empty() {
            anyhow::bail!("Redis URL is required");
        }

        if self.redis.pool_size == 0 {
            anyhow::bail!("Redis pool size must be greater than 0");
        }

        // Validate S3 config
        if self.s3.bucket.is_empty() {
            anyhow::bail!("S3 bucket name is required");
        }

        if self.s3.region.is_empty() {
            anyhow::bail!("S3 region is required");
        }

        // Validate auth config
        if self.auth.jwt_secret.is_empty() {
            anyhow::bail!("JWT secret is required");
        }

        if self.auth.jwt_secret.len() < 32 {
            anyhow::bail!("JWT secret must be at least 32 characters long");
        }

        if self.auth.token_expiry_seconds == 0 {
            anyhow::bail!("Token expiry must be greater than 0");
        }

        // Validate telemetry config
        let valid_log_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_log_levels.contains(&self.telemetry.log_level.as_str()) {
            anyhow::bail!(
                "Invalid log level '{}'. Must be one of: {}",
                self.telemetry.log_level,
                valid_log_levels.join(", ")
            );
        }

        Ok(())
    }

    /// Get the database connection timeout as a Duration
    pub fn database_timeout(&self) -> Duration {
        Duration::from_secs(self.database.timeout_seconds)
    }

    /// Get the token expiry as a Duration
    pub fn token_expiry(&self) -> Duration {
        Duration::from_secs(self.auth.token_expiry_seconds)
    }

    /// Create a development configuration with sensible defaults
    pub fn development() -> Self {
        Self {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
                workers: 4,
            },
            database: DatabaseConfig {
                url: "postgres://localhost:5432/llm_benchmark_dev".to_string(),
                pool_size: 5,
                timeout_seconds: 30,
            },
            redis: RedisConfig {
                url: "redis://localhost:6379".to_string(),
                pool_size: 5,
            },
            s3: S3Config {
                bucket: "llm-benchmark-dev".to_string(),
                region: "us-east-1".to_string(),
                endpoint: Some("http://localhost:9000".to_string()), // MinIO
            },
            auth: AuthConfig {
                jwt_secret: "development-secret-key-minimum-32-chars".to_string(),
                token_expiry_seconds: 86400, // 24 hours
            },
            telemetry: TelemetryConfig {
                otlp_endpoint: None,
                service_name: "llm-benchmark-dev".to_string(),
                json_logging: false,
                log_level: "debug".to_string(),
            },
            architecture: ArchitectureConfig {
                cache_provider: CacheProvider::InMemory,
                storage_provider: StorageProvider::LocalFileSystem,
                messaging_provider: MessagingProvider::InMemory,
                validation_mode: ValidationMode::Lenient,
                authorization_mode: AuthorizationMode::None,
            },
            features: FeatureFlags {
                rate_limiting: false,
                caching: true,
                event_sourcing: false,
                strict_validation: false,
                audit_logging: false,
                experimental: true,
                custom: HashMap::new(),
            },
        }
    }

    /// Create a production configuration
    pub fn production() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                workers: num_cpus::get(),
            },
            database: DatabaseConfig {
                url: String::new(), // Must be provided
                pool_size: 20,
                timeout_seconds: 30,
            },
            redis: RedisConfig {
                url: String::new(), // Must be provided
                pool_size: 20,
            },
            s3: S3Config {
                bucket: String::new(), // Must be provided
                region: "us-east-1".to_string(),
                endpoint: None,
            },
            auth: AuthConfig {
                jwt_secret: String::new(), // Must be provided
                token_expiry_seconds: 3600,
            },
            telemetry: TelemetryConfig {
                otlp_endpoint: None,
                service_name: "llm-benchmark-exchange".to_string(),
                json_logging: true,
                log_level: "info".to_string(),
            },
            architecture: ArchitectureConfig::production(),
            features: FeatureFlags::production(),
        }
    }
}

// ============================================================================
// ARCHITECTURE OPTIONS
// ============================================================================

/// Architecture configuration for flexible component selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureConfig {
    /// Cache provider selection
    #[serde(default)]
    pub cache_provider: CacheProvider,
    /// Storage provider selection
    #[serde(default)]
    pub storage_provider: StorageProvider,
    /// Messaging provider selection
    #[serde(default)]
    pub messaging_provider: MessagingProvider,
    /// Validation strictness mode
    #[serde(default)]
    pub validation_mode: ValidationMode,
    /// Authorization mode
    #[serde(default)]
    pub authorization_mode: AuthorizationMode,
}

impl Default for ArchitectureConfig {
    fn default() -> Self {
        Self {
            cache_provider: CacheProvider::Redis,
            storage_provider: StorageProvider::S3,
            messaging_provider: MessagingProvider::RedisPubSub,
            validation_mode: ValidationMode::Standard,
            authorization_mode: AuthorizationMode::Rbac,
        }
    }
}

impl ArchitectureConfig {
    /// Production-ready architecture configuration
    pub fn production() -> Self {
        Self {
            cache_provider: CacheProvider::Redis,
            storage_provider: StorageProvider::S3,
            messaging_provider: MessagingProvider::RedisStreams,
            validation_mode: ValidationMode::Strict,
            authorization_mode: AuthorizationMode::Rbac,
        }
    }

    /// High-scale enterprise configuration
    pub fn enterprise() -> Self {
        Self {
            cache_provider: CacheProvider::Redis,
            storage_provider: StorageProvider::S3,
            messaging_provider: MessagingProvider::Kafka,
            validation_mode: ValidationMode::Strict,
            authorization_mode: AuthorizationMode::Policy,
        }
    }
}

/// Available cache providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CacheProvider {
    /// In-memory cache (development/testing only)
    InMemory,
    /// Redis cache (recommended for production)
    #[default]
    Redis,
    /// Memcached
    Memcached,
    /// No caching
    None,
}

impl CacheProvider {
    /// Check if this provider requires external infrastructure
    pub fn requires_infrastructure(&self) -> bool {
        matches!(self, Self::Redis | Self::Memcached)
    }
}

/// Available storage providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StorageProvider {
    /// AWS S3 or S3-compatible (MinIO, etc.)
    #[default]
    S3,
    /// Google Cloud Storage
    Gcs,
    /// Azure Blob Storage
    AzureBlob,
    /// Local filesystem (development only)
    LocalFileSystem,
}

/// Available messaging providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MessagingProvider {
    /// In-memory (development/testing only)
    InMemory,
    /// Redis Pub/Sub (simple, at-most-once delivery)
    #[default]
    RedisPubSub,
    /// Redis Streams (durable, at-least-once delivery)
    RedisStreams,
    /// Apache Kafka (enterprise scale)
    Kafka,
    /// AWS SQS + SNS
    AwsSqsSns,
    /// Google Cloud Pub/Sub
    GcpPubSub,
}

impl MessagingProvider {
    /// Check if this provider supports durable message delivery
    pub fn is_durable(&self) -> bool {
        matches!(
            self,
            Self::RedisStreams | Self::Kafka | Self::AwsSqsSns | Self::GcpPubSub
        )
    }

    /// Check if this provider supports consumer groups
    pub fn supports_consumer_groups(&self) -> bool {
        matches!(
            self,
            Self::RedisStreams | Self::Kafka | Self::AwsSqsSns | Self::GcpPubSub
        )
    }
}

/// Validation strictness mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ValidationMode {
    /// Lenient - allow flexibility, minimal validation
    Lenient,
    /// Standard - balanced validation (default)
    #[default]
    Standard,
    /// Strict - enterprise-grade validation, no flexibility
    Strict,
}

impl ValidationMode {
    /// Get minimum password length for this mode
    pub fn min_password_length(&self) -> usize {
        match self {
            Self::Lenient => 8,
            Self::Standard => 12,
            Self::Strict => 16,
        }
    }

    /// Check if this mode requires complex passwords
    pub fn requires_complex_password(&self) -> bool {
        matches!(self, Self::Standard | Self::Strict)
    }

    /// Check if this mode validates URLs strictly
    pub fn strict_url_validation(&self) -> bool {
        matches!(self, Self::Strict)
    }
}

/// Authorization mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuthorizationMode {
    /// No authorization (development only - DANGEROUS in production!)
    None,
    /// Role-based access control (default)
    #[default]
    Rbac,
    /// Attribute-based access control
    Abac,
    /// Policy-based (e.g., Open Policy Agent)
    Policy,
}

// ============================================================================
// FEATURE FLAGS
// ============================================================================

/// Feature flags for toggling functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    /// Enable rate limiting for API endpoints
    #[serde(default = "default_true")]
    pub rate_limiting: bool,
    /// Enable response caching
    #[serde(default = "default_true")]
    pub caching: bool,
    /// Enable event sourcing for audit trail
    #[serde(default)]
    pub event_sourcing: bool,
    /// Enable strict input validation
    #[serde(default = "default_true")]
    pub strict_validation: bool,
    /// Enable audit logging
    #[serde(default = "default_true")]
    pub audit_logging: bool,
    /// Enable experimental features
    #[serde(default)]
    pub experimental: bool,
    /// Custom feature flags
    #[serde(default)]
    pub custom: HashMap<String, bool>,
}

fn default_true() -> bool {
    true
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            rate_limiting: true,
            caching: true,
            event_sourcing: false,
            strict_validation: true,
            audit_logging: true,
            experimental: false,
            custom: HashMap::new(),
        }
    }
}

impl FeatureFlags {
    /// Production feature flags
    pub fn production() -> Self {
        Self {
            rate_limiting: true,
            caching: true,
            event_sourcing: true,
            strict_validation: true,
            audit_logging: true,
            experimental: false,
            custom: HashMap::new(),
        }
    }

    /// Check if a feature is enabled
    pub fn is_enabled(&self, feature: &str) -> bool {
        match feature {
            "rate_limiting" => self.rate_limiting,
            "caching" => self.caching,
            "event_sourcing" => self.event_sourcing,
            "strict_validation" => self.strict_validation,
            "audit_logging" => self.audit_logging,
            "experimental" => self.experimental,
            _ => self.custom.get(feature).copied().unwrap_or(false),
        }
    }

    /// Set a custom feature flag
    pub fn set_custom(&mut self, feature: impl Into<String>, enabled: bool) {
        self.custom.insert(feature.into(), enabled);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let mut config = AppConfig {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                workers: 4,
            },
            database: DatabaseConfig {
                url: "postgres://localhost/test".to_string(),
                pool_size: 10,
                timeout_seconds: 30,
            },
            redis: RedisConfig {
                url: "redis://localhost".to_string(),
                pool_size: 5,
            },
            s3: S3Config {
                bucket: "test-bucket".to_string(),
                region: "us-east-1".to_string(),
                endpoint: None,
            },
            auth: AuthConfig {
                jwt_secret: "a".repeat(32),
                token_expiry_seconds: 3600,
            },
            telemetry: TelemetryConfig {
                otlp_endpoint: None,
                service_name: "test".to_string(),
                json_logging: false,
                log_level: "info".to_string(),
            },
            architecture: ArchitectureConfig::default(),
            features: FeatureFlags::default(),
        };

        // Valid config should pass
        assert!(config.validate().is_ok());

        // Invalid port
        config.server.port = 0;
        assert!(config.validate().is_err());
        config.server.port = 8080;

        // Invalid JWT secret (too short)
        config.auth.jwt_secret = "short".to_string();
        assert!(config.validate().is_err());
        config.auth.jwt_secret = "a".repeat(32);

        // Invalid log level
        config.telemetry.log_level = "invalid".to_string();
        assert!(config.validate().is_err());
    }
}
