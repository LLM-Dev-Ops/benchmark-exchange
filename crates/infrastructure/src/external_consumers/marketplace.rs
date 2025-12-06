//! LLM-Marketplace Consumer Adapter
//!
//! Thin runtime adapter for consuming data from LLM-Marketplace:
//! - Shared test suites
//! - Shield filters
//! - Evaluation templates
//!
//! This adapter provides read-only consumption without modifying existing APIs.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, instrument};

use super::{ExternalConsumerError, ExternalConsumerResult, ServiceHealth};

/// Configuration for LLM-Marketplace connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceConfig {
    /// Base URL for the marketplace API
    pub base_url: String,
    /// API key for authentication (optional)
    pub api_key: Option<String>,
    /// Tenant ID for multi-tenant access
    pub tenant_id: Option<String>,
    /// Request timeout in milliseconds
    pub timeout_ms: u64,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Enable caching
    pub enable_cache: bool,
    /// Cache TTL in seconds
    pub cache_ttl_secs: u64,
}

impl Default for MarketplaceConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.llm-marketplace.dev".to_string(),
            api_key: None,
            tenant_id: None,
            timeout_ms: 30000,
            max_retries: 3,
            enable_cache: true,
            cache_ttl_secs: 600,
        }
    }
}

/// Shared test suite from LLM-Marketplace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedTestSuite {
    /// Unique test suite identifier
    pub suite_id: String,
    /// Test suite name
    pub name: String,
    /// Version
    pub version: String,
    /// Description
    pub description: String,
    /// Author/organization
    pub author: String,
    /// Target benchmark categories
    pub categories: Vec<String>,
    /// Number of test cases
    pub test_case_count: u32,
    /// Difficulty level
    pub difficulty: TestDifficulty,
    /// Supported languages/modalities
    pub supported_languages: Vec<String>,
    /// License
    pub license: String,
    /// Test suite configuration schema
    pub config_schema: Option<serde_json::Value>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Test difficulty level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TestDifficulty {
    /// Basic tests
    Easy,
    /// Standard tests
    Medium,
    /// Challenging tests
    Hard,
    /// Expert-level tests
    Expert,
}

/// Shield filter from LLM-Marketplace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShieldFilter {
    /// Unique filter identifier
    pub filter_id: String,
    /// Filter name
    pub name: String,
    /// Version
    pub version: String,
    /// Description
    pub description: String,
    /// Filter type
    pub filter_type: ShieldFilterType,
    /// Categories this filter applies to
    pub applicable_categories: Vec<String>,
    /// Filter rules/patterns
    pub rules: Vec<FilterRule>,
    /// Severity level
    pub severity: FilterSeverity,
    /// Whether filter is blocking or advisory
    pub is_blocking: bool,
    /// Author/organization
    pub author: String,
}

/// Shield filter type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShieldFilterType {
    /// Content safety filter
    ContentSafety,
    /// Prompt injection detection
    PromptInjection,
    /// Data leakage prevention
    DataLeakage,
    /// Bias detection
    BiasDetection,
    /// Fairness constraint
    Fairness,
    /// Custom filter type
    Custom,
}

/// Filter severity level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FilterSeverity {
    /// Informational only
    Info,
    /// Warning level
    Warning,
    /// Error level
    Error,
    /// Critical level
    Critical,
}

/// Individual filter rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterRule {
    /// Rule identifier
    pub rule_id: String,
    /// Rule name
    pub name: String,
    /// Pattern type
    pub pattern_type: PatternType,
    /// Pattern value
    pub pattern: String,
    /// Action to take on match
    pub action: FilterAction,
}

/// Pattern matching type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PatternType {
    /// Regex pattern
    Regex,
    /// Exact match
    Exact,
    /// Contains substring
    Contains,
    /// Semantic similarity
    Semantic,
    /// ML-based classification
    Classifier,
}

/// Action to take when filter matches
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FilterAction {
    /// Allow and log
    Allow,
    /// Warn but allow
    Warn,
    /// Block the request
    Block,
    /// Redact matched content
    Redact,
    /// Escalate for review
    Escalate,
}

