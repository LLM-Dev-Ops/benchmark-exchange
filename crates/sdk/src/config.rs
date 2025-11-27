//! SDK configuration
//!
//! This module provides configuration options for the SDK client.

use crate::error::{SdkError, SdkResult};
use std::time::Duration;

/// Client configuration
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Base URL for the API
    pub base_url: String,

    /// API key for authentication
    pub api_key: Option<String>,

    /// Bearer token for authentication
    pub bearer_token: Option<String>,

    /// Request timeout
    pub timeout: Duration,

    /// Number of retry attempts for failed requests
    pub retry_count: u32,

    /// Initial backoff duration for retries
    pub retry_initial_backoff: Duration,

    /// Maximum backoff duration for retries
    pub retry_max_backoff: Duration,

    /// User agent string
    pub user_agent: String,

    /// Enable request/response logging
    pub debug: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            base_url: crate::DEFAULT_API_URL.to_string(),
            api_key: None,
            bearer_token: None,
            timeout: Duration::from_secs(30),
            retry_count: 3,
            retry_initial_backoff: Duration::from_millis(100),
            retry_max_backoff: Duration::from_secs(10),
            user_agent: format!("llm-benchmark-sdk/{}", crate::VERSION),
            debug: false,
        }
    }
}

impl ClientConfig {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from environment variables
    ///
    /// Supported environment variables:
    /// - `LLM_BENCHMARK_API_URL`: Base URL for the API
    /// - `LLM_BENCHMARK_API_KEY`: API key for authentication
    /// - `LLM_BENCHMARK_TOKEN`: Bearer token for authentication
    /// - `LLM_BENCHMARK_TIMEOUT`: Request timeout in seconds
    /// - `LLM_BENCHMARK_DEBUG`: Enable debug logging
    pub fn from_env() -> SdkResult<Self> {
        let mut config = Self::default();

        if let Ok(url) = std::env::var("LLM_BENCHMARK_API_URL") {
            config.base_url = url;
        }

        if let Ok(key) = std::env::var("LLM_BENCHMARK_API_KEY") {
            config.api_key = Some(key);
        }

        if let Ok(token) = std::env::var("LLM_BENCHMARK_TOKEN") {
            config.bearer_token = Some(token);
        }

        if let Ok(timeout) = std::env::var("LLM_BENCHMARK_TIMEOUT") {
            let secs: u64 = timeout.parse().map_err(|_| SdkError::ConfigError {
                message: format!("Invalid timeout value: {}", timeout),
            })?;
            config.timeout = Duration::from_secs(secs);
        }

        if std::env::var("LLM_BENCHMARK_DEBUG").is_ok() {
            config.debug = true;
        }

        Ok(config)
    }

    /// Set the base URL
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Set the API key
    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Set the bearer token
    pub fn with_bearer_token(mut self, token: impl Into<String>) -> Self {
        self.bearer_token = Some(token.into());
        self
    }

    /// Set the request timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the retry count
    pub fn with_retry_count(mut self, count: u32) -> Self {
        self.retry_count = count;
        self
    }

    /// Enable debug mode
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> SdkResult<()> {
        if self.base_url.is_empty() {
            return Err(SdkError::ConfigError {
                message: "Base URL cannot be empty".to_string(),
            });
        }

        // Parse URL to validate it
        url::Url::parse(&self.base_url).map_err(|e| SdkError::ConfigError {
            message: format!("Invalid base URL: {}", e),
        })?;

        Ok(())
    }

    /// Get the authentication header value
    pub fn auth_header(&self) -> Option<String> {
        if let Some(ref key) = self.api_key {
            Some(format!("X-API-Key {}", key))
        } else if let Some(ref token) = self.bearer_token {
            Some(format!("Bearer {}", token))
        } else {
            None
        }
    }

    /// Check if authentication is configured
    pub fn has_auth(&self) -> bool {
        self.api_key.is_some() || self.bearer_token.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ClientConfig::default();
        assert_eq!(config.base_url, crate::DEFAULT_API_URL);
        assert!(config.api_key.is_none());
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.retry_count, 3);
    }

    #[test]
    fn test_config_builder() {
        let config = ClientConfig::new()
            .with_base_url("https://custom.api.com")
            .with_api_key("my-key")
            .with_timeout(Duration::from_secs(60))
            .with_retry_count(5)
            .with_debug(true);

        assert_eq!(config.base_url, "https://custom.api.com");
        assert_eq!(config.api_key, Some("my-key".to_string()));
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.retry_count, 5);
        assert!(config.debug);
    }

    #[test]
    fn test_auth_header() {
        let config = ClientConfig::new().with_api_key("my-key");
        assert_eq!(config.auth_header(), Some("X-API-Key my-key".to_string()));

        let config = ClientConfig::new().with_bearer_token("my-token");
        assert_eq!(config.auth_header(), Some("Bearer my-token".to_string()));

        let config = ClientConfig::new();
        assert_eq!(config.auth_header(), None);
    }

    #[test]
    fn test_config_validation() {
        let config = ClientConfig::new();
        assert!(config.validate().is_ok());

        let config = ClientConfig::new().with_base_url("");
        assert!(config.validate().is_err());

        let config = ClientConfig::new().with_base_url("not-a-url");
        assert!(config.validate().is_err());
    }
}
