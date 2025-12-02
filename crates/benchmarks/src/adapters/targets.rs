//! Concrete benchmark target implementations.
//!
//! This module contains implementations of representative Benchmark Exchange operations
//! exposed as benchmark targets via the adapter system.

use std::time::Instant;

use async_trait::async_trait;
use serde_json::json;

use super::BenchTarget;
use crate::result::BenchmarkResult;

/// Benchmark for test-suite ingestion speed.
///
/// Measures the performance of loading and parsing test case definitions,
/// simulating the ingestion of benchmark test suites.
pub struct TestSuiteIngestionBenchmark {
    /// Number of test cases to simulate
    test_case_count: usize,
}

impl TestSuiteIngestionBenchmark {
    /// Creates a new test suite ingestion benchmark with default parameters.
    pub fn new() -> Self {
        Self {
            test_case_count: 1000,
        }
    }

    /// Creates a new test suite ingestion benchmark with custom test case count.
    pub fn with_count(test_case_count: usize) -> Self {
        Self { test_case_count }
    }
}

impl Default for TestSuiteIngestionBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BenchTarget for TestSuiteIngestionBenchmark {
    fn id(&self) -> &'static str {
        "test-suite-ingestion"
    }

    fn description(&self) -> &'static str {
        "Measures test suite ingestion and parsing speed"
    }

    fn category(&self) -> &'static str {
        "ingestion"
    }

    async fn run(&self) -> anyhow::Result<BenchmarkResult> {
        let start = Instant::now();

        // Simulate test case ingestion
        let mut total_bytes = 0usize;
        for i in 0..self.test_case_count {
            // Simulate parsing a test case definition
            let test_case = json!({
                "id": format!("test-case-{}", i),
                "prompt": format!("This is a test prompt for case {} with some additional content to make it realistic.", i),
                "expected_output": format!("Expected response for case {}", i),
                "metadata": {
                    "category": "general",
                    "difficulty": i % 5,
                    "tags": ["test", "benchmark", "automated"]
                }
            });

            let serialized = serde_json::to_string(&test_case)?;
            total_bytes += serialized.len();

            // Simulate validation
            let _: serde_json::Value = serde_json::from_str(&serialized)?;
        }

        let duration = start.elapsed();
        let duration_ms = duration.as_secs_f64() * 1000.0;
        let items_per_second = self.test_case_count as f64 / duration.as_secs_f64();
        let throughput_mb_s = (total_bytes as f64 / (1024.0 * 1024.0)) / duration.as_secs_f64();

        let metrics = json!({
            "duration_ms": duration_ms,
            "items_processed": self.test_case_count,
            "items_per_second": items_per_second,
            "total_bytes": total_bytes,
            "throughput_mb_s": throughput_mb_s
        });

        Ok(BenchmarkResult::new(self.id().to_string(), metrics))
    }
}

/// Benchmark for corpus hashing and compression.
///
/// Measures the performance of hashing and compressing evaluation corpus data,
/// which is essential for data integrity and storage optimization.
pub struct CorpusHashingBenchmark {
    /// Size of corpus data in bytes to simulate
    corpus_size: usize,
}

impl CorpusHashingBenchmark {
    /// Creates a new corpus hashing benchmark with default parameters.
    pub fn new() -> Self {
        Self {
            corpus_size: 1024 * 1024, // 1 MB
        }
    }

    /// Creates a new corpus hashing benchmark with custom corpus size.
    pub fn with_size(corpus_size: usize) -> Self {
        Self { corpus_size }
    }
}

impl Default for CorpusHashingBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BenchTarget for CorpusHashingBenchmark {
    fn id(&self) -> &'static str {
        "corpus-hashing"
    }

    fn description(&self) -> &'static str {
        "Measures corpus hashing and checksum computation speed"
    }

    fn category(&self) -> &'static str {
        "processing"
    }

    async fn run(&self) -> anyhow::Result<BenchmarkResult> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let start = Instant::now();

        // Generate simulated corpus data
        let corpus_data: Vec<u8> = (0..self.corpus_size)
            .map(|i| (i % 256) as u8)
            .collect();

        let generation_time = start.elapsed();

        // Compute hash
        let hash_start = Instant::now();
        let mut hasher = DefaultHasher::new();
        corpus_data.hash(&mut hasher);
        let hash = hasher.finish();
        let hash_time = hash_start.elapsed();

        // Simulate checksum computation (simple sum)
        let checksum_start = Instant::now();
        let checksum: u64 = corpus_data.iter().map(|&b| b as u64).sum();
        let checksum_time = checksum_start.elapsed();

        let total_duration = start.elapsed();
        let duration_ms = total_duration.as_secs_f64() * 1000.0;
        let throughput_mb_s = (self.corpus_size as f64 / (1024.0 * 1024.0)) / total_duration.as_secs_f64();

        let metrics = json!({
            "duration_ms": duration_ms,
            "corpus_size_bytes": self.corpus_size,
            "generation_time_ms": generation_time.as_secs_f64() * 1000.0,
            "hash_time_ms": hash_time.as_secs_f64() * 1000.0,
            "checksum_time_ms": checksum_time.as_secs_f64() * 1000.0,
            "throughput_mb_s": throughput_mb_s,
            "hash_value": format!("{:016x}", hash),
            "checksum_value": checksum
        });

        Ok(BenchmarkResult::new(self.id().to_string(), metrics))
    }
}

