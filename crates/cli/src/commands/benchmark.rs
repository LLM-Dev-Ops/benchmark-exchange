//! Benchmark management commands

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::commands::CommandContext;
use crate::interactive::{confirm_default_yes, spinner};
use crate::output::{colors, TableFormatter};

#[derive(Debug, Serialize, Deserialize)]
pub struct Benchmark {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub status: String,
    pub version: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BenchmarkList {
    pub benchmarks: Vec<Benchmark>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct BenchmarkCreateRequest {
    pub name: String,
    pub slug: String,
    pub description: String,
    pub category: String,
    pub definition: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct BenchmarkUpdateRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub definition: Option<serde_json::Value>,
}

/// List benchmarks with optional filters
pub async fn list(
    ctx: &CommandContext,
    category: Option<String>,
    status: Option<String>,
) -> Result<()> {
    let sp = spinner("Fetching benchmarks...");

    let mut path = "/api/v1/benchmarks?".to_string();
    if let Some(cat) = category {
        path.push_str(&format!("category={}&", cat));
    }
    if let Some(st) = status {
        path.push_str(&format!("status={}&", st));
    }

    let list: BenchmarkList = ctx.client.get(&path).await?;

    sp.finish_and_clear();

    if list.benchmarks.is_empty() {
        println!("{}", colors::warning("No benchmarks found."));
        return Ok(());
    }

    // Create table
    let headers = vec!["ID", "Slug", "Name", "Category", "Status", "Version"];
    let rows: Vec<Vec<String>> = list
        .benchmarks
        .iter()
        .map(|b| {
            vec![
                b.id.clone(),
                b.slug.clone(),
                b.name.clone(),
                b.category.clone(),
                b.status.clone(),
                b.version.clone(),
            ]
        })
        .collect();

    let table = TableFormatter::simple(headers, rows)?;
    println!("{}", table);
    println!("{} benchmarks found", colors::dim(&list.total.to_string()));

    Ok(())
}

/// Show detailed benchmark information
pub async fn show(ctx: &CommandContext, id_or_slug: String) -> Result<()> {
    let sp = spinner("Fetching benchmark details...");

    let benchmark: Benchmark = ctx
        .client
        .get(&format!("/api/v1/benchmarks/{}", id_or_slug))
        .await?;

    sp.finish_and_clear();

    // Display as key-value table
    let items = vec![
        ("ID", benchmark.id),
        ("Slug", benchmark.slug),
        ("Name", benchmark.name),
        ("Description", benchmark.description),
        ("Category", benchmark.category),
        ("Status", benchmark.status),
        ("Version", benchmark.version),
        ("Created", benchmark.created_at),
        ("Updated", benchmark.updated_at),
    ];

    let table = TableFormatter::key_value(items)?;
    println!("{}", table);

    Ok(())
}

/// Create a new benchmark from YAML or JSON file
pub async fn create(ctx: &CommandContext, file_path: String) -> Result<()> {
    ctx.require_auth()?;

    let path = Path::new(&file_path);
    if !path.exists() {
        anyhow::bail!("File not found: {}", file_path);
    }

    let content = fs::read_to_string(path)
        .context("Failed to read benchmark definition file")?;

    let definition: serde_json::Value = if file_path.ends_with(".yaml")
        || file_path.ends_with(".yml")
    {
        let yaml: serde_yaml::Value = serde_yaml::from_str(&content)
            .context("Failed to parse YAML")?;
        serde_json::to_value(yaml)?
    } else {
        serde_json::from_str(&content).context("Failed to parse JSON")?
    };

    // Extract required fields
    let name = definition
        .get("name")
        .and_then(|v| v.as_str())
        .context("Missing 'name' field in definition")?
        .to_string();

    let slug = definition
        .get("slug")
        .and_then(|v| v.as_str())
        .context("Missing 'slug' field in definition")?
        .to_string();

    let description = definition
        .get("description")
        .and_then(|v| v.as_str())
        .context("Missing 'description' field in definition")?
        .to_string();

    let category = definition
        .get("category")
        .and_then(|v| v.as_str())
        .context("Missing 'category' field in definition")?
        .to_string();

    println!("{}", colors::bold("Creating new benchmark:"));
    println!("  Name:     {}", name);
    println!("  Slug:     {}", slug);
    println!("  Category: {}", category);
    println!();

    let confirmed = confirm_default_yes("Create this benchmark?")?;
    if !confirmed {
        println!("Cancelled.");
        return Ok(());
    }

    let sp = spinner("Creating benchmark...");

    let request = BenchmarkCreateRequest {
        name,
        slug,
        description,
        category,
        definition,
    };

    let benchmark: Benchmark = ctx
        .client
        .post("/api/v1/benchmarks", &request)
        .await?;

    sp.finish_and_clear();

    println!("{}", colors::success("Benchmark created successfully!"));
    println!("ID: {}", benchmark.id);
    println!("Slug: {}", benchmark.slug);

    Ok(())
}

/// Update an existing benchmark
pub async fn update(
    ctx: &CommandContext,
    id: String,
    file_path: Option<String>,
) -> Result<()> {
    ctx.require_auth()?;

    let definition = if let Some(path) = file_path {
        let path = Path::new(&path);
        if !path.exists() {
            anyhow::bail!("File not found: {}", path.display());
        }

        let content = fs::read_to_string(path)
            .context("Failed to read benchmark definition file")?;

        let def: serde_json::Value = if path.extension().and_then(|s| s.to_str()) == Some("yaml")
            || path.extension().and_then(|s| s.to_str()) == Some("yml")
        {
            let yaml: serde_yaml::Value = serde_yaml::from_str(&content)
                .context("Failed to parse YAML")?;
            serde_json::to_value(yaml)?
        } else {
            serde_json::from_str(&content).context("Failed to parse JSON")?
        };

        Some(def)
    } else {
        None
    };

    let request = BenchmarkUpdateRequest {
        name: None,
        description: None,
        definition,
    };

    let sp = spinner("Updating benchmark...");

    let benchmark: Benchmark = ctx
        .client
        .put(&format!("/api/v1/benchmarks/{}", id), &request)
        .await?;

    sp.finish_and_clear();

    println!("{}", colors::success("Benchmark updated successfully!"));
    println!("ID: {}", benchmark.id);

    Ok(())
}

/// Submit a benchmark for review
pub async fn submit_for_review(ctx: &CommandContext, id: String) -> Result<()> {
    ctx.require_auth()?;

    let confirmed = confirm_default_yes("Submit this benchmark for review?")?;
    if !confirmed {
        println!("Cancelled.");
        return Ok(());
    }

    let sp = spinner("Submitting for review...");

    let _: serde_json::Value = ctx
        .client
        .post(&format!("/api/v1/benchmarks/{}/submit", id), &())
        .await?;

    sp.finish_and_clear();

    println!("{}", colors::success("Benchmark submitted for review!"));

    Ok(())
}

/// Validate a benchmark definition file
pub async fn validate(file_path: String) -> Result<()> {
    let path = Path::new(&file_path);
    if !path.exists() {
        anyhow::bail!("File not found: {}", file_path);
    }

    println!("{}", colors::info("Validating benchmark definition..."));

    let content = fs::read_to_string(path)
        .context("Failed to read benchmark definition file")?;

    let definition: serde_json::Value = if file_path.ends_with(".yaml")
        || file_path.ends_with(".yml")
    {
        let yaml: serde_yaml::Value = serde_yaml::from_str(&content)
            .context("Failed to parse YAML")?;
        serde_json::to_value(yaml)?
    } else {
        serde_json::from_str(&content).context("Failed to parse JSON")?
    };

    // Basic validation checks
    let required_fields = ["name", "slug", "description", "category"];
    let mut errors = Vec::new();

    for field in &required_fields {
        if definition.get(field).is_none() {
            errors.push(format!("Missing required field: {}", field));
        }
    }

    if !errors.is_empty() {
        println!("{}", colors::error("Validation failed:"));
        for error in errors {
            println!("  - {}", error);
        }
        anyhow::bail!("Validation failed");
    }

    println!("{}", colors::success("Validation successful!"));
    println!("  Name:     {}", definition["name"].as_str().unwrap());
    println!("  Slug:     {}", definition["slug"].as_str().unwrap());
    println!("  Category: {}", definition["category"].as_str().unwrap());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_serialization() {
        let benchmark = Benchmark {
            id: "test-id".to_string(),
            slug: "test-slug".to_string(),
            name: "Test Benchmark".to_string(),
            description: "A test benchmark".to_string(),
            category: "test".to_string(),
            status: "active".to_string(),
            version: "1.0.0".to_string(),
            created_at: "2024-01-01".to_string(),
            updated_at: "2024-01-01".to_string(),
        };

        let json = serde_json::to_string(&benchmark).unwrap();
        assert!(json.contains("test-id"));
    }
}
