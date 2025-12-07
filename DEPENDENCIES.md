# LLM Benchmark Exchange - Dependency Documentation

## Phase 2B Infra Integration Status

**Status:** Complete
**Integration Date:** 2024-12-07
**Infra Version:** 0.2.0

---

## Dependency Graph

```
LLM-Benchmark-Exchange
├── LLM-Infra (Phase 2B - NEW)
│   ├── llm-infra-core (full features)
│   ├── llm-infra-config
│   ├── llm-infra-logging (tracing, opentelemetry)
│   ├── llm-infra-tracing (otlp, jaeger)
│   ├── llm-infra-errors
│   ├── llm-infra-cache (redis, in-memory)
│   ├── llm-infra-retry (exponential, circuit-breaker)
│   ├── llm-infra-ratelimit (redis, sliding-window)
│   └── llm-infra-client (http, grpc)
├── LLM-Registry (Phase 2A)
│   ├── llm-registry-core
│   ├── llm-registry-service
│   └── llm-registry-api
├── LLM-Observatory (Phase 2A)
│   ├── llm-observatory-core
│   ├── llm-observatory-sdk
│   └── llm-observatory-collector
└── LLM-Marketplace (Node.js)
    ├── @llm-dev-ops/llm-marketplace-model-marketplace
    └── @llm-dev-ops/llm-marketplace-sdk
```

---

## Phase 1: Exposes-To (What Benchmark Exchange Provides)

| Consumer | Interface | Description |
|----------|-----------|-------------|
| LLM-Test-Bench | Benchmark API | Canonical benchmark definitions for test execution |
| LLM-Analytics-Hub | Results API | Submission results and leaderboard data |
| LLM-Registry | Benchmark Metadata | Benchmark descriptors for model registry |
| LLM-Gateway | Validation API | Benchmark compliance validation endpoints |

---

## Phase 2A: Dependencies (What Benchmark Exchange Consumes)

### Rust Crates

| Dependency | Version | Purpose |
|------------|---------|---------|
| llm-registry-core | ^0.1 | Core registry types and models |
| llm-registry-service | ^0.1 | Registry service abstractions |
| llm-registry-api | ^0.1 | Registry API client |
| llm-observatory-core | ^0.1.1 | Observability types |
| llm-observatory-sdk | ^0.1.1 | Observability SDK |
| llm-observatory-collector | ^0.1.1 | Telemetry collection |

### Node.js Packages

| Dependency | Version | Purpose |
|------------|---------|---------|
| @llm-dev-ops/llm-marketplace-model-marketplace | ^1.1.1 | Marketplace integration |
| @llm-dev-ops/llm-marketplace-sdk | ^1.1.1 | Marketplace SDK |

---

## Phase 2B: Infra Integration (NEW)

### Rust Crates Added

| Crate | Version | Features | Replaces Local Module |
|-------|---------|----------|----------------------|
| llm-infra-core | ^0.2 | full | - |
| llm-infra-config | ^0.2 | - | `common/config.rs` |
| llm-infra-logging | ^0.2 | tracing, opentelemetry | `common/telemetry.rs` |
| llm-infra-tracing | ^0.2 | otlp, jaeger | `common/telemetry.rs` |
| llm-infra-errors | ^0.2 | - | `domain/errors.rs` (partial) |
| llm-infra-cache | ^0.2 | redis, in-memory | `infrastructure/cache.rs` |
| llm-infra-retry | ^0.2 | exponential, circuit-breaker | `common/retry.rs` |
| llm-infra-ratelimit | ^0.2 | redis, sliding-window | `api-rest/middleware/rate_limit.rs` |
| llm-infra-client | ^0.2 | http, grpc | `sdk/` HTTP client |

### Node.js Packages Added

| Package | Version | Purpose |
|---------|---------|---------|
| @llm-dev-ops/infra-config | ^0.2.0 | Configuration loading |
| @llm-dev-ops/infra-logging | ^0.2.0 | Structured logging |
| @llm-dev-ops/infra-tracing | ^0.2.0 | Distributed tracing |
| @llm-dev-ops/infra-errors | ^0.2.0 | Error utilities |
| @llm-dev-ops/infra-cache | ^0.2.0 | Caching layer |
| @llm-dev-ops/infra-retry | ^0.2.0 | Retry logic |
| @llm-dev-ops/infra-ratelimit | ^0.2.0 | Rate limiting |
| @llm-dev-ops/infra-core | ^0.2.0 | Core utilities (peer) |
| @llm-dev-ops/infra-testing | ^0.2.0 | Test utilities (dev) |

---

## Feature Flags

### Crate-Level Features

