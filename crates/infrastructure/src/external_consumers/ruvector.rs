//! RuVector Service Client for Benchmark Publication Agent
//!
//! This module provides the client for persisting data to ruvector-service.
//! Per the constitution:
//! - ALL persistence is handled via ruvector-service
//! - This agent NEVER connects directly to Google SQL
//! - This agent NEVER executes SQL
//! - Data is persisted ONLY via ruvector-service client calls
//!
//! ruvector-service is backed by Google SQL (Postgres) but this client
//! only communicates via the ruvector-service API.

use super::{ExternalConsumerError, ExternalConsumerResult, ServiceHealth};
use llm_benchmark_domain::publication::{
    DecisionEvent, Publication, PublicationId, PublicationStatus,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};

// =============================================================================
// Configuration
// =============================================================================

/// Configuration for the RuVector service client
#[derive(Debug, Clone)]
pub struct RuVectorConfig {
    /// Base URL for ruvector-service
    pub base_url: String,

    /// API key for authentication
    pub api_key: Option<String>,

    /// Request timeout in milliseconds
    pub timeout_ms: u64,

    /// Maximum retries for transient failures
    pub max_retries: u32,

    /// Retry backoff base in milliseconds
    pub retry_backoff_ms: u64,

    /// Enable request compression
    pub compression_enabled: bool,

    /// Batch size for bulk operations
    pub batch_size: usize,
}

impl Default for RuVectorConfig {
    fn default() -> Self {
        Self {
            base_url: std::env::var("RUVECTOR_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            api_key: std::env::var("RUVECTOR_API_KEY").ok(),
            timeout_ms: 30_000,
            max_retries: 3,
            retry_backoff_ms: 1000,
            compression_enabled: true,
            batch_size: 100,
        }
    }
}

// =============================================================================
// Request/Response Types
// =============================================================================

/// Request to store a DecisionEvent
#[derive(Debug, Clone, Serialize)]
pub struct StoreDecisionEventRequest {
    pub event: DecisionEvent,
    pub idempotency_key: Option<String>,
}

/// Response from storing a DecisionEvent
#[derive(Debug, Clone, Deserialize)]
pub struct StoreDecisionEventResponse {
    pub event_id: String,
    pub stored_at: DateTime<Utc>,
    pub status: String,
}

/// Request to store a Publication
#[derive(Debug, Clone, Serialize)]
pub struct StorePublicationRequest {
    pub publication: Publication,
    pub idempotency_key: Option<String>,
}

/// Response from storing a Publication
#[derive(Debug, Clone, Deserialize)]
pub struct StorePublicationResponse {
    pub publication_id: String,
    pub stored_at: DateTime<Utc>,
    pub version: u64,
}

/// Query parameters for listing publications
#[derive(Debug, Clone, Serialize, Default)]
pub struct PublicationQuery {
    pub benchmark_id: Option<String>,
    pub model_provider: Option<String>,
    pub model_name: Option<String>,
    pub status: Option<PublicationStatus>,
    pub published_after: Option<DateTime<Utc>>,
    pub published_before: Option<DateTime<Utc>>,
    pub min_confidence: Option<f64>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub order_by: Option<String>,
    pub order_dir: Option<String>,
}

/// Paginated list response
#[derive(Debug, Clone, Deserialize)]
pub struct PaginatedPublications {
    pub items: Vec<Publication>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

/// Telemetry event for LLM-Observatory compatibility
#[derive(Debug, Clone, Serialize)]
pub struct TelemetryEvent {
    pub event_type: String,
    pub agent_id: String,
    pub timestamp: DateTime<Utc>,
    pub duration_ms: Option<u64>,
    pub status: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

// =============================================================================
// RuVector Client Trait
// =============================================================================

/// Trait defining the RuVector service client interface
/// This allows for easy mocking in tests
#[async_trait]
pub trait RuVectorClient: Send + Sync {
    // DecisionEvent operations
    async fn store_decision_event(
        &self,
        request: StoreDecisionEventRequest,
    ) -> ExternalConsumerResult<StoreDecisionEventResponse>;

