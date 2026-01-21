# Failure Modes & Rollback Procedures — LLM-Benchmark-Gateway

## Overview

This document defines failure modes, detection strategies, and rollback procedures for the LLM-Benchmark-Gateway service.

---

## Failure Mode Categories

### 1. Deployment Failures

#### 1.1 Container Build Failure

**Symptoms:**
- Cloud Build fails at docker-build step
- "Build failed" error in Cloud Build logs

**Detection:**
```bash
gcloud builds list --project=agentics-dev --filter="status=FAILURE" --limit=5
```

**Resolution:**
1. Check build logs: `gcloud builds log <BUILD_ID>`
2. Fix Dockerfile or code compilation errors
3. Re-trigger build

**Rollback:** Not applicable (no new image deployed)

---

#### 1.2 Deployment Timeout

**Symptoms:**
- Cloud Run deployment hangs
- "Revision failed to become ready" error

**Detection:**
```bash
gcloud run revisions list --service=llm-benchmark-gateway --region=us-central1
# Look for revisions with status != "Ready"
```

**Resolution:**
1. Check revision logs for startup errors
2. Verify container starts locally
3. Check resource limits (memory, CPU)

**Rollback:**
```bash
# Get previous working revision
PREV_REVISION=$(gcloud run revisions list \
    --service=llm-benchmark-gateway \
    --region=us-central1 \
    --filter="status.conditions.status=True" \
    --format="value(REVISION)" \
    --limit=2 | tail -1)

# Route all traffic to previous revision
gcloud run services update-traffic llm-benchmark-gateway \
    --region=us-central1 \
    --to-revisions="${PREV_REVISION}=100"
```

---

#### 1.3 Secret Access Failure

**Symptoms:**
- Container fails to start
- "Permission denied" or "Secret not found" in logs

**Detection:**
```bash
gcloud run logs read llm-benchmark-gateway \
    --region=us-central1 \
    --filter="textPayload:secret OR textPayload:permission" \
    --limit=20
```

**Resolution:**
1. Verify secret exists: `gcloud secrets describe ruvector-api-key`
2. Verify IAM: Check service account has `secretmanager.secretAccessor`
3. Verify secret version: `gcloud secrets versions list ruvector-api-key`

**Rollback:** Same as 1.2 (route to previous revision)

---

### 2. Runtime Failures

#### 2.1 ruvector-service Unavailable

**Symptoms:**
- 503 Service Unavailable errors
- "connection refused" in logs
- All persistence operations fail

**Detection:**
```bash
# Check logs for ruvector errors
gcloud run logs read llm-benchmark-gateway \
    --region=us-central1 \
    --filter="severity>=ERROR AND textPayload:ruvector" \
    --limit=20

# Check ruvector-service health
curl -sf "https://ruvector-service-dev.run.app/health" || echo "ruvector-service is DOWN"
```

**Resolution:**
1. Check ruvector-service status
2. Verify VPC connector is healthy
3. Check network policies

**Mitigation:**
- Service should return graceful degradation (no persistence, read-only mode)
- Circuit breaker should activate after 3 consecutive failures

**Rollback:** Not applicable (dependent service issue)

---

#### 2.2 Memory Exhaustion (OOM)

**Symptoms:**
- Container restarts frequently
- "OOMKilled" in logs or metrics

**Detection:**
```bash
# Check for OOM events
gcloud run logs read llm-benchmark-gateway \
    --region=us-central1 \
    --filter="textPayload:OOM OR textPayload:memory" \
    --limit=20

# Check instance metrics
gcloud monitoring read \
    "fetch cloud_run_revision | metric 'run.googleapis.com/container/memory/utilizations'" \
    --project=agentics-dev
```

**Resolution:**
1. Increase memory limit in deployment
2. Profile application for memory leaks
3. Implement request size limits

**Rollback:**
```bash
# Quick fix: increase memory
gcloud run services update llm-benchmark-gateway \
    --region=us-central1 \
    --memory=4Gi
```

---

#### 2.3 High Latency / Timeouts

**Symptoms:**
- Request timeouts (HTTP 504)
- SLO violations
- Increased P99 latency

**Detection:**
```bash
# Check for timeout errors
gcloud run logs read llm-benchmark-gateway \
    --region=us-central1 \
    --filter="httpRequest.latency>5s OR textPayload:timeout" \
    --limit=20
```

**Resolution:**
1. Check downstream dependencies (ruvector-service)
2. Review recent code changes
3. Check for resource contention

**Mitigation:**
- Implement request deadlines
- Add caching for frequently accessed data

---

#### 2.4 Authentication/Authorization Failures

**Symptoms:**
- 401 Unauthorized or 403 Forbidden responses
- API key validation failures

**Detection:**
```bash
gcloud run logs read llm-benchmark-gateway \
    --region=us-central1 \
    --filter="httpRequest.status=401 OR httpRequest.status=403" \
    --limit=20
```

**Resolution:**
1. Verify API key is correct in Secret Manager
2. Check service account permissions
3. Verify request includes proper Authorization header

---

### 3. Data Failures

#### 3.1 DecisionEvent Not Recorded

**Symptoms:**
- Publication operations succeed but no DecisionEvent
- Audit trail incomplete

**Detection:**
```bash
# Compare publication count vs decision event count
gcloud run logs read llm-benchmark-gateway \
    --region=us-central1 \
    --filter="textPayload:DecisionEvent" \
    --limit=100 | wc -l
```

