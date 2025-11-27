//! SDK client implementation
//!
//! This module provides the main client for interacting with the LLM Benchmark Exchange API.

use crate::config::ClientConfig;
use crate::error::{SdkError, SdkResult};
use crate::services::{BenchmarkService, GovernanceService, LeaderboardService, SubmissionService};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error};

/// Main SDK client
#[derive(Clone)]
pub struct Client {
    inner: Arc<ClientInner>,
}

struct ClientInner {
    http: reqwest::Client,
    config: ClientConfig,
}

impl Client {
    /// Create a new client builder
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// Create a new client with the given configuration
    pub fn new(config: ClientConfig) -> SdkResult<Self> {
        config.validate()?;

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(&config.user_agent).unwrap_or_else(|_| {
                HeaderValue::from_static("llm-benchmark-sdk")
            }),
        );

        if let Some(auth) = config.auth_header() {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&auth).map_err(|_| SdkError::ConfigError {
                    message: "Invalid authorization header".to_string(),
                })?,
            );
        }

        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .default_headers(headers)
            .build()
            .map_err(|e| SdkError::ConfigError {
                message: format!("Failed to create HTTP client: {}", e),
            })?;

        Ok(Self {
            inner: Arc::new(ClientInner { http, config }),
        })
    }

    /// Create a client from environment variables
    pub fn from_env() -> SdkResult<Self> {
        let config = ClientConfig::from_env()?;
        Self::new(config)
    }

    /// Get the configuration
    pub fn config(&self) -> &ClientConfig {
        &self.inner.config
    }

    /// Get the benchmark service
    pub fn benchmarks(&self) -> BenchmarkService {
        BenchmarkService::new(self.clone())
    }

    /// Get the submission service
    pub fn submissions(&self) -> SubmissionService {
        SubmissionService::new(self.clone())
    }

    /// Get the leaderboard service
    pub fn leaderboards(&self) -> LeaderboardService {
        LeaderboardService::new(self.clone())
    }

    /// Get the governance service
    pub fn governance(&self) -> GovernanceService {
        GovernanceService::new(self.clone())
    }

    /// Make a GET request
    pub(crate) async fn get<T: DeserializeOwned>(&self, path: &str) -> SdkResult<T> {
        self.request(reqwest::Method::GET, path, Option::<&()>::None)
            .await
    }

    /// Make a GET request with query parameters
    pub(crate) async fn get_with_query<T: DeserializeOwned, Q: Serialize>(
        &self,
        path: &str,
        query: &Q,
    ) -> SdkResult<T> {
        let url = format!("{}{}", self.inner.config.base_url, path);

        let response = self
            .inner
            .http
            .get(&url)
            .query(query)
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Make a POST request
    pub(crate) async fn post<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> SdkResult<T> {
        self.request(reqwest::Method::POST, path, Some(body)).await
    }

    /// Make a PUT request
    pub(crate) async fn put<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> SdkResult<T> {
        self.request(reqwest::Method::PUT, path, Some(body)).await
    }

    /// Make a PATCH request
    pub(crate) async fn patch<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> SdkResult<T> {
        self.request(reqwest::Method::PATCH, path, Some(body)).await
    }

    /// Make a DELETE request
    pub(crate) async fn delete(&self, path: &str) -> SdkResult<()> {
        let url = format!("{}{}", self.inner.config.base_url, path);

        let response = self.inner.http.delete(&url).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(self.handle_error_response(response).await)
        }
    }

    /// Make a request with optional body
    async fn request<T: DeserializeOwned, B: Serialize>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&B>,
    ) -> SdkResult<T> {
        let url = format!("{}{}", self.inner.config.base_url, path);

        if self.inner.config.debug {
            debug!("SDK request: {} {}", method, url);
        }

        let mut request = self.inner.http.request(method.clone(), &url);

        if let Some(body) = body {
            request = request.json(body);
        }

        let response = self.execute_with_retry(request, &method, &url).await?;

        self.handle_response(response).await
    }

    /// Execute request with retry logic
    async fn execute_with_retry(
        &self,
        request: reqwest::RequestBuilder,
        method: &reqwest::Method,
        url: &str,
    ) -> SdkResult<reqwest::Response> {
        let max_retries = self.inner.config.retry_count;
        let mut attempt = 0;
        let mut last_error: Option<SdkError> = None;

        loop {
            attempt += 1;

            // Clone the request for retry (reqwest doesn't support direct retry)
            let request_clone = request
                .try_clone()
                .ok_or_else(|| SdkError::NetworkError {
                    message: "Request cannot be cloned for retry".to_string(),
                    source: None,
                })?;

            match request_clone.send().await {
                Ok(response) => {
                    if response.status().is_success() || !is_retryable_status(response.status()) {
                        return Ok(response);
                    }

                    // Retryable error
                    if attempt > max_retries {
                        return Ok(response); // Let handle_response deal with the error
                    }

                    last_error = Some(SdkError::ServerError {
                        status_code: response.status().as_u16(),
                        message: format!("Request failed with status {}", response.status()),
                    });
                }
                Err(e) => {
                    let err: SdkError = e.into();
                    if !err.is_retryable() || attempt > max_retries {
                        return Err(err);
                    }
                    last_error = Some(err);
                }
            }

            // Calculate backoff
            let backoff = calculate_backoff(
                attempt,
                self.inner.config.retry_initial_backoff,
                self.inner.config.retry_max_backoff,
            );

            if self.inner.config.debug {
                debug!(
                    "SDK retry {}/{} for {} {} after {:?}",
                    attempt, max_retries, method, url, backoff
                );
            }

            tokio::time::sleep(backoff).await;
        }
    }

    /// Handle successful response
    async fn handle_response<T: DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> SdkResult<T> {
        if response.status().is_success() {
            let text = response.text().await?;

            if self.inner.config.debug {
                debug!("SDK response body: {}", text);
            }

            serde_json::from_str(&text).map_err(|e| {
                error!("Failed to parse response: {}", e);
                SdkError::InvalidResponse {
                    message: format!("Failed to parse response: {}", e),
                }
            })
        } else {
            Err(self.handle_error_response(response).await)
        }
    }

    /// Handle error response
    async fn handle_error_response(&self, response: reqwest::Response) -> SdkError {
        let status = response.status();
        let status_code = status.as_u16();

        // Try to parse error body
        let body = response.text().await.unwrap_or_default();

        if self.inner.config.debug {
            debug!("SDK error response ({}): {}", status_code, body);
        }

        // Try to parse as API error
        if let Ok(api_error) = serde_json::from_str::<ApiErrorResponse>(&body) {
            return match status_code {
                401 => SdkError::Unauthorized {
                    message: api_error.message,
                    status_code,
                },
                403 => SdkError::Forbidden {
                    message: api_error.message,
                    resource: None,
                },
                404 => SdkError::NotFound {
                    resource_type: "Resource".to_string(),
                    resource_id: api_error.message,
                },
                409 => SdkError::Conflict {
                    message: api_error.message,
                },
                422 | 400 => SdkError::ValidationError {
                    message: api_error.message,
                    field_errors: api_error
                        .field_errors
                        .unwrap_or_default()
                        .into_iter()
                        .map(|f| crate::error::FieldError::new(f.field, f.message))
                        .collect(),
                },
                429 => SdkError::RateLimited { retry_after: None },
                500..=599 => SdkError::ServerError {
                    status_code,
                    message: api_error.message,
                },
                _ => SdkError::ApiError {
                    code: api_error.code.unwrap_or_else(|| status_code.to_string()),
                    message: api_error.message,
                    details: api_error.details,
                },
            };
        }

        // Fallback to generic error
        match status_code {
            401 => SdkError::Unauthorized {
                message: "Unauthorized".to_string(),
                status_code,
            },
            403 => SdkError::Forbidden {
                message: "Forbidden".to_string(),
                resource: None,
            },
            404 => SdkError::NotFound {
                resource_type: "Resource".to_string(),
                resource_id: "unknown".to_string(),
            },
            429 => SdkError::RateLimited { retry_after: None },
            _ => SdkError::ServerError {
                status_code,
                message: body,
            },
        }
    }
}

