//! Evaluation and scoring types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

/// Comprehensive evaluation criteria
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationCriteria {
    pub primary_metric: MetricDefinition,
    pub secondary_metrics: Vec<MetricDefinition>,
    pub aggregation_method: AggregationMethod,
    pub score_normalization: ScoreNormalization,
    pub minimum_test_cases: usize,
    pub confidence_level: f64,
}

/// Metric definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDefinition {
    pub name: String,
    pub description: String,
    pub metric_type: MetricType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    pub higher_is_better: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<MetricRange>,
}

/// Types of metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum MetricType {
    Accuracy,
    F1Score,
    Bleu,
    Rouge,
    ExactMatch,
    Perplexity,
    Latency,
    Throughput,
    CostPerToken,
    Custom { formula: String },
}

/// Valid range for metric values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricRange {
    pub min: f64,
    pub max: f64,
}

/// Methods for aggregating scores
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AggregationMethod {
    Mean,
    WeightedMean { weights: HashMap<String, f64> },
    Median,
    GeometricMean,
    HarmonicMean,
    Min,
    Max,
    Percentile { percentile: f64 },
    Custom { formula: String },
}

/// Score normalization methods
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScoreNormalization {
    None,
    MinMax { min: f64, max: f64 },
    ZScore,
    Percentile,
    LogScale,
}

/// Execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    pub timeout_per_test_ms: u64,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub parallelism: ParallelismConfig,
    pub model_parameters: ModelParameters,
    pub environment_requirements: EnvironmentRequirements,
}

/// Parallelism configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelismConfig {
    pub max_concurrent_requests: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_per_minute: Option<u32>,
}

/// Model parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelParameters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    pub stop_sequences: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub random_seed: Option<u64>,
    pub additional_params: HashMap<String, serde_json::Value>,
}

/// Environment requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentRequirements {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub python_version: Option<String>,
    pub required_packages: Vec<PackageRequirement>,
    pub gpu_required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_memory_gb: Option<u32>,
}

/// Package dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageRequirement {
    pub name: String,
    pub version_constraint: String,
}

/// Dataset reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetReference {
    pub name: String,
    pub version: String,
    pub source_url: Url,
    pub checksum: DatasetChecksum,
    pub size_bytes: u64,
    pub format: DatasetFormat,
}

/// Dataset checksum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetChecksum {
    pub algorithm: ChecksumAlgorithm,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChecksumAlgorithm {
    Sha256,
    Sha384,
    Sha512,
    Blake3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatasetFormat {
    Json,
    Jsonl,
    Csv,
    Parquet,
    Arrow,
}
