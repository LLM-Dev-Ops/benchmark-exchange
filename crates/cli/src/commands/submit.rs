//! Submission management commands

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::commands::CommandContext;
use crate::interactive::{confirm_default_yes, spinner};
use crate::output::{colors, TableFormatter};

#[derive(Debug, Serialize, Deserialize)]
pub struct Submission {
    pub id: String,
    pub benchmark_id: String,
    pub model_name: String,
    pub model_version: String,
    pub submitter_id: String,
    pub status: String,
    pub submitted_at: String,
    pub verified: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmissionList {
    pub submissions: Vec<Submission>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct SubmitResultsRequest {
    pub benchmark_id: String,
    pub model_name: String,
    pub model_version: String,
    pub results: serde_json::Value,
    pub metadata: Option<serde_json::Value>,
}

/// Submit results to a benchmark
pub async fn submit(
    ctx: &CommandContext,
    benchmark_id: String,
    results_file: String,
    model_name: String,
    model_version: String,
) -> Result<()> {
    ctx.require_auth()?;

    let path = Path::new(&results_file);
    if !path.exists() {
        anyhow::bail!("Results file not found: {}", results_file);
    }

    println!("{}", colors::info("Reading results file..."));

    let content = fs::read_to_string(path)
        .context("Failed to read results file")?;

    let results: serde_json::Value = if results_file.ends_with(".yaml")
        || results_file.ends_with(".yml")
    {
        let yaml: serde_yaml::Value = serde_yaml::from_str(&content)
            .context("Failed to parse YAML")?;
        serde_json::to_value(yaml)?
    } else {
        serde_json::from_str(&content).context("Failed to parse JSON")?
    };

    println!("{}", colors::bold("Submitting results:"));
    println!("  Benchmark: {}", benchmark_id);
    println!("  Model:     {} ({})", model_name, model_version);
    println!();

    let confirmed = confirm_default_yes("Submit these results?")?;
    if !confirmed {
        println!("Cancelled.");
        return Ok(());
    }

    let sp = spinner("Submitting results...");

    let request = SubmitResultsRequest {
        benchmark_id: benchmark_id.clone(),
        model_name,
        model_version,
        results,
        metadata: None,
    };

    let submission: Submission = ctx
        .client
        .post("/api/v1/submissions", &request)
        .await?;

    sp.finish_and_clear();

    println!("{}", colors::success("Results submitted successfully!"));
    println!("Submission ID: {}", submission.id);
    println!("Status: {}", submission.status);

    Ok(())
}

/// Show submission details
pub async fn show(ctx: &CommandContext, submission_id: String) -> Result<()> {
    let sp = spinner("Fetching submission details...");

    let submission: Submission = ctx
        .client
        .get(&format!("/api/v1/submissions/{}", submission_id))
        .await?;

    sp.finish_and_clear();

    let items = vec![
        ("ID", submission.id),
        ("Benchmark ID", submission.benchmark_id),
        ("Model", submission.model_name),
        ("Version", submission.model_version),
        ("Submitter ID", submission.submitter_id),
        ("Status", submission.status),
        ("Verified", submission.verified.to_string()),
        ("Submitted At", submission.submitted_at),
    ];

    let table = TableFormatter::key_value(items)?;
    println!("{}", table);

    Ok(())
}

/// List submissions with optional filters
pub async fn list(ctx: &CommandContext, benchmark_id: Option<String>) -> Result<()> {
    let sp = spinner("Fetching submissions...");

    let path = if let Some(id) = benchmark_id {
        format!("/api/v1/submissions?benchmark_id={}", id)
    } else {
        "/api/v1/submissions".to_string()
    };

    let list: SubmissionList = ctx.client.get(&path).await?;

    sp.finish_and_clear();

    if list.submissions.is_empty() {
        println!("{}", colors::warning("No submissions found."));
        return Ok(());
    }

    let headers = vec![
        "ID",
        "Benchmark",
        "Model",
        "Version",
        "Status",
        "Verified",
        "Submitted",
    ];
    let rows: Vec<Vec<String>> = list
        .submissions
        .iter()
        .map(|s| {
            vec![
                s.id.clone(),
                s.benchmark_id.clone(),
                s.model_name.clone(),
                s.model_version.clone(),
                s.status.clone(),
                if s.verified { "Yes" } else { "No" }.to_string(),
                s.submitted_at.clone(),
            ]
        })
        .collect();

    let table = TableFormatter::simple(headers, rows)?;
    println!("{}", table);
    println!("{} submissions found", colors::dim(&list.total.to_string()));

    Ok(())
}

/// Request verification for a submission
pub async fn request_verification(ctx: &CommandContext, submission_id: String) -> Result<()> {
    ctx.require_auth()?;

    let confirmed = confirm_default_yes("Request verification for this submission?")?;
    if !confirmed {
        println!("Cancelled.");
        return Ok(());
    }

    let sp = spinner("Requesting verification...");

    let _: serde_json::Value = ctx
        .client
        .post(
            &format!("/api/v1/submissions/{}/verify", submission_id),
            &(),
        )
        .await?;

    sp.finish_and_clear();

    println!("{}", colors::success("Verification requested!"));
    println!(
        "Your submission will be reviewed and verified by the community."
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_submission_serialization() {
        let submission = Submission {
            id: "sub-123".to_string(),
            benchmark_id: "bench-456".to_string(),
            model_name: "GPT-4".to_string(),
            model_version: "2024-01".to_string(),
            submitter_id: "user-789".to_string(),
            status: "pending".to_string(),
            verified: false,
            submitted_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&submission).unwrap();
        assert!(json.contains("sub-123"));
    }
}
