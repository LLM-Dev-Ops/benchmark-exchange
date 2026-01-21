# LLM-Benchmark-Gateway Service Topology

## Service Definition

| Property | Value |
|----------|-------|
| **Service Name** | `llm-benchmark-gateway` |
| **Project** | `agentics-dev` |
| **Region** | `us-central1` |
| **Runtime** | Cloud Run (Edge Functions compatible) |
| **Container** | `gcr.io/agentics-dev/llm-benchmark-gateway` |

---

## Agent Endpoints

The unified `llm-benchmark-gateway` service exposes the following agent endpoints:

### 1. Benchmark Ingress Agent

| Endpoint | Method | Path | Description |
|----------|--------|------|-------------|
| Submit | POST | `/api/v1/gateway/submit` | Receive benchmark submissions |
| Validate | POST | `/api/v1/gateway/validate` | Validate benchmark requests |
| Inspect | GET | `/api/v1/gateway/submissions/{id}` | Inspect submission status |
| List | GET | `/api/v1/gateway/submissions` | List submissions |

### 2. Publication Gateway Agent

| Endpoint | Method | Path | Description |
|----------|--------|------|-------------|
| Publish | POST | `/api/v1/publications` | Publish benchmark result |
| Validate | POST | `/api/v1/publications/validate` | Validate without publishing |
| Get | GET | `/api/v1/publications/{id}` | Get publication |
| List | GET | `/api/v1/publications` | List publications |
| Inspect | GET | `/api/v1/publications/{id}/inspect` | Full metadata |
| Update | PUT | `/api/v1/publications/{id}` | Update metadata |
| Status | POST | `/api/v1/publications/{id}/status` | Transition status |

### 3. Health & Telemetry Endpoints

| Endpoint | Method | Path | Description |
|----------|--------|------|-------------|
| Health | GET | `/health` | Health check |
| Ready | GET | `/ready` | Readiness probe |
| Live | GET | `/live` | Liveness probe |
| Metrics | GET | `/metrics` | Prometheus metrics |

---

## Deployment Confirmation

### ✅ Single Unified Service
- All agents deployed as ONE Cloud Run service
- No standalone agent services
- Shared runtime, configuration, and telemetry stack

### ✅ Stateless Design
- No local state persistence
- All state via ruvector-service
- Horizontal scaling enabled

### ✅ Edge Function Compatible
- Cold start optimized
- Request-scoped execution
- Deterministic behavior

---

## Internal Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                   llm-benchmark-gateway                         │
│                    (Cloud Run Service)                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────┐  ┌──────────────────┐  ┌──────────────────┐  │
│  │   Ingress    │  │   Publication    │  │     Health       │  │
│  │    Agent     │  │     Gateway      │  │    Endpoints     │  │
│  └──────┬───────┘  └────────┬─────────┘  └──────────────────┘  │
│         │                   │                                   │
│         └─────────┬─────────┘                                   │
│                   │                                             │
│         ┌─────────▼─────────┐                                   │
│         │  Shared Runtime   │                                   │
│         │  - Config         │                                   │
│         │  - Telemetry      │                                   │
│         │  - Auth           │                                   │
│         └─────────┬─────────┘                                   │
│                   │                                             │
└───────────────────┼─────────────────────────────────────────────┘
                    │
        ┌───────────┼───────────┐
        │           │           │
        ▼           ▼           ▼
┌───────────┐ ┌───────────┐ ┌───────────────┐
│ ruvector  │ │   LLM-    │ │    LLM-       │
│ -service  │ │Observatory│ │  Registry     │
└───────────┘ └───────────┘ └───────────────┘
```

---

## Resource Allocation

| Resource | Value |
|----------|-------|
| Min Instances | 1 (prod), 0 (dev/staging) |
| Max Instances | 100 (prod), 10 (dev/staging) |
| CPU | 2 vCPU |
| Memory | 2Gi |
| Concurrency | 80 |
| Timeout | 300s |

---

## Networking

| Setting | Value |
|---------|-------|
| Ingress | `internal-and-cloud-load-balancing` |
| VPC Connector | `agentics-vpc-connector` |
| Egress | `private-ranges-only` |
