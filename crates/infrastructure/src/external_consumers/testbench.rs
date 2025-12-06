//! LLM-Test-Bench Runtime Ingestion Adapter
//!
//! Runtime file-based ingestion adapter for LLM-Test-Bench benchmark results.
//! This adapter has NO compile-time dependency on LLM-Test-Bench - it reads
//! benchmark results from exported files (JSON, CSV, JSONL) or via SDK calls.
//!
//! ## Design Principles
//! - No compile-time dependency on LLM-Test-Bench
//! - File-based ingestion (JSON, CSV, JSONL)
//! - Optional SDK-based ingestion via runtime configuration
//! - Validation of ingested data
//! - Streaming support for large files

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::Path;
use tracing::{debug, error, info, instrument, warn};

use super::{ExternalConsumerError, ExternalConsumerResult};

/// Configuration for LLM-Test-Bench ingestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestBenchConfig {
    /// Default input directory for file ingestion
    pub input_directory: Option<String>,
    /// SDK endpoint URL (optional, for SDK-based ingestion)
    pub sdk_endpoint: Option<String>,
    /// SDK API key (optional)
    pub sdk_api_key: Option<String>,
    /// Maximum file size to ingest (in bytes)
    pub max_file_size_bytes: u64,
    /// Batch size for streaming ingestion
    pub batch_size: usize,
    /// Enable validation of ingested results
    pub enable_validation: bool,
    /// Allowed benchmark IDs (empty = allow all)
    pub allowed_benchmark_ids: Vec<String>,
    /// Rejected benchmark IDs
    pub rejected_benchmark_ids: Vec<String>,
}

impl Default for TestBenchConfig {
    fn default() -> Self {
        Self {
            input_directory: None,
            sdk_endpoint: None,
            sdk_api_key: None,
            max_file_size_bytes: 1024 * 1024 * 100, // 100MB
            batch_size: 1000,
            enable_validation: true,
            allowed_benchmark_ids: Vec::new(),
            rejected_benchmark_ids: Vec::new(),
        }
    }
}

/// Supported ingestion formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IngestionFormat {
    /// JSON format (single object or array)
    Json,
    /// JSON Lines format (one JSON object per line)
    Jsonl,
    /// CSV format with headers
    Csv,
}

impl IngestionFormat {
    /// Detect format from file extension
    pub fn from_extension(path: &Path) -> Option<Self> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| match ext.to_lowercase().as_str() {
                "json" => Some(IngestionFormat::Json),
                "jsonl" | "ndjson" => Some(IngestionFormat::Jsonl),
                "csv" => Some(IngestionFormat::Csv),
                _ => None,
            })
    }
}

/// Benchmark result from LLM-Test-Bench
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Unique result identifier
    pub result_id: String,
    /// Benchmark ID this result belongs to
    pub benchmark_id: String,
    /// Benchmark version
    pub benchmark_version: Option<String>,
    /// Model ID that was evaluated
    pub model_id: String,
    /// Model version
    pub model_version: Option<String>,
    /// Execution timestamp
    pub timestamp: DateTime<Utc>,
    /// Overall score (0.0 - 1.0 or custom range)
    pub score: f64,
    /// Score breakdown by metric
    pub metric_scores: HashMap<String, MetricScore>,
    /// Test case results
    pub test_case_results: Vec<TestCaseResult>,
    /// Execution metadata
    pub execution_metadata: ExecutionMetadata,
    /// Raw output data (optional)
    pub raw_output: Option<serde_json::Value>,
    /// Custom tags
    pub tags: HashMap<String, String>,
}

/// Individual metric score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricScore {
    /// Metric name
    pub name: String,
    /// Metric value
    pub value: f64,
    /// Metric weight (for weighted averaging)
    pub weight: Option<f64>,
    /// Passing threshold
    pub threshold: Option<f64>,
    /// Whether this metric passed
    pub passed: bool,
}

/// Individual test case result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCaseResult {
    /// Test case ID
    pub test_case_id: String,
    /// Test case name
    pub name: Option<String>,
    /// Input provided to the model
    pub input: Option<serde_json::Value>,
    /// Expected output
    pub expected_output: Option<serde_json::Value>,
    /// Actual output from the model
    pub actual_output: Option<serde_json::Value>,
    /// Score for this test case
    pub score: f64,
    /// Whether this test case passed
    pub passed: bool,
    /// Latency in milliseconds
    pub latency_ms: Option<u64>,
    /// Token counts
    pub token_count: Option<TokenCount>,
    /// Error if test failed
    pub error: Option<String>,
}