/// API error response structure
#[derive(Debug, serde::Deserialize)]
struct ApiErrorResponse {
    message: String,
    code: Option<String>,
    details: Option<serde_json::Value>,
    field_errors: Option<Vec<ApiFieldError>>,
}

#[derive(Debug, serde::Deserialize)]
struct ApiFieldError {
    field: String,
    message: String,
}

/// Check if a status code is retryable
fn is_retryable_status(status: reqwest::StatusCode) -> bool {
    status.is_server_error() || status == reqwest::StatusCode::TOO_MANY_REQUESTS
}

/// Calculate exponential backoff
fn calculate_backoff(attempt: u32, initial: Duration, max: Duration) -> Duration {
    let backoff = initial.saturating_mul(2u32.saturating_pow(attempt - 1));
    backoff.min(max)
}

/// Client builder for ergonomic configuration
#[derive(Default)]
pub struct ClientBuilder {
    config: ClientConfig,
}

impl ClientBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            config: ClientConfig::default(),
        }
    }

    /// Load configuration from environment
    pub fn from_env(mut self) -> SdkResult<Self> {
        self.config = ClientConfig::from_env()?;
        Ok(self)
    }

    /// Set the base URL
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.config.base_url = url.into();
        self
    }

    /// Set the API key
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.config.api_key = Some(key.into());
        self
    }

    /// Set the bearer token
    pub fn bearer_token(mut self, token: impl Into<String>) -> Self {
        self.config.bearer_token = Some(token.into());
        self
    }

    /// Set the request timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    /// Set the retry count
    pub fn retry_count(mut self, count: u32) -> Self {
        self.config.retry_count = count;
        self
    }

    /// Set the user agent
    pub fn user_agent(mut self, agent: impl Into<String>) -> Self {
        self.config.user_agent = agent.into();
        self
    }

    /// Enable debug mode
    pub fn debug(mut self, debug: bool) -> Self {
        self.config.debug = debug;
        self
    }

    /// Build the client
    pub fn build(self) -> SdkResult<Client> {
        Client::new(self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_builder() {
        let client = Client::builder()
            .base_url("https://api.example.com")
            .api_key("test-key")
            .timeout(Duration::from_secs(60))
            .retry_count(5)
            .build()
            .unwrap();

        assert_eq!(client.config().base_url, "https://api.example.com");
        assert_eq!(client.config().api_key, Some("test-key".to_string()));
        assert_eq!(client.config().timeout, Duration::from_secs(60));
        assert_eq!(client.config().retry_count, 5);
    }

    #[test]
    fn test_calculate_backoff() {
        let initial = Duration::from_millis(100);
        let max = Duration::from_secs(10);

        assert_eq!(calculate_backoff(1, initial, max), Duration::from_millis(100));
        assert_eq!(calculate_backoff(2, initial, max), Duration::from_millis(200));
        assert_eq!(calculate_backoff(3, initial, max), Duration::from_millis(400));
        assert_eq!(calculate_backoff(10, initial, max), max); // Capped at max
    }
}
