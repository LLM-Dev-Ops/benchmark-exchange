//! API configuration.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// Server host to bind to
    pub host: String,

    /// Server port to bind to
    pub port: u16,

    /// JWT secret for token signing
    pub jwt_secret: String,

    /// JWT token expiration duration in seconds
    pub jwt_expiration_seconds: u64,

    /// CORS allowed origins
    pub cors_allowed_origins: Vec<String>,

    /// Maximum request body size in bytes
    pub max_body_size: usize,

    /// Request timeout in seconds
    pub request_timeout_seconds: u64,

    /// Rate limit: maximum requests per minute
    pub rate_limit_per_minute: u32,

    /// Database connection pool size
    pub db_pool_size: u32,

    /// Enable OpenAPI documentation
    pub enable_swagger: bool,

    /// Log level
    pub log_level: String,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            jwt_secret: "change-me-in-production".to_string(),
            jwt_expiration_seconds: 24 * 60 * 60, // 24 hours
            cors_allowed_origins: vec!["*".to_string()],
            max_body_size: 10 * 1024 * 1024, // 10 MB
            request_timeout_seconds: 30,
            rate_limit_per_minute: 60,
            db_pool_size: 10,
            enable_swagger: true,
            log_level: "info".to_string(),
        }
    }
}

impl ApiConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> anyhow::Result<Self> {
        let config = Self {
            host: std::env::var("API_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("API_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(8080),
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "change-me-in-production".to_string()),
            jwt_expiration_seconds: std::env::var("JWT_EXPIRATION_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(24 * 60 * 60),
            cors_allowed_origins: std::env::var("CORS_ALLOWED_ORIGINS")
                .ok()
                .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_else(|| vec!["*".to_string()]),
            max_body_size: std::env::var("MAX_BODY_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10 * 1024 * 1024),
            request_timeout_seconds: std::env::var("REQUEST_TIMEOUT_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
            rate_limit_per_minute: std::env::var("RATE_LIMIT_PER_MINUTE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(60),
            db_pool_size: std::env::var("DB_POOL_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
            enable_swagger: std::env::var("ENABLE_SWAGGER")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(true),
            log_level: std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
        };

        Ok(config)
    }

    /// Get JWT expiration as Duration
    pub fn jwt_expiration(&self) -> Duration {
        Duration::from_secs(self.jwt_expiration_seconds)
    }

    /// Get request timeout as Duration
    pub fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.request_timeout_seconds)
    }

    /// Get server address
    pub fn server_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