/// Benchmark for metadata aggregation.
///
/// Measures the performance of aggregating benchmark metadata from multiple sources,
/// which is essential for search, filtering, and statistics computation.
pub struct MetadataAggregationBenchmark {
    /// Number of metadata records to aggregate
    record_count: usize,
}

impl MetadataAggregationBenchmark {
    /// Creates a new metadata aggregation benchmark with default parameters.
    pub fn new() -> Self {
        Self { record_count: 5000 }
    }

    /// Creates a new metadata aggregation benchmark with custom record count.
    pub fn with_count(record_count: usize) -> Self {
        Self { record_count }
    }
}

impl Default for MetadataAggregationBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BenchTarget for MetadataAggregationBenchmark {
    fn id(&self) -> &'static str {
        "metadata-aggregation"
    }

    fn description(&self) -> &'static str {
        "Measures metadata aggregation and statistics computation speed"
    }

    fn category(&self) -> &'static str {
        "processing"
    }

    async fn run(&self) -> anyhow::Result<BenchmarkResult> {
        let start = Instant::now();

        // Generate simulated metadata records
        let records: Vec<serde_json::Value> = (0..self.record_count)
            .map(|i| {
                json!({
                    "id": format!("record-{}", i),
                    "benchmark_id": format!("benchmark-{}", i % 100),
                    "model": format!("model-{}", i % 50),
                    "score": (i % 100) as f64 / 100.0,
                    "latency_ms": 50.0 + (i % 200) as f64,
                    "timestamp": "2024-01-01T00:00:00Z",
                    "tags": vec!["tag1", "tag2", "tag3"],
                    "category": ["performance", "accuracy", "reliability"][i % 3]
                })
            })
            .collect();

        let generation_time = start.elapsed();

        // Aggregate statistics
        let agg_start = Instant::now();

        let mut total_score = 0.0f64;
        let mut total_latency = 0.0f64;
        let mut model_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let mut category_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

        for record in &records {
            if let Some(score) = record["score"].as_f64() {
                total_score += score;
            }
            if let Some(latency) = record["latency_ms"].as_f64() {
                total_latency += latency;
            }
            if let Some(model) = record["model"].as_str() {
                *model_counts.entry(model.to_string()).or_insert(0) += 1;
            }
            if let Some(category) = record["category"].as_str() {
                *category_counts.entry(category.to_string()).or_insert(0) += 1;
            }
        }

        let avg_score = total_score / self.record_count as f64;
        let avg_latency = total_latency / self.record_count as f64;
        let unique_models = model_counts.len();
        let unique_categories = category_counts.len();

        let aggregation_time = agg_start.elapsed();

        let total_duration = start.elapsed();
        let duration_ms = total_duration.as_secs_f64() * 1000.0;
        let records_per_second = self.record_count as f64 / total_duration.as_secs_f64();

        let metrics = json!({
            "duration_ms": duration_ms,
            "record_count": self.record_count,
            "generation_time_ms": generation_time.as_secs_f64() * 1000.0,
            "aggregation_time_ms": aggregation_time.as_secs_f64() * 1000.0,
            "records_per_second": records_per_second,
            "avg_score": avg_score,
            "avg_latency_ms": avg_latency,
            "unique_models": unique_models,
            "unique_categories": unique_categories
        });

        Ok(BenchmarkResult::new(self.id().to_string(), metrics))
    }
}

/// Benchmark for leaderboard recomputation latency.
///
/// Measures the performance of recomputing leaderboard rankings,
/// which is critical for maintaining up-to-date benchmark standings.
pub struct LeaderboardRecomputationBenchmark {
    /// Number of entries in the leaderboard
    entry_count: usize,
}

impl LeaderboardRecomputationBenchmark {
    /// Creates a new leaderboard recomputation benchmark with default parameters.
    pub fn new() -> Self {
        Self { entry_count: 10000 }
    }

    /// Creates a new leaderboard recomputation benchmark with custom entry count.
    pub fn with_count(entry_count: usize) -> Self {
        Self { entry_count }
    }
}

