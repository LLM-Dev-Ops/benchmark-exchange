//! LLM-Observatory Consumer Adapter
//!
//! Thin runtime adapter for consuming telemetry and metrics from LLM-Observatory:
//! - Benchmark execution statistics
//! - Performance metadata
//! - Telemetry data for analysis
//!
//! This adapter provides read-only consumption without modifying existing APIs.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, instrument};

use super::{ExternalConsumerError, ExternalConsumerResult, ServiceHealth};

/// Configuration for LLM-Observatory connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservatoryConfig {
    /// Base URL for the observatory API
    pub base_url: String,
    /// API key for authentication
    pub api_key: Option<String>,
    /// Request timeout in milliseconds
    pub timeout_ms: u64,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Enable caching of telemetry data
    pub enable_cache: bool,
    /// Cache TTL in seconds
    pub cache_ttl_secs: u64,
    /// Maximum time range for queries (in hours)
    pub max_time_range_hours: u32,
}

impl Default for ObservatoryConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.llm-observatory.dev".to_string(),
            api_key: None,
            timeout_ms: 30000,
            max_retries: 3,
            enable_cache: true,
            cache_ttl_secs: 60,
            max_time_range_hours: 168, // 1 week
        }
    }
}

/// Execution telemetry from LLM-Observatory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTelemetry {
    /// Unique telemetry record ID
    pub telemetry_id: String,
    /// Benchmark execution ID
    pub execution_id: String,
    /// Benchmark ID
    pub benchmark_id: String,
    /// Model ID being evaluated
    pub model_id: String,
    /// Execution timestamp
    pub timestamp: DateTime<Utc>,
    /// Duration of execution in milliseconds
    pub duration_ms: u64,
    /// Execution status
    pub status: ExecutionStatus,
    /// Resource usage metrics
    pub resource_usage: ResourceUsage,
    /// Request/response metrics
    pub request_metrics: RequestMetrics,
    /// Error details if failed
    pub error: Option<ExecutionError>,
    /// Custom dimensions for filtering
    pub dimensions: HashMap<String, String>,
    /// Custom metrics
    pub metrics: HashMap<String, f64>,
}

/// Execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    /// Execution completed successfully
    Success,
    /// Execution failed
    Failed,
    /// Execution timed out
    Timeout,
    /// Execution was cancelled
    Cancelled,
    /// Execution is in progress
    InProgress,
}

/// Resource usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU usage percentage
    pub cpu_percent: Option<f64>,
    /// Memory usage in bytes
    pub memory_bytes: Option<u64>,
    /// GPU memory usage in bytes
    pub gpu_memory_bytes: Option<u64>,
    /// Network bytes sent
    pub network_sent_bytes: Option<u64>,
    /// Network bytes received
    pub network_recv_bytes: Option<u64>,
    /// Tokens processed (input)
    pub input_tokens: Option<u64>,
    /// Tokens processed (output)
    pub output_tokens: Option<u64>,
}

/// Request/response metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMetrics {
    /// Total request count
    pub request_count: u64,
    /// Successful request count
    pub success_count: u64,
    /// Failed request count
    pub failure_count: u64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    /// P50 latency in milliseconds
    pub p50_latency_ms: f64,
    /// P95 latency in milliseconds
    pub p95_latency_ms: f64,
    /// P99 latency in milliseconds
    pub p99_latency_ms: f64,
    /// Requests per second
    pub requests_per_second: f64,
}

/// Execution error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionError {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Error category
    pub category: ErrorCategory,
    /// Stack trace if available
    pub stack_trace: Option<String>,
}

/// Error category
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    /// Network-related error
    Network,
    /// Authentication error
    Authentication,
    /// Rate limiting error
    RateLimit,
    /// Timeout error
    Timeout,
    /// Model error
    Model,
    /// Validation error
    Validation,
    /// Internal error
    Internal,
    /// Unknown error
    Unknown,
}

/// Performance metadata from LLM-Observatory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetadata {
    /// Benchmark ID
    pub benchmark_id: String,
    /// Model ID
    pub model_id: String,
    /// Time period start
    pub period_start: DateTime<Utc>,
    /// Time period end
    pub period_end: DateTime<Utc>,
    /// Aggregation interval
    pub aggregation: AggregationInterval,
    /// Summary statistics
    pub summary: PerformanceSummary,
    /// Time series data points
    pub time_series: Vec<TimeSeriesPoint>,
    /// Comparison with baseline
    pub baseline_comparison: Option<BaselineComparison>,
}

