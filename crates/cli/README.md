# LLM Benchmark Exchange CLI

Command-line interface for the LLM Benchmark Exchange platform.

## Installation

```bash
cargo install --path crates/cli
```

Or build from source:

```bash
cargo build --release -p llm-benchmark-cli
```

The binary will be available at `target/release/llm-benchmark`.

## Quick Start

### 1. Authentication

Login to the platform:

```bash
# Interactive login
llm-benchmark auth login

# Or use a token
llm-benchmark auth login --token YOUR_TOKEN

# Check current user
llm-benchmark auth whoami

# Logout
llm-benchmark auth logout
```

### 2. Browse Benchmarks

```bash
# List all benchmarks
llm-benchmark benchmark list

# Filter by category
llm-benchmark benchmark list --category nlp

# Show benchmark details
llm-benchmark benchmark show my-benchmark-slug
```

### 3. Create a Benchmark

Initialize a new benchmark project:

```bash
# Interactive initialization
llm-benchmark init

# Or provide name directly
llm-benchmark init --name "My Benchmark"
```

This creates a complete project structure with:
- `benchmark.yaml` - Benchmark configuration
- `test-cases/` - Test case directory
- `evaluators/` - Evaluation scripts
- `README.md` - Documentation

Validate and create:

```bash
# Validate definition
llm-benchmark benchmark validate benchmark.yaml

# Create on platform (requires authentication)
llm-benchmark benchmark create benchmark.yaml
```

### 4. Submit Results

```bash
llm-benchmark submit submit \
  --benchmark my-benchmark-id \
  --results results.json \
  --model "GPT-4" \
  --version "2024-01"
```

### 5. View Leaderboards

```bash
# Show leaderboard
llm-benchmark leaderboard show benchmark-id

# Compare two models
llm-benchmark leaderboard compare \
  --benchmark benchmark-id \
  model1-name \
  model2-name

# Export leaderboard
llm-benchmark leaderboard export benchmark-id --format csv --output leaderboard.csv
```

### 6. Governance

```bash
# List proposals
llm-benchmark proposal list

# View proposal details
llm-benchmark proposal show proposal-id

# Vote on a proposal (requires authentication)
llm-benchmark proposal vote proposal-id --vote approve

# Create a proposal
llm-benchmark proposal create \
  --type new-benchmark \
  --file proposal.yaml
```

## Commands

### Authentication Commands

- `auth login [--token TOKEN]` - Login to the platform
- `auth logout` - Logout from the platform
- `auth whoami` - Show current user information

### Benchmark Commands

- `benchmark list [--category CAT] [--status STATUS]` - List benchmarks
- `benchmark show <ID>` - Show benchmark details
- `benchmark create <FILE>` - Create new benchmark from YAML/JSON
- `benchmark update <ID> [--file FILE]` - Update existing benchmark
- `benchmark submit-for-review <ID>` - Submit benchmark for review
- `benchmark validate <FILE>` - Validate benchmark definition

### Submission Commands

- `submit submit -b <BENCHMARK> -r <FILE> -m <MODEL> -v <VERSION>` - Submit results
- `submit show <ID>` - Show submission details
- `submit list [--benchmark ID]` - List submissions
- `submit request-verification <ID>` - Request verification for submission

### Leaderboard Commands

- `leaderboard show <BENCHMARK_ID>` - Show leaderboard
- `leaderboard compare -b <ID> <MODEL1> <MODEL2>` - Compare two models
- `leaderboard export <BENCHMARK_ID> [-f FORMAT] [-o FILE]` - Export leaderboard

### Proposal Commands

- `proposal list [--status STATUS]` - List proposals
- `proposal show <ID>` - Show proposal details
- `proposal create -t <TYPE> [-f FILE]` - Create new proposal
- `proposal vote <ID> -v <VOTE>` - Vote on proposal (approve/reject/abstain)
- `proposal comment <ID> [-m MESSAGE]` - Comment on proposal

### Initialization Commands

- `init [-n NAME]` - Initialize new benchmark project
- `scaffold <TYPE>` - Generate template files (test-case, results, benchmark)

## Configuration

Configuration is stored in `~/.llm-benchmark/config.toml`:

```toml
api_endpoint = "http://localhost:3000"
output_format = "table"  # table, json, or plain
colored = true

# Auth token (set via login command)
# auth_token = "your-token"
```

You can manually edit this file or use environment variables:

```bash
export LLM_BENCHMARK_API_ENDPOINT="https://api.llm-benchmark.example.com"
```

## Output Formats

The CLI supports multiple output formats:

### Table (default)
```bash
llm-benchmark benchmark list
```

### JSON
```bash
llm-benchmark benchmark list --format json
```

### Plain text
```bash
llm-benchmark benchmark list --format plain
```

## Examples

### Complete workflow: Create and submit

```bash
# 1. Initialize project
llm-benchmark init --name "Math Reasoning Benchmark"

# 2. Edit benchmark.yaml and add test cases
cd math-reasoning-benchmark

# 3. Validate
llm-benchmark benchmark validate benchmark.yaml

# 4. Login
llm-benchmark auth login

# 5. Create benchmark
llm-benchmark benchmark create benchmark.yaml

# 6. Run your evaluation
python evaluators/evaluate.py --model gpt-4 --output results.json

# 7. Submit results
llm-benchmark submit submit \
  --benchmark math-reasoning-benchmark \
  --results results.json \
  --model "GPT-4" \
  --version "2024-01"

# 8. View leaderboard
llm-benchmark leaderboard show math-reasoning-benchmark
```

### Scaffold templates

```bash
# Generate test case template
llm-benchmark scaffold test-case

# Generate results template
llm-benchmark scaffold results

# Generate benchmark template
llm-benchmark scaffold benchmark
```

## Error Handling

The CLI provides clear error messages:

```bash
$ llm-benchmark submit show invalid-id
Error: Submission not found: invalid-id

$ llm-benchmark benchmark create missing.yaml
Error: File not found: missing.yaml
```

Authentication errors:

```bash
$ llm-benchmark benchmark create test.yaml
Error: Not authenticated. Please run 'llm-benchmark auth login' first.
```

## Development

### Building

```bash
cargo build -p llm-benchmark-cli
```

### Testing

```bash
cargo test -p llm-benchmark-cli
```

### Running locally

```bash
cargo run -p llm-benchmark-cli -- --help
```

## License

MIT
