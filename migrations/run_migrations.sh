#!/bin/bash
# ============================================================================
# Migration Runner Script
# Description: Run PostgreSQL migrations for LLM Benchmark Exchange
# Usage: ./run_migrations.sh [options]
# ============================================================================

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
DATABASE_URL="${DATABASE_URL:-postgresql://postgres:postgres@localhost:5432/llm_benchmark_exchange}"
MIGRATIONS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DRY_RUN=false
VERBOSE=false
STOP_ON_ERROR=true

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -d|--database)
            DATABASE_URL="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        --continue-on-error)
            STOP_ON_ERROR=false
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  -d, --database URL        Database connection URL"
            echo "  --dry-run                 Show what would be executed without running"
            echo "  -v, --verbose             Verbose output"
            echo "  --continue-on-error       Continue even if a migration fails"
            echo "  -h, --help                Show this help message"
            echo ""
            echo "Environment variables:"
            echo "  DATABASE_URL              Database connection URL (default: postgresql://postgres:postgres@localhost:5432/llm_benchmark_exchange)"
            echo ""
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

# Function to print colored output
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to execute SQL
execute_sql() {
    local file=$1
    local description=$2

    if [ "$DRY_RUN" = true ]; then
        print_info "Would execute: $description ($file)"
        return 0
    fi

    print_info "Executing: $description"

    if [ "$VERBOSE" = true ]; then
        psql "$DATABASE_URL" -f "$file" -v ON_ERROR_STOP=1
    else
        psql "$DATABASE_URL" -f "$file" -v ON_ERROR_STOP=1 > /dev/null 2>&1
    fi

    if [ $? -eq 0 ]; then
        print_success "Completed: $description"
        return 0
    else
        print_error "Failed: $description"
        return 1
    fi
}

# Print banner
echo ""
echo "╔════════════════════════════════════════════════════════════╗"
echo "║     LLM Benchmark Exchange - Database Migration Runner    ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

# Check if psql is available
if ! command -v psql &> /dev/null; then
    print_error "psql command not found. Please install PostgreSQL client."
    exit 1
fi

# Test database connection
print_info "Testing database connection..."
if [ "$DRY_RUN" = false ]; then
    if ! psql "$DATABASE_URL" -c "SELECT 1" > /dev/null 2>&1; then
        print_error "Cannot connect to database: $DATABASE_URL"
        exit 1
    fi
    print_success "Database connection successful"
fi

# Check if migrations directory exists
if [ ! -d "$MIGRATIONS_DIR" ]; then
    print_error "Migrations directory not found: $MIGRATIONS_DIR"
    exit 1
fi

# Count migration files
MIGRATION_COUNT=$(ls -1 "$MIGRATIONS_DIR"/[0-9]*.sql 2>/dev/null | wc -l)
if [ "$MIGRATION_COUNT" -eq 0 ]; then
    print_error "No migration files found in $MIGRATIONS_DIR"
    exit 1
fi

print_info "Found $MIGRATION_COUNT migration file(s)"
echo ""

# Run migrations in order
MIGRATIONS=(
    "00001_initial_setup.sql:Database extensions and enum types"
    "00002_users_organizations.sql:User accounts and organizations"
    "00003_benchmarks.sql:Benchmark definitions"
    "00004_test_cases.sql:Test cases and evaluation"
    "00005_submissions.sql:Benchmark submissions"
    "00006_verifications.sql:Verification workflows"
    "00007_governance.sql:Governance and proposals"
    "00008_events_audit.sql:Event sourcing and audit logs"
    "00009_materialized_views.sql:Performance views"
    "00010_functions.sql:Database functions and triggers"
)

FAILED_MIGRATIONS=()
SUCCESS_COUNT=0

for migration in "${MIGRATIONS[@]}"; do
    IFS=':' read -r file description <<< "$migration"
    filepath="$MIGRATIONS_DIR/$file"

    if [ ! -f "$filepath" ]; then
        print_warning "Migration file not found: $file (skipping)"
        continue
    fi

    if execute_sql "$filepath" "$description"; then
        SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
    else
        FAILED_MIGRATIONS+=("$file")
        if [ "$STOP_ON_ERROR" = true ]; then
            print_error "Migration failed. Stopping."
            exit 1
        fi
    fi

    echo ""
done

# Summary
echo "╔════════════════════════════════════════════════════════════╗"
echo "║                     Migration Summary                      ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""
print_info "Total migrations: ${#MIGRATIONS[@]}"
print_success "Successful: $SUCCESS_COUNT"

if [ ${#FAILED_MIGRATIONS[@]} -gt 0 ]; then
    print_error "Failed: ${#FAILED_MIGRATIONS[@]}"
    echo ""
    print_error "Failed migrations:"
    for failed in "${FAILED_MIGRATIONS[@]}"; do
        echo "  - $failed"
    done
    exit 1
else
    echo ""
    print_success "All migrations completed successfully!"
    echo ""
    print_info "Next steps:"
    echo "  1. Verify schema: psql $DATABASE_URL -c '\dt'"
    echo "  2. Create initial partitions: SELECT create_next_month_partitions();"
    echo "  3. Refresh materialized views: SELECT refresh_all_materialized_views();"
    echo ""
fi

exit 0