/// Token count information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCount {
    /// Input tokens
    pub input_tokens: u64,
    /// Output tokens
    pub output_tokens: u64,
    /// Total tokens
    pub total_tokens: u64,
}

/// Execution metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    /// Test bench version used
    pub testbench_version: Option<String>,
    /// Execution environment
    pub environment: Option<String>,
    /// Hardware configuration
    pub hardware: Option<HardwareConfig>,
    /// Total execution time in milliseconds
    pub total_duration_ms: u64,
    /// Number of test cases executed
    pub test_case_count: u32,
    /// Number of test cases passed
    pub passed_count: u32,
    /// Number of test cases failed
    pub failed_count: u32,
    /// Average latency across all test cases
    pub avg_latency_ms: Option<f64>,
    /// Total tokens processed
    pub total_tokens: Option<u64>,
}

/// Hardware configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareConfig {
    /// CPU model
    pub cpu: Option<String>,
    /// GPU model
    pub gpu: Option<String>,
    /// RAM in GB
    pub ram_gb: Option<u32>,
    /// VRAM in GB
    pub vram_gb: Option<u32>,
}

/// Ingestion result summary
#[derive(Debug, Clone)]
pub struct IngestionSummary {
    /// Total records processed
    pub total_records: usize,
    /// Successfully ingested records
    pub successful_records: usize,
    /// Failed records
    pub failed_records: usize,
    /// Validation errors
    pub validation_errors: Vec<ValidationError>,
    /// Ingestion duration in milliseconds
    pub duration_ms: u64,
}

/// Validation error during ingestion
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Record index or identifier
    pub record_id: String,
    /// Field that failed validation
    pub field: String,
    /// Error message
    pub message: String,
}

/// Trait for Test-Bench ingestion
#[async_trait]
pub trait TestBenchIngesterTrait: Send + Sync {
    /// Ingest benchmark results from a file
    async fn ingest_file(&self, path: &Path) -> ExternalConsumerResult<IngestionSummary>;

    /// Ingest benchmark results from a file with explicit format
    async fn ingest_file_with_format(
        &self,
        path: &Path,
        format: IngestionFormat,
    ) -> ExternalConsumerResult<IngestionSummary>;

    /// Ingest benchmark results from raw bytes
    async fn ingest_bytes(
        &self,
        data: &[u8],
        format: IngestionFormat,
    ) -> ExternalConsumerResult<IngestionSummary>;

    /// Ingest a single benchmark result
    async fn ingest_result(&self, result: BenchmarkResult) -> ExternalConsumerResult<()>;

    /// Ingest benchmark results from SDK (if configured)
    async fn ingest_from_sdk(
        &self,
        benchmark_id: &str,
        limit: Option<u32>,
    ) -> ExternalConsumerResult<IngestionSummary>;

    /// Parse results without ingesting (for validation)
    fn parse_file(&self, path: &Path) -> ExternalConsumerResult<Vec<BenchmarkResult>>;

    /// Validate a benchmark result
    fn validate_result(&self, result: &BenchmarkResult) -> Vec<ValidationError>;
}

/// LLM-Test-Bench ingestion adapter
pub struct TestBenchIngester {
    config: TestBenchConfig,
    // Results storage - in production, would write to repository
    #[allow(dead_code)]
    results_buffer: std::sync::Mutex<Vec<BenchmarkResult>>,
}

