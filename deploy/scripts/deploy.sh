#!/bin/bash
# Deployment Script for LLM-Benchmark-Gateway
# Deploys to Google Cloud Run with environment-specific configuration

set -euo pipefail

# Default configuration
PROJECT_ID="${PROJECT_ID:-agentics-dev}"
REGION="${REGION:-us-central1}"
PLATFORM_ENV="${PLATFORM_ENV:-dev}"
SERVICE_NAME="llm-benchmark-gateway"
SERVICE_VERSION="${SERVICE_VERSION:-$(git rev-parse --short HEAD 2>/dev/null || echo 'latest')}"

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --env)
            PLATFORM_ENV="$2"
            shift 2
            ;;
        --project)
            PROJECT_ID="$2"
            shift 2
            ;;
        --region)
            REGION="$2"
            shift 2
            ;;
        --version)
            SERVICE_VERSION="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --help)
            echo "Usage: deploy.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --env ENV        Environment (dev, staging, prod). Default: dev"
            echo "  --project ID     GCP Project ID. Default: agentics-dev"
            echo "  --region REGION  GCP Region. Default: us-central1"
            echo "  --version VER    Service version. Default: git short SHA"
            echo "  --dry-run        Show what would be deployed without deploying"
            echo "  --help           Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Load environment-specific configuration
ENV_FILE="deploy/config/env.${PLATFORM_ENV}.yaml"
if [ ! -f "${ENV_FILE}" ]; then
    echo "ERROR: Environment file not found: ${ENV_FILE}"
    exit 1
fi

echo "=== LLM-Benchmark-Gateway Deployment ==="
echo "Project:     ${PROJECT_ID}"
echo "Region:      ${REGION}"
echo "Environment: ${PLATFORM_ENV}"
echo "Version:     ${SERVICE_VERSION}"
echo "Config:      ${ENV_FILE}"
echo ""

# Set scaling based on environment
case ${PLATFORM_ENV} in
    dev)
        MIN_INSTANCES=0
        MAX_INSTANCES=3
        ;;
    staging)
        MIN_INSTANCES=1
        MAX_INSTANCES=5
        ;;
    prod)
        MIN_INSTANCES=2
        MAX_INSTANCES=20
        ;;
    *)
        echo "ERROR: Unknown environment: ${PLATFORM_ENV}"
        exit 1
        ;;
esac

# Parse environment variables from YAML
parse_env_var() {
    grep "^${1}:" "${ENV_FILE}" | sed 's/^[^:]*: *//' | tr -d '"'
}

RUVECTOR_SERVICE_URL=$(parse_env_var "RUVECTOR_SERVICE_URL")
TELEMETRY_ENDPOINT=$(parse_env_var "TELEMETRY_ENDPOINT")
RUST_LOG=$(parse_env_var "RUST_LOG")
LLM_REGISTRY_URL=$(parse_env_var "LLM_REGISTRY_URL")
LLM_EXCHANGE_URL=$(parse_env_var "LLM_EXCHANGE_URL")

IMAGE_TAG="gcr.io/${PROJECT_ID}/${SERVICE_NAME}:${SERVICE_VERSION}"

if [ "${DRY_RUN:-false}" = "true" ]; then
    echo "[DRY RUN] Would execute the following deployment:"
    echo ""
    echo "gcloud run deploy ${SERVICE_NAME} \\"
    echo "    --project=${PROJECT_ID} \\"
    echo "    --region=${REGION} \\"
    echo "    --image=${IMAGE_TAG} \\"
    echo "    --platform=managed \\"
    echo "    --min-instances=${MIN_INSTANCES} \\"
    echo "    --max-instances=${MAX_INSTANCES} \\"
    echo "    --memory=2Gi \\"
    echo "    --cpu=2 \\"
    echo "    --timeout=300 \\"
    echo "    --concurrency=80 \\"
    echo "    --service-account=${SERVICE_NAME}@${PROJECT_ID}.iam.gserviceaccount.com \\"
    echo "    --set-env-vars=SERVICE_NAME=${SERVICE_NAME} \\"
    echo "    --set-env-vars=SERVICE_VERSION=${SERVICE_VERSION} \\"
    echo "    --set-env-vars=PLATFORM_ENV=${PLATFORM_ENV} \\"
    echo "    --set-env-vars=RUVECTOR_SERVICE_URL=${RUVECTOR_SERVICE_URL} \\"
    echo "    --set-env-vars=TELEMETRY_ENDPOINT=${TELEMETRY_ENDPOINT} \\"
    echo "    --set-env-vars=RUST_LOG=${RUST_LOG} \\"
    echo "    --set-secrets=RUVECTOR_API_KEY=ruvector-api-key:latest \\"
    echo "    --ingress=internal-and-cloud-load-balancing"
    exit 0