    async fn get_decision_event(
        &self,
        event_id: &str,
    ) -> ExternalConsumerResult<Option<DecisionEvent>>;

    async fn list_decision_events(
        &self,
        agent_id: &str,
        limit: u32,
        offset: u32,
    ) -> ExternalConsumerResult<Vec<DecisionEvent>>;

    // Publication operations
    async fn store_publication(
        &self,
        request: StorePublicationRequest,
    ) -> ExternalConsumerResult<StorePublicationResponse>;

    async fn get_publication(
        &self,
        publication_id: &str,
    ) -> ExternalConsumerResult<Option<Publication>>;

    async fn update_publication(
        &self,
        publication: &Publication,
    ) -> ExternalConsumerResult<StorePublicationResponse>;

    async fn delete_publication(&self, publication_id: &str) -> ExternalConsumerResult<()>;

    async fn list_publications(
        &self,
        query: PublicationQuery,
    ) -> ExternalConsumerResult<PaginatedPublications>;

    // Telemetry operations (LLM-Observatory compatibility)
    async fn emit_telemetry(&self, event: TelemetryEvent) -> ExternalConsumerResult<()>;

    // Health check
    async fn health_check(&self) -> ExternalConsumerResult<ServiceHealth>;
}

// =============================================================================
// HTTP Client Implementation
// =============================================================================

/// HTTP-based RuVector service client
pub struct HttpRuVectorClient {
    config: RuVectorConfig,
    http_client: reqwest::Client,
}

impl HttpRuVectorClient {
    /// Create a new HTTP client for ruvector-service
    pub fn new(config: RuVectorConfig) -> ExternalConsumerResult<Self> {
        let mut builder = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms))
            .pool_max_idle_per_host(10);

        if config.compression_enabled {
            builder = builder.gzip(true);
        }

        let http_client = builder.build().map_err(|e| {
            ExternalConsumerError::ConfigurationError(format!("Failed to create HTTP client: {}", e))
        })?;

        Ok(Self {
            config,
            http_client,
        })
    }

    /// Build the full URL for an endpoint
    fn url(&self, path: &str) -> String {
        format!("{}{}", self.config.base_url.trim_end_matches('/'), path)
    }

    /// Add authentication headers to a request
    fn auth_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(ref api_key) = self.config.api_key {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", api_key)
                    .parse()
                    .expect("Invalid API key format"),
            );
        }
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        headers.insert(
            reqwest::header::ACCEPT,
            "application/json".parse().unwrap(),
        );
        headers
    }

    /// Execute a request with retry logic
    async fn execute_with_retry<T, F, Fut>(&self, operation: F) -> ExternalConsumerResult<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = ExternalConsumerResult<T>>,
    {
        let mut last_error = None;
        let mut backoff = self.config.retry_backoff_ms;

        for attempt in 0..=self.config.max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) if e.is_retryable() && attempt < self.config.max_retries => {
                    warn!(
                        attempt = attempt + 1,
                        max_retries = self.config.max_retries,
                        error = %e,
                        "Request failed, retrying..."
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(backoff)).await;
                    backoff *= 2; // Exponential backoff
                    last_error = Some(e);
                }
                Err(e) => return Err(e),
            }
        }

        Err(last_error.unwrap_or_else(|| {
            ExternalConsumerError::ConnectionFailed {
                service: "ruvector-service".to_string(),
                message: "Max retries exceeded".to_string(),
            }
        }))
    }
}

