//! Canonical BenchmarkResult struct for the LLM Benchmark Exchange.
//!
//! This module defines the standardized result type used across all benchmark targets.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Canonical benchmark result structure.
///
/// This struct contains exactly the fields required by the canonical benchmark interface:
/// - `target_id`: Unique identifier for the benchmark target
/// - `metrics`: JSON value containing the benchmark metrics
/// - `timestamp`: UTC timestamp when the benchmark was executed
///
/// # Example
///
/// ```rust
/// use llm_benchmark_benchmarks::result::BenchmarkResult;
/// use serde_json::json;
/// use chrono::Utc;
///
/// let result = BenchmarkResult::new(
///     "test-suite-ingestion".to_string(),
///     json!({
///         "duration_ms": 150,
///         "items_processed": 1000,
///         "throughput": 6666.67
///     }),
/// );
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BenchmarkResult {
    /// Unique identifier for the benchmark target that produced this result.
    pub target_id: String,

    /// JSON value containing the benchmark metrics.
    /// The structure of this value depends on the specific benchmark target.
    pub metrics: serde_json::Value,

    /// UTC timestamp when the benchmark was executed.
    pub timestamp: DateTime<Utc>,
}

impl BenchmarkResult {
    /// Creates a new BenchmarkResult with the current UTC timestamp.
    ///
    /// # Arguments
    ///
    /// * `target_id` - Unique identifier for the benchmark target
    /// * `metrics` - JSON value containing the benchmark metrics
    ///
    /// # Returns
    ///
    /// A new `BenchmarkResult` with the provided values and current timestamp.
    pub fn new(target_id: String, metrics: serde_json::Value) -> Self {
        Self {
            target_id,
            metrics,
            timestamp: Utc::now(),
        }
    }

    /// Creates a new BenchmarkResult with a specific timestamp.
    ///
    /// # Arguments
    ///
    /// * `target_id` - Unique identifier for the benchmark target
    /// * `metrics` - JSON value containing the benchmark metrics
    /// * `timestamp` - Specific UTC timestamp for the result
    ///
    /// # Returns
    ///
    /// A new `BenchmarkResult` with the provided values.
    pub fn with_timestamp(
        target_id: String,
        metrics: serde_json::Value,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            target_id,
            metrics,
            timestamp,
        }
    }

    /// Returns a reference to the target ID.
    pub fn target_id(&self) -> &str {
        &self.target_id
    }

    /// Returns a reference to the metrics.
    pub fn metrics(&self) -> &serde_json::Value {
        &self.metrics
    }

    /// Returns the timestamp.
    pub fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    /// Converts the result to a JSON string.
    ///
    /// # Returns
    ///
    /// A `Result` containing the JSON string representation or a serialization error.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Creates a BenchmarkResult from a JSON string.
    ///
    /// # Arguments
    ///
    /// * `json` - JSON string representation of a BenchmarkResult
    ///
    /// # Returns
    ///
    /// A `Result` containing the deserialized `BenchmarkResult` or a deserialization error.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_benchmark_result_new() {
        let result = BenchmarkResult::new(
            "test-target".to_string(),
            json!({"duration_ms": 100}),
        );

        assert_eq!(result.target_id(), "test-target");
        assert_eq!(result.metrics()["duration_ms"], 100);
    }

    #[test]
    fn test_benchmark_result_serialization() {
        let result = BenchmarkResult::new(
            "test-target".to_string(),
            json!({"value": 42}),
        );

        let json_str = result.to_json().unwrap();
        let deserialized = BenchmarkResult::from_json(&json_str).unwrap();

        assert_eq!(result.target_id, deserialized.target_id);
        assert_eq!(result.metrics, deserialized.metrics);
    }
}
