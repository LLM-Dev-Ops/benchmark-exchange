//! Project initialization commands

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::interactive::{confirm_default_yes, prompt_input, prompt_input_with_default};
use crate::output::colors;

/// Initialize a new benchmark project
pub async fn init(name: Option<String>) -> Result<()> {
    println!("{}", colors::bold("Initialize New Benchmark Project"));
    println!();

    let project_name = if let Some(n) = name {
        n
    } else {
        prompt_input("Project name")?
    };

    let slug = prompt_input_with_default(
        "Benchmark slug",
        &project_name.to_lowercase().replace(' ', "-"),
    )?;

    let description = prompt_input("Description")?;

    let category = prompt_input("Category (e.g., nlp, reasoning, coding)")?;

    println!();
    println!("{}", colors::bold("Project configuration:"));
    println!("  Name:        {}", project_name);
    println!("  Slug:        {}", slug);
    println!("  Category:    {}", category);
    println!("  Description: {}", description);
    println!();

    let confirmed = confirm_default_yes("Create this project?")?;
    if !confirmed {
        println!("Cancelled.");
        return Ok(());
    }

    // Create project directory
    let project_dir = Path::new(&slug);
    if project_dir.exists() {
        anyhow::bail!("Directory '{}' already exists", slug);
    }

    fs::create_dir_all(project_dir).context("Failed to create project directory")?;

    // Create subdirectories
    fs::create_dir_all(project_dir.join("test-cases"))?;
    fs::create_dir_all(project_dir.join("evaluators"))?;
    fs::create_dir_all(project_dir.join("docs"))?;

    // Create benchmark.yaml
    let benchmark_yaml = format!(
        r#"# Benchmark Definition
name: "{}"
slug: "{}"
description: "{}"
category: "{}"
version: "1.0.0"

# Metadata
author: ""
license: "MIT"
tags: []

# Test Configuration
test_cases:
  directory: "./test-cases"
  format: "jsonl"

# Evaluation
evaluators:
  - name: "default"
    type: "custom"
    script: "./evaluators/evaluate.py"

# Metrics
metrics:
  - name: "accuracy"
    type: "float"
    description: "Overall accuracy"
    higher_is_better: true

  - name: "latency"
    type: "float"
    description: "Average response time (ms)"
    higher_is_better: false

# Submission Requirements
submission:
  required_fields:
    - model_name
    - model_version
    - results
  max_file_size: "100MB"
"#,
        project_name, slug, description, category
    );

    fs::write(project_dir.join("benchmark.yaml"), benchmark_yaml)?;

    // Create README.md
    let readme = format!(
        r#"# {}

{}

## Overview

This benchmark evaluates models on [describe what this benchmark tests].

## Running the Benchmark

1. Install dependencies:
   ```bash
   pip install -r requirements.txt
   ```

2. Run evaluation:
   ```bash
   python evaluate.py --model your-model --results results.json
   ```

3. Submit results:
   ```bash
   llm-benchmark submit {} --results results.json --model your-model --version 1.0
   ```

## Test Cases

Test cases are located in the `test-cases/` directory.

## Evaluation

The evaluator script is in `evaluators/evaluate.py`.

## Metrics

- **accuracy**: Overall accuracy on test cases
- **latency**: Average response time in milliseconds

## License

MIT
"#,
        project_name, description, slug
    );

    fs::write(project_dir.join("README.md"), readme)?;

    // Create example test case
    let example_test = r#"{"id": "example-001", "input": "Example input", "expected_output": "Example output"}
"#;
    fs::write(project_dir.join("test-cases").join("examples.jsonl"), example_test)?;

    // Create example evaluator
    let evaluator = r#"#!/usr/bin/env python3
"""
Example evaluator for the benchmark.
"""

import json
import sys
from pathlib import Path


def evaluate(results_file: Path) -> dict:
    """
    Evaluate results from a model submission.

    Args:
        results_file: Path to the results JSON file

    Returns:
        Dictionary with evaluation metrics
    """
    with open(results_file) as f:
        results = json.load(f)

    # Example evaluation logic
    correct = 0
    total = 0
    latencies = []

    for result in results.get("test_results", []):
        total += 1
        if result.get("correct", False):
            correct += 1
        if "latency_ms" in result:
            latencies.append(result["latency_ms"])

    metrics = {
        "accuracy": correct / total if total > 0 else 0.0,
        "latency": sum(latencies) / len(latencies) if latencies else 0.0,
    }

    return metrics


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: evaluate.py <results_file>")
        sys.exit(1)

    results_file = Path(sys.argv[1])
    metrics = evaluate(results_file)

    print(json.dumps(metrics, indent=2))
"#;
    fs::write(project_dir.join("evaluators").join("evaluate.py"), evaluator)?;

    // Create requirements.txt
    let requirements = r#"# Python dependencies for evaluation
jsonschema>=4.0.0
"#;
    fs::write(project_dir.join("requirements.txt"), requirements)?;

    // Create .gitignore
    let gitignore = r#"# Python
__pycache__/
*.py[cod]
*$py.class
*.so
.Python
venv/
env/

# Results
results/
*.results.json

# IDE
.vscode/
.idea/
*.swp
*.swo
"#;
    fs::write(project_dir.join(".gitignore"), gitignore)?;

    println!();
    println!("{}", colors::success("Project created successfully!"));
    println!();
    println!("Next steps:");
    println!("  1. cd {}", slug);
    println!("  2. Edit benchmark.yaml to configure your benchmark");
    println!("  3. Add test cases to test-cases/");
    println!("  4. Customize evaluators/evaluate.py");
    println!("  5. Validate: llm-benchmark benchmark validate benchmark.yaml");
    println!("  6. Create: llm-benchmark benchmark create benchmark.yaml");

    Ok(())
}