#[async_trait]
impl RuVectorClient for HttpRuVectorClient {
    #[instrument(skip(self, request))]
    async fn store_decision_event(
        &self,
        request: StoreDecisionEventRequest,
    ) -> ExternalConsumerResult<StoreDecisionEventResponse> {
        self.execute_with_retry(|| async {
            let response = self
                .http_client
                .post(self.url("/api/v1/decision-events"))
                .headers(self.auth_headers())
                .json(&request)
                .send()
                .await
                .map_err(|e| ExternalConsumerError::ConnectionFailed {
                    service: "ruvector-service".to_string(),
                    message: e.to_string(),
                })?;

            if response.status().is_success() {
                response
                    .json::<StoreDecisionEventResponse>()
                    .await
                    .map_err(|e| ExternalConsumerError::InvalidResponse {
                        service: "ruvector-service".to_string(),
                        message: e.to_string(),
                    })
            } else if response.status() == reqwest::StatusCode::NOT_FOUND {
                Err(ExternalConsumerError::NotFound(
                    "Decision event endpoint not found".to_string(),
                ))
            } else if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                Err(ExternalConsumerError::RateLimitExceeded {
                    service: "ruvector-service".to_string(),
                })
            } else {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                Err(ExternalConsumerError::InvalidResponse {
                    service: "ruvector-service".to_string(),
                    message: format!("HTTP {}: {}", status, body),
                })
            }
        })
        .await
    }

    #[instrument(skip(self))]
    async fn get_decision_event(
        &self,
        event_id: &str,
    ) -> ExternalConsumerResult<Option<DecisionEvent>> {
        let response = self
            .http_client
            .get(self.url(&format!("/api/v1/decision-events/{}", event_id)))
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| ExternalConsumerError::ConnectionFailed {
                service: "ruvector-service".to_string(),
                message: e.to_string(),
            })?;

        if response.status().is_success() {
            let event = response
                .json::<DecisionEvent>()
                .await
                .map_err(|e| ExternalConsumerError::InvalidResponse {
                    service: "ruvector-service".to_string(),
                    message: e.to_string(),
                })?;
            Ok(Some(event))
        } else if response.status() == reqwest::StatusCode::NOT_FOUND {
            Ok(None)
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(ExternalConsumerError::InvalidResponse {
                service: "ruvector-service".to_string(),
                message: format!("HTTP {}: {}", status, body),
            })
        }
    }

    #[instrument(skip(self))]
    async fn list_decision_events(
        &self,
        agent_id: &str,
        limit: u32,
        offset: u32,
    ) -> ExternalConsumerResult<Vec<DecisionEvent>> {
        let response = self
            .http_client
            .get(self.url("/api/v1/decision-events"))
            .headers(self.auth_headers())
            .query(&[
                ("agent_id", agent_id),
                ("limit", &limit.to_string()),
                ("offset", &offset.to_string()),
            ])
            .send()
            .await
            .map_err(|e| ExternalConsumerError::ConnectionFailed {
                service: "ruvector-service".to_string(),
                message: e.to_string(),
            })?;

        if response.status().is_success() {
            response
                .json::<Vec<DecisionEvent>>()
                .await
                .map_err(|e| ExternalConsumerError::InvalidResponse {
                    service: "ruvector-service".to_string(),
                    message: e.to_string(),
                })
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(ExternalConsumerError::InvalidResponse {
                service: "ruvector-service".to_string(),
                message: format!("HTTP {}: {}", status, body),
            })
        }
    }

    #[instrument(skip(self, request))]
    async fn store_publication(
        &self,
        request: StorePublicationRequest,
    ) -> ExternalConsumerResult<StorePublicationResponse> {
        self.execute_with_retry(|| async {
            let response = self
                .http_client
                .post(self.url("/api/v1/publications"))
                .headers(self.auth_headers())
                .json(&request)
                .send()
                .await
                .map_err(|e| ExternalConsumerError::ConnectionFailed {
                    service: "ruvector-service".to_string(),
                    message: e.to_string(),
                })?;

            if response.status().is_success() {
                response
                    .json::<StorePublicationResponse>()
                    .await
                    .map_err(|e| ExternalConsumerError::InvalidResponse {
                        service: "ruvector-service".to_string(),
                        message: e.to_string(),
                    })
            } else if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                Err(ExternalConsumerError::RateLimitExceeded {
                    service: "ruvector-service".to_string(),
                })
            } else {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                Err(ExternalConsumerError::InvalidResponse {
                    service: "ruvector-service".to_string(),
                    message: format!("HTTP {}: {}", status, body),
                })
            }
        })
        .await
    }

    #[instrument(skip(self))]
    async fn get_publication(
        &self,
        publication_id: &str,
    ) -> ExternalConsumerResult<Option<Publication>> {
        let response = self
            .http_client
            .get(self.url(&format!("/api/v1/publications/{}", publication_id)))
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| ExternalConsumerError::ConnectionFailed {
                service: "ruvector-service".to_string(),
                message: e.to_string(),
            })?;

        if response.status().is_success() {
            let publication = response
                .json::<Publication>()
                .await
                .map_err(|e| ExternalConsumerError::InvalidResponse {
                    service: "ruvector-service".to_string(),
                    message: e.to_string(),
                })?;
            Ok(Some(publication))
        } else if response.status() == reqwest::StatusCode::NOT_FOUND {
            Ok(None)
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(ExternalConsumerError::InvalidResponse {
                service: "ruvector-service".to_string(),
                message: format!("HTTP {}: {}", status, body),
            })
        }
    }

    #[instrument(skip(self, publication))]
    async fn update_publication(
        &self,
        publication: &Publication,
    ) -> ExternalConsumerResult<StorePublicationResponse> {
        self.execute_with_retry(|| async {
            let response = self
                .http_client
                .put(self.url(&format!(
                    "/api/v1/publications/{}",
                    publication.id
                )))
                .headers(self.auth_headers())
                .json(publication)
                .send()
                .await
                .map_err(|e| ExternalConsumerError::ConnectionFailed {
                    service: "ruvector-service".to_string(),
                    message: e.to_string(),
                })?;

            if response.status().is_success() {
                response
                    .json::<StorePublicationResponse>()
                    .await
                    .map_err(|e| ExternalConsumerError::InvalidResponse {
                        service: "ruvector-service".to_string(),
                        message: e.to_string(),
                    })
            } else if response.status() == reqwest::StatusCode::NOT_FOUND {
                Err(ExternalConsumerError::NotFound(format!(
                    "Publication not found: {}",
                    publication.id
                )))
            } else {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                Err(ExternalConsumerError::InvalidResponse {
                    service: "ruvector-service".to_string(),
                    message: format!("HTTP {}: {}", status, body),
                })
            }
        })
        .await
    }

    #[instrument(skip(self))]
    async fn delete_publication(&self, publication_id: &str) -> ExternalConsumerResult<()> {
        let response = self
            .http_client
            .delete(self.url(&format!("/api/v1/publications/{}", publication_id)))
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| ExternalConsumerError::ConnectionFailed {
                service: "ruvector-service".to_string(),
                message: e.to_string(),
            })?;

        if response.status().is_success() || response.status() == reqwest::StatusCode::NO_CONTENT {
            Ok(())
        } else if response.status() == reqwest::StatusCode::NOT_FOUND {
            Err(ExternalConsumerError::NotFound(format!(
                "Publication not found: {}",
                publication_id
            )))
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(ExternalConsumerError::InvalidResponse {
                service: "ruvector-service".to_string(),
                message: format!("HTTP {}: {}", status, body),
            })
        }
    }

    #[instrument(skip(self, query))]
    async fn list_publications(
        &self,
        query: PublicationQuery,
    ) -> ExternalConsumerResult<PaginatedPublications> {
        let response = self
            .http_client
            .get(self.url("/api/v1/publications"))
            .headers(self.auth_headers())
            .query(&query)
            .send()
            .await
            .map_err(|e| ExternalConsumerError::ConnectionFailed {
                service: "ruvector-service".to_string(),
                message: e.to_string(),
            })?;

        if response.status().is_success() {
            response
                .json::<PaginatedPublications>()
                .await
                .map_err(|e| ExternalConsumerError::InvalidResponse {
                    service: "ruvector-service".to_string(),
                    message: e.to_string(),
                })
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(ExternalConsumerError::InvalidResponse {
                service: "ruvector-service".to_string(),
                message: format!("HTTP {}: {}", status, body),
            })
        }
    }

    #[instrument(skip(self, event))]
    async fn emit_telemetry(&self, event: TelemetryEvent) -> ExternalConsumerResult<()> {
        // Telemetry is fire-and-forget, don't retry
        let result = self
            .http_client
            .post(self.url("/api/v1/telemetry"))
            .headers(self.auth_headers())
            .json(&event)
            .send()
            .await;

        match result {
            Ok(response) if response.status().is_success() => {
                debug!(event_type = %event.event_type, "Telemetry emitted");
                Ok(())
            }
            Ok(response) => {
                warn!(
                    status = %response.status(),
                    "Telemetry emission failed (non-critical)"
                );
                Ok(()) // Don't fail for telemetry
            }
            Err(e) => {
                warn!(error = %e, "Telemetry emission failed (non-critical)");
                Ok(()) // Don't fail for telemetry
            }
        }
    }

    #[instrument(skip(self))]
    async fn health_check(&self) -> ExternalConsumerResult<ServiceHealth> {
        let start = std::time::Instant::now();

        let response = self
            .http_client
            .get(self.url("/health"))
            .headers(self.auth_headers())
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await;

        let latency_ms = start.elapsed().as_millis() as u64;

        match response {
            Ok(resp) if resp.status().is_success() => Ok(ServiceHealth {
                healthy: true,
                latency_ms,
                error: None,
            }),
            Ok(resp) => Ok(ServiceHealth {
                healthy: false,
                latency_ms,
                error: Some(format!("HTTP {}", resp.status())),
            }),
            Err(e) => Ok(ServiceHealth {
                healthy: false,
                latency_ms,
                error: Some(e.to_string()),
            }),
        }
    }
}

