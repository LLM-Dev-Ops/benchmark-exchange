# Persistence Wiring Confirmation — LLM-Benchmark-Gateway

## Constitutional Requirement

> **NO direct SQL access. ALL persistence flows through ruvector-service.**

This document confirms the persistence architecture and wiring for the LLM-Benchmark-Gateway service.

---

## Persistence Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        LLM-Benchmark-Gateway                             │
│                         (Cloud Run Service)                              │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌────────────────┐    ┌─────────────────┐    ┌────────────────────┐   │
│  │  Publication   │───▶│  RuVectorClient │───▶│  HTTP/gRPC Client  │   │
│  │    Service     │    │    (Adapter)    │    │                    │   │
│  └────────────────┘    └─────────────────┘    └─────────┬──────────┘   │
│                                                          │              │
└──────────────────────────────────────────────────────────┼──────────────┘
                                                           │
                                                           │ HTTPS (mTLS)
                                                           ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         ruvector-service                                 │
│                        (Persistence Layer)                               │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌────────────────┐    ┌─────────────────┐    ┌────────────────────┐   │
│  │   API Layer    │───▶│ Business Logic  │───▶│  Repository Layer  │   │
│  │   (gRPC/REST)  │    │                 │    │                    │   │
│  └────────────────┘    └─────────────────┘    └─────────┬──────────┘   │
│                                                          │              │
└──────────────────────────────────────────────────────────┼──────────────┘
                                                           │
                                                           │ Private IP
                                                           ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        Cloud SQL (PostgreSQL)                            │
│                     (Managed Database Instance)                          │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  • Instance: benchmark-memory-{env}                                      │
│  • Database: benchmarks                                                  │
│  • Tables: publications, decision_events, normalized_metrics             │
│  • Connection: Private VPC only (no public IP)                           │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Wiring Confirmation

### 1. Environment Variables

| Variable | Purpose | Source |
|----------|---------|--------|
| `RUVECTOR_SERVICE_URL` | ruvector-service endpoint | Service configuration |
| `RUVECTOR_API_KEY` | Authentication token | Secret Manager |

### 2. Service Account Permissions

The `llm-benchmark-gateway` service account has:
- ✅ `roles/run.invoker` on ruvector-service
- ✅ `roles/secretmanager.secretAccessor` for API key access
- ✅ NO Cloud SQL permissions (by design)

### 3. Network Configuration

```yaml
# VPC Connector for internal communication
run.googleapis.com/vpc-access-connector: agentics-vpc-connector
run.googleapis.com/vpc-access-egress: private-ranges-only
```

### 4. No Direct SQL Access

The following confirms NO direct SQL connections:

```rust
// crates/infrastructure/src/external_consumers/ruvector.rs
// ALL database operations go through RuVectorClient

pub trait RuVectorClient: Send + Sync {
    async fn store_publication(&self, publication: &Publication) -> Result<(), RuVectorError>;
    async fn get_publication(&self, id: &str) -> Result<Option<Publication>, RuVectorError>;
    async fn query_publications(&self, filter: &PublicationFilter) -> Result<Vec<Publication>, RuVectorError>;
    async fn store_decision_event(&self, event: &DecisionEvent) -> Result<(), RuVectorError>;
}
```

**No `sqlx`, `diesel`, `postgres`, or `tokio-postgres` direct dependencies in benchmark-gateway.**

---

## RuVector API Contract

### Store Publication

```http
POST /api/v1/publications
Authorization: Bearer {RUVECTOR_API_KEY}
Content-Type: application/json

{
  "id": "01916a2b-7c3d-7000-8000-000000000001",
  "benchmark_id": "benchmark-001",
  "status": "published",
  "confidence": {
    "reproducibility_score": 0.95,
    "sample_size": 1000,
    "variance": 0.02
  },
  "normalized_metrics": { ... },
  "published_at": "2024-01-15T10:30:00Z"
}
```

### Query Publications

```http
GET /api/v1/publications?benchmark_id=xxx&status=published
Authorization: Bearer {RUVECTOR_API_KEY}
```

### Store Decision Event

```http
POST /api/v1/decision-events
Authorization: Bearer {RUVECTOR_API_KEY}
Content-Type: application/json

{
  "id": "01916a2b-7c3d-7000-8000-000000000002",
  "publication_id": "01916a2b-7c3d-7000-8000-000000000001",
  "decision_type": "benchmark_publish",
  "actor": "publication-agent",
  "timestamp": "2024-01-15T10:30:00Z",
  "outcome": "approved",
  "rationale": "All validation checks passed"
}
```

---

## Verification Checklist

- [x] No SQL drivers in Cargo.toml (benchmark-gateway crates)
- [x] All persistence calls go through RuVectorClient trait
- [x] RUVECTOR_SERVICE_URL configured in all environments
- [x] RUVECTOR_API_KEY stored in Secret Manager
- [x] VPC connector configured for internal communication
- [x] Service account has NO Cloud SQL roles
- [x] DecisionEvent stored for every operation

---

## Telemetry Emission

All operations emit telemetry to LLM-Observatory:

```rust
// Every RuVectorClient method emits telemetry
async fn store_publication(&self, publication: &Publication) -> Result<(), RuVectorError> {
    let span = tracing::info_span!("ruvector.store_publication",
        publication_id = %publication.id,
        benchmark_id = %publication.benchmark_id
    );
    // ... operation with telemetry
}
```

---

## Confirmation Statement

> **This document confirms that LLM-Benchmark-Gateway has NO direct SQL access.**
> **ALL persistence operations flow through ruvector-service via authenticated HTTP/gRPC.**
> **The service adheres to the constitutional requirement for persistence isolation.**

Confirmed by: Deployment Configuration
Date: 2024-01-15
