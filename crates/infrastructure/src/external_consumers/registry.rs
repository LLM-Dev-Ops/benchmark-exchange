//! LLM-Registry Consumer Adapter
//!
//! Thin runtime adapter for consuming data from LLM-Registry:
//! - Model metadata
//! - Benchmark descriptors
//! - Registry-linked corpora
//!
//! This adapter provides read-only consumption without modifying existing APIs.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, instrument, warn};

use super::{ExternalConsumerError, ExternalConsumerResult, ServiceHealth};

/// Configuration for LLM-Registry connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// Base URL for the registry API
    pub base_url: String,
    /// API key for authentication (optional)
    pub api_key: Option<String>,
    /// Request timeout in milliseconds
    pub timeout_ms: u64,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Enable caching of registry data
    pub enable_cache: bool,
    /// Cache TTL in seconds
    pub cache_ttl_secs: u64,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.llm-registry.dev".to_string(),
            api_key: None,
            timeout_ms: 30000,
            max_retries: 3,
            enable_cache: true,
            cache_ttl_secs: 300,
        }
    }
}

/// Model metadata from LLM-Registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    /// Unique model identifier
    pub model_id: String,
    /// Model name
    pub name: String,
    /// Model version
    pub version: String,
    /// Provider/organization
    pub provider: String,
    /// Model family (e.g., GPT, Claude, LLaMA)
    pub family: Option<String>,
    /// Parameter count
    pub parameter_count: Option<u64>,
    /// Context window size
    pub context_window: Option<u32>,
    /// Supported modalities
    pub modalities: Vec<String>,
    /// Model capabilities
    pub capabilities: Vec<String>,
    /// License information
    pub license: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Benchmark descriptor from LLM-Registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkDescriptor {
    /// Unique benchmark identifier in registry
    pub registry_id: String,
    /// Benchmark name
    pub name: String,
    /// Benchmark version
    pub version: String,
    /// Category (reasoning, coding, etc.)
    pub category: String,
    /// Subcategories
    pub subcategories: Vec<String>,
    /// Description
    pub description: String,
    /// Supported evaluation metrics
    pub metrics: Vec<String>,
    /// Dataset references
    pub dataset_refs: Vec<String>,
    /// Official benchmark URL
    pub official_url: Option<String>,
    /// Citation information
    pub citation: Option<String>,
}

/// Registry-linked corpus reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryCorpus {
    /// Corpus identifier
    pub corpus_id: String,
    /// Corpus name
    pub name: String,
    /// Associated benchmark IDs
    pub benchmark_ids: Vec<String>,
    /// Storage location type
    pub storage_type: CorpusStorageType,
    /// Access URL or path
    pub access_path: String,
    /// Format (jsonl, csv, parquet, etc.)
    pub format: String,
    /// Size in bytes
    pub size_bytes: Option<u64>,
    /// Number of records
    pub record_count: Option<u64>,
    /// Checksum for integrity
    pub checksum: Option<String>,
}

/// Corpus storage type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorpusStorageType {
    /// S3-compatible storage
    S3,
    /// HTTP/HTTPS URL
    Http,
    /// Git LFS
    GitLfs,
    /// HuggingFace Hub
    HuggingFace,
}

/// Trait for consuming data from LLM-Registry
#[async_trait]
pub trait RegistryConsumerTrait: Send + Sync {
    /// Fetch model metadata by ID
    async fn get_model_metadata(&self, model_id: &str) -> ExternalConsumerResult<ModelMetadata>;

    /// List models with optional filters
    async fn list_models(
        &self,
        provider: Option<&str>,
        family: Option<&str>,
        limit: Option<u32>,
    ) -> ExternalConsumerResult<Vec<ModelMetadata>>;

    /// Fetch benchmark descriptor by registry ID
    async fn get_benchmark_descriptor(
        &self,
        registry_id: &str,
    ) -> ExternalConsumerResult<BenchmarkDescriptor>;

    /// List benchmark descriptors by category
    async fn list_benchmark_descriptors(
        &self,
        category: Option<&str>,
        limit: Option<u32>,
    ) -> ExternalConsumerResult<Vec<BenchmarkDescriptor>>;

    /// Fetch corpus metadata
    async fn get_corpus(&self, corpus_id: &str) -> ExternalConsumerResult<RegistryCorpus>;

    /// List corpora linked to a benchmark
    async fn list_corpora_for_benchmark(
        &self,
        benchmark_id: &str,
    ) -> ExternalConsumerResult<Vec<RegistryCorpus>>;

    /// Health check
    async fn health_check(&self) -> ServiceHealth;
}

/// LLM-Registry consumer implementation
pub struct RegistryConsumer {
    config: RegistryConfig,
    // In production, this would hold an HTTP client
    // For now, we use a mock implementation that can be replaced
    #[allow(dead_code)]
    client: Option<Arc<dyn std::any::Any + Send + Sync>>,
}