// =============================================================================
// In-Memory Client (for testing)
// =============================================================================

/// In-memory RuVector client for testing
pub struct InMemoryRuVectorClient {
    decision_events: RwLock<HashMap<String, DecisionEvent>>,
    publications: RwLock<HashMap<String, Publication>>,
    telemetry_events: RwLock<Vec<TelemetryEvent>>,
}

impl InMemoryRuVectorClient {
    /// Create a new in-memory client
    pub fn new() -> Self {
        Self {
            decision_events: RwLock::new(HashMap::new()),
            publications: RwLock::new(HashMap::new()),
            telemetry_events: RwLock::new(Vec::new()),
        }
    }

    /// Get all stored telemetry events (for testing)
    pub async fn get_telemetry_events(&self) -> Vec<TelemetryEvent> {
        self.telemetry_events.read().await.clone()
    }
}

impl Default for InMemoryRuVectorClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RuVectorClient for InMemoryRuVectorClient {
    async fn store_decision_event(
        &self,
        request: StoreDecisionEventRequest,
    ) -> ExternalConsumerResult<StoreDecisionEventResponse> {
        let event_id = uuid::Uuid::now_v7().to_string();
        let stored_at = Utc::now();

        self.decision_events
            .write()
            .await
            .insert(event_id.clone(), request.event);

        Ok(StoreDecisionEventResponse {
            event_id,
            stored_at,
            status: "stored".to_string(),
        })
    }

