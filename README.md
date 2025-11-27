<div align="center">

# LLM Benchmark Exchange

<img src="https://img.shields.io/badge/🦀_Rust-1.70+-orange?style=for-the-badge&logo=rust" alt="Rust 1.70+"/>

**A decentralized platform for standardized LLM benchmarking with community governance**

[![CI](https://img.shields.io/github/actions/workflow/status/globalbusinessadvisors/llm-benchmark-exchange/ci.yml?branch=main&style=flat-square&logo=github&label=CI)](https://github.com/globalbusinessadvisors/llm-benchmark-exchange/actions)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE.md)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-14%2B-336791?style=flat-square&logo=postgresql&logoColor=white)](https://www.postgresql.org/)
[![Redis](https://img.shields.io/badge/Redis-7%2B-DC382D?style=flat-square&logo=redis&logoColor=white)](https://redis.io/)
[![Docker](https://img.shields.io/badge/Docker-Ready-2496ED?style=flat-square&logo=docker&logoColor=white)](https://www.docker.com/)
[![Kubernetes](https://img.shields.io/badge/Kubernetes-Ready-326CE5?style=flat-square&logo=kubernetes&logoColor=white)](https://kubernetes.io/)

---

[Features](#-features) •
[Quick Start](#-quick-start) •
[CLI](#-cli) •
[SDK](#-sdk) •
[API](#-api) •
[Development](#-development) •
[Architecture](#-architecture)

</div>

---

## Overview

LLM Benchmark Exchange provides a unified platform for the AI community to:

- **Define** standardized benchmarks with versioning and metadata
- **Submit** model evaluation results with verification
- **Compare** models across multiple benchmarks via leaderboards
- **Govern** the platform through community proposals and voting

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         LLM Benchmark Exchange                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐  │
│   │  Benchmarks │    │ Submissions │    │ Leaderboards│    │ Governance  │  │
│   │  ─────────  │    │  ─────────  │    │  ─────────  │    │  ─────────  │  │
│   │  • MMLU     │───▶│  • Results  │───▶│  • Rankings │    │  • Proposals│  │
│   │  • HumanEval│    │  • Metrics  │    │  • Compare  │    │  • Voting   │  │
│   │  • Custom   │    │  • Verify   │    │  • Export   │    │  • Comments │  │
│   └─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘  │
│                                                                              │
│   ┌──────────────────────────────────────────────────────────────────────┐  │
│   │                         Access Methods                                │  │
│   │   🖥️  CLI          📦  SDK (Rust)         🌐  REST API       gRPC    │  │
│   └──────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## ✨ Features

<table>
<tr>
<td width="50%">

### 📊 Standardized Benchmarks
- Version-controlled benchmark definitions
- Rich metadata and documentation
- Multiple evaluation methods
- Test case management

</td>
<td width="50%">

### 🏆 Leaderboards & Rankings
- Real-time leaderboard updates
- Model-to-model comparisons
- Verification levels
- Export capabilities

</td>
</tr>
<tr>
<td width="50%">

### 📝 Submissions & Verification
- Submit evaluation results
- Multi-level verification
- Detailed metrics tracking
- Reproducibility support

</td>
<td width="50%">

### 🗳️ Community Governance
- Proposal system
- Democratic voting
- Transparent decision-making
- Comment threads

</td>
</tr>
<tr>
<td width="50%">

### 🛠️ Developer Tools
- Type-safe Rust SDK
- Full-featured CLI
- Shell completions
- Comprehensive docs

</td>
<td width="50%">

### 🚀 Production Ready
- Docker & Kubernetes
- Helm charts
- CI/CD pipelines
- Horizontal scaling

</td>
</tr>
</table>

---

## 🚀 Quick Start

### Using Docker Compose

```bash
# Clone the repository
git clone https://github.com/globalbusinessadvisors/llm-benchmark-exchange.git
cd llm-benchmark-exchange

# Start all services
docker-compose up -d

# The API will be available at http://localhost:8080
```

### Using the CLI

```bash
# Install the CLI
cargo install --path crates/cli

# Configure
export LLM_BENCHMARK_API_URL="https://api.llm-benchmark.org"
export LLM_BENCHMARK_TOKEN="your-token"

# Explore benchmarks
llm-benchmark benchmark list
llm-benchmark leaderboard mmlu --limit 10
```

---

## 📁 Project Structure

```
llm-benchmark-exchange/
├── crates/
│   ├── api-rest/        # 🌐 REST API (Axum)
│   ├── api-grpc/        # ⚡ gRPC API (Tonic)
│   ├── application/     # 📋 Application services
│   ├── cli/             # 🖥️  Command-line interface
│   ├── common/          # 🔧 Shared utilities
│   ├── domain/          # 🏛️  Core domain types
│   ├── infrastructure/  # 🗄️  Database & external services
│   ├── sdk/             # 📦 Rust SDK
│   ├── testing/         # 🧪 Test utilities
│   └── worker/          # ⚙️  Background jobs
├── migrations/          # 📊 Database migrations
├── docker/              # 🐳 Docker configurations
├── helm/                # ☸️  Helm charts
└── k8s/                 # ☸️  Kubernetes manifests
```

---

## 🖥️ CLI

The `llm-benchmark` CLI provides full access to platform functionality.

### Installation

```bash
cargo install --path crates/cli
```

### Configuration

```bash
# Environment variables
export LLM_BENCHMARK_API_URL="https://api.llm-benchmark.org"
export LLM_BENCHMARK_TOKEN="your-api-token"
export LLM_BENCHMARK_OUTPUT_FORMAT="table"

# Or use the config command
llm-benchmark config set api_endpoint https://api.llm-benchmark.org
llm-benchmark config set token your-api-token
```

### Global Options

```
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

<details>
<summary><b>📊 Benchmarks</b></summary>

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
</details>

<details>
<summary><b>📝 Submissions</b></summary>

```bash
# Submit results
llm-benchmark submit --benchmark mmlu --model gpt-4 --version 0613 --score 0.86

# List submissions
llm-benchmark submit list --benchmark mmlu
```
</details>

<details>
<summary><b>🏆 Leaderboards</b></summary>

```bash
# View leaderboard
llm-benchmark leaderboard mmlu
llm-benchmark lb mmlu                   # Short alias

# View with options
llm-benchmark leaderboard mmlu --limit 10 --verified-only
```
</details>

<details>
<summary><b>🗳️ Governance</b></summary>

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
</details>

<details>
<summary><b>🔐 Authentication</b></summary>

```bash
# Login
llm-benchmark auth login

# Check authentication status
llm-benchmark auth status

# Logout
llm-benchmark auth logout
```
</details>

<details>
<summary><b>🐚 Shell Completions</b></summary>

```bash
# Bash
llm-benchmark completions bash > ~/.local/share/bash-completion/completions/llm-benchmark

# Zsh
llm-benchmark completions zsh > ~/.zfunc/_llm-benchmark

# Fish
llm-benchmark completions fish > ~/.config/fish/completions/llm-benchmark.fish

# PowerShell
llm-benchmark completions powershell > llm-benchmark.ps1
```
</details>

---

## 📦 SDK

The Rust SDK provides type-safe access to the LLM Benchmark Exchange API.

### Installation

```toml
[dependencies]
llm-benchmark-sdk = { git = "https://github.com/globalbusinessadvisors/llm-benchmark-exchange", package = "llm-benchmark-sdk" }
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

**Environment Variables:**
| Variable | Description |
|----------|-------------|
| `LLM_BENCHMARK_API_URL` | API endpoint URL |
| `LLM_BENCHMARK_API_KEY` | Authentication key |
| `LLM_BENCHMARK_TIMEOUT` | Request timeout (seconds) |
| `LLM_BENCHMARK_DEBUG` | Enable debug mode |

### Services

<details>
<summary><b>Benchmarks</b></summary>

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
</details>

<details>
<summary><b>Submissions</b></summary>

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
</details>

<details>
<summary><b>Leaderboards</b></summary>

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
</details>

<details>
<summary><b>Governance</b></summary>

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
```
</details>

### Error Handling

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
    Err(SdkError::RateLimited { retry_after }) => {
        println!("Rate limited, retry after {:?}s", retry_after);
    }
    Err(e) if e.is_retryable() => {
        println!("Transient error (already retried): {}", e);
    }
    Err(e) => println!("Other error: {}", e),
}
```

---

## 🌐 API

### REST API

The REST API is built with [Axum](https://github.com/tokio-rs/axum) and provides a comprehensive HTTP interface.

**Base URL:** `https://api.llm-benchmark.org/api/v1`

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/benchmarks` | GET | List benchmarks |
| `/benchmarks/{id}` | GET | Get benchmark details |
| `/benchmarks` | POST | Create benchmark |
| `/submissions` | GET | List submissions |
| `/submissions` | POST | Create submission |
| `/leaderboards/{benchmark_id}` | GET | Get leaderboard |
| `/proposals` | GET | List proposals |
| `/proposals` | POST | Create proposal |
| `/proposals/{id}/vote` | POST | Vote on proposal |

### gRPC API

Protocol buffer definitions are available in `crates/api-grpc/proto/`.

---

## 🛠️ Development

### Prerequisites

| Requirement | Version |
|-------------|---------|
| Rust | 1.70+ |
| PostgreSQL | 14+ |
| Redis | 7+ |
| Docker | 20+ (optional) |

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

# Run with all features
cargo build --all-features
```

### Running Locally

```bash
# Start dependencies
docker-compose up -d postgres redis

# Run migrations
./migrations/run_migrations.sh

# Start the API server
cargo run -p llm-benchmark-api

# Start the worker (in another terminal)
cargo run -p llm-benchmark-worker
```

### Environment Variables

```bash
# Database
DATABASE_URL="postgresql://user:pass@localhost/llm_benchmark"

# Redis
REDIS_URL="redis://localhost:6379"

# API Configuration
API_HOST="0.0.0.0"
API_PORT="8080"
JWT_SECRET="your-secret-key"

# Telemetry
RUST_LOG="info,llm_benchmark=debug"
OTEL_EXPORTER_OTLP_ENDPOINT="http://localhost:4317"
```

---

## 🏛️ Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Presentation Layer                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │   REST API  │  │  gRPC API   │  │     CLI     │  │        SDK          │ │
│  │   (Axum)    │  │   (Tonic)   │  │   (Clap)    │  │   (reqwest/async)   │ │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘ │
└─────────┼────────────────┼────────────────┼────────────────────┼────────────┘
          │                │                │                    │
          ▼                ▼                ▼                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                             Application Layer                                │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │                         Application Services                             ││
│  │  • BenchmarkService  • SubmissionService  • GovernanceService           ││
│  │  • UserService       • OrganizationService • ScoringEngine              ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │                              Validation                                  ││
│  │  • Input validation  • Business rules  • Authorization                  ││
│  └─────────────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                               Domain Layer                                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐ │
│  │  Benchmark   │  │  Submission  │  │  Governance  │  │       User       │ │
│  │   Entities   │  │   Entities   │  │   Entities   │  │     Entities     │ │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────────┘ │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │                          Domain Events                                   ││
│  └─────────────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Infrastructure Layer                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐ │
│  │  PostgreSQL  │  │    Redis     │  │   S3/Minio   │  │    RabbitMQ      │ │
│  │  Repository  │  │    Cache     │  │   Storage    │  │    Messaging     │ │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Layer Responsibilities

| Layer | Responsibility | Crates |
|-------|----------------|--------|
| **Presentation** | HTTP/gRPC handlers, CLI commands, SDK | `api-rest`, `api-grpc`, `cli`, `sdk` |
| **Application** | Use cases, orchestration, validation | `application` |
| **Domain** | Business entities, rules, events | `domain` |
| **Infrastructure** | Database, cache, external services | `infrastructure` |

---

## 🚢 Deployment

### Docker

```bash
# Build images
docker build -f docker/Dockerfile -t llm-benchmark-api .
docker build -f docker/Dockerfile.worker -t llm-benchmark-worker .

# Run with docker-compose
docker-compose -f docker-compose.prod.yml up -d
```

### Kubernetes

```bash
# Apply manifests
kubectl apply -f k8s/namespace.yaml
kubectl apply -f k8s/

# Or use Helm
helm install llm-benchmark ./helm \
  --namespace llm-benchmark \
  --create-namespace \
  --values helm/values.yaml
```

---

## 🤝 Contributing

Contributions are welcome! Please read our contributing guidelines before submitting a PR.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE.md](LICENSE.md) file for details.

---

<div align="center">

**Built with ❤️ by the LLM Benchmark Exchange Community**

[Report Bug](https://github.com/globalbusinessadvisors/llm-benchmark-exchange/issues) •
[Request Feature](https://github.com/globalbusinessadvisors/llm-benchmark-exchange/issues) •
[Discussions](https://github.com/globalbusinessadvisors/llm-benchmark-exchange/discussions)

</div>
