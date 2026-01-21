# Benchmark Publication Agent

## Agent Contract & Boundary Definition

**Agent Name:** Benchmark Publication Agent
**Agent ID:** `benchmark-publication-agent`
**Agent Version:** `1.0.0`
**Classification:** BENCHMARK PUBLICATION

---

## Purpose Statement

The Benchmark Publication Agent is responsible for publishing, validating, and normalizing LLM benchmark results as authoritative, reproducible artifacts for cross-model comparison.

This agent operates **outside the critical execution path** and serves as the authoritative source of benchmark truth within the LLM-Benchmark-Exchange repository.

---

## Decision Types

| Decision Type | Description |
|---------------|-------------|
| `benchmark_publish` | Publishing a new benchmark result |
| `benchmark_update` | Updating an existing publication |
| `benchmark_validate` | Validating a benchmark submission |
| `benchmark_normalize` | Normalizing benchmark metrics |
| `benchmark_retract` | Retracting a publication |
| `benchmark_inspect` | Inspecting publication metadata |

---

## Confidence Semantics

| Metric | Description | Range |
|--------|-------------|-------|
| `reproducibility_score` | How reproducible the benchmark results are | 0.0 - 1.0 |
| `sample_size` | Number of evaluation samples used | > 0 |
| `variance` | Statistical variance in results | >= 0.0 |
| `std_dev` | Standard deviation of scores | >= 0.0 |
| `reproduction_count` | Number of independent reproductions | >= 1 |
| `confidence_interval` | 95% confidence interval | (lower, upper) |
| `coefficient_of_variation` | CV = std_dev / mean | >= 0.0 |

### Confidence Levels

| Level | Reproducibility Score |
|-------|----------------------|
| Very High | >= 0.95 |
| High | >= 0.85 |
| Medium | >= 0.70 |
| Low | >= 0.50 |
| Very Low | < 0.50 |

---

## Constraints Applied Semantics

### Methodology Constraints

| Field | Description |
|-------|-------------|
| `framework` | Evaluation framework (lm-eval, helm, custom) |
| `evaluation_method` | Evaluation method (zero-shot, few-shot, chain-of-thought) |
| `prompt_template_hash` | Hash of prompt template for reproducibility |
| `scoring_method` | Scoring methodology (exact_match, f1, bleu) |
| `normalized` | Whether results are normalized |
| `normalization_method` | Normalization method if applicable |

### Dataset Scope Constraints

| Field | Description |
|-------|-------------|
| `dataset_id` | Dataset identifier/name |
| `dataset_version` | Dataset version/hash |
| `subset` | Subset used (if applicable) |
| `example_count` | Number of examples used |
| `split` | Data split (train/validation/test) |
| `publicly_available` | Whether dataset is publicly available |

### Model Version Constraints

| Field | Description |
|-------|-------------|
| `provider` | Model provider (openai, anthropic, meta) |
| `model_name` | Model name |
| `version` | Model version/checkpoint |
| `parameter_count` | Model size (parameters) |
| `quantization` | Quantization method if applicable |
| `context_window` | Context window size |

---

## Input Schema References

All input schemas are defined in `agentics-contracts` (referenced from `llm_benchmark_domain::publication`):

- `PublishBenchmarkRequest`
- `ValidateBenchmarkRequest`
- `UpdatePublicationRequest`
- `TransitionStatusRequest`
- `PublicationFilters`

---

## Output Schema References

- `Publication` - Core publication entity
- `PublicationDto` - DTO for API responses
- `ValidationResults` - Validation result structure
- `NormalizedMetrics` - Normalized benchmark metrics

---

## DecisionEvent Mapping

Every invocation emits exactly ONE `DecisionEvent` with the following structure:

```rust
DecisionEvent {
    agent_id: String,              // "benchmark-publication-agent"
    agent_version: String,         // "1.0.0"
    decision_type: String,         // From PublicationDecisionType
    inputs_hash: String,           // SHA256 of input data
    outputs: DecisionOutputs,      // Decision results
    confidence: PublicationConfidence,
    constraints_applied: PublicationConstraints,
    execution_ref: String,         // Correlation ID
    timestamp: DateTime<Utc>,      // UTC timestamp
}
```

---

## CLI Contract

### Commands

| Command | Description | Auth Required |
|---------|-------------|---------------|
| `llm-benchmark publication list` | List publications with filters | No |
| `llm-benchmark publication show <id>` | Show publication details | No |
| `llm-benchmark publication inspect <id>` | Inspect with full metadata | No |
| `llm-benchmark publication publish <file>` | Publish from YAML/JSON | Yes |
| `llm-benchmark publication validate <file>` | Validate without publishing | No |
| `llm-benchmark publication update <id>` | Update metadata | Yes |
| `llm-benchmark publication status <id>` | Transition status | Yes |

### CLI Invocation Shape

```bash
# Publish a benchmark result
llm-benchmark publication publish benchmark-result.yaml

# Validate without publishing
llm-benchmark publication validate benchmark-submission.json

# Inspect full metadata
llm-benchmark publication inspect <publication-id>

# List with filters
llm-benchmark publication list --model-provider anthropic --status published

# Transition status
llm-benchmark publication status <id> --status published --reason "Peer reviewed"
```

