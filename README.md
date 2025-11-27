# LLM Benchmark Exchange

A decentralized platform for standardized LLM benchmarking with community governance.

## Features

- **Standardized Benchmarks**: Create, version, and manage benchmarks with comprehensive metadata
- **Submissions & Leaderboards**: Submit model results and view rankings across benchmarks
- **Community Governance**: Propose changes, vote on decisions, and participate in platform evolution
- **Type-Safe SDK**: Rust SDK with builder patterns and comprehensive error handling
- **CLI Tool**: Command-line interface for all platform operations

## Project Structure

```
crates/
├── api/            # REST API server (Axum-based)
├── application/    # Application services and use cases
├── cli/            # Command-line interface
├── domain/         # Core domain types and business logic
├── infrastructure/ # Database, caching, and external services
└── sdk/            # Rust SDK for API integration
```

## CLI

The `llm-benchmark` CLI provides full access to platform functionality.

### Installation

```bash
cargo install --path crates/cli
```

### Configuration

Configure via environment variables or the config file:

```bash
export LLM_BENCHMARK_API_URL="https://api.llm-benchmark.org"
export LLM_BENCHMARK_TOKEN="your-api-token"
export LLM_BENCHMARK_OUTPUT_FORMAT="table"  # json, table, or plain
```

Or use the config command:

```bash
llm-benchmark config set api_endpoint https://api.llm-benchmark.org
llm-benchmark config set token your-api-token
llm-benchmark config show
```

### Global Options

```bash
llm-benchmark [OPTIONS] <COMMAND>

Options:
  -o, --format <FORMAT>    Output format: json, table, plain [default: table]
      --api-url <URL>      API endpoint URL (overrides config)
      --token <TOKEN>      Authentication token (overrides config)
  -v, --verbose            Enable verbose output
      --no-color           Disable colored output
  -h, --help               Print help
  -V, --version            Print version
```

### Commands

#### Benchmarks

```bash
# List all benchmarks
llm-benchmark benchmark list
llm-benchmark b list                    # Short alias

# List with filters
llm-benchmark benchmark list --category accuracy --status active --limit 20

# Get benchmark details
llm-benchmark benchmark get mmlu

# Create a new benchmark (interactive)
llm-benchmark benchmark create
```

#### Submissions

```bash
# Submit results
llm-benchmark submit --benchmark mmlu --model gpt-4 --version 0613 --score 0.86

# List submissions
llm-benchmark submit list --benchmark mmlu
```

#### Leaderboards

```bash
# View leaderboard
llm-benchmark leaderboard mmlu
llm-benchmark lb mmlu                   # Short alias

# View with options
llm-benchmark leaderboard mmlu --limit 10 --verified-only
```

#### Governance

```bash
# List proposals
llm-benchmark proposal list
llm-benchmark p list                    # Short alias

# View proposal details
llm-benchmark proposal get <proposal-id>

# Create a proposal
llm-benchmark proposal create

# Vote on a proposal
llm-benchmark proposal vote <proposal-id> --approve
llm-benchmark proposal vote <proposal-id> --reject --reason "Needs more detail"

# Comment on a proposal
llm-benchmark proposal comment <proposal-id> "I have a question..."
```

#### Authentication

```bash
# Login
llm-benchmark auth login

# Check authentication status
llm-benchmark auth status

# Logout
llm-benchmark auth logout
```

#### Shell Completions

```bash
# Generate shell completions
llm-benchmark completions bash > ~/.local/share/bash-completion/completions/llm-benchmark
llm-benchmark completions zsh > ~/.zfunc/_llm-benchmark
llm-benchmark completions fish > ~/.config/fish/completions/llm-benchmark.fish
llm-benchmark completions powershell > llm-benchmark.ps1
```

## SDK

The Rust SDK provides type-safe access to the LLM Benchmark Exchange API.

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
llm-benchmark-sdk = { path = "crates/sdk" }
```

### Quick Start

```rust
use llm_benchmark_sdk::{Client, BenchmarkFilter, SdkError};

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

### Client Configuration

```rust
use llm_benchmark_sdk::Client;
use std::time::Duration;

let client = Client::builder()
    .base_url("https://api.llm-benchmark.org")
    .api_key("your-api-key")
    .timeout(Duration::from_secs(60))
    .retry_count(5)
    .debug(true)
    .build()?;
```

Environment variables are also supported:
- `LLM_BENCHMARK_API_URL`
- `LLM_BENCHMARK_API_KEY`
- `LLM_BENCHMARK_TIMEOUT`
- `LLM_BENCHMARK_DEBUG`

### Services

#### Benchmarks

```rust
use llm_benchmark_sdk::{BenchmarkFilter, BenchmarkCategory, BenchmarkStatus};

// List with filters
let filter = BenchmarkFilter::new()
    .category(BenchmarkCategory::Accuracy)
    .status(BenchmarkStatus::Active)
    .page_size(50);

let benchmarks = client.benchmarks().list_with_filter(filter).await?;

// Create a benchmark
let request = CreateBenchmarkRequest::new(
    "My Benchmark",
    "A benchmark for testing X",
    BenchmarkCategory::Accuracy,
);
let benchmark = client.benchmarks().create(request).await?;
```

#### Submissions

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

#### Leaderboards

```rust
// Get full leaderboard
let leaderboard = client.leaderboards().get("mmlu").await?;

// Get top N entries
let top10 = client.leaderboards().top("mmlu", 10).await?;

// Compare two models
let comparison = client.leaderboards()
    .compare("mmlu", "gpt-4", "claude-3")
    .await?;
println!("Score difference: {:.2}%", comparison.score_diff * 100.0);

// Export leaderboard data
let export = client.leaderboards().export("mmlu").await?;
```

#### Governance

```rust
use llm_benchmark_sdk::{CreateProposalRequest, ProposalType, VoteType};

// Create a proposal
let proposal = client.governance().create(CreateProposalRequest {
    title: "Add new benchmark category".to_string(),
    description: "Proposing to add...".to_string(),
    proposal_type: ProposalType::NewBenchmark,
    rationale: "This would help...".to_string(),
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

// Get voting results
let results = client.governance()
    .get_voting_results(&proposal.id.to_string())
    .await?;
```

### Error Handling

The SDK provides detailed error types:

```rust
use llm_benchmark_sdk::SdkError;

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
    Err(e) if e.is_retryable() => {
        println!("Transient error (already retried): {}", e);
    }
    Err(e) => println!("Other error: {}", e),
}
```

## Development

### Prerequisites

- Rust 1.70+
- PostgreSQL 14+
- Redis 7+

### Building

```bash
# Build all crates
cargo build

# Build in release mode
cargo build --release

# Run tests
cargo test

# Run specific crate tests
cargo test -p llm-benchmark-sdk
cargo test -p llm-benchmark-cli
```

### Running the API Server

```bash
# Set environment variables
export DATABASE_URL="postgresql://user:pass@localhost/llm_benchmark"
export REDIS_URL="redis://localhost:6379"

# Run the server
cargo run -p llm-benchmark-api
```

## Architecture

The platform follows a clean architecture pattern:

- **Domain Layer** (`crates/domain`): Core business entities and rules
- **Application Layer** (`crates/application`): Use cases and application services
- **Infrastructure Layer** (`crates/infrastructure`): Database, cache, and external integrations
- **API Layer** (`crates/api`): REST API endpoints
- **SDK** (`crates/sdk`): Client library for API consumers
- **CLI** (`crates/cli`): Command-line interface

## License

MIT License - see [LICENSE.md](LICENSE.md) for details.