/// Aggregation interval
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AggregationInterval {
    /// Per-minute aggregation
    Minute,
    /// Per-hour aggregation
    Hour,
    /// Per-day aggregation
    Day,
    /// Per-week aggregation
    Week,
}

/// Performance summary statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSummary {
    /// Total executions
    pub total_executions: u64,
    /// Successful executions
    pub successful_executions: u64,
    /// Failed executions
    pub failed_executions: u64,
    /// Success rate (0.0 - 1.0)
    pub success_rate: f64,
    /// Average score
    pub avg_score: f64,
    /// Score standard deviation
    pub score_stddev: f64,
    /// Minimum score
    pub min_score: f64,
    /// Maximum score
    pub max_score: f64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    /// Total tokens processed
    pub total_tokens: u64,
    /// Average tokens per second
    pub tokens_per_second: f64,
}

/// Time series data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Execution count
    pub execution_count: u64,
    /// Success count
    pub success_count: u64,
    /// Average score
    pub avg_score: f64,
    /// Average latency
    pub avg_latency_ms: f64,
    /// Tokens per second
    pub tokens_per_second: f64,
}

/// Baseline comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineComparison {
    /// Baseline model ID
    pub baseline_model_id: String,
    /// Score difference (positive = better than baseline)
    pub score_diff: f64,
    /// Score difference percentage
    pub score_diff_percent: f64,
    /// Latency difference (negative = faster than baseline)
    pub latency_diff_ms: f64,
    /// Latency difference percentage
    pub latency_diff_percent: f64,
    /// Statistical significance
    pub is_significant: bool,
    /// P-value for significance test
    pub p_value: Option<f64>,
}

/// Query parameters for telemetry
#[derive(Debug, Clone, Default)]
pub struct TelemetryQuery {
    /// Filter by benchmark ID
    pub benchmark_id: Option<String>,
    /// Filter by model ID
    pub model_id: Option<String>,
    /// Start time
    pub start_time: Option<DateTime<Utc>>,
    /// End time
    pub end_time: Option<DateTime<Utc>>,
    /// Filter by status
    pub status: Option<ExecutionStatus>,
    /// Maximum records to return
    pub limit: Option<u32>,
    /// Offset for pagination
    pub offset: Option<u32>,
}

/// Trait for consuming data from LLM-Observatory
#[async_trait]
pub trait ObservatoryConsumerTrait: Send + Sync {
    /// Fetch execution telemetry by ID
    async fn get_execution_telemetry(
        &self,
        telemetry_id: &str,
    ) -> ExternalConsumerResult<ExecutionTelemetry>;

    /// Query execution telemetry
    async fn query_execution_telemetry(
        &self,
        query: TelemetryQuery,
    ) -> ExternalConsumerResult<Vec<ExecutionTelemetry>>;