impl Default for LeaderboardRecomputationBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BenchTarget for LeaderboardRecomputationBenchmark {
    fn id(&self) -> &'static str {
        "leaderboard-recomputation"
    }

    fn description(&self) -> &'static str {
        "Measures leaderboard ranking recomputation latency"
    }

    fn category(&self) -> &'static str {
        "ranking"
    }

    async fn run(&self) -> anyhow::Result<BenchmarkResult> {
        let start = Instant::now();

        // Generate simulated leaderboard entries
        let mut entries: Vec<(String, f64, f64, u64)> = (0..self.entry_count)
            .map(|i| {
                let model_id = format!("model-{}", i);
                let primary_score = 0.5 + (i as f64 * 0.0001) % 0.5;
                let secondary_score = 100.0 - (i % 100) as f64;
                let submission_time = 1700000000u64 + (i as u64 * 1000);
                (model_id, primary_score, secondary_score, submission_time)
            })
            .collect();

        let generation_time = start.elapsed();

        // Sort by primary score (descending), then by secondary (descending), then by time (ascending)
        let sort_start = Instant::now();
        entries.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal))
                .then_with(|| a.3.cmp(&b.3))
        });
        let sort_time = sort_start.elapsed();

        // Assign rankings
        let rank_start = Instant::now();
        let rankings: Vec<(usize, String, f64)> = entries
            .iter()
            .enumerate()
            .map(|(i, (model, score, _, _))| (i + 1, model.clone(), *score))
            .collect();
        let rank_time = rank_start.elapsed();

        // Compute statistics
        let stats_start = Instant::now();
        let scores: Vec<f64> = entries.iter().map(|(_, s, _, _)| *s).collect();
        let mean_score: f64 = scores.iter().sum::<f64>() / scores.len() as f64;
        let variance: f64 = scores.iter().map(|s| (s - mean_score).powi(2)).sum::<f64>() / scores.len() as f64;
        let std_dev = variance.sqrt();

        // Percentile calculation
        let p50_idx = scores.len() / 2;
        let p90_idx = (scores.len() as f64 * 0.9) as usize;
        let p99_idx = (scores.len() as f64 * 0.99) as usize;

        let mut sorted_scores = scores.clone();
        sorted_scores.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let p50 = sorted_scores.get(p50_idx).copied().unwrap_or(0.0);
        let p90 = sorted_scores.get(p90_idx).copied().unwrap_or(0.0);
        let p99 = sorted_scores.get(p99_idx).copied().unwrap_or(0.0);

        let stats_time = stats_start.elapsed();

        let total_duration = start.elapsed();
        let duration_ms = total_duration.as_secs_f64() * 1000.0;
        let entries_per_second = self.entry_count as f64 / total_duration.as_secs_f64();

        let metrics = json!({
            "duration_ms": duration_ms,
            "entry_count": self.entry_count,
            "generation_time_ms": generation_time.as_secs_f64() * 1000.0,
            "sort_time_ms": sort_time.as_secs_f64() * 1000.0,
            "rank_time_ms": rank_time.as_secs_f64() * 1000.0,
            "stats_time_ms": stats_time.as_secs_f64() * 1000.0,
            "entries_per_second": entries_per_second,
            "top_entry": rankings.first().map(|(r, m, s)| json!({"rank": r, "model": m, "score": s})),
            "statistics": {
                "mean": mean_score,
                "std_dev": std_dev,
                "p50": p50,
                "p90": p90,
                "p99": p99
            }
        });

        Ok(BenchmarkResult::new(self.id().to_string(), metrics))
    }
}

/// Benchmark for crowd-sourced results validation.
///
/// Measures the performance of validating benchmark results from multiple sources,
/// which is essential for ensuring data quality in a decentralized system.
pub struct ResultsValidationBenchmark {
    /// Number of results to validate
    result_count: usize,
}

impl ResultsValidationBenchmark {
    /// Creates a new results validation benchmark with default parameters.
    pub fn new() -> Self {
        Self { result_count: 2000 }
    }

    /// Creates a new results validation benchmark with custom result count.
    pub fn with_count(result_count: usize) -> Self {
        Self { result_count }
    }
}

