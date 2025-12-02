//! Adapters module for benchmark targets.
//!
//! This module defines the canonical `BenchTarget` trait and provides a registry
//! of all available benchmark targets through the `all_targets()` function.

mod targets;

use async_trait::async_trait;

use crate::result::BenchmarkResult;

/// Canonical trait for benchmark targets.
///
/// All benchmark targets must implement this trait to be included in the
/// benchmark suite. The trait provides a standardized interface for running
/// benchmarks and identifying targets.
///
/// # Example
///
/// ```rust
/// use async_trait::async_trait;
/// use llm_benchmark_benchmarks::adapters::BenchTarget;
/// use llm_benchmark_benchmarks::result::BenchmarkResult;
/// use serde_json::json;
///
/// struct MyBenchmark;
///
/// #[async_trait]
/// impl BenchTarget for MyBenchmark {
///     fn id(&self) -> &'static str {
///         "my-benchmark"
///     }
///
///     async fn run(&self) -> anyhow::Result<BenchmarkResult> {
///         // Perform benchmark operations
///         let metrics = json!({
///             "duration_ms": 100,
///             "operations": 1000
///         });
///
///         Ok(BenchmarkResult::new(self.id().to_string(), metrics))
///     }
/// }
/// ```
#[async_trait]
pub trait BenchTarget: Send + Sync {
    /// Returns the unique identifier for this benchmark target.
    ///
    /// The ID should be a kebab-case string that uniquely identifies
    /// the benchmark target (e.g., "test-suite-ingestion", "leaderboard-recomputation").
    fn id(&self) -> &'static str;

    /// Executes the benchmark and returns the result.
    ///
    /// This method should:
    /// 1. Perform the benchmark operations
    /// 2. Collect metrics (timing, throughput, etc.)
    /// 3. Return a `BenchmarkResult` with the collected metrics
    ///
    /// # Errors
    ///
    /// Returns an error if the benchmark fails to execute.
    async fn run(&self) -> anyhow::Result<BenchmarkResult>;

    /// Returns a human-readable description of this benchmark target.
    ///
    /// Default implementation returns the ID.
    fn description(&self) -> &'static str {
        self.id()
    }

    /// Returns the category of this benchmark target.
    ///
    /// Default implementation returns "general".
    fn category(&self) -> &'static str {
        "general"
    }
}

/// Returns a vector of all registered benchmark targets.
///
/// This function provides the canonical registry of benchmark targets
/// for the LLM Benchmark Exchange platform.
///
/// # Returns
///
/// A vector of boxed trait objects implementing `BenchTarget`.
///
/// # Example
///
/// ```rust
/// use llm_benchmark_benchmarks::adapters::all_targets;
///
/// #[tokio::main]
/// async fn main() {
///     let targets = all_targets();
///     println!("Available benchmarks: {}", targets.len());
///
///     for target in &targets {
///         println!("  - {}: {}", target.id(), target.description());
///     }
/// }
/// ```
pub fn all_targets() -> Vec<Box<dyn BenchTarget>> {
    vec![
        Box::new(targets::TestSuiteIngestionBenchmark::new()),
        Box::new(targets::CorpusHashingBenchmark::new()),
        Box::new(targets::MetadataAggregationBenchmark::new()),
        Box::new(targets::LeaderboardRecomputationBenchmark::new()),
        Box::new(targets::ResultsValidationBenchmark::new()),
    ]
}

/// Returns a specific benchmark target by ID.
///
/// # Arguments
///
/// * `id` - The unique identifier of the benchmark target
///
/// # Returns
///
/// `Some(target)` if found, `None` otherwise.
pub fn get_target(id: &str) -> Option<Box<dyn BenchTarget>> {
    all_targets().into_iter().find(|t| t.id() == id)
}

/// Returns the IDs of all registered benchmark targets.
///
/// # Returns
///
/// A vector of target IDs as static string slices.
pub fn target_ids() -> Vec<&'static str> {
    all_targets().iter().map(|t| t.id()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_targets_not_empty() {
        let targets = all_targets();
        assert!(!targets.is_empty(), "Should have at least one benchmark target");
    }

    #[test]
    fn test_unique_target_ids() {
        let ids = target_ids();
        let unique_ids: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(ids.len(), unique_ids.len(), "Target IDs should be unique");
    }

    #[test]
    fn test_get_target() {
        let targets = all_targets();
        if let Some(first) = targets.first() {
            let found = get_target(first.id());
            assert!(found.is_some());
            assert_eq!(found.unwrap().id(), first.id());
        }
    }

    #[test]
    fn test_get_nonexistent_target() {
        let found = get_target("nonexistent-target");
        assert!(found.is_none());
    }
}
