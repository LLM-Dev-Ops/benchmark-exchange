# Post-Deploy Verification Checklist — LLM-Benchmark-Gateway

## Pre-Deployment Verification

### Infrastructure Ready
- [ ] GCP Project configured (`agentics-dev`)
- [ ] Service account created (`llm-benchmark-gateway@agentics-dev.iam.gserviceaccount.com`)
- [ ] IAM roles assigned (run.invoker, secretmanager.secretAccessor)
- [ ] Secret Manager contains `ruvector-api-key`
- [ ] VPC connector available (`agentics-vpc-connector`)
- [ ] Container image built and pushed to GCR

### Dependencies Verified
- [ ] ruvector-service is deployed and healthy
- [ ] LLM-Observatory telemetry endpoint is reachable
- [ ] LLM-Registry service is available (if applicable)
- [ ] LLM-Exchange service is available (if applicable)

---

## Post-Deployment Verification

### 1. Service Health (Critical)

```bash
# Execute these commands after deployment

SERVICE_URL=$(gcloud run services describe llm-benchmark-gateway \
    --project=agentics-dev \
    --region=us-central1 \
    --format='value(status.url)')

# Health endpoint
curl -sf "${SERVICE_URL}/health" && echo "✓ Health check passed" || echo "✗ Health check FAILED"

# Liveness probe
curl -sf "${SERVICE_URL}/live" && echo "✓ Liveness check passed" || echo "✗ Liveness check FAILED"

# Readiness probe
curl -sf "${SERVICE_URL}/ready" && echo "✓ Readiness check passed" || echo "✗ Readiness check FAILED"
```

**Expected Result:**
- [ ] `/health` returns HTTP 200
- [ ] `/live` returns HTTP 200
- [ ] `/ready` returns HTTP 200

### 2. API Endpoints Functional

```bash
# List publications (should return empty array or existing data)
curl -s "${SERVICE_URL}/api/v1/publications" | jq .

# Validate endpoint responds
curl -sf -X POST "${SERVICE_URL}/api/v1/publications/validate" \
    -H "Content-Type: application/json" \
    -d '{"benchmark_id":"test","model_id":"test","metrics":{}}' \
    && echo "✓ Validate endpoint responded" || echo "✗ Validate endpoint FAILED"

# OpenAPI spec available
curl -sf "${SERVICE_URL}/api/v1/openapi.json" > /dev/null \
    && echo "✓ OpenAPI spec available" || echo "✗ OpenAPI spec FAILED"
```

**Expected Result:**
- [ ] `GET /api/v1/publications` returns 200
- [ ] `POST /api/v1/publications/validate` responds (may return 400 for invalid data)
- [ ] `GET /api/v1/openapi.json` returns OpenAPI spec

### 3. Persistence Connectivity (ruvector-service)

```bash
# Check if service can reach ruvector-service
# This is verified through a successful publication operation

# Note: ruvector-service connectivity is internal-only
# Verification is done through application logs
gcloud run logs read llm-benchmark-gateway \
    --project=agentics-dev \
    --region=us-central1 \
    --filter="textPayload:ruvector" \
    --limit=10
```

**Expected Result:**
- [ ] No "connection refused" errors in logs
- [ ] No "authentication failed" errors for ruvector-api-key
- [ ] Successful persistence operations logged

### 4. Environment Configuration

```bash
# Verify environment variables are set
gcloud run services describe llm-benchmark-gateway \
    --project=agentics-dev \
    --region=us-central1 \
    --format='yaml(spec.template.spec.containers[0].env)'
```

**Expected Result:**
- [ ] SERVICE_NAME is set
- [ ] SERVICE_VERSION is set
- [ ] PLATFORM_ENV matches deployment target
- [ ] RUVECTOR_SERVICE_URL is set
- [ ] RUVECTOR_API_KEY references secret
- [ ] TELEMETRY_ENDPOINT is set (if telemetry enabled)

### 5. Scaling Configuration

```bash
# Check scaling annotations
gcloud run services describe llm-benchmark-gateway \
    --project=agentics-dev \
    --region=us-central1 \
    --format='yaml(spec.template.metadata.annotations)'
```

**Expected Result:**
- [ ] minScale matches environment (dev: 0, staging: 1, prod: 2)
- [ ] maxScale matches environment (dev: 3, staging: 5, prod: 20)
- [ ] CPU throttling is disabled (`run.googleapis.com/cpu-throttling: "false"`)