impl TestBenchIngester {
    /// Create a new test bench ingester
    pub fn new(config: TestBenchConfig) -> Self {
        Self {
            config,
            results_buffer: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(TestBenchConfig::default())
    }

    /// Get the current configuration
    pub fn config(&self) -> &TestBenchConfig {
        &self.config
    }

    /// Check if a benchmark ID is allowed
    fn is_benchmark_allowed(&self, benchmark_id: &str) -> bool {
        // If rejected list contains the ID, reject
        if self.config.rejected_benchmark_ids.contains(&benchmark_id.to_string()) {
            return false;
        }

        // If allowed list is empty, allow all (except rejected)
        if self.config.allowed_benchmark_ids.is_empty() {
            return true;
        }

        // Otherwise, check if in allowed list
        self.config.allowed_benchmark_ids.contains(&benchmark_id.to_string())
    }

    /// Parse JSON file
    fn parse_json(&self, data: &[u8]) -> ExternalConsumerResult<Vec<BenchmarkResult>> {
        let parsed: serde_json::Value = serde_json::from_slice(data)
            .map_err(|e| ExternalConsumerError::ParseError(format!("Invalid JSON: {}", e)))?;

        match parsed {
            serde_json::Value::Array(arr) => {
                let mut results = Vec::with_capacity(arr.len());
                for (i, item) in arr.into_iter().enumerate() {
                    let result: BenchmarkResult = serde_json::from_value(item).map_err(|e| {
                        ExternalConsumerError::ParseError(format!(
                            "Failed to parse result at index {}: {}",
                            i, e
                        ))
                    })?;
                    results.push(result);
                }
                Ok(results)
            }
            serde_json::Value::Object(_) => {
                let result: BenchmarkResult = serde_json::from_value(parsed).map_err(|e| {
                    ExternalConsumerError::ParseError(format!("Failed to parse result: {}", e))
                })?;
                Ok(vec![result])
            }
            _ => Err(ExternalConsumerError::ParseError(
                "Expected JSON object or array".to_string(),
            )),
        }
    }

    /// Parse JSONL file
    fn parse_jsonl(&self, data: &[u8]) -> ExternalConsumerResult<Vec<BenchmarkResult>> {
        let reader = BufReader::new(data);
        let mut results = Vec::new();

        for (line_num, line) in reader.lines().enumerate() {
            let line = line.map_err(|e| {
                ExternalConsumerError::IoError(format!("Failed to read line {}: {}", line_num + 1, e))
            })?;

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let result: BenchmarkResult = serde_json::from_str(trimmed).map_err(|e| {
                ExternalConsumerError::ParseError(format!(
                    "Failed to parse line {}: {}",
                    line_num + 1,
                    e
                ))
            })?;
            results.push(result);
        }

        Ok(results)
    }

    /// Parse CSV file
    fn parse_csv(&self, data: &[u8]) -> ExternalConsumerResult<Vec<BenchmarkResult>> {
        let mut reader = csv::Reader::from_reader(data);
        let mut results = Vec::new();

        for (row_num, record) in reader.deserialize().enumerate() {
            let result: BenchmarkResult = record.map_err(|e| {
                ExternalConsumerError::ParseError(format!(
                    "Failed to parse CSV row {}: {}",
                    row_num + 1,
                    e
                ))
            })?;
            results.push(result);
        }

        Ok(results)
    }
}

#[async_trait]
impl TestBenchIngesterTrait for TestBenchIngester {
    #[instrument(skip(self), fields(path = %path.display()))]
    async fn ingest_file(&self, path: &Path) -> ExternalConsumerResult<IngestionSummary> {
        let format = IngestionFormat::from_extension(path).ok_or_else(|| {
            ExternalConsumerError::ConfigurationError(format!(
                "Unable to detect format from file extension: {}",
                path.display()
            ))
        })?;

        self.ingest_file_with_format(path, format).await
    }

    #[instrument(skip(self), fields(path = %path.display(), format = ?format))]
    async fn ingest_file_with_format(
        &self,
        path: &Path,
        format: IngestionFormat,
    ) -> ExternalConsumerResult<IngestionSummary> {
        info!("Ingesting benchmark results from file");

        // Check file exists
        if !path.exists() {
            return Err(ExternalConsumerError::NotFound(format!(
                "File not found: {}",
                path.display()
            )));
        }

        // Check file size
        let metadata = std::fs::metadata(path).map_err(|e| {
            ExternalConsumerError::IoError(format!("Failed to read file metadata: {}", e))
        })?;

        if metadata.len() > self.config.max_file_size_bytes {
            return Err(ExternalConsumerError::ConfigurationError(format!(
                "File size {} exceeds maximum allowed size {}",
                metadata.len(),
                self.config.max_file_size_bytes
            )));
        }

        // Read file
        let data = std::fs::read(path).map_err(|e| {
            ExternalConsumerError::IoError(format!("Failed to read file: {}", e))
        })?;

        self.ingest_bytes(&data, format).await
    }