fi

# Step 1: Build the Docker image
echo "Step 1: Building Docker image..."
docker build \
    -t "${IMAGE_TAG}" \
    -f deploy/cloudrun/Dockerfile \
    .

# Step 2: Push to Google Container Registry
echo ""
echo "Step 2: Pushing image to GCR..."
docker push "${IMAGE_TAG}"

# Step 3: Deploy to Cloud Run
echo ""
echo "Step 3: Deploying to Cloud Run..."
gcloud run deploy "${SERVICE_NAME}" \
    --project="${PROJECT_ID}" \
    --region="${REGION}" \
    --image="${IMAGE_TAG}" \
    --platform=managed \
    --min-instances="${MIN_INSTANCES}" \
    --max-instances="${MAX_INSTANCES}" \
    --memory=2Gi \
    --cpu=2 \
    --timeout=300 \
    --concurrency=80 \
    --service-account="${SERVICE_NAME}@${PROJECT_ID}.iam.gserviceaccount.com" \
    --set-env-vars="SERVICE_NAME=${SERVICE_NAME}" \
    --set-env-vars="SERVICE_VERSION=${SERVICE_VERSION}" \
    --set-env-vars="PLATFORM_ENV=${PLATFORM_ENV}" \
    --set-env-vars="RUVECTOR_SERVICE_URL=${RUVECTOR_SERVICE_URL}" \
    --set-env-vars="TELEMETRY_ENDPOINT=${TELEMETRY_ENDPOINT}" \
    --set-env-vars="RUST_LOG=${RUST_LOG}" \
    --set-env-vars="LLM_REGISTRY_URL=${LLM_REGISTRY_URL}" \
    --set-env-vars="LLM_EXCHANGE_URL=${LLM_EXCHANGE_URL}" \
    --set-secrets="RUVECTOR_API_KEY=ruvector-api-key:latest" \
    --ingress=internal-and-cloud-load-balancing \
    --allow-unauthenticated

# Step 4: Get service URL
echo ""
echo "Step 4: Retrieving service URL..."
SERVICE_URL=$(gcloud run services describe "${SERVICE_NAME}" \
    --project="${PROJECT_ID}" \
    --region="${REGION}" \
    --format='value(status.url)')

echo ""
echo "=== Deployment Complete ==="
echo ""
echo "Service URL: ${SERVICE_URL}"
echo ""

# Step 5: Health check verification
echo "Step 5: Running health check..."
for i in {1..12}; do
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" "${SERVICE_URL}/health" 2>/dev/null || echo "000")
    if [ "${STATUS}" = "200" ]; then
        echo "  Health check PASSED (HTTP 200)"
        break
    fi
    echo "  Waiting for service... (attempt ${i}/12, status: ${STATUS})"
    sleep 5
done

if [ "${STATUS}" != "200" ]; then
    echo "  WARNING: Health check did not pass within 60 seconds"
    echo "  Check logs: gcloud run logs read ${SERVICE_NAME} --project=${PROJECT_ID} --region=${REGION}"
fi

echo ""
echo "Deployment Summary:"
echo "  Service:     ${SERVICE_NAME}"
echo "  Version:     ${SERVICE_VERSION}"
echo "  Environment: ${PLATFORM_ENV}"
echo "  URL:         ${SERVICE_URL}"
echo "  Instances:   ${MIN_INSTANCES}-${MAX_INSTANCES}"
echo ""
