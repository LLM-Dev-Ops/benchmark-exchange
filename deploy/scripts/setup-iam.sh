#!/bin/bash
# IAM Setup Script for LLM-Benchmark-Gateway
# Creates service account and assigns minimal required permissions

set -euo pipefail

# Configuration
PROJECT_ID="${PROJECT_ID:-agentics-dev}"
SERVICE_NAME="llm-benchmark-gateway"
REGION="${REGION:-us-central1}"
SA_EMAIL="${SERVICE_NAME}@${PROJECT_ID}.iam.gserviceaccount.com"

echo "=== LLM-Benchmark-Gateway IAM Setup ==="
echo "Project: ${PROJECT_ID}"
echo "Service: ${SERVICE_NAME}"
echo "Region: ${REGION}"
echo ""

# Step 1: Create service account
echo "Step 1: Creating service account..."
if gcloud iam service-accounts describe "${SA_EMAIL}" --project="${PROJECT_ID}" &>/dev/null; then
    echo "  Service account already exists: ${SA_EMAIL}"
else
    gcloud iam service-accounts create "${SERVICE_NAME}" \
        --project="${PROJECT_ID}" \
        --display-name="LLM Benchmark Gateway Service Account" \
        --description="Service account for llm-benchmark-gateway Cloud Run service"
    echo "  Created service account: ${SA_EMAIL}"
fi

# Step 2: Grant Cloud Run invoker role on ruvector-service
echo ""
echo "Step 2: Granting Cloud Run invoker role on ruvector-service..."
RUVECTOR_SERVICE="ruvector-service"
gcloud run services add-iam-policy-binding "${RUVECTOR_SERVICE}" \
    --project="${PROJECT_ID}" \
    --region="${REGION}" \
    --member="serviceAccount:${SA_EMAIL}" \
    --role="roles/run.invoker" \
    --quiet 2>/dev/null || echo "  Note: ruvector-service may not exist yet in ${REGION}"

# Step 3: Grant Secret Manager accessor role
echo ""
echo "Step 3: Granting Secret Manager accessor role..."
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
    --member="serviceAccount:${SA_EMAIL}" \
    --role="roles/secretmanager.secretAccessor" \
    --condition=None \
    --quiet

# Step 4: Grant logging writer role (for Cloud Logging)
echo ""
echo "Step 4: Granting logging writer role..."
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
    --member="serviceAccount:${SA_EMAIL}" \
    --role="roles/logging.logWriter" \
    --condition=None \
    --quiet

# Step 5: Grant monitoring metric writer role (for Cloud Monitoring)
echo ""
echo "Step 5: Granting monitoring metric writer role..."
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
    --member="serviceAccount:${SA_EMAIL}" \
    --role="roles/monitoring.metricWriter" \
    --condition=None \
    --quiet

# Step 6: Grant trace agent role (for Cloud Trace)
echo ""
echo "Step 6: Granting trace agent role..."
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
    --member="serviceAccount:${SA_EMAIL}" \
    --role="roles/cloudtrace.agent" \
    --condition=None \
    --quiet

# Step 7: Create the API key secret if it doesn't exist
echo ""
echo "Step 7: Creating ruvector-api-key secret..."
if gcloud secrets describe "ruvector-api-key" --project="${PROJECT_ID}" &>/dev/null; then
    echo "  Secret already exists: ruvector-api-key"
else
    # Generate a placeholder - should be replaced with actual key
    echo "  Creating secret placeholder (update with actual API key)"
    echo -n "REPLACE_WITH_ACTUAL_API_KEY" | gcloud secrets create "ruvector-api-key" \
        --project="${PROJECT_ID}" \
        --replication-policy="automatic" \
        --data-file=-
    echo "  Created secret: ruvector-api-key"
    echo "  WARNING: Update this secret with the actual ruvector-service API key"
fi

# Step 8: Verify NO Cloud SQL permissions
echo ""
echo "Step 8: Verifying NO Cloud SQL permissions..."
SQL_ROLES=$(gcloud projects get-iam-policy "${PROJECT_ID}" \
    --flatten="bindings[].members" \
    --format="table(bindings.role)" \
    --filter="bindings.members:${SA_EMAIL} AND bindings.role:cloudsql" 2>/dev/null || true)

if [ -n "${SQL_ROLES}" ]; then
    echo "  WARNING: Service account has Cloud SQL roles (should have NONE):"
    echo "${SQL_ROLES}"
else
    echo "  VERIFIED: No Cloud SQL permissions (as required by constitution)"
fi

echo ""
echo "=== IAM Setup Complete ==="
echo ""
echo "Service Account: ${SA_EMAIL}"
echo ""
echo "Granted Roles:"
echo "  - roles/run.invoker (on ruvector-service)"
echo "  - roles/secretmanager.secretAccessor"
echo "  - roles/logging.logWriter"
echo "  - roles/monitoring.metricWriter"
echo "  - roles/cloudtrace.agent"
echo ""
echo "NOT Granted (by design):"
echo "  - roles/cloudsql.* (NO direct SQL access)"
echo ""
