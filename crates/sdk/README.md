# LLM Benchmark Exchange SDK

Official Rust SDK for the LLM Benchmark Exchange API.

## Features

- **Type-safe API**: Fully typed request and response models
- **Async/await**: Built on Tokio for high-performance async operations
- **Automatic retries**: Configurable retry logic with exponential backoff
- **Builder patterns**: Ergonomic client and request construction
- **Error handling**: Detailed error types for all failure scenarios

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
llm-benchmark-sdk = "0.1"
```

## Quick Start

```rust
use llm_benchmark_sdk::{Client, BenchmarkCategory, BenchmarkFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client
    let client = Client::builder()
        .api_key("your-api-key")
        .build()?;

    // List benchmarks
    let benchmarks = client.benchmarks().list().await?;
    for benchmark in benchmarks.items {
        println!("{}: {}", benchmark.name, benchmark.description);
    }

    // Get a specific benchmark
    let mmlu = client.benchmarks().get("mmlu").await?;
    println!("MMLU has {} submissions", mmlu.submission_count);

    // View leaderboard
    let leaderboard = client.leaderboards().get("mmlu").await?;
    for entry in leaderboard.entries.iter().take(10) {
        println!("#{} {} - {:.1}%", entry.rank, entry.model_name, entry.score * 100.0);
    }

    Ok(())
}
```

## Configuration

### Environment Variables

The SDK can be configured using environment variables:

```bash
export LLM_BENCHMARK_API_URL="https://api.llm-benchmark.org"
export LLM_BENCHMARK_API_KEY="your-api-key"
export LLM_BENCHMARK_TIMEOUT="30"
export LLM_BENCHMARK_DEBUG="1"  # Enable debug logging
```

### Programmatic Configuration

```rust
use llm_benchmark_sdk::{Client, ClientConfig};
use std::time::Duration;

let client = Client::builder()
    .base_url("https://api.llm-benchmark.org")
    .api_key("your-api-key")
    .timeout(Duration::from_secs(60))
    .retry_count(5)
    .debug(true)
    .build()?;
```

## Services

### Benchmarks

```rust
// List with filters
let filter = BenchmarkFilter::new()
    .category(BenchmarkCategory::Performance)
    .status(BenchmarkStatus::Active)
    .page_size(50);

let benchmarks = client.benchmarks().list_with_filter(filter).await?;

// Create a benchmark
let request = CreateBenchmarkRequest::new(
    "My Benchmark",
    "A benchmark for testing X",
    BenchmarkCategory::Accuracy,
)
.with_tags(vec!["nlp".to_string()]);

let benchmark = client.benchmarks().create(request).await?;
```

### Submissions

```rust
use llm_benchmark_sdk::{CreateSubmissionRequest, SubmissionResults};
use std::collections::HashMap;

let results = SubmissionResults {
    aggregate_score: 0.95,
    metrics: HashMap::from([("accuracy".to_string(), 0.95)]),
    test_case_results: None,
};

let request = CreateSubmissionRequest {
    benchmark_id: "mmlu".to_string(),
    model_name: "my-model".to_string(),
    model_version: "1.0".to_string(),
    results,
    provider: Some("My Company".to_string()),
    visibility: None,
    notes: None,
};

let submission = client.submissions().create(request).await?;
```

### Leaderboards

```rust
// Get leaderboard
let leaderboard = client.leaderboards().get("mmlu").await?;

// Get top 10
let top10 = client.leaderboards().top("mmlu", 10).await?;

// Compare models
let comparison = client.leaderboards()
    .compare("mmlu", "gpt-4", "claude-3")
    .await?;
```

### Governance

```rust
use llm_benchmark_sdk::{CreateProposalRequest, ProposalType, VoteType};

// Create a proposal
let proposal = client.governance().create(CreateProposalRequest {
    title: "Add new benchmark".to_string(),
    description: "Proposal to add...".to_string(),
    proposal_type: ProposalType::NewBenchmark,
    rationale: "This benchmark would help...".to_string(),
    benchmark_id: None,
}).await?;

// Vote on a proposal
client.governance()
    .vote(&proposal.id.to_string(), VoteType::Approve, Some("Great idea!"))
    .await?;

// Comment on a proposal
client.governance()
    .comment(&proposal.id.to_string(), "I have a question...")
    .await?;
```

## Error Handling

The SDK provides detailed error types:

```rust
use llm_benchmark_sdk::{Client, SdkError};

match client.benchmarks().get("invalid-id").await {
    Ok(benchmark) => println!("Found: {}", benchmark.name),
    Err(SdkError::NotFound { resource_type, resource_id }) => {
        println!("{} '{}' not found", resource_type, resource_id);
    }
    Err(SdkError::Unauthorized { message, .. }) => {
        println!("Auth failed: {}", message);
    }
    Err(SdkError::ValidationError { message, field_errors }) => {
        println!("Validation failed: {}", message);
        for err in field_errors {
            println!("  - {}: {}", err.field, err.message);
        }
    }
    Err(SdkError::RateLimited { retry_after }) => {
        println!("Rate limited, retry after {:?}s", retry_after);
    }
    Err(e) => println!("Other error: {}", e),
}
```

### Retryable Errors

Some errors are automatically retried:

```rust
// Check if an error is retryable
if error.is_retryable() {
    // The SDK already retried this automatically
}
```

## License

MIT License - see LICENSE file for details.
