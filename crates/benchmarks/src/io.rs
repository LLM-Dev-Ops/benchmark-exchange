//! I/O operations for benchmark results.
//!
//! This module provides functions for reading and writing benchmark results
//! to the canonical output directories.

use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;

use crate::result::BenchmarkResult;

/// Default output directory for benchmark results.
pub const DEFAULT_OUTPUT_DIR: &str = "benchmarks/output";

/// Subdirectory for raw benchmark results.
pub const RAW_OUTPUT_DIR: &str = "benchmarks/output/raw";

/// Default summary file name.
pub const SUMMARY_FILE: &str = "benchmarks/output/summary.md";

/// Writes a single benchmark result to the raw output directory.
///
/// # Arguments
///
/// * `result` - The benchmark result to write
/// * `base_path` - Optional base path (defaults to current directory)
///
/// # Returns
///
/// The path to the written file on success.
pub fn write_result(result: &BenchmarkResult, base_path: Option<&Path>) -> Result<PathBuf> {
    let base = base_path.unwrap_or(Path::new("."));
    let raw_dir = base.join(RAW_OUTPUT_DIR);

    // Ensure the directory exists
    fs::create_dir_all(&raw_dir)
        .with_context(|| format!("Failed to create directory: {}", raw_dir.display()))?;

    // Generate filename with timestamp
    let timestamp = result.timestamp.format("%Y%m%d_%H%M%S");
    let filename = format!("{}_{}.json", result.target_id, timestamp);
    let file_path = raw_dir.join(&filename);

    // Write the result
    let file = File::create(&file_path)
        .with_context(|| format!("Failed to create file: {}", file_path.display()))?;
    let mut writer = BufWriter::new(file);

    serde_json::to_writer_pretty(&mut writer, result)
        .with_context(|| "Failed to serialize benchmark result")?;

    writer.flush()?;

    Ok(file_path)
}

/// Writes multiple benchmark results to the raw output directory.
///
/// # Arguments
///
/// * `results` - The benchmark results to write
/// * `base_path` - Optional base path (defaults to current directory)
///
/// # Returns
///
/// A vector of paths to the written files on success.
pub fn write_results(results: &[BenchmarkResult], base_path: Option<&Path>) -> Result<Vec<PathBuf>> {
    results
        .iter()
        .map(|r| write_result(r, base_path))
        .collect()
}

/// Writes all benchmark results to a single combined JSON file.
///
/// # Arguments
///
/// * `results` - The benchmark results to write
/// * `base_path` - Optional base path (defaults to current directory)
///
/// # Returns
///
/// The path to the written file on success.
pub fn write_combined_results(results: &[BenchmarkResult], base_path: Option<&Path>) -> Result<PathBuf> {
    let base = base_path.unwrap_or(Path::new("."));
    let output_dir = base.join(DEFAULT_OUTPUT_DIR);

    // Ensure the directory exists
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("Failed to create directory: {}", output_dir.display()))?;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("benchmark_results_{}.json", timestamp);
    let file_path = output_dir.join(&filename);

    let file = File::create(&file_path)
        .with_context(|| format!("Failed to create file: {}", file_path.display()))?;
    let mut writer = BufWriter::new(file);

    serde_json::to_writer_pretty(&mut writer, results)
        .with_context(|| "Failed to serialize benchmark results")?;

    writer.flush()?;

    Ok(file_path)
}

/// Reads a benchmark result from a JSON file.
///
/// # Arguments
///
/// * `path` - Path to the JSON file
///
/// # Returns
///
/// The deserialized `BenchmarkResult`.
pub fn read_result(path: &Path) -> Result<BenchmarkResult> {
    let file = File::open(path)
        .with_context(|| format!("Failed to open file: {}", path.display()))?;
    let reader = BufReader::new(file);

    serde_json::from_reader(reader)
        .with_context(|| format!("Failed to parse benchmark result from: {}", path.display()))
}

/// Reads all benchmark results from the raw output directory.
///
/// # Arguments
///
/// * `base_path` - Optional base path (defaults to current directory)
///
/// # Returns
///
/// A vector of all benchmark results found in the directory.
pub fn read_all_results(base_path: Option<&Path>) -> Result<Vec<BenchmarkResult>> {
    let base = base_path.unwrap_or(Path::new("."));
    let raw_dir = base.join(RAW_OUTPUT_DIR);

    if !raw_dir.exists() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();

    for entry in fs::read_dir(&raw_dir)
        .with_context(|| format!("Failed to read directory: {}", raw_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map_or(false, |ext| ext == "json") {
            match read_result(&path) {
                Ok(result) => results.push(result),
                Err(e) => {
                    eprintln!("Warning: Failed to read {}: {}", path.display(), e);
                }
            }
        }
    }

    // Sort by timestamp, newest first
    results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    Ok(results)
}

/// Ensures the canonical output directories exist.
///
/// # Arguments
///
/// * `base_path` - Optional base path (defaults to current directory)
///
/// # Returns
///
/// `Ok(())` if directories were created or already exist.
pub fn ensure_output_dirs(base_path: Option<&Path>) -> Result<()> {
    let base = base_path.unwrap_or(Path::new("."));

    let output_dir = base.join(DEFAULT_OUTPUT_DIR);
    let raw_dir = base.join(RAW_OUTPUT_DIR);

    fs::create_dir_all(&output_dir)
        .with_context(|| format!("Failed to create directory: {}", output_dir.display()))?;
    fs::create_dir_all(&raw_dir)
        .with_context(|| format!("Failed to create directory: {}", raw_dir.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    #[test]
    fn test_write_and_read_result() {
        let temp_dir = TempDir::new().unwrap();
        let result = BenchmarkResult::new(
            "test-target".to_string(),
            json!({"duration_ms": 100}),
        );

        let path = write_result(&result, Some(temp_dir.path())).unwrap();
        assert!(path.exists());

        let read_back = read_result(&path).unwrap();
        assert_eq!(result.target_id, read_back.target_id);
        assert_eq!(result.metrics, read_back.metrics);
    }

    #[test]
    fn test_ensure_output_dirs() {
        let temp_dir = TempDir::new().unwrap();
        ensure_output_dirs(Some(temp_dir.path())).unwrap();

        assert!(temp_dir.path().join(DEFAULT_OUTPUT_DIR).exists());
        assert!(temp_dir.path().join(RAW_OUTPUT_DIR).exists());
    }
}
