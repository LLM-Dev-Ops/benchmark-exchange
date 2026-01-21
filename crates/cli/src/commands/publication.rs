//! Publication Agent CLI Commands
//!
//! This module provides CLI commands for the Benchmark Publication Agent.
//! All commands are designed to be CLI-invokable per the constitution.
//!
//! ## Agent Classification: BENCHMARK PUBLICATION
//!
//! ## CLI Operations:
//! - `publish` - Publish a new benchmark result
//! - `validate` - Validate a benchmark submission (without publishing)
//! - `inspect` - Inspect publication with full metadata
//! - `list` - List publications with filters
//! - `show` - Show publication details
//! - `update` - Update publication metadata
//! - `status` - Transition publication status
//!
//! ## What this agent MUST NOT do (via CLI or any other means):
//! - Execute benchmark workloads
//! - Trigger model execution
//! - Intercept runtime requests
//! - Apply optimizations automatically
//! - Enforce policies or rankings

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::commands::CommandContext;
use crate::interactive::{confirm_default_yes, spinner};
use crate::output::{colors, TableFormatter};

// =============================================================================
// Response Types
// =============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct Publication {
    pub id: String,
    pub benchmark_id: String,
    pub submission_id: Option<String>,
    pub status: String,
    pub version: String,
    pub model_provider: String,
    pub model_name: String,
    pub model_version: String,
    pub aggregate_score: f64,
    pub normalized_score: f64,
    pub confidence_level: String,
    pub reproducibility_score: f64,
    pub published_by: String,
    pub organization_id: Option<String>,
    pub tags: Vec<String>,
    pub is_latest: bool,
    pub created_at: String,
    pub updated_at: String,
    pub published_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PublicationList {
    pub items: Vec<Publication>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    pub passed: bool,
    pub score: f64,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationError {
    pub code: String,
    pub message: String,
    pub field: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub code: String,
    pub message: String,
    pub field: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FullPublication {
    pub id: String,
    pub benchmark_id: String,
    pub submission_id: Option<String>,
    pub status: String,
    pub version: String,
    pub model_provider: String,
    pub model_name: String,
    pub model_version: String,
    pub metrics: Metrics,
    pub confidence: Confidence,
    pub constraints: Constraints,
    pub published_by: String,
    pub organization_id: Option<String>,
    pub tags: Vec<String>,
    pub is_latest: bool,
    pub citation: Option<Citation>,
    pub created_at: String,
    pub updated_at: String,
    pub published_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Metrics {
    pub aggregate_score: f64,
    pub normalized_score: f64,
    pub metric_scores: HashMap<String, MetricValue>,
    pub percentile_rank: Option<f64>,
    pub z_score: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricValue {
    pub value: f64,
    pub normalized: f64,
    pub unit: Option<String>,
    pub higher_is_better: bool,
    pub range: Option<(f64, f64)>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Confidence {
    pub reproducibility_score: f64,
    pub sample_size: u32,
    pub variance: f64,
    pub std_dev: Option<f64>,
    pub reproduction_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Constraints {
    pub methodology: MethodologyConstraints,
    pub dataset_scope: DatasetScopeConstraints,
    pub model_version: ModelVersionConstraints,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MethodologyConstraints {
    pub framework: String,
    pub evaluation_method: String,
    pub prompt_template_hash: Option<String>,
    pub scoring_method: String,
    pub normalized: bool,
    pub normalization_method: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatasetScopeConstraints {
    pub dataset_id: String,
    pub dataset_version: String,
    pub subset: Option<String>,
    pub example_count: u32,
    pub split: String,
    pub publicly_available: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelVersionConstraints {
    pub provider: String,
    pub model_name: String,
    pub version: String,
    pub parameter_count: Option<u64>,
    pub quantization: Option<String>,
    pub context_window: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Citation {
    pub doi: Option<String>,
    pub arxiv_id: Option<String>,
    pub bibtex: Option<String>,
    pub plain_text: String,
}

// =============================================================================
// Request Types
// =============================================================================

#[derive(Debug, Serialize)]
pub struct PublishRequest {
    pub benchmark_id: String,
    pub submission_id: Option<String>,
    pub model_provider: String,
    pub model_name: String,
    pub model_version: String,
    pub aggregate_score: f64,
    pub metric_scores: HashMap<String, MetricScoreInput>,
    pub methodology: MethodologyInput,
    pub dataset: DatasetInput,
    pub sample_size: u32,
    pub variance: f64,
    pub reproduction_count: u32,
    pub tags: Vec<String>,
    pub citation: Option<CitationInput>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricScoreInput {
    pub value: f64,
    pub unit: Option<String>,
    #[serde(default = "default_true")]
    pub higher_is_better: bool,
    pub range: Option<(f64, f64)>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MethodologyInput {
    pub framework: String,
    pub evaluation_method: String,
    pub prompt_template_hash: Option<String>,
    pub scoring_method: String,
    #[serde(default)]
    pub normalized: bool,
    pub normalization_method: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatasetInput {
    pub dataset_id: String,
    pub dataset_version: String,
    pub subset: Option<String>,
    pub example_count: u32,
    pub split: String,
    #[serde(default)]
    pub publicly_available: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CitationInput {
    pub doi: Option<String>,
    pub arxiv_id: Option<String>,
    pub bibtex: Option<String>,
    pub plain_text: String,
}

#[derive(Debug, Serialize)]
pub struct ValidateRequest {
    pub benchmark_id: String,
    pub model_provider: String,
    pub model_name: String,
    pub aggregate_score: f64,
    pub methodology: MethodologyInput,
    pub dataset: DatasetInput,
}

#[derive(Debug, Serialize)]
pub struct UpdateRequest {
    pub tags: Option<Vec<String>>,
    pub citation: Option<CitationInput>,
}

#[derive(Debug, Serialize)]
pub struct StatusTransitionRequest {
    pub target_status: String,
    pub reason: Option<String>,
}

// =============================================================================
// CLI Commands
// =============================================================================

/// List publications with optional filters
pub async fn list(
    ctx: &CommandContext,
    benchmark_id: Option<String>,
    model_provider: Option<String>,
    model_name: Option<String>,
    status: Option<String>,
    min_confidence: Option<f64>,
) -> Result<()> {
    let sp = spinner("Fetching publications...");

    let mut path = "/api/v1/publications?".to_string();
    if let Some(id) = benchmark_id {
        path.push_str(&format!("benchmark_id={}&", id));
    }
    if let Some(provider) = model_provider {
        path.push_str(&format!("model_provider={}&", provider));
    }
    if let Some(name) = model_name {
        path.push_str(&format!("model_name={}&", name));
    }
    if let Some(st) = status {
        path.push_str(&format!("status={}&", st));
    }
    if let Some(conf) = min_confidence {
        path.push_str(&format!("min_confidence={}&", conf));
    }

    let list: PublicationList = ctx.client.get(&path).await?;

    sp.finish_and_clear();

    if list.items.is_empty() {
        println!("{}", colors::warning("No publications found."));
        return Ok(());
    }

    // Create table
    let headers = vec![
        "ID",
        "Model",
        "Score",
        "Normalized",
        "Confidence",
        "Status",
        "Latest",
    ];
    let rows: Vec<Vec<String>> = list
        .items
        .iter()
        .map(|p| {
            vec![
                p.id[..8].to_string(), // Truncate ID for display
                format!("{}/{}", p.model_provider, p.model_name),
                format!("{:.2}", p.aggregate_score),
                format!("{:.3}", p.normalized_score),
                p.confidence_level.clone(),
                p.status.clone(),
                if p.is_latest { "Yes" } else { "No" }.to_string(),
            ]
        })
        .collect();

    let table = TableFormatter::simple(headers, rows)?;
    println!("{}", table);
    println!(
        "{} publications found (page {}/{})",
        colors::dim(&list.total.to_string()),
        list.page,
        list.total_pages
    );

    Ok(())
}

/// Show detailed publication information
pub async fn show(ctx: &CommandContext, id: String) -> Result<()> {
    let sp = spinner("Fetching publication details...");

    let publication: Publication = ctx
        .client
        .get(&format!("/api/v1/publications/{}", id))
        .await?;

    sp.finish_and_clear();

    // Display as key-value table
    let items = vec![
        ("ID", publication.id),
        ("Benchmark ID", publication.benchmark_id),
        ("Status", publication.status),
        ("Version", publication.version),
        ("Model Provider", publication.model_provider),
        ("Model Name", publication.model_name),
        ("Model Version", publication.model_version),
        ("Aggregate Score", format!("{:.4}", publication.aggregate_score)),
        ("Normalized Score", format!("{:.4}", publication.normalized_score)),
        ("Confidence Level", publication.confidence_level),
        (
            "Reproducibility",
            format!("{:.2}%", publication.reproducibility_score * 100.0),
        ),
        ("Is Latest", publication.is_latest.to_string()),
        ("Created", publication.created_at),
        ("Updated", publication.updated_at),
        (
            "Published",
            publication.published_at.unwrap_or_else(|| "N/A".to_string()),
        ),
    ];

    let table = TableFormatter::key_value(items)?;
    println!("{}", table);

    if !publication.tags.is_empty() {
        println!("\n{}: {}", colors::bold("Tags"), publication.tags.join(", "));
    }

    Ok(())
}

/// Inspect publication with full metadata
pub async fn inspect(ctx: &CommandContext, id: String) -> Result<()> {
    let sp = spinner("Inspecting publication...");

    let publication: FullPublication = ctx
        .client
        .get(&format!("/api/v1/publications/{}/inspect", id))
        .await?;

    sp.finish_and_clear();

    // Basic info
    println!("{}", colors::bold("=== Publication Details ==="));
    let basic_items = vec![
        ("ID", publication.id),
        ("Benchmark ID", publication.benchmark_id),
        ("Status", publication.status),
        ("Version", publication.version),
        (
            "Model",
            format!(
                "{}/{}/{}",
                publication.model_provider, publication.model_name, publication.model_version
            ),
        ),
    ];
    let table = TableFormatter::key_value(basic_items)?;
    println!("{}", table);

    // Metrics
    println!("\n{}", colors::bold("=== Metrics ==="));
    let metric_items = vec![
        (
            "Aggregate Score",
            format!("{:.4}", publication.metrics.aggregate_score),
        ),
        (
            "Normalized Score",
            format!("{:.4}", publication.metrics.normalized_score),
        ),
        (
            "Percentile Rank",
            publication
                .metrics
                .percentile_rank
                .map(|r| format!("{:.1}%", r * 100.0))
                .unwrap_or_else(|| "N/A".to_string()),
        ),
        (
            "Z-Score",
            publication
                .metrics
                .z_score
                .map(|z| format!("{:.2}", z))
                .unwrap_or_else(|| "N/A".to_string()),
        ),
    ];
    let table = TableFormatter::key_value(metric_items)?;
    println!("{}", table);

    if !publication.metrics.metric_scores.is_empty() {
        println!("\n{}", colors::dim("Individual Metrics:"));
        for (name, value) in &publication.metrics.metric_scores {
            println!(
                "  {}: {:.4} (normalized: {:.4}){}",
                name,
                value.value,
                value.normalized,
                value.unit.as_ref().map(|u| format!(" {}", u)).unwrap_or_default()
            );
        }
    }

    // Confidence
    println!("\n{}", colors::bold("=== Confidence ==="));
    let confidence_items = vec![
        (
            "Reproducibility Score",
            format!("{:.2}%", publication.confidence.reproducibility_score * 100.0),
        ),
        ("Sample Size", publication.confidence.sample_size.to_string()),
        ("Variance", format!("{:.6}", publication.confidence.variance)),
        (
            "Std Dev",
            publication
                .confidence
                .std_dev
                .map(|s| format!("{:.6}", s))
                .unwrap_or_else(|| "N/A".to_string()),
        ),
        (
            "Reproduction Count",
            publication.confidence.reproduction_count.to_string(),
        ),
    ];
    let table = TableFormatter::key_value(confidence_items)?;
    println!("{}", table);

    // Constraints
    println!("\n{}", colors::bold("=== Methodology Constraints ==="));
    let method_items = vec![
        ("Framework", publication.constraints.methodology.framework),
        (
            "Evaluation Method",
            publication.constraints.methodology.evaluation_method,
        ),
        (
            "Scoring Method",
            publication.constraints.methodology.scoring_method,
        ),
        (
            "Normalized",
            publication.constraints.methodology.normalized.to_string(),
        ),
    ];
    let table = TableFormatter::key_value(method_items)?;
    println!("{}", table);

    println!("\n{}", colors::bold("=== Dataset Constraints ==="));
    let dataset_items = vec![
        ("Dataset ID", publication.constraints.dataset_scope.dataset_id),
        (
            "Dataset Version",
            publication.constraints.dataset_scope.dataset_version,
        ),
        ("Split", publication.constraints.dataset_scope.split),
        (
            "Example Count",
            publication.constraints.dataset_scope.example_count.to_string(),
        ),
        (
            "Publicly Available",
            publication
                .constraints
                .dataset_scope
                .publicly_available
                .to_string(),
        ),
    ];
    let table = TableFormatter::key_value(dataset_items)?;
    println!("{}", table);

    // Citation
    if let Some(citation) = &publication.citation {
        println!("\n{}", colors::bold("=== Citation ==="));
        if let Some(doi) = &citation.doi {
            println!("DOI: {}", doi);
        }
        if let Some(arxiv) = &citation.arxiv_id {
            println!("arXiv: {}", arxiv);
        }
        println!("\n{}", citation.plain_text);
    }

    Ok(())
}

/// Publish a new benchmark result from YAML or JSON file
pub async fn publish(ctx: &CommandContext, file_path: String) -> Result<()> {
    ctx.require_auth()?;

    let path = Path::new(&file_path);
    if !path.exists() {
        anyhow::bail!("File not found: {}", file_path);
    }

    let content = fs::read_to_string(path).context("Failed to read publication file")?;

    let request: PublishRequest = if file_path.ends_with(".yaml") || file_path.ends_with(".yml") {
        serde_yaml::from_str(&content).context("Failed to parse YAML")?
    } else {
        serde_json::from_str(&content).context("Failed to parse JSON")?
    };

    println!("{}", colors::bold("Publishing benchmark result:"));
    println!("  Benchmark ID: {}", request.benchmark_id);
    println!(
        "  Model: {}/{}/{}",
        request.model_provider, request.model_name, request.model_version
    );
    println!("  Aggregate Score: {:.4}", request.aggregate_score);
    println!("  Sample Size: {}", request.sample_size);
    println!();

    let confirmed = confirm_default_yes("Publish this benchmark result?")?;
    if !confirmed {
        println!("Cancelled.");
        return Ok(());
    }

    let sp = spinner("Publishing benchmark result...");

    let publication: Publication = ctx
        .client
        .post("/api/v1/publications", &request)
        .await?;

    sp.finish_and_clear();

    println!("{}", colors::success("Publication created successfully!"));
    println!("ID: {}", publication.id);
    println!("Status: {}", publication.status);
    println!("Normalized Score: {:.4}", publication.normalized_score);
    println!("Confidence Level: {}", publication.confidence_level);
    println!(
        "\nTo publish, run: {} publication status {} --status published",
        colors::bold("llm-benchmark"),
        publication.id
    );

    Ok(())
}

/// Validate a benchmark submission without publishing
pub async fn validate(ctx: &CommandContext, file_path: String) -> Result<()> {
    let path = Path::new(&file_path);
    if !path.exists() {
        anyhow::bail!("File not found: {}", file_path);
    }

    let content = fs::read_to_string(path).context("Failed to read validation file")?;

    let request: ValidateRequest = if file_path.ends_with(".yaml") || file_path.ends_with(".yml") {
        serde_yaml::from_str(&content).context("Failed to parse YAML")?
    } else {
        serde_json::from_str(&content).context("Failed to parse JSON")?
    };

    println!("{}", colors::bold("Validating benchmark submission:"));
    println!("  Benchmark ID: {}", request.benchmark_id);
    println!(
        "  Model: {}/{}",
        request.model_provider, request.model_name
    );
    println!("  Aggregate Score: {:.4}", request.aggregate_score);
    println!();

    let sp = spinner("Validating...");

    let result: ValidationResult = ctx
        .client
        .post("/api/v1/publications/validate", &request)
        .await?;

    sp.finish_and_clear();

    if result.passed {
        println!("{}", colors::success("Validation PASSED"));
        println!("Validation Score: {:.2}", result.score);
    } else {
        println!("{}", colors::error("Validation FAILED"));
        println!("Validation Score: {:.2}", result.score);
    }

    if !result.errors.is_empty() {
        println!("\n{}:", colors::error("Errors"));
        for error in &result.errors {
            println!(
                "  [{}] {}{}",
                error.code,
                error.message,
                error
                    .field
                    .as_ref()
                    .map(|f| format!(" (field: {})", f))
                    .unwrap_or_default()
            );
        }
    }

    if !result.warnings.is_empty() {
        println!("\n{}:", colors::warning("Warnings"));
        for warning in &result.warnings {
            println!(
                "  [{}] {}{}",
                warning.code,
                warning.message,
                warning
                    .field
                    .as_ref()
                    .map(|f| format!(" (field: {})", f))
                    .unwrap_or_default()
            );
        }
    }

    Ok(())
}

/// Update publication metadata
pub async fn update(
    ctx: &CommandContext,
    id: String,
    tags: Option<Vec<String>>,
    citation_file: Option<String>,
) -> Result<()> {
    ctx.require_auth()?;

    let citation = if let Some(path) = citation_file {
        let content = fs::read_to_string(&path).context("Failed to read citation file")?;
        let citation: CitationInput = serde_json::from_str(&content).context("Failed to parse citation JSON")?;
        Some(citation)
    } else {
        None
    };

    let request = UpdateRequest { tags, citation };

    let sp = spinner("Updating publication...");

    let publication: Publication = ctx
        .client
        .put(&format!("/api/v1/publications/{}", id), &request)
        .await?;

    sp.finish_and_clear();

    println!("{}", colors::success("Publication updated successfully!"));
    println!("ID: {}", publication.id);
    println!("Updated: {}", publication.updated_at);

    Ok(())
}

/// Transition publication status
pub async fn transition_status(
    ctx: &CommandContext,
    id: String,
    target_status: String,
    reason: Option<String>,
) -> Result<()> {
    ctx.require_auth()?;

    // Validate status
    let valid_statuses = [
        "draft",
        "pending_validation",
        "normalizing",
        "published",
        "superseded",
        "retracted",
        "archived",
    ];

    if !valid_statuses.contains(&target_status.to_lowercase().as_str()) {
        anyhow::bail!(
            "Invalid status. Valid statuses: {}",
            valid_statuses.join(", ")
        );
    }

    println!(
        "Transitioning publication {} to status: {}",
        id, target_status
    );

    if target_status.to_lowercase() == "retracted" {
        let confirmed = confirm_default_yes(
            "Retracting a publication is a significant action. Are you sure?",
        )?;
        if !confirmed {
            println!("Cancelled.");
            return Ok(());
        }
    }

    let sp = spinner("Transitioning status...");

    let request = StatusTransitionRequest {
        target_status,
        reason,
    };

    let publication: Publication = ctx
        .client
        .post(&format!("/api/v1/publications/{}/status", id), &request)
        .await?;

    sp.finish_and_clear();

    println!("{}", colors::success("Status transitioned successfully!"));
    println!("ID: {}", publication.id);
    println!("New Status: {}", publication.status);

    if publication.status == "published" {
        if let Some(published_at) = publication.published_at {
            println!("Published At: {}", published_at);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_publication_serialization() {
        let publication = Publication {
            id: "test-id".to_string(),
            benchmark_id: "bench-id".to_string(),
            submission_id: None,
            status: "draft".to_string(),
            version: "1.0.0".to_string(),
            model_provider: "anthropic".to_string(),
            model_name: "claude-3".to_string(),
            model_version: "20240101".to_string(),
            aggregate_score: 85.5,
            normalized_score: 0.855,
            confidence_level: "High".to_string(),
            reproducibility_score: 0.92,
            published_by: "user-id".to_string(),
            organization_id: None,
            tags: vec!["benchmark".to_string()],
            is_latest: true,
            created_at: "2024-01-01".to_string(),
            updated_at: "2024-01-01".to_string(),
            published_at: None,
        };

        let json = serde_json::to_string(&publication).unwrap();
        assert!(json.contains("test-id"));
        assert!(json.contains("claude-3"));
    }

    #[test]
    fn test_validation_result_serialization() {
        let result = ValidationResult {
            passed: true,
            score: 0.95,
            errors: vec![],
            warnings: vec![ValidationWarning {
                code: "WARN001".to_string(),
                message: "Test warning".to_string(),
                field: Some("test_field".to_string()),
            }],
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("WARN001"));
    }
}