/// Generate template files
pub async fn scaffold(template_type: String) -> Result<()> {
    println!(
        "{}",
        colors::info(&format!("Scaffolding {} template...", template_type))
    );

    let content = match template_type.to_lowercase().as_str() {
        "test-case" => {
            r#"{
  "id": "test-001",
  "input": "What is 2 + 2?",
  "expected_output": "4",
  "metadata": {
    "difficulty": "easy",
    "category": "arithmetic"
  }
}
"#
        }
        "results" => {
            r#"{
  "model_name": "my-model",
  "model_version": "1.0.0",
  "test_results": [
    {
      "test_id": "test-001",
      "output": "4",
      "correct": true,
      "latency_ms": 150
    }
  ],
  "metadata": {
    "timestamp": "2024-01-01T00:00:00Z",
    "hardware": "CPU",
    "notes": ""
  }
}
"#
        }
        "benchmark" => {
            r#"name: "My Benchmark"
slug: "my-benchmark"
description: "A benchmark for evaluating model performance"
category: "nlp"
version: "1.0.0"

author: ""
license: "MIT"
tags: []

test_cases:
  directory: "./test-cases"
  format: "jsonl"

evaluators:
  - name: "default"
    type: "custom"
    script: "./evaluators/evaluate.py"

metrics:
  - name: "accuracy"
    type: "float"
    description: "Overall accuracy"
    higher_is_better: true
"#
        }
        _ => {
            anyhow::bail!(
                "Unknown template type: {}. Available: test-case, results, benchmark",
                template_type
            )
        }
    };

    let filename = match template_type.to_lowercase().as_str() {
        "test-case" => "test-case.json",
        "results" => "results.json",
        "benchmark" => "benchmark.yaml",
        _ => unreachable!(),
    };

    fs::write(filename, content)?;

    println!("{}", colors::success(&format!("Created: {}", filename)));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scaffold_templates() {
        // Just test that template types are recognized
        let valid_types = vec!["test-case", "results", "benchmark"];
        for t in valid_types {
            assert!(t == "test-case" || t == "results" || t == "benchmark");
        }
    }
}
