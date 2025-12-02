//! LLM Benchmark Exchange Canonical Benchmark Interface
//!
//! This crate provides the canonical benchmark interface for the LLM Benchmark Exchange
//! platform. It defines standardized types, traits, and utilities for running and
//! reporting benchmarks.
//!
//! ## Architecture
//!
//! The benchmark interface is organized into the following modules:
//!
//! - **result**: The canonical `BenchmarkResult` struct with `target_id`, `metrics`, and `timestamp` fields
//! - **io**: I/O operations for reading and writing benchmark results
//! - **markdown**: Markdown generation for benchmark reports
//! - **adapters**: The `BenchTarget` trait and target registry
//!
//! ## Usage
//!
//! ```rust,no_run
//! use llm_benchmark_benchmarks::{run_all_benchmarks, adapters::all_targets};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // List all available benchmark targets
//!     for target in all_targets() {
//!         println!("Available: {} - {}", target.id(), target.description());
//!     }
//!
//!     // Run all benchmarks
//!     let results = run_all_benchmarks().await?;
//!     println!("Executed {} benchmarks", results.len());
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Canonical Files
//!
//! This crate is organized according to the canonical benchmark interface:
//!
//! - `benchmarks/mod.rs` - Main module exports
//! - `benchmarks/result.rs` - BenchmarkResult struct
//! - `benchmarks/io.rs` - I/O operations
//! - `benchmarks/markdown.rs` - Markdown generation
//! - `benchmarks/adapters/mod.rs` - BenchTarget trait and registry

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod result;
pub mod io;
pub mod markdown;
pub mod adapters;

use anyhow::Result;

pub use result::BenchmarkResult;
pub use adapters::{BenchTarget, all_targets, get_target, target_ids};

/// Runs all registered benchmark targets and returns their results.
///
/// This is the canonical entrypoint for the benchmark suite. It executes
/// all registered benchmark targets in sequence and collects their results.
///
/// # Returns
///
/// A vector of `BenchmarkResult` containing the results from all benchmark targets.
///
/// # Errors
///
/// Returns an error if any benchmark target fails to execute.
///
/// # Example
///
/// ```rust,no_run
/// use llm_benchmark_benchmarks::run_all_benchmarks;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let results = run_all_benchmarks().await?;
///
///     for result in &results {
///         println!("{}: {:?}", result.target_id, result.metrics);
///     }
///
///     Ok(())
/// }
/// ```
pub async fn run_all_benchmarks() -> Result<Vec<BenchmarkResult>> {
    let targets = all_targets();
    let mut results = Vec::with_capacity(targets.len());

    for target in targets {
        let result = target.run().await?;
        results.push(result);
    }

    Ok(results)
}

/// Runs a specific benchmark target by ID.
///
/// # Arguments
///
/// * `target_id` - The unique identifier of the benchmark target to run
///
/// # Returns
///
/// The `BenchmarkResult` if the target was found and executed successfully.
///
/// # Errors
///
/// Returns an error if the target is not found or fails to execute.
///
/// # Example
///
/// ```rust,no_run
/// use llm_benchmark_benchmarks::run_benchmark;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let result = run_benchmark("test-suite-ingestion").await?;
///     println!("Duration: {:?}", result.metrics["duration_ms"]);
///     Ok(())
/// }
/// ```
pub async fn run_benchmark(target_id: &str) -> Result<BenchmarkResult> {
    let target = get_target(target_id)
        .ok_or_else(|| anyhow::anyhow!("Benchmark target not found: {}", target_id))?;

    target.run().await
}

/// Runs all benchmarks and writes results to the canonical output directories.
///
/// This function:
/// 1. Runs all registered benchmark targets
/// 2. Writes individual results to `benchmarks/output/raw/`
/// 3. Writes a combined JSON file to `benchmarks/output/`
/// 4. Generates and writes a markdown summary to `benchmarks/output/summary.md`
///
/// # Arguments
///
/// * `base_path` - Optional base path for output (defaults to current directory)
///
/// # Returns
///
/// The vector of `BenchmarkResult` on success.
///
/// # Errors
///
/// Returns an error if benchmarks fail to execute or results fail to write.
///
/// # Example
///
/// ```rust,no_run
/// use llm_benchmark_benchmarks::run_all_benchmarks_with_output;
/// use std::path::Path;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let results = run_all_benchmarks_with_output(Some(Path::new("."))).await?;
///     println!("Completed {} benchmarks", results.len());
///     Ok(())
/// }
/// ```
pub async fn run_all_benchmarks_with_output(base_path: Option<&std::path::Path>) -> Result<Vec<BenchmarkResult>> {
    // Ensure output directories exist
    io::ensure_output_dirs(base_path)?;

    // Run all benchmarks
    let results = run_all_benchmarks().await?;

    // Write individual results
    io::write_results(&results, base_path)?;

    // Write combined results
    io::write_combined_results(&results, base_path)?;

    // Generate and write summary
    markdown::write_summary(&results, base_path)?;

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_all_benchmarks() {
        let results = run_all_benchmarks().await.unwrap();
        assert!(!results.is_empty(), "Should return at least one result");

        for result in &results {
            assert!(!result.target_id.is_empty(), "Target ID should not be empty");
            assert!(!result.metrics.is_null(), "Metrics should not be null");
        }
    }

    #[tokio::test]
    async fn test_run_benchmark() {
        let result = run_benchmark("test-suite-ingestion").await.unwrap();
        assert_eq!(result.target_id, "test-suite-ingestion");
    }

    #[tokio::test]
    async fn test_run_nonexistent_benchmark() {
        let result = run_benchmark("nonexistent-benchmark").await;
        assert!(result.is_err());
    }
}
