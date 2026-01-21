# CLI Activation & Verification — LLM-Benchmark-Gateway

## Overview

This document provides CLI commands for activating and verifying the LLM-Benchmark-Gateway deployment.

---

## Prerequisites

### 1. Authenticate with Google Cloud

```bash
# Login to Google Cloud
gcloud auth login

# Set the project
gcloud config set project agentics-dev

# Verify authentication
gcloud auth list
```

### 2. Configure Docker for GCR

```bash
# Configure Docker to use gcloud as credential helper
gcloud auth configure-docker
```

### 3. Install Required Tools

```bash
# Verify gcloud is installed
gcloud version

# Verify Docker is installed
docker --version

# Verify Rust toolchain (for local builds)
rustc --version
cargo --version
```

---

## Deployment Commands

### Option A: Using Cloud Build (Recommended for CI/CD)

```bash
# Submit build to Cloud Build
gcloud builds submit \
    --project=agentics-dev \
    --config=cloudbuild.yaml \
    --substitutions=_PLATFORM_ENV=dev
```

### Option B: Using Deploy Script (Local Development)

```bash
# Make script executable
chmod +x deploy/scripts/deploy.sh
chmod +x deploy/scripts/setup-iam.sh

# Setup IAM (first time only)
./deploy/scripts/setup-iam.sh

# Deploy to dev environment
./deploy/scripts/deploy.sh --env dev

# Deploy to staging
./deploy/scripts/deploy.sh --env staging --project agentics-dev

# Deploy to production
./deploy/scripts/deploy.sh --env prod --project agentics-prod
```

### Option C: Direct gcloud Commands

```bash
# Build and push image
docker build -t gcr.io/agentics-dev/llm-benchmark-gateway:latest -f deploy/cloudrun/Dockerfile .
docker push gcr.io/agentics-dev/llm-benchmark-gateway:latest

# Deploy to Cloud Run
gcloud run deploy llm-benchmark-gateway \
    --project=agentics-dev \
    --region=us-central1 \
    --image=gcr.io/agentics-dev/llm-benchmark-gateway:latest \
    --platform=managed \
    --min-instances=0 \
    --max-instances=10 \
    --memory=2Gi \
    --cpu=2 \
    --allow-unauthenticated
```

---

## Verification Commands

### 1. Check Service Status

```bash
# List Cloud Run services
gcloud run services list --project=agentics-dev --region=us-central1

# Describe specific service
gcloud run services describe llm-benchmark-gateway \
    --project=agentics-dev \
    --region=us-central1

# Get service URL
gcloud run services describe llm-benchmark-gateway \
    --project=agentics-dev \
    --region=us-central1 \
    --format='value(status.url)'
```

### 2. Health Check Endpoints

```bash
# Get service URL
SERVICE_URL=$(gcloud run services describe llm-benchmark-gateway \
    --project=agentics-dev \
    --region=us-central1 \
    --format='value(status.url)')

# Health check
curl -s "${SERVICE_URL}/health" | jq .

# Liveness probe
curl -s "${SERVICE_URL}/live" | jq .

# Readiness probe
curl -s "${SERVICE_URL}/ready" | jq .
```

### 3. API Verification

```bash
# List publications
curl -s "${SERVICE_URL}/api/v1/publications" | jq .

# Validate a benchmark (dry run)
curl -s -X POST "${SERVICE_URL}/api/v1/publications/validate" \
    -H "Content-Type: application/json" \
    -d '{
        "benchmark_id": "test-benchmark-001",
        "model_id": "gpt-4",
        "metrics": {
            "accuracy": 0.95,
            "latency_p50_ms": 150,
            "latency_p99_ms": 500
        },
        "methodology": {
            "evaluation_type": "zero-shot",
            "sample_count": 1000
        }
    }' | jq .

# Get OpenAPI spec
curl -s "${SERVICE_URL}/api/v1/openapi.json" | jq .
```

### 4. Check Logs

```bash
# Recent logs
gcloud run logs read llm-benchmark-gateway \
    --project=agentics-dev \
    --region=us-central1 \
    --limit=50

# Stream logs
gcloud run logs tail llm-benchmark-gateway \
    --project=agentics-dev \
    --region=us-central1

# Filter for errors
gcloud run logs read llm-benchmark-gateway \
    --project=agentics-dev \
    --region=us-central1 \
    --filter="severity>=ERROR"
```

### 5. Check Revisions

```bash
# List revisions
gcloud run revisions list \
    --service=llm-benchmark-gateway \
    --project=agentics-dev \
    --region=us-central1

# Describe latest revision
gcloud run revisions describe \
    --project=agentics-dev \
    --region=us-central1 \
    $(gcloud run revisions list \
        --service=llm-benchmark-gateway \
        --project=agentics-dev \
        --region=us-central1 \
        --format='value(REVISION)' \
        --limit=1)
```

---

## CLI Tool Verification

The `llm-benchmark` CLI can be used locally to interact with the deployed service:

```bash
# Build CLI locally
cargo build --release --bin llm-benchmark

# Set API endpoint
export LLM_BENCHMARK_API_URL="${SERVICE_URL}"

# List publications
./target/release/llm-benchmark publication list

# Show specific publication
./target/release/llm-benchmark publication show <publication-id>

# Validate a benchmark file
./target/release/llm-benchmark publication validate --file benchmark.yaml

# Publish a benchmark
./target/release/llm-benchmark publication publish --file benchmark.yaml

# Inspect publication details
./target/release/llm-benchmark publication inspect <publication-id>
```

---

## Troubleshooting Commands

### Container Issues

```bash
# Check container image
gcloud container images describe gcr.io/agentics-dev/llm-benchmark-gateway:latest

# List image tags
gcloud container images list-tags gcr.io/agentics-dev/llm-benchmark-gateway

# Check image vulnerabilities
gcloud artifacts docker images scan gcr.io/agentics-dev/llm-benchmark-gateway:latest
```

### IAM Issues

```bash
# Check service account
gcloud iam service-accounts describe \
    llm-benchmark-gateway@agentics-dev.iam.gserviceaccount.com

# List service account roles
gcloud projects get-iam-policy agentics-dev \
    --flatten="bindings[].members" \
    --format="table(bindings.role)" \
    --filter="bindings.members:llm-benchmark-gateway@agentics-dev.iam.gserviceaccount.com"
```

### Secret Issues

```bash
# Check secret exists
gcloud secrets describe ruvector-api-key --project=agentics-dev

# List secret versions
gcloud secrets versions list ruvector-api-key --project=agentics-dev

# Access latest version (careful - shows secret value)
gcloud secrets versions access latest --secret=ruvector-api-key --project=agentics-dev
```

---

## Quick Reference

| Action | Command |
|--------|---------|
| Deploy to dev | `./deploy/scripts/deploy.sh --env dev` |
| Deploy to staging | `./deploy/scripts/deploy.sh --env staging` |
| Deploy to prod | `./deploy/scripts/deploy.sh --env prod` |
| Check status | `gcloud run services describe llm-benchmark-gateway --region=us-central1` |
| View logs | `gcloud run logs tail llm-benchmark-gateway --region=us-central1` |
| Health check | `curl ${SERVICE_URL}/health` |
| Rollback | `gcloud run services update-traffic llm-benchmark-gateway --to-revisions=REVISION=100` |