impl RegistryConsumer {
    /// Create a new registry consumer
    pub fn new(config: RegistryConfig) -> Self {
        Self {
            config,
            client: None,
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(RegistryConfig::default())
    }

    /// Get the current configuration
    pub fn config(&self) -> &RegistryConfig {
        &self.config
    }

    /// Build request URL
    fn build_url(&self, path: &str) -> String {
        format!("{}{}", self.config.base_url.trim_end_matches('/'), path)
    }
}

#[async_trait]
impl RegistryConsumerTrait for RegistryConsumer {
    #[instrument(skip(self), fields(model_id = %model_id))]
    async fn get_model_metadata(&self, model_id: &str) -> ExternalConsumerResult<ModelMetadata> {
        debug!("Fetching model metadata from registry");

        // Runtime integration - would make HTTP call to registry API
        // For now, return a structured error indicating the service call would be made
        Err(ExternalConsumerError::ServiceUnavailable(format!(
            "Registry API call to {} not yet connected - model_id: {}",
            self.build_url(&format!("/v1/models/{}", model_id)),
            model_id
        )))
    }

    #[instrument(skip(self))]
    async fn list_models(
        &self,
        provider: Option<&str>,
        family: Option<&str>,
        limit: Option<u32>,
    ) -> ExternalConsumerResult<Vec<ModelMetadata>> {
        debug!(
            provider = ?provider,
            family = ?family,
            limit = ?limit,
            "Listing models from registry"
        );

        Err(ExternalConsumerError::ServiceUnavailable(format!(
            "Registry API call to {} not yet connected",
            self.build_url("/v1/models")
        )))
    }

    #[instrument(skip(self), fields(registry_id = %registry_id))]
    async fn get_benchmark_descriptor(
        &self,
        registry_id: &str,
    ) -> ExternalConsumerResult<BenchmarkDescriptor> {
        debug!("Fetching benchmark descriptor from registry");

        Err(ExternalConsumerError::ServiceUnavailable(format!(
            "Registry API call to {} not yet connected - registry_id: {}",
            self.build_url(&format!("/v1/benchmarks/{}", registry_id)),
            registry_id
        )))
    }

    #[instrument(skip(self))]
    async fn list_benchmark_descriptors(
        &self,
        category: Option<&str>,
        limit: Option<u32>,
    ) -> ExternalConsumerResult<Vec<BenchmarkDescriptor>> {
        debug!(
            category = ?category,
            limit = ?limit,
            "Listing benchmark descriptors from registry"
        );

        Err(ExternalConsumerError::ServiceUnavailable(format!(
            "Registry API call to {} not yet connected",
            self.build_url("/v1/benchmarks")
        )))
    }

    #[instrument(skip(self), fields(corpus_id = %corpus_id))]
    async fn get_corpus(&self, corpus_id: &str) -> ExternalConsumerResult<RegistryCorpus> {
        debug!("Fetching corpus from registry");

        Err(ExternalConsumerError::ServiceUnavailable(format!(
            "Registry API call to {} not yet connected - corpus_id: {}",
            self.build_url(&format!("/v1/corpora/{}", corpus_id)),
            corpus_id
        )))
    }

    #[instrument(skip(self), fields(benchmark_id = %benchmark_id))]
    async fn list_corpora_for_benchmark(
        &self,
        benchmark_id: &str,
    ) -> ExternalConsumerResult<Vec<RegistryCorpus>> {
        debug!("Listing corpora for benchmark from registry");

        Err(ExternalConsumerError::ServiceUnavailable(format!(
            "Registry API call to {} not yet connected - benchmark_id: {}",
            self.build_url(&format!("/v1/benchmarks/{}/corpora", benchmark_id)),
            benchmark_id
        )))
    }

    async fn health_check(&self) -> ServiceHealth {
        // Would perform actual health check against registry API
        ServiceHealth {
            healthy: false,
            latency_ms: 0,
            error: Some("Registry connection not yet established".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_config_default() {
        let config = RegistryConfig::default();
        assert!(config.base_url.contains("llm-registry"));
        assert_eq!(config.timeout_ms, 30000);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_registry_consumer_creation() {
        let consumer = RegistryConsumer::with_defaults();
        assert!(consumer.config().enable_cache);
    }

    #[test]
    fn test_build_url() {
        let consumer = RegistryConsumer::new(RegistryConfig {
            base_url: "https://api.example.com/".to_string(),
            ..Default::default()
        });
        assert_eq!(
            consumer.build_url("/v1/models/test"),
            "https://api.example.com/v1/models/test"
        );
    }

    #[tokio::test]
    async fn test_health_check() {
        let consumer = RegistryConsumer::with_defaults();
        let health = consumer.health_check().await;
        // In mock mode, health should indicate not connected
        assert!(!health.healthy);
    }
}