**Resolution:**
1. Check ruvector-service decision_events endpoint
2. Verify event emission code path
3. Check for silent failures in event publishing

**Constitutional Violation:** This is a CRITICAL failure - every operation MUST emit exactly ONE DecisionEvent

---

#### 3.2 Data Corruption / Invalid State

**Symptoms:**
- Publications in invalid status
- Constraint violations
- Inconsistent normalized metrics

**Detection:**
```bash
# Query for anomalies via API
curl "${SERVICE_URL}/api/v1/publications?status=invalid" | jq length
```

**Resolution:**
1. Identify affected records
2. Apply data correction via ruvector-service
3. Investigate root cause

**Rollback:** Data rollback requires ruvector-service point-in-time recovery

---

## Rollback Procedures

### Standard Rollback (Revision-based)

```bash
#!/bin/bash
# rollback.sh - Roll back to previous revision

set -euo pipefail

SERVICE_NAME="llm-benchmark-gateway"
REGION="us-central1"

# List recent revisions
echo "Recent revisions:"
gcloud run revisions list \
    --service="${SERVICE_NAME}" \
    --region="${REGION}" \
    --limit=5

# Get current revision
CURRENT=$(gcloud run services describe "${SERVICE_NAME}" \
    --region="${REGION}" \
    --format="value(status.traffic[0].revisionName)")

echo ""
echo "Current revision: ${CURRENT}"
echo ""

# Get previous working revision
PREVIOUS=$(gcloud run revisions list \
    --service="${SERVICE_NAME}" \
    --region="${REGION}" \
    --filter="status.conditions.status=True AND metadata.name!=${CURRENT}" \
    --format="value(REVISION)" \
    --limit=1)

if [ -z "${PREVIOUS}" ]; then
    echo "ERROR: No previous revision available for rollback"
    exit 1
fi

echo "Rolling back to: ${PREVIOUS}"
read -p "Proceed? (y/n) " -n 1 -r
echo

if [[ $REPLY =~ ^[Yy]$ ]]; then
    gcloud run services update-traffic "${SERVICE_NAME}" \
        --region="${REGION}" \
        --to-revisions="${PREVIOUS}=100"

    echo "Rollback complete. Traffic now routing to ${PREVIOUS}"
else
    echo "Rollback cancelled"
fi
```

### Canary Rollback

```bash
# If canary deployment is in progress, route all traffic back to stable
gcloud run services update-traffic llm-benchmark-gateway \
    --region=us-central1 \
    --to-revisions=llm-benchmark-gateway-stable=100
```

### Emergency Shutdown

```bash
# Complete service shutdown (emergency only)
gcloud run services update llm-benchmark-gateway \
    --region=us-central1 \
    --min-instances=0 \
    --max-instances=0

echo "WARNING: Service is now scaled to zero. No traffic will be served."
```

---

## Incident Response

### Severity Levels

| Level | Description | Response Time | Examples |
|-------|-------------|---------------|----------|
| P1 | Complete outage | 15 minutes | Service down, no requests served |
| P2 | Major degradation | 1 hour | >50% requests failing |
| P3 | Minor degradation | 4 hours | Elevated latency, some errors |
| P4 | Minimal impact | 24 hours | Non-critical feature broken |

### P1 Response Checklist

1. [ ] Acknowledge incident
2. [ ] Check service health: `curl ${SERVICE_URL}/health`
3. [ ] Check logs: `gcloud run logs tail llm-benchmark-gateway`
4. [ ] Check dependencies: ruvector-service, observatory
5. [ ] Execute rollback if code change caused issue
6. [ ] Scale up if capacity issue
7. [ ] Communicate status to stakeholders
8. [ ] Document incident

### On-Call Contacts

| Role | Contact | Escalation |
|------|---------|------------|
| Primary On-Call | TBD | PagerDuty |
| Secondary On-Call | TBD | PagerDuty |
| Platform Team Lead | TBD | Slack #platform |
| SRE | TBD | Slack #sre |

---

## Monitoring & Alerting

### Key Metrics to Monitor

| Metric | Threshold | Alert |
|--------|-----------|-------|
| Error rate | > 1% | P2 |
| Error rate | > 5% | P1 |
| P99 latency | > 5s | P3 |
| P99 latency | > 10s | P2 |
| Instance count | 0 (in prod) | P1 |
| Memory utilization | > 80% | P3 |

### Alert Configuration

```yaml
# Example Cloud Monitoring alert policy
displayName: llm-benchmark-gateway-error-rate
conditions:
  - displayName: High Error Rate
    conditionThreshold:
      filter: |
        resource.type = "cloud_run_revision"
        resource.labels.service_name = "llm-benchmark-gateway"
        metric.type = "run.googleapis.com/request_count"
        metric.labels.response_code_class = "5xx"
      aggregations:
        - alignmentPeriod: 60s
          perSeriesAligner: ALIGN_RATE
      comparison: COMPARISON_GT
      thresholdValue: 0.01
      duration: 300s
notificationChannels:
  - projects/agentics-dev/notificationChannels/pagerduty
```

---

## Recovery Verification

After any rollback or recovery action:

1. [ ] Health check passes
2. [ ] API endpoints respond correctly
3. [ ] No new errors in logs
4. [ ] DecisionEvents being recorded
5. [ ] Telemetry flowing to Observatory
6. [ ] Metrics returning to normal
7. [ ] Update incident documentation