/// Evaluation template from LLM-Marketplace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationTemplate {
    /// Unique template identifier
    pub template_id: String,
    /// Template name
    pub name: String,
    /// Version
    pub version: String,
    /// Description
    pub description: String,
    /// Template type
    pub template_type: TemplateType,
    /// Target benchmark categories
    pub categories: Vec<String>,
    /// Metrics included in template
    pub metrics: Vec<TemplateMetric>,
    /// Scoring configuration
    pub scoring_config: ScoringConfig,
    /// Author/organization
    pub author: String,
    /// License
    pub license: String,
}

/// Evaluation template type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TemplateType {
    /// Accuracy-focused evaluation
    Accuracy,
    /// Performance/latency evaluation
    Performance,
    /// Safety evaluation
    Safety,
    /// Comprehensive evaluation
    Comprehensive,
    /// Custom evaluation
    Custom,
}

/// Metric definition in template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateMetric {
    /// Metric name
    pub name: String,
    /// Metric type
    pub metric_type: String,
    /// Weight in overall score
    pub weight: f64,
    /// Whether metric is required
    pub required: bool,
    /// Threshold for passing
    pub threshold: Option<f64>,
}

/// Scoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringConfig {
    /// Aggregation method
    pub aggregation: AggregationMethod,
    /// Minimum passing score
    pub min_passing_score: f64,
    /// Maximum score
    pub max_score: f64,
    /// Normalization method
    pub normalization: Option<String>,
}

/// Score aggregation method
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AggregationMethod {
    /// Weighted average
    WeightedAverage,
    /// Simple average
    Average,
    /// Minimum of all scores
    Minimum,
    /// Maximum of all scores
    Maximum,
    /// Geometric mean
    GeometricMean,
}

/// Trait for consuming data from LLM-Marketplace
#[async_trait]
pub trait MarketplaceConsumerTrait: Send + Sync {
    /// Fetch test suite by ID
    async fn get_test_suite(&self, suite_id: &str) -> ExternalConsumerResult<SharedTestSuite>;

    /// List available test suites
    async fn list_test_suites(
        &self,
        category: Option<&str>,
        difficulty: Option<TestDifficulty>,
        limit: Option<u32>,
    ) -> ExternalConsumerResult<Vec<SharedTestSuite>>;

    /// Fetch shield filter by ID
    async fn get_shield_filter(&self, filter_id: &str) -> ExternalConsumerResult<ShieldFilter>;

    /// List shield filters by type
    async fn list_shield_filters(
        &self,
        filter_type: Option<ShieldFilterType>,
        limit: Option<u32>,
    ) -> ExternalConsumerResult<Vec<ShieldFilter>>;

    /// Fetch evaluation template by ID
    async fn get_evaluation_template(
        &self,
        template_id: &str,
    ) -> ExternalConsumerResult<EvaluationTemplate>;

    /// List evaluation templates
    async fn list_evaluation_templates(
        &self,
        template_type: Option<TemplateType>,
        category: Option<&str>,
        limit: Option<u32>,
    ) -> ExternalConsumerResult<Vec<EvaluationTemplate>>;

    /// Health check
    async fn health_check(&self) -> ServiceHealth;
}

/// LLM-Marketplace consumer implementation
pub struct MarketplaceConsumer {
    config: MarketplaceConfig,
    #[allow(dead_code)]
    client: Option<Arc<dyn std::any::Any + Send + Sync>>,
}

impl MarketplaceConsumer {
    /// Create a new marketplace consumer
    pub fn new(config: MarketplaceConfig) -> Self {
        Self {
            config,
            client: None,
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(MarketplaceConfig::default())
    }

    /// Get the current configuration
    pub fn config(&self) -> &MarketplaceConfig {
        &self.config
    }

    /// Build request URL
    fn build_url(&self, path: &str) -> String {
        format!("{}{}", self.config.base_url.trim_end_matches('/'), path)
    }
}

#[async_trait]
impl MarketplaceConsumerTrait for MarketplaceConsumer {
    #[instrument(skip(self), fields(suite_id = %suite_id))]
    async fn get_test_suite(&self, suite_id: &str) -> ExternalConsumerResult<SharedTestSuite> {
        debug!("Fetching test suite from marketplace");

        Err(ExternalConsumerError::ServiceUnavailable(format!(
            "Marketplace API call to {} not yet connected - suite_id: {}",
            self.build_url(&format!("/v1/test-suites/{}", suite_id)),
            suite_id
        )))
    }