### 6. Security Configuration

```bash
# Verify service account
gcloud run services describe llm-benchmark-gateway \
    --project=agentics-dev \
    --region=us-central1 \
    --format='value(spec.template.spec.serviceAccountName)'

# Verify ingress setting
gcloud run services describe llm-benchmark-gateway \
    --project=agentics-dev \
    --region=us-central1 \
    --format='value(metadata.annotations.run.googleapis.com/ingress)'
```

**Expected Result:**
- [ ] Service account is `llm-benchmark-gateway@agentics-dev.iam.gserviceaccount.com`
- [ ] Ingress is `internal-and-cloud-load-balancing` (not `all`)
- [ ] NO Cloud SQL roles assigned to service account

### 7. Telemetry & Observability

```bash
# Check recent logs for telemetry emission
gcloud run logs read llm-benchmark-gateway \
    --project=agentics-dev \
    --region=us-central1 \
    --filter="textPayload:telemetry" \
    --limit=10

# Check for tracing
gcloud run logs read llm-benchmark-gateway \
    --project=agentics-dev \
    --region=us-central1 \
    --filter="trace" \
    --limit=5
```

**Expected Result:**
- [ ] Telemetry events being emitted
- [ ] Trace IDs present in logs
- [ ] No telemetry connection errors

---

## Functional Verification

### 8. End-to-End Publication Flow

```bash
# Create a test publication
curl -X POST "${SERVICE_URL}/api/v1/publications" \
    -H "Content-Type: application/json" \
    -d '{
        "benchmark_id": "verification-test-001",
        "model_id": "gpt-4",
        "metrics": {
            "accuracy": 0.95,
            "latency_p50_ms": 150
        },
        "methodology": {
            "evaluation_type": "zero-shot",
            "sample_count": 1000
        }
    }' | tee /tmp/publication-response.json | jq .

# Extract publication ID
PUBLICATION_ID=$(jq -r '.id' /tmp/publication-response.json)

# Verify publication was stored (through ruvector-service)
curl -s "${SERVICE_URL}/api/v1/publications/${PUBLICATION_ID}" | jq .

# Verify DecisionEvent was created
gcloud run logs read llm-benchmark-gateway \
    --project=agentics-dev \
    --region=us-central1 \
    --filter="textPayload:DecisionEvent AND textPayload:${PUBLICATION_ID}" \
    --limit=5
```

**Expected Result:**
- [ ] Publication created successfully
- [ ] Publication ID returned (UUID v7 format)
- [ ] Publication retrievable by ID
- [ ] DecisionEvent logged for the operation

---

## Performance Verification

### 9. Response Times

```bash
# Measure health check latency
time curl -s "${SERVICE_URL}/health" > /dev/null

# Measure API latency
time curl -s "${SERVICE_URL}/api/v1/publications" > /dev/null
```

**Expected Result:**
- [ ] Health check < 100ms
- [ ] API list < 500ms (cold start may be longer)

### 10. Cold Start Time

```bash
# Scale to zero and measure cold start
gcloud run services update llm-benchmark-gateway \
    --project=agentics-dev \
    --region=us-central1 \
    --min-instances=0

# Wait for scale-down
sleep 120

# Measure cold start
time curl -s "${SERVICE_URL}/health" > /dev/null
```

**Expected Result:**
- [ ] Cold start < 10 seconds

---

## Checklist Summary

| Category | Check | Status |
|----------|-------|--------|
| **Health** | Health endpoint | ☐ |
| **Health** | Liveness probe | ☐ |
| **Health** | Readiness probe | ☐ |
| **API** | List publications | ☐ |
| **API** | Validate endpoint | ☐ |
| **API** | OpenAPI spec | ☐ |
| **Persistence** | ruvector-service connectivity | ☐ |
| **Config** | Environment variables set | ☐ |
| **Scaling** | Min/Max instances correct | ☐ |
| **Security** | Service account configured | ☐ |
| **Security** | Ingress restricted | ☐ |
| **Security** | No SQL permissions | ☐ |
| **Telemetry** | Events being emitted | ☐ |
| **E2E** | Publication create/retrieve | ☐ |
| **E2E** | DecisionEvent logged | ☐ |
| **Perf** | Response times acceptable | ☐ |

---

## Sign-Off

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Deployer | | | |
| Reviewer | | | |
| Platform Owner | | | |