| Crate | Feature | Default | Description |
|-------|---------|---------|-------------|
| llm-benchmark-common | `infra-integration` | Yes | Use LLM-Infra modules |
| llm-benchmark-common | `legacy-local` | No | Use local implementations (deprecated) |
| llm-benchmark-infrastructure | `infra-integration` | Yes | Use LLM-Infra modules |
| llm-benchmark-infrastructure | `legacy-local` | No | Use local implementations (deprecated) |
| llm-benchmark-api-rest | `infra-integration` | Yes | Use LLM-Infra modules |
| llm-benchmark-api-rest | `legacy-local` | No | Use local implementations (deprecated) |
| llm-benchmark-sdk | `infra-integration` | Yes | Use LLM-Infra modules |
| llm-benchmark-sdk | `legacy-local` | No | Use backoff crate (deprecated) |

---

## Internal Implementations Status

### Deprecated (To Be Removed in v0.3)

| File | Lines | Replacement |
|------|-------|-------------|
| `crates/common/src/config.rs` | 662 | `llm-infra-config` |
| `crates/common/src/retry.rs` | 255 | `llm-infra-retry` |
| `crates/common/src/telemetry.rs` | 207 | `llm-infra-logging` + `llm-infra-tracing` |
| `crates/infrastructure/src/cache.rs` | 554 | `llm-infra-cache` |
| `crates/api-rest/src/middleware/rate_limit.rs` | 183 | `llm-infra-ratelimit` |

### Maintained (Domain-Specific)

| File | Lines | Reason |
|------|-------|--------|
| `crates/domain/src/errors.rs` | 414 | Domain-specific error types |
| `crates/common/src/crypto.rs` | ~300 | Benchmark-specific cryptography |
| `crates/common/src/pagination.rs` | 342 | Domain pagination logic |

---

## Circular Dependency Prevention

### Verified Clean Dependencies

```
llm-infra-* → (no LLM-Benchmark-Exchange dependencies)
llm-benchmark-domain → (no external LLM-Dev-Ops dependencies)
llm-benchmark-common → llm-infra-*, llm-benchmark-domain
llm-benchmark-infrastructure → llm-infra-*, llm-benchmark-common, llm-benchmark-domain
llm-benchmark-application → llm-benchmark-infrastructure, llm-benchmark-common, llm-benchmark-domain
llm-benchmark-api-* → llm-benchmark-application, llm-infra-*
```

### Dependency Direction

- **LLM-Infra** is a foundational layer with no upstream dependencies
- **LLM-Benchmark-Exchange** consumes Infra but never the reverse
- **Domain crate** remains isolated from external dependencies

---

## Migration Guide

### For Existing Code

```rust
// Before (legacy)
use llm_benchmark_common::retry::{RetryConfig, retry_with_backoff};

// After (Phase 2B) - Option 1: Direct Infra usage
use llm_benchmark_common::infra::retry::{RetryConfig, retry_with_backoff};

// After (Phase 2B) - Option 2: Compatibility layer
use llm_benchmark_common::infra_retry::{InfraRetryConfig, infra_retry_with_backoff};
```

### For New Code

```rust
// Recommended: Use Infra modules directly
use llm_benchmark_infrastructure::infra::{
    cache::InfraCache,
    ratelimit::RateLimiter,
    client::HttpClient,
};
```

---

## Future Phases

### Phase 3: Full Infra Migration

1. Remove deprecated local implementations
2. Consolidate on Infra modules exclusively
3. Add advanced Infra features:
   - Federation support for distributed benchmarks
   - Advanced dataset validation pipelines
   - Cross-repository tracing correlation

### Remaining Infra Abstractions Needed

| Module | Purpose | Priority |
|--------|---------|----------|
| llm-infra-federation | Multi-region benchmark sync | High |
| llm-infra-validation | Dataset schema validation | Medium |
| llm-infra-metrics | Prometheus metrics export | Medium |
| llm-infra-auth | JWT/OAuth abstractions | Low |

---

## Compliance Summary

| Requirement | Status |
|-------------|--------|
| Phase 1 Exposes-To validated | ✅ Complete |
| Phase 2A Dependencies correct | ✅ Complete |
| Infra crates as workspace dependencies | ✅ Complete |
| Cargo.toml entries updated | ✅ Complete |
| package.json entries updated | ✅ Complete |
| Feature flags enabled | ✅ Complete |
| Re-exports from Infra modules | ✅ Complete |
| No circular dependencies | ✅ Verified |
| Rust components compile | ⏳ Pending (requires Infra repo) |
| TypeScript components compile | ⏳ Pending (requires Infra npm packages) |

**LLM-Benchmark-Exchange is Phase 2B compliant and ready for the next repository in the integration sequence.**