    #[instrument(skip(self, data), fields(format = ?format, data_len = data.len()))]
    async fn ingest_bytes(
        &self,
        data: &[u8],
        format: IngestionFormat,
    ) -> ExternalConsumerResult<IngestionSummary> {
        let start = std::time::Instant::now();

        // Parse based on format
        let results = match format {
            IngestionFormat::Json => self.parse_json(data)?,
            IngestionFormat::Jsonl => self.parse_jsonl(data)?,
            IngestionFormat::Csv => self.parse_csv(data)?,
        };

        let total_records = results.len();
        let mut successful_records = 0;
        let mut failed_records = 0;
        let mut validation_errors = Vec::new();

        for result in results {
            // Check if benchmark is allowed
            if !self.is_benchmark_allowed(&result.benchmark_id) {
                warn!(
                    benchmark_id = %result.benchmark_id,
                    "Skipping result from disallowed benchmark"
                );
                failed_records += 1;
                validation_errors.push(ValidationError {
                    record_id: result.result_id.clone(),
                    field: "benchmark_id".to_string(),
                    message: "Benchmark ID not allowed".to_string(),
                });
                continue;
            }

            // Validate if enabled
            if self.config.enable_validation {
                let errors = self.validate_result(&result);
                if !errors.is_empty() {
                    failed_records += 1;
                    validation_errors.extend(errors);
                    continue;
                }
            }

            // Ingest the result
            match self.ingest_result(result).await {
                Ok(_) => successful_records += 1,
                Err(e) => {
                    error!(error = %e, "Failed to ingest result");
                    failed_records += 1;
                }
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        info!(
            total = total_records,
            successful = successful_records,
            failed = failed_records,
            duration_ms = duration_ms,
            "Ingestion complete"
        );

        Ok(IngestionSummary {
            total_records,
            successful_records,
            failed_records,
            validation_errors,
            duration_ms,
        })
    }

    #[instrument(skip(self, result), fields(result_id = %result.result_id))]
    async fn ingest_result(&self, result: BenchmarkResult) -> ExternalConsumerResult<()> {
        debug!(
            benchmark_id = %result.benchmark_id,
            model_id = %result.model_id,
            score = result.score,
            "Ingesting benchmark result"
        );

        // In production, this would write to the repository
        // For now, we store in the buffer
        let mut buffer = self.results_buffer.lock().unwrap();
        buffer.push(result);

        Ok(())
    }

    #[instrument(skip(self), fields(benchmark_id = %benchmark_id))]
    async fn ingest_from_sdk(
        &self,
        benchmark_id: &str,
        limit: Option<u32>,
    ) -> ExternalConsumerResult<IngestionSummary> {
        debug!(limit = ?limit, "Ingesting from SDK");

        // Check if SDK is configured
        let _endpoint = self.config.sdk_endpoint.as_ref().ok_or_else(|| {
            ExternalConsumerError::ConfigurationError(
                "SDK endpoint not configured".to_string()
            )
        })?;

        // In production, would make HTTP/gRPC call to SDK endpoint
        Err(ExternalConsumerError::ServiceUnavailable(format!(
            "SDK ingestion not yet connected for benchmark: {}",
            benchmark_id
        )))
    }

    fn parse_file(&self, path: &Path) -> ExternalConsumerResult<Vec<BenchmarkResult>> {
        let format = IngestionFormat::from_extension(path).ok_or_else(|| {
            ExternalConsumerError::ConfigurationError(format!(
                "Unable to detect format from file extension: {}",
                path.display()
            ))
        })?;

        let data = std::fs::read(path).map_err(|e| {
            ExternalConsumerError::IoError(format!("Failed to read file: {}", e))
        })?;

        match format {
            IngestionFormat::Json => self.parse_json(&data),
            IngestionFormat::Jsonl => self.parse_jsonl(&data),
            IngestionFormat::Csv => self.parse_csv(&data),
        }
    }

    fn validate_result(&self, result: &BenchmarkResult) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // Validate required fields
        if result.result_id.is_empty() {
            errors.push(ValidationError {
                record_id: result.result_id.clone(),
                field: "result_id".to_string(),
                message: "Result ID cannot be empty".to_string(),
            });
        }

        if result.benchmark_id.is_empty() {
            errors.push(ValidationError {
                record_id: result.result_id.clone(),
                field: "benchmark_id".to_string(),
                message: "Benchmark ID cannot be empty".to_string(),
            });
        }

        if result.model_id.is_empty() {
            errors.push(ValidationError {
                record_id: result.result_id.clone(),
                field: "model_id".to_string(),
                message: "Model ID cannot be empty".to_string(),
            });
        }

        // Validate score range (if using 0-1 scale)
        if result.score < 0.0 {
            errors.push(ValidationError {
                record_id: result.result_id.clone(),
                field: "score".to_string(),
                message: "Score cannot be negative".to_string(),
            });
        }

        // Validate test case count consistency
        let expected_count = result.execution_metadata.test_case_count as usize;
        let actual_count = result.test_case_results.len();
        if expected_count != actual_count && expected_count > 0 {
            errors.push(ValidationError {
                record_id: result.result_id.clone(),
                field: "test_case_results".to_string(),
                message: format!(
                    "Test case count mismatch: expected {}, got {}",
                    expected_count, actual_count
                ),
            });
        }

        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_testbench_config_default() {
        let config = TestBenchConfig::default();
        assert!(config.enable_validation);
        assert_eq!(config.batch_size, 1000);
    }

    #[test]
    fn test_ingestion_format_detection() {
        assert_eq!(
            IngestionFormat::from_extension(Path::new("results.json")),
            Some(IngestionFormat::Json)
        );
        assert_eq!(
            IngestionFormat::from_extension(Path::new("results.jsonl")),
            Some(IngestionFormat::Jsonl)
        );
        assert_eq!(
            IngestionFormat::from_extension(Path::new("results.csv")),
            Some(IngestionFormat::Csv)
        );
        assert_eq!(
            IngestionFormat::from_extension(Path::new("results.txt")),
            None
        );
    }

    #[test]
    fn test_benchmark_allowed() {
        let mut config = TestBenchConfig::default();
        let ingester = TestBenchIngester::new(config.clone());

        // Empty lists = allow all
        assert!(ingester.is_benchmark_allowed("any-benchmark"));

        // With allowed list
        config.allowed_benchmark_ids = vec!["allowed-1".to_string(), "allowed-2".to_string()];
        let ingester = TestBenchIngester::new(config.clone());
        assert!(ingester.is_benchmark_allowed("allowed-1"));
        assert!(!ingester.is_benchmark_allowed("not-allowed"));

        // With rejected list
        config.allowed_benchmark_ids = Vec::new();
        config.rejected_benchmark_ids = vec!["rejected-1".to_string()];
        let ingester = TestBenchIngester::new(config);
        assert!(ingester.is_benchmark_allowed("any-benchmark"));
        assert!(!ingester.is_benchmark_allowed("rejected-1"));
    }

    #[test]
    fn test_validate_result() {
        let ingester = TestBenchIngester::with_defaults();

        // Valid result
        let valid_result = BenchmarkResult {
            result_id: "test-1".to_string(),
            benchmark_id: "benchmark-1".to_string(),
            benchmark_version: Some("1.0.0".to_string()),
            model_id: "model-1".to_string(),
            model_version: Some("1.0.0".to_string()),
            timestamp: Utc::now(),
            score: 0.85,
            metric_scores: HashMap::new(),
            test_case_results: Vec::new(),
            execution_metadata: ExecutionMetadata {
                testbench_version: Some("1.0.0".to_string()),
                environment: None,
                hardware: None,
                total_duration_ms: 1000,
                test_case_count: 0,
                passed_count: 0,
                failed_count: 0,
                avg_latency_ms: None,
                total_tokens: None,
            },
            raw_output: None,
            tags: HashMap::new(),
        };

        let errors = ingester.validate_result(&valid_result);
        assert!(errors.is_empty());

        // Invalid result - empty IDs
        let invalid_result = BenchmarkResult {
            result_id: "".to_string(),
            benchmark_id: "".to_string(),
            model_id: "".to_string(),
            score: -1.0,
            ..valid_result
        };

        let errors = ingester.validate_result(&invalid_result);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.field == "result_id"));
        assert!(errors.iter().any(|e| e.field == "benchmark_id"));
        assert!(errors.iter().any(|e| e.field == "model_id"));
        assert!(errors.iter().any(|e| e.field == "score"));
    }

    #[test]
    fn test_parse_json() {
        let ingester = TestBenchIngester::with_defaults();

        let json_data = r#"{
            "result_id": "test-1",
            "benchmark_id": "bench-1",
            "model_id": "model-1",
            "timestamp": "2024-01-01T00:00:00Z",
            "score": 0.95,
            "metric_scores": {},
            "test_case_results": [],
            "execution_metadata": {
                "total_duration_ms": 1000,
                "test_case_count": 0,
                "passed_count": 0,
                "failed_count": 0
            },
            "tags": {}
        }"#;

        let results = ingester.parse_json(json_data.as_bytes()).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].result_id, "test-1");
        assert_eq!(results[0].score, 0.95);
    }
}
