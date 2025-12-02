//! Run benchmark commands
//!
//! This module provides CLI commands for running the canonical benchmark suite.

use std::path::PathBuf;

use anyhow::Result;
use colored::Colorize;

use llm_benchmark_benchmarks::{
    all_targets, get_target, io, markdown, run_all_benchmarks, run_benchmark,
};

/// List all available benchmark targets
pub async fn list() -> Result<()> {
    println!("{}", "Available Benchmark Targets".bold().cyan());
    println!("{}", "=".repeat(60));
    println!();

    let targets = all_targets();

    for target in &targets {
        println!(
            "  {} {}",
            target.id().bold().green(),
            format!("({})", target.category()).dimmed()
        );
        println!("    {}", target.description());
        println!();
    }

    println!(
        "Total: {} benchmark targets available",
        targets.len().to_string().bold()
    );

    Ok(())
}

/// Run all benchmarks
pub async fn run_all(output_dir: Option<PathBuf>, json: bool) -> Result<()> {
    let base_path = output_dir.as_deref();

    println!("{}", "Running All Benchmarks".bold().cyan());
    println!("{}", "=".repeat(60));
    println!();

    // Ensure output directories exist
    io::ensure_output_dirs(base_path)?;

    let targets = all_targets();
    let total = targets.len();

    println!("Found {} benchmark targets to run\n", total);

    let mut results = Vec::with_capacity(total);
    let mut successes = 0;
    let mut failures = 0;

    for (i, target) in targets.iter().enumerate() {
        print!(
            "[{}/{}] Running {} ... ",
            i + 1,
            total,
            target.id().bold()
        );

        match target.run().await {
            Ok(result) => {
                println!("{}", "OK".green().bold());

                if let Some(duration) = result.metrics.get("duration_ms") {
                    println!(
                        "       Duration: {:.2}ms",
                        duration.as_f64().unwrap_or(0.0)
                    );
                }

                results.push(result);
                successes += 1;
            }
            Err(e) => {
                println!("{}", "FAILED".red().bold());
                println!("       Error: {}", e);
                failures += 1;
            }
        }
    }

    println!();
    println!("{}", "=".repeat(60));
    println!(
        "Results: {} passed, {} failed",
        successes.to_string().green().bold(),
        if failures > 0 {
            failures.to_string().red().bold()
        } else {
            failures.to_string().dimmed()
        }
    );

    if !results.is_empty() {
        // Write results
        io::write_results(&results, base_path)?;
        let combined_path = io::write_combined_results(&results, base_path)?;
        let summary_path = markdown::write_summary(&results, base_path)?;

        println!();
        println!("{}", "Output files:".bold());
        println!("  Combined results: {}", combined_path.display());
        println!("  Summary: {}", summary_path.display());
        println!(
            "  Raw results: {}",
            base_path
                .unwrap_or(std::path::Path::new("."))
                .join(io::RAW_OUTPUT_DIR)
                .display()
        );

        if json {
            println!();
            println!("{}", "JSON Results:".bold());
            println!("{}", serde_json::to_string_pretty(&results)?);
        }
    }

    if failures > 0 {
        anyhow::bail!("{} benchmark(s) failed", failures);
    }

    Ok(())
}

/// Run a specific benchmark by ID
pub async fn run_single(target_id: String, output_dir: Option<PathBuf>, json: bool) -> Result<()> {
    let base_path = output_dir.as_deref();

    println!(
        "{} {}",
        "Running Benchmark:".bold().cyan(),
        target_id.bold()
    );
    println!("{}", "=".repeat(60));
    println!();

    let target = get_target(&target_id)
        .ok_or_else(|| anyhow::anyhow!("Benchmark target not found: {}", target_id))?;

    println!("Target: {}", target.id().bold());
    println!("Description: {}", target.description());
    println!("Category: {}", target.category());
    println!();

    print!("Running ... ");

    let result = target.run().await?;

    println!("{}", "OK".green().bold());
    println!();

    // Display metrics
    println!("{}", "Metrics:".bold());
    println!("{}", serde_json::to_string_pretty(&result.metrics)?);
    println!();

    // Write result if output directory specified
    if let Some(ref base) = base_path {
        io::ensure_output_dirs(Some(base))?;
        let path = io::write_result(&result, Some(base))?;
        println!("Result written to: {}", path.display());
    }

    if json {
        println!();
        println!("{}", "Full Result JSON:".bold());
        println!("{}", serde_json::to_string_pretty(&result)?);
    }

    Ok(())
}

/// Show benchmark results summary
pub async fn show_summary(output_dir: Option<PathBuf>) -> Result<()> {
    let base_path = output_dir.as_deref();

    let results = io::read_all_results(base_path)?;

    if results.is_empty() {
        println!("{}", "No benchmark results found.".yellow());
        println!("Run 'llm-benchmark run all' to execute benchmarks.");
        return Ok(());
    }

    let summary = markdown::generate_summary(&results);
    println!("{}", summary);

    Ok(())
}
