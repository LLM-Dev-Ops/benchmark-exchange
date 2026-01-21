#!/bin/bash
# Rollback Script for LLM-Benchmark-Gateway
# Rolls back to previous working revision

set -euo pipefail

PROJECT_ID="${PROJECT_ID:-agentics-dev}"
REGION="${REGION:-us-central1}"
SERVICE_NAME="llm-benchmark-gateway"

echo "=== LLM-Benchmark-Gateway Rollback ==="
echo "Project: ${PROJECT_ID}"
echo "Region: ${REGION}"
echo ""

# List recent revisions
echo "Recent revisions:"
gcloud run revisions list \
    --service="${SERVICE_NAME}" \
    --project="${PROJECT_ID}" \
    --region="${REGION}" \
    --limit=5

echo ""

# Get current revision
CURRENT=$(gcloud run services describe "${SERVICE_NAME}" \
    --project="${PROJECT_ID}" \
    --region="${REGION}" \
    --format="value(status.traffic[0].revisionName)")

echo "Current revision: ${CURRENT}"
echo ""

# Get previous working revision
PREVIOUS=$(gcloud run revisions list \
    --service="${SERVICE_NAME}" \
    --project="${PROJECT_ID}" \
    --region="${REGION}" \
    --filter="status.conditions.status=True AND metadata.name!=${CURRENT}" \
    --format="value(REVISION)" \
    --limit=1)

if [ -z "${PREVIOUS}" ]; then
    echo "ERROR: No previous revision available for rollback"
    exit 1
fi

echo "Rolling back to: ${PREVIOUS}"
echo ""
read -p "Proceed with rollback? (y/n) " -n 1 -r
echo

if [[ $REPLY =~ ^[Yy]$ ]]; then
    gcloud run services update-traffic "${SERVICE_NAME}" \
        --project="${PROJECT_ID}" \
        --region="${REGION}" \
        --to-revisions="${PREVIOUS}=100"

    echo ""
    echo "=== Rollback Complete ==="
    echo "Traffic now routing to: ${PREVIOUS}"
    echo ""

    # Verify health
    echo "Verifying service health..."
    SERVICE_URL=$(gcloud run services describe "${SERVICE_NAME}" \
        --project="${PROJECT_ID}" \
        --region="${REGION}" \
        --format='value(status.url)')

    sleep 5

    STATUS=$(curl -s -o /dev/null -w "%{http_code}" "${SERVICE_URL}/health" 2>/dev/null || echo "000")
    if [ "${STATUS}" = "200" ]; then
        echo "Health check PASSED (HTTP 200)"
    else
        echo "WARNING: Health check returned ${STATUS}"
    fi
else
    echo "Rollback cancelled"
fi
