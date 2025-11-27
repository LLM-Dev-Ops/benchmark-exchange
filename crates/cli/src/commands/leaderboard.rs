//! Leaderboard commands

use anyhow::Result;
use colored::Colorize;
use serde::{Deserialize, Serialize};

use crate::commands::CommandContext;
use crate::interactive::spinner;
use crate::output::{colors, TableFormatter};

#[derive(Debug, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub rank: usize,
    pub model_name: String,
    pub model_version: String,
    pub score: f64,
    pub submission_id: String,
    pub submitted_at: String,
    pub verified: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Leaderboard {
    pub benchmark_id: String,
    pub benchmark_name: String,
    pub entries: Vec<LeaderboardEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelComparison {
    pub model1: ModelComparisonData,
    pub model2: ModelComparisonData,
    pub metrics: Vec<MetricComparison>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelComparisonData {
    pub name: String,
    pub version: String,
    pub overall_score: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricComparison {
    pub name: String,
    pub model1_value: f64,
    pub model2_value: f64,
    pub difference: f64,
}

/// Show leaderboard for a benchmark
pub async fn show(ctx: &CommandContext, benchmark_id: String) -> Result<()> {
    let sp = spinner("Fetching leaderboard...");

    let leaderboard: Leaderboard = ctx
        .client
        .get(&format!("/api/v1/leaderboards/{}", benchmark_id))
        .await?;

    sp.finish_and_clear();

    println!("{}", colors::bold(&format!("Leaderboard: {}", leaderboard.benchmark_name)));
    println!();

    if leaderboard.entries.is_empty() {
        println!("{}", colors::warning("No entries yet."));
        return Ok(());
    }

    let headers = vec!["Rank", "Model", "Version", "Score", "Verified", "Submitted"];
    let rows: Vec<Vec<String>> = leaderboard
        .entries
        .iter()
        .map(|e| {
            vec![
                format!("#{}", e.rank),
                e.model_name.clone(),
                e.model_version.clone(),
                format!("{:.4}", e.score),
                if e.verified { "✓" } else { "-" }.to_string(),
                e.submitted_at.clone(),
            ]
        })
        .collect();

    let table = TableFormatter::simple(headers, rows)?;
    println!("{}", table);

    Ok(())
}

/// Compare two models
pub async fn compare(
    ctx: &CommandContext,
    benchmark_id: String,
    model1: String,
    model2: String,
) -> Result<()> {
    let sp = spinner("Comparing models...");

    let comparison: ModelComparison = ctx
        .client
        .get(&format!(
            "/api/v1/leaderboards/{}/compare?model1={}&model2={}",
            benchmark_id, model1, model2
        ))
        .await?;

    sp.finish_and_clear();

    println!("{}", colors::bold("Model Comparison"));
    println!();

    // Overall scores
    println!("Model 1: {} ({})", comparison.model1.name, comparison.model1.version);
    println!("  Overall Score: {:.4}", comparison.model1.overall_score);
    println!();
    println!("Model 2: {} ({})", comparison.model2.name, comparison.model2.version);
    println!("  Overall Score: {:.4}", comparison.model2.overall_score);
    println!();

    // Metric breakdown
    if !comparison.metrics.is_empty() {
        let headers = vec!["Metric", "Model 1", "Model 2", "Difference"];
        let rows: Vec<Vec<String>> = comparison
            .metrics
            .iter()
            .map(|m| {
                let diff_str = if m.difference > 0.0 {
                    format!("+{:.4}", m.difference).green().to_string()
                } else if m.difference < 0.0 {
                    format!("{:.4}", m.difference).red().to_string()
                } else {
                    "0.0000".to_string()
                };

                vec![
                    m.name.clone(),
                    format!("{:.4}", m.model1_value),
                    format!("{:.4}", m.model2_value),
                    diff_str,
                ]
            })
            .collect();

        let table = TableFormatter::simple(headers, rows)?;
        println!("{}", table);
    }

    Ok(())
}

/// Export leaderboard data
pub async fn export(
    ctx: &CommandContext,
    benchmark_id: String,
    format: String,
    output_file: Option<String>,
) -> Result<()> {
    let sp = spinner("Fetching leaderboard data...");

    let leaderboard: Leaderboard = ctx
        .client
        .get(&format!("/api/v1/leaderboards/{}", benchmark_id))
        .await?;

    sp.finish_and_clear();

    let output = match format.to_lowercase().as_str() {
        "json" => serde_json::to_string_pretty(&leaderboard)?,
        "csv" => {
            let mut csv = String::new();
            csv.push_str("Rank,Model,Version,Score,Verified,Submitted At\n");
            for entry in &leaderboard.entries {
                csv.push_str(&format!(
                    "{},{},{},{},{},{}\n",
                    entry.rank,
                    entry.model_name,
                    entry.model_version,
                    entry.score,
                    entry.verified,
                    entry.submitted_at
                ));
            }
            csv
        }
        _ => anyhow::bail!("Unsupported format: {}. Use 'json' or 'csv'.", format),
    };

    if let Some(file) = output_file {
        std::fs::write(&file, output)?;
        println!("{}", colors::success(&format!("Exported to: {}", file)));
    } else {
        println!("{}", output);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leaderboard_entry_serialization() {
        let entry = LeaderboardEntry {
            rank: 1,
            model_name: "GPT-4".to_string(),
            model_version: "2024-01".to_string(),
            score: 0.95,
            submission_id: "sub-123".to_string(),
            submitted_at: "2024-01-01".to_string(),
            verified: true,
        };

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("GPT-4"));
    }
}