impl Default for ResultsValidationBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BenchTarget for ResultsValidationBenchmark {
    fn id(&self) -> &'static str {
        "results-validation"
    }

    fn description(&self) -> &'static str {
        "Measures crowd-sourced results validation speed"
    }

    fn category(&self) -> &'static str {
        "validation"
    }

    async fn run(&self) -> anyhow::Result<BenchmarkResult> {
        let start = Instant::now();

        // Generate simulated results to validate
        let results: Vec<serde_json::Value> = (0..self.result_count)
            .map(|i| {
                json!({
                    "submission_id": format!("sub-{}", i),
                    "benchmark_id": format!("bench-{}", i % 50),
                    "model_id": format!("model-{}", i % 100),
                    "score": (i % 100) as f64 / 100.0,
                    "test_results": (0..10).map(|j| json!({
                        "test_id": format!("test-{}", j),
                        "passed": (i + j) % 3 != 0,
                        "output": format!("Result for test {}", j),
                        "latency_ms": 50.0 + (j % 50) as f64
                    })).collect::<Vec<_>>(),
                    "metadata": {
                        "submitter": format!("user-{}", i % 20),
                        "timestamp": "2024-01-01T00:00:00Z",
                        "environment": {
                            "os": "linux",
                            "hardware": "cpu"
                        }
                    }
                })
            })
            .collect();

        let generation_time = start.elapsed();

        // Validate results
        let validation_start = Instant::now();

        let mut valid_count = 0usize;
        let mut invalid_count = 0usize;
        let mut validation_errors: Vec<String> = Vec::new();

        for result in &results {
            let mut is_valid = true;

            // Check required fields
            if result["submission_id"].is_null() {
                is_valid = false;
                validation_errors.push("Missing submission_id".to_string());
            }

            // Validate score range
            if let Some(score) = result["score"].as_f64() {
                if !(0.0..=1.0).contains(&score) {
                    is_valid = false;
                    validation_errors.push(format!("Invalid score: {}", score));
                }
            } else {
                is_valid = false;
                validation_errors.push("Missing or invalid score".to_string());
            }

            // Validate test results
            if let Some(test_results) = result["test_results"].as_array() {
                for test_result in test_results {
                    if test_result["test_id"].is_null() {
                        is_valid = false;
                        validation_errors.push("Missing test_id in test results".to_string());
                    }
                }
            }

            // Check for anomalies
            if let Some(score) = result["score"].as_f64() {
                if score == 1.0 {
                    // Perfect score might be suspicious
                    validation_errors.push("Perfect score - flagged for review".to_string());
                }
            }

            if is_valid {
                valid_count += 1;
            } else {
                invalid_count += 1;
            }
        }

        let validation_time = validation_start.elapsed();

        // Compute validation rate
        let total_duration = start.elapsed();
        let duration_ms = total_duration.as_secs_f64() * 1000.0;
        let validations_per_second = self.result_count as f64 / total_duration.as_secs_f64();
        let validation_rate = valid_count as f64 / self.result_count as f64;

        let metrics = json!({
            "duration_ms": duration_ms,
            "result_count": self.result_count,
            "generation_time_ms": generation_time.as_secs_f64() * 1000.0,
            "validation_time_ms": validation_time.as_secs_f64() * 1000.0,
            "validations_per_second": validations_per_second,
            "valid_count": valid_count,
            "invalid_count": invalid_count,
            "validation_rate": validation_rate,
            "unique_error_types": validation_errors.iter().collect::<std::collections::HashSet<_>>().len()
        });

        Ok(BenchmarkResult::new(self.id().to_string(), metrics))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_test_suite_ingestion_benchmark() {
        let benchmark = TestSuiteIngestionBenchmark::with_count(100);
        let result = benchmark.run().await.unwrap();
        assert_eq!(result.target_id, "test-suite-ingestion");
        assert!(result.metrics["items_processed"].as_u64().unwrap() == 100);
    }

    #[tokio::test]
    async fn test_corpus_hashing_benchmark() {
        let benchmark = CorpusHashingBenchmark::with_size(10240);
        let result = benchmark.run().await.unwrap();
        assert_eq!(result.target_id, "corpus-hashing");
        assert!(result.metrics["corpus_size_bytes"].as_u64().unwrap() == 10240);
    }

    #[tokio::test]
    async fn test_metadata_aggregation_benchmark() {
        let benchmark = MetadataAggregationBenchmark::with_count(100);
        let result = benchmark.run().await.unwrap();
        assert_eq!(result.target_id, "metadata-aggregation");
        assert!(result.metrics["record_count"].as_u64().unwrap() == 100);
    }

    #[tokio::test]
    async fn test_leaderboard_recomputation_benchmark() {
        let benchmark = LeaderboardRecomputationBenchmark::with_count(100);
        let result = benchmark.run().await.unwrap();
        assert_eq!(result.target_id, "leaderboard-recomputation");
        assert!(result.metrics["entry_count"].as_u64().unwrap() == 100);
    }

    #[tokio::test]
    async fn test_results_validation_benchmark() {
        let benchmark = ResultsValidationBenchmark::with_count(100);
        let result = benchmark.run().await.unwrap();
        assert_eq!(result.target_id, "results-validation");
        assert!(result.metrics["result_count"].as_u64().unwrap() == 100);
    }
}