    async fn get_decision_event(
        &self,
        event_id: &str,
    ) -> ExternalConsumerResult<Option<DecisionEvent>> {
        Ok(self.decision_events.read().await.get(event_id).cloned())
    }

    async fn list_decision_events(
        &self,
        agent_id: &str,
        limit: u32,
        offset: u32,
    ) -> ExternalConsumerResult<Vec<DecisionEvent>> {
        let events: Vec<_> = self
            .decision_events
            .read()
            .await
            .values()
            .filter(|e| e.agent_id == agent_id)
            .skip(offset as usize)
            .take(limit as usize)
            .cloned()
            .collect();
        Ok(events)
    }

    async fn store_publication(
        &self,
        request: StorePublicationRequest,
    ) -> ExternalConsumerResult<StorePublicationResponse> {
        let publication_id = request.publication.id.to_string();
        let stored_at = Utc::now();

        self.publications
            .write()
            .await
            .insert(publication_id.clone(), request.publication);

        Ok(StorePublicationResponse {
            publication_id,
            stored_at,
            version: 1,
        })
    }

    async fn get_publication(
        &self,
        publication_id: &str,
    ) -> ExternalConsumerResult<Option<Publication>> {
        Ok(self.publications.read().await.get(publication_id).cloned())
    }

    async fn update_publication(
        &self,
        publication: &Publication,
    ) -> ExternalConsumerResult<StorePublicationResponse> {
        let publication_id = publication.id.to_string();

        let mut publications = self.publications.write().await;
        if publications.contains_key(&publication_id) {
            publications.insert(publication_id.clone(), publication.clone());
            Ok(StorePublicationResponse {
                publication_id,
                stored_at: Utc::now(),
                version: 2,
            })
        } else {
            Err(ExternalConsumerError::NotFound(format!(
                "Publication not found: {}",
                publication_id
            )))
        }
    }