    #[instrument(skip(self))]
    async fn list_test_suites(
        &self,
        category: Option<&str>,
        difficulty: Option<TestDifficulty>,
        limit: Option<u32>,
    ) -> ExternalConsumerResult<Vec<SharedTestSuite>> {
        debug!(
            category = ?category,
            difficulty = ?difficulty,
            limit = ?limit,
            "Listing test suites from marketplace"
        );

        Err(ExternalConsumerError::ServiceUnavailable(format!(
            "Marketplace API call to {} not yet connected",
            self.build_url("/v1/test-suites")
        )))
    }

    #[instrument(skip(self), fields(filter_id = %filter_id))]
    async fn get_shield_filter(&self, filter_id: &str) -> ExternalConsumerResult<ShieldFilter> {
        debug!("Fetching shield filter from marketplace");

        Err(ExternalConsumerError::ServiceUnavailable(format!(
            "Marketplace API call to {} not yet connected - filter_id: {}",
            self.build_url(&format!("/v1/shield-filters/{}", filter_id)),
            filter_id
        )))
    }

    #[instrument(skip(self))]
    async fn list_shield_filters(
        &self,
        filter_type: Option<ShieldFilterType>,
        limit: Option<u32>,
    ) -> ExternalConsumerResult<Vec<ShieldFilter>> {
        debug!(
            filter_type = ?filter_type,
            limit = ?limit,
            "Listing shield filters from marketplace"
        );

        Err(ExternalConsumerError::ServiceUnavailable(format!(
            "Marketplace API call to {} not yet connected",
            self.build_url("/v1/shield-filters")
        )))
    }

    #[instrument(skip(self), fields(template_id = %template_id))]
    async fn get_evaluation_template(
        &self,
        template_id: &str,
    ) -> ExternalConsumerResult<EvaluationTemplate> {
        debug!("Fetching evaluation template from marketplace");

        Err(ExternalConsumerError::ServiceUnavailable(format!(
            "Marketplace API call to {} not yet connected - template_id: {}",
            self.build_url(&format!("/v1/evaluation-templates/{}", template_id)),
            template_id
        )))
    }

    #[instrument(skip(self))]
    async fn list_evaluation_templates(
        &self,
        template_type: Option<TemplateType>,
        category: Option<&str>,
        limit: Option<u32>,
    ) -> ExternalConsumerResult<Vec<EvaluationTemplate>> {
        debug!(
            template_type = ?template_type,
            category = ?category,
            limit = ?limit,
            "Listing evaluation templates from marketplace"
        );

        Err(ExternalConsumerError::ServiceUnavailable(format!(
            "Marketplace API call to {} not yet connected",
            self.build_url("/v1/evaluation-templates")
        )))
    }

    async fn health_check(&self) -> ServiceHealth {
        ServiceHealth {
            healthy: false,
            latency_ms: 0,
            error: Some("Marketplace connection not yet established".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_marketplace_config_default() {
        let config = MarketplaceConfig::default();
        assert!(config.base_url.contains("llm-marketplace"));
        assert_eq!(config.timeout_ms, 30000);
    }

    #[test]
    fn test_marketplace_consumer_creation() {
        let consumer = MarketplaceConsumer::with_defaults();
        assert!(consumer.config().enable_cache);
    }

    #[test]
    fn test_test_difficulty_serialization() {
        let difficulty = TestDifficulty::Hard;
        let json = serde_json::to_string(&difficulty).unwrap();
        assert_eq!(json, "\"hard\"");
    }

    #[test]
    fn test_filter_type_serialization() {
        let filter_type = ShieldFilterType::PromptInjection;
        let json = serde_json::to_string(&filter_type).unwrap();
        assert_eq!(json, "\"prompt_injection\"");
    }

    #[tokio::test]
    async fn test_health_check() {
        let consumer = MarketplaceConsumer::with_defaults();
        let health = consumer.health_check().await;
        assert!(!health.healthy);
    }
}