---

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/v1/publications` | Publish benchmark result |
| `POST` | `/api/v1/publications/validate` | Validate without publishing |
| `GET` | `/api/v1/publications` | List publications |
| `GET` | `/api/v1/publications/:id` | Get publication by ID |
| `GET` | `/api/v1/publications/:id/inspect` | Inspect with full metadata |
| `PUT` | `/api/v1/publications/:id` | Update publication |
| `POST` | `/api/v1/publications/:id/status` | Transition status |

---

## Systems That MAY Consume This Agent's Output

1. **LLM-Registry** - MAY reference benchmark metadata
2. **LLM-Observatory** - MAY consume benchmark outputs
3. **LLM-Orchestrator** - MAY consume benchmarks for planning only
4. **Governance systems** - Consume benchmark artifacts for auditing
5. **Audit systems** - Consume benchmark artifacts for compliance

---

## Explicit Non-Responsibilities

### This agent MUST NOT:

1. ❌ Execute benchmark workloads
2. ❌ Trigger model execution
3. ❌ Intercept runtime requests
4. ❌ Modify routing or execution behavior
5. ❌ Apply optimizations automatically
6. ❌ Enforce policies or rankings
7. ❌ Connect directly to Google SQL
8. ❌ Execute SQL queries
9. ❌ Invoke other Benchmark-Exchange agents directly

### This agent MAY:

1. ✅ Publish benchmark results
2. ✅ Normalize benchmark inputs and outputs
3. ✅ Validate benchmark methodology metadata
4. ✅ Emit benchmark comparison artifacts
5. ✅ Surface benchmark confidence and reproducibility signals
6. ✅ Emit telemetry compatible with LLM-Observatory
7. ✅ Emit DecisionEvents to ruvector-service

---

## Persistence Rules

### MUST persist via ruvector-service:

- `DecisionEvent` (exactly ONE per invocation)
- `Publication` entities
- Telemetry events

### MUST NOT persist:

- Raw model outputs
- Execution logs
- Temporary validation data
- Cached scores

---

## Failure Modes

| Error Code | Description | Retryable |
|------------|-------------|-----------|
| `NOT_FOUND` | Resource not found | No |
| `UNAUTHORIZED` | Authentication required | No |
| `FORBIDDEN` | Permission denied | No |
| `INVALID_INPUT` | Invalid input data | No |
| `VALIDATION_FAILED` | Validation errors | No |
| `CONFLICT` | Resource conflict | No |
| `INTERNAL_ERROR` | Internal error | Yes |
| `SERVICE_UNAVAILABLE` | ruvector-service unavailable | Yes |
| `RATE_LIMIT_EXCEEDED` | Rate limit exceeded | Yes |
| `TIMEOUT` | Request timeout | Yes |

---

## Versioning Rules

1. Publication versions follow semantic versioning (MAJOR.MINOR.PATCH)
2. Each update creates a new version, maintaining history
3. Only one version can be marked as `is_latest`
4. Superseded versions remain accessible but marked `superseded`
5. Version transitions: Draft → PendingValidation → Normalizing → Published

---

## Deployment Model

- **Runtime:** Google Cloud Edge Function
- **Service:** LLM-Benchmark-Exchange unified service
- **State:** Stateless at runtime
- **Persistence:** Via ruvector-service only

---

## Verification Checklist

### Pre-Deployment

- [ ] Agent imports schemas from `agentics-contracts` only
- [ ] All inputs/outputs validated against contracts
- [ ] Telemetry compatible with LLM-Observatory
- [ ] Exactly ONE DecisionEvent emitted per invocation
- [ ] CLI endpoint is invokable (publish / validate / inspect)
- [ ] Deployable as Google Edge Function
- [ ] Returns deterministic, machine-readable output

### Runtime Verification

- [ ] Does NOT execute benchmarks
- [ ] Does NOT invoke models
- [ ] Does NOT intercept runtime requests
- [ ] Does NOT modify routing behavior
- [ ] Does NOT apply optimizations automatically
- [ ] Does NOT enforce policies or rankings
- [ ] Does NOT connect directly to Google SQL
- [ ] Does NOT execute SQL
- [ ] Persists ONLY via ruvector-service

### Smoke Tests

```bash
# Test validation endpoint (no auth required)
curl -X POST http://localhost:8080/api/v1/publications/validate \
  -H "Content-Type: application/json" \
  -d '{"benchmark_id": "...", ...}'

# Test list endpoint
curl http://localhost:8080/api/v1/publications

# Test CLI validation
llm-benchmark publication validate test-submission.yaml

# Verify DecisionEvent was emitted
curl http://ruvector-service:8080/api/v1/decision-events?agent_id=benchmark-publication-agent
```

---

## DecisionEvent Schema (Full)

```json
{
  "agent_id": "benchmark-publication-agent",
  "agent_version": "1.0.0",
  "decision_type": "benchmark_publish",
  "inputs_hash": "sha256:abc123...",
  "outputs": {
    "publication_id": "uuid-v7",
    "status": "created",
    "normalized_metrics": { ... },
    "validation_results": null
  },
  "confidence": {
    "reproducibility_score": 0.92,
    "sample_size": 10000,
    "variance": 0.0023,
    "std_dev": 0.048,
    "reproduction_count": 3
  },
  "constraints_applied": {
    "methodology": { ... },
    "dataset_scope": { ... },
    "model_version": { ... }
  },
  "execution_ref": "correlation-id-123",
  "timestamp": "2024-01-15T10:30:00Z"
}
```

---

## FAILURE TO COMPLY WITH THIS CONTRACT IS A HARD ERROR