    async fn delete_publication(&self, publication_id: &str) -> ExternalConsumerResult<()> {
        let mut publications = self.publications.write().await;
        if publications.remove(publication_id).is_some() {
            Ok(())
        } else {
            Err(ExternalConsumerError::NotFound(format!(
                "Publication not found: {}",
                publication_id
            )))
        }
    }

    async fn list_publications(
        &self,
        query: PublicationQuery,
    ) -> ExternalConsumerResult<PaginatedPublications> {
        let publications = self.publications.read().await;
        let mut items: Vec<_> = publications.values().cloned().collect();

        // Apply filters
        if let Some(ref benchmark_id) = query.benchmark_id {
            items.retain(|p| p.benchmark_id.to_string() == *benchmark_id);
        }
        if let Some(ref model_provider) = query.model_provider {
            items.retain(|p| &p.model_provider == model_provider);
        }
        if let Some(ref model_name) = query.model_name {
            items.retain(|p| &p.model_name == model_name);
        }
        if let Some(status) = query.status {
            items.retain(|p| p.status == status);
        }

        let total = items.len() as u64;
        let offset = query.offset.unwrap_or(0) as usize;
        let limit = query.limit.unwrap_or(20) as usize;

        let items: Vec<_> = items.into_iter().skip(offset).take(limit).collect();

        Ok(PaginatedPublications {
            items,
            total,
            page: (offset / limit + 1) as u32,
            page_size: limit as u32,
            total_pages: ((total as f64) / (limit as f64)).ceil() as u32,
        })
    }

    async fn emit_telemetry(&self, event: TelemetryEvent) -> ExternalConsumerResult<()> {
        self.telemetry_events.write().await.push(event);
        Ok(())
    }

    async fn health_check(&self) -> ExternalConsumerResult<ServiceHealth> {
        Ok(ServiceHealth {
            healthy: true,
            latency_ms: 0,
            error: None,
        })
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use llm_benchmark_domain::publication::{
        DecisionOutputs, PublicationConfidence, PublicationConstraints, PublicationDecisionType,
    };

    #[tokio::test]
    async fn test_in_memory_client_decision_events() {
        let client = InMemoryRuVectorClient::new();

        let event = DecisionEvent::new(
            PublicationDecisionType::BenchmarkPublish,
            "test-hash".to_string(),
            DecisionOutputs {
                publication_id: None,
                status: "success".to_string(),
                normalized_metrics: None,
                validation_results: None,
                metadata: HashMap::new(),
            },
            PublicationConfidence::default(),
            PublicationConstraints::default(),
            "exec-ref".to_string(),
        );

        let request = StoreDecisionEventRequest {
            event: event.clone(),
            idempotency_key: None,
        };

        let response = client.store_decision_event(request).await.unwrap();
        assert!(!response.event_id.is_empty());

        let retrieved = client.get_decision_event(&response.event_id).await.unwrap();
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_in_memory_client_health() {
        let client = InMemoryRuVectorClient::new();
        let health = client.health_check().await.unwrap();
        assert!(health.healthy);
    }
}