    /// Fetch performance metadata for a benchmark/model pair
    async fn get_performance_metadata(
        &self,
        benchmark_id: &str,
        model_id: &str,
        aggregation: AggregationInterval,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> ExternalConsumerResult<PerformanceMetadata>;

    /// Get aggregated statistics for a benchmark
    async fn get_benchmark_statistics(
        &self,
        benchmark_id: &str,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> ExternalConsumerResult<PerformanceSummary>;

    /// Get aggregated statistics for a model across benchmarks
    async fn get_model_statistics(
        &self,
        model_id: &str,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> ExternalConsumerResult<PerformanceSummary>;

    /// Health check
    async fn health_check(&self) -> ServiceHealth;
}

/// LLM-Observatory consumer implementation
pub struct ObservatoryConsumer {
    config: ObservatoryConfig,
    #[allow(dead_code)]
    client: Option<Arc<dyn std::any::Any + Send + Sync>>,
}

impl ObservatoryConsumer {
    /// Create a new observatory consumer
    pub fn new(config: ObservatoryConfig) -> Self {
        Self {
            config,
            client: None,
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(ObservatoryConfig::default())
    }

    /// Get the current configuration
    pub fn config(&self) -> &ObservatoryConfig {
        &self.config
    }

    /// Build request URL
    fn build_url(&self, path: &str) -> String {
        format!("{}{}", self.config.base_url.trim_end_matches('/'), path)
    }
}

#[async_trait]
impl ObservatoryConsumerTrait for ObservatoryConsumer {
    #[instrument(skip(self), fields(telemetry_id = %telemetry_id))]
    async fn get_execution_telemetry(
        &self,
        telemetry_id: &str,
    ) -> ExternalConsumerResult<ExecutionTelemetry> {
        debug!("Fetching execution telemetry from observatory");

        Err(ExternalConsumerError::ServiceUnavailable(format!(
            "Observatory API call to {} not yet connected - telemetry_id: {}",
            self.build_url(&format!("/v1/telemetry/{}", telemetry_id)),
            telemetry_id
        )))
    }

    #[instrument(skip(self))]
    async fn query_execution_telemetry(
        &self,
        query: TelemetryQuery,
    ) -> ExternalConsumerResult<Vec<ExecutionTelemetry>> {
        debug!(
            benchmark_id = ?query.benchmark_id,
            model_id = ?query.model_id,
            "Querying execution telemetry from observatory"
        );

        Err(ExternalConsumerError::ServiceUnavailable(format!(
            "Observatory API call to {} not yet connected",
            self.build_url("/v1/telemetry")
        )))
    }

    #[instrument(skip(self), fields(benchmark_id = %benchmark_id, model_id = %model_id))]
    async fn get_performance_metadata(
        &self,
        benchmark_id: &str,
        model_id: &str,
        aggregation: AggregationInterval,
        _start_time: Option<DateTime<Utc>>,
        _end_time: Option<DateTime<Utc>>,
    ) -> ExternalConsumerResult<PerformanceMetadata> {
        debug!(
            aggregation = ?aggregation,
            "Fetching performance metadata from observatory"
        );

        Err(ExternalConsumerError::ServiceUnavailable(format!(
            "Observatory API call to {} not yet connected",
            self.build_url(&format!(
                "/v1/performance/{}/{}",
                benchmark_id, model_id
            ))
        )))
    }

    #[instrument(skip(self), fields(benchmark_id = %benchmark_id))]
    async fn get_benchmark_statistics(
        &self,
        benchmark_id: &str,
        _start_time: Option<DateTime<Utc>>,
        _end_time: Option<DateTime<Utc>>,
    ) -> ExternalConsumerResult<PerformanceSummary> {
        debug!("Fetching benchmark statistics from observatory");

        Err(ExternalConsumerError::ServiceUnavailable(format!(
            "Observatory API call to {} not yet connected - benchmark_id: {}",
            self.build_url(&format!("/v1/statistics/benchmark/{}", benchmark_id)),
            benchmark_id
        )))
    }

    #[instrument(skip(self), fields(model_id = %model_id))]
    async fn get_model_statistics(
        &self,
        model_id: &str,
        _start_time: Option<DateTime<Utc>>,
        _end_time: Option<DateTime<Utc>>,
    ) -> ExternalConsumerResult<PerformanceSummary> {
        debug!("Fetching model statistics from observatory");

        Err(ExternalConsumerError::ServiceUnavailable(format!(
            "Observatory API call to {} not yet connected - model_id: {}",
            self.build_url(&format!("/v1/statistics/model/{}", model_id)),
            model_id
        )))
    }

    async fn health_check(&self) -> ServiceHealth {
        ServiceHealth {
            healthy: false,
            latency_ms: 0,
            error: Some("Observatory connection not yet established".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observatory_config_default() {
        let config = ObservatoryConfig::default();
        assert!(config.base_url.contains("llm-observatory"));
        assert_eq!(config.timeout_ms, 30000);
        assert_eq!(config.max_time_range_hours, 168);
    }

    #[test]
    fn test_observatory_consumer_creation() {
        let consumer = ObservatoryConsumer::with_defaults();
        assert!(consumer.config().enable_cache);
    }

    #[test]
    fn test_execution_status_serialization() {
        let status = ExecutionStatus::Success;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"success\"");
    }

    #[test]
    fn test_aggregation_interval_serialization() {
        let interval = AggregationInterval::Hour;
        let json = serde_json::to_string(&interval).unwrap();
        assert_eq!(json, "\"hour\"");
    }

    #[tokio::test]
    async fn test_health_check() {
        let consumer = ObservatoryConsumer::with_defaults();
        let health = consumer.health_check().await;
        assert!(!health.healthy);
    }

    #[test]
    fn test_telemetry_query_default() {
        let query = TelemetryQuery::default();
        assert!(query.benchmark_id.is_none());
        assert!(query.model_id.is_none());
        assert!(query.limit.is_none());
    }
}
