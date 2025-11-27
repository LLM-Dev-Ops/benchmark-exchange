# Database Migrations

This directory contains PostgreSQL database migrations for the LLM Benchmark Exchange platform.

## Overview

The migrations are organized sequentially and designed to be run in order. Each migration builds upon the previous ones to create a complete database schema.

## Migration Files

### 00001_initial_setup.sql
**Purpose**: Database extensions and enum types

- PostgreSQL extensions (uuid-ossp, pgcrypto, pg_trgm)
- All enum types used throughout the schema
- Foundation for type safety and full-text search

**Key Features**:
- UUID v7 support for time-ordered identifiers
- Cryptographic functions for password hashing
- Trigram similarity for fuzzy text search

### 00002_users_organizations.sql
**Purpose**: User accounts and organization management

- `users` table with authentication and profile data
- `organizations` table for entity management
- `organization_members` junction table with roles
- Soft delete support
- Email validation and formatting constraints

**Indexes**:
- Email and username lookups
- Role-based queries
- Activity tracking

### 00003_benchmarks.sql
**Purpose**: Benchmark definitions with versioning

- `benchmarks` table with metadata and citations
- `benchmark_versions` table with semantic versioning
- `benchmark_maintainers` junction table
- `benchmark_tags` for discovery
- `benchmark_subcategories` for hierarchical organization

**Key Features**:
- Semantic versioning (major.minor.patch)
- Version history tracking with parent references
- Flexible metric definitions (JSONB)
- Full-text search on names and descriptions
- Citation management (BibTeX support)

### 00004_test_cases.sql
**Purpose**: Test cases and evaluation definitions

- `test_cases` table with prompt templates
- `expected_outputs` table for reference answers
- `output_constraints` table for validation rules
- `test_case_tags` for categorization

**Key Features**:
- Flexible input templating with JSONB variables
- Multiple evaluation methods support
- Few-shot example storage
- Difficulty levels and weighting
- Output schema validation (JSON Schema)

### 00005_submissions.sql
**Purpose**: Benchmark results and submissions

- `submissions` table with model information and scores
- `metric_scores` table for individual metrics
- `test_case_results` table for granular results
- `execution_metadata` table for extensibility

**Key Features**:
- Comprehensive verification tracking
- Environment reproducibility metadata
- Statistical measures (confidence intervals)
- Performance metrics (latency, tokens, cost)
- Visibility controls (public/unlisted/private)

**Indexes**:
- Leaderboard queries (composite index)
- Model comparisons
- User and organization filters
- Verification level filtering

### 00006_verifications.sql
**Purpose**: Verification workflows and attestations

- `verifications` table for platform/auditor verification
- `verification_attempts` table for retry tracking
- `verification_steps` table for transparency
- `community_verifications` table for community participation
- `verification_votes` table for community endorsement

**Key Features**:
- Multi-attempt verification support
- Step-by-step verification tracking
- Community verification with voting
- Cryptographic attestation support
- Environment matching validation

### 00007_governance.sql
**Purpose**: Governance proposals and voting

- `proposals` table for platform governance
- `votes` table for voting records
- `reviews` table for expert reviews
- `review_comments` table for detailed feedback
- `proposal_amendments` table for iteration

**Key Features**:
- Multiple proposal types (new benchmark, updates, policy changes)
- Configurable voting thresholds and quorum
- Review workflow with inline comments
- Amendment tracking
- Discussion integration

### 00008_events_audit.sql
**Purpose**: Event sourcing and audit logging

- `domain_events` table with monthly partitioning
- `audit_log` table with compliance-friendly retention
- `event_subscriptions` table for integrations

**Key Features**:
- Outbox pattern for event processing
- Correlation and causation tracking
- Monthly partitioning (2025-2026)
- Comprehensive audit trail
- Event subscription management

**Partitioning Strategy**:
- Monthly range partitions for scalability
- Automatic partition management functions
- 2-year retention for events, 7-year for audit logs

### 00009_materialized_views.sql
**Purpose**: Performance optimizations

- `leaderboard_cache` - Pre-computed leaderboard rankings
- `benchmark_stats` - Aggregate statistics per benchmark
- `user_stats` - User activity and achievements
- `model_comparison` - Cross-category model performance

**Key Features**:
- CONCURRENT refresh support (no locks)
- Multiple ranking strategies (overall, verified, official)
- Statistical aggregations
- Refresh functions for scheduled updates

**Refresh Strategy**:
- Leaderboard: After each submission
- Stats: Daily or on-demand
- Use `REFRESH MATERIALIZED VIEW CONCURRENTLY` to avoid locking

### 00010_functions.sql
**Purpose**: Database functions and triggers

**Functions**:
- `uuid_generate_v7()` - Time-ordered UUID generation
- `update_updated_at_column()` - Automatic timestamp updates
- `increment_benchmark_version()` - Semantic version management
- `calculate_aggregate_score()` - Score calculation
- `search_benchmarks()` - Full-text search
- `get_leaderboard()` - Optimized leaderboard queries
- `create_next_month_partitions()` - Partition maintenance
- `cleanup_old_partitions()` - Retention management

**Triggers**:
- Automatic `updated_at` updates on all relevant tables
- Vote count maintenance for proposals
- Community verification vote tracking

## Running Migrations

### Using SQLx CLI

```bash
# Install sqlx-cli
cargo install sqlx-cli --no-default-features --features postgres

# Run all migrations
sqlx migrate run

# Revert last migration
sqlx migrate revert

# Check migration status
sqlx migrate info
```

### Using psql

```bash
# Run all migrations in order
for file in migrations/*.sql; do
    psql -d llm_benchmark_exchange -f "$file"
done

# Or individually
psql -d llm_benchmark_exchange -f migrations/00001_initial_setup.sql
```

### Using Docker

```bash
# Start PostgreSQL
docker-compose up -d postgres

# Run migrations
docker-compose exec postgres psql -U postgres -d llm_benchmark_exchange -f /migrations/00001_initial_setup.sql
```

## Database Setup

### Create Database

```sql
CREATE DATABASE llm_benchmark_exchange
    WITH
    OWNER = postgres
    ENCODING = 'UTF8'
    LC_COLLATE = 'en_US.UTF-8'
    LC_CTYPE = 'en_US.UTF-8'
    TEMPLATE = template0;
```

### Environment Variables

```bash
export DATABASE_URL="postgresql://user:password@localhost:5432/llm_benchmark_exchange"
```

## Maintenance Tasks

### Refresh Materialized Views

```sql
-- Refresh all views
SELECT refresh_all_materialized_views();

-- Refresh leaderboard only (fast)
SELECT refresh_leaderboard_cache();

-- Manual concurrent refresh
REFRESH MATERIALIZED VIEW CONCURRENTLY leaderboard_cache;
```

### Partition Management

```sql
-- Create partitions for next month (run monthly)
SELECT create_next_month_partitions();

-- Cleanup old partitions (2-year retention)
SELECT cleanup_old_partitions(24);
```

### Vacuum and Analyze

```sql
-- Run after bulk operations
VACUUM ANALYZE;

-- Specific tables
VACUUM ANALYZE submissions;
VACUUM ANALYZE test_case_results;
```

## Schema Conventions

### Naming Conventions

- Tables: `snake_case`, plural nouns (e.g., `users`, `benchmarks`)
- Columns: `snake_case` (e.g., `created_at`, `model_name`)
- Indexes: `idx_tablename_columns` (e.g., `idx_users_email`)
- Foreign keys: implicit naming
- Enums: `snake_case` type names, lowercase values

### Timestamps

- All tables use `TIMESTAMPTZ` for timezone awareness
- Standard columns: `created_at`, `updated_at`, `deleted_at`
- Automatic `updated_at` via triggers

### Soft Deletes

- Use `deleted_at TIMESTAMPTZ` column
- Indexes include `WHERE deleted_at IS NULL` for performance
- Use `soft_delete()` function for consistency

### JSONB Usage

- Configuration: `model_parameters`, `environment_requirements`
- Flexible data: `evaluation_method`, `aggregation_method`
- Arrays: `secondary_metrics`, `dataset_references`
- Always include GIN indexes for JSONB columns with queries

## Performance Considerations

### Indexing Strategy

1. **B-tree indexes**: Primary keys, foreign keys, timestamps
2. **GIN indexes**: JSONB columns, full-text search
3. **Partial indexes**: Filtered queries (e.g., `WHERE deleted_at IS NULL`)
4. **Composite indexes**: Multi-column queries (e.g., leaderboard)

### Query Optimization

- Use materialized views for expensive aggregations
- Partition large tables (events, audit logs)
- Regular VACUUM and ANALYZE
- Monitor slow query log

### Scaling Considerations

- Connection pooling (configured in sqlx.toml)
- Read replicas for leaderboard queries
- Partition pruning for time-series data
- Index-only scans where possible

## Security

### Constraints

- Email format validation
- Username format validation (alphanumeric, hyphens, underscores)
- Score ranges and validity checks
- Referential integrity via foreign keys

### Sensitive Data

- Passwords hashed via `pgcrypto`
- Personal data in `users` table (GDPR compliance via soft delete)
- Audit trail for all changes

## Testing

### Test Database

```bash
# Create test database
createdb llm_benchmark_exchange_test

# Run migrations
DATABASE_URL="postgresql://localhost/llm_benchmark_exchange_test" sqlx migrate run

# Reset test database
dropdb llm_benchmark_exchange_test
createdb llm_benchmark_exchange_test
```

### Sample Data

See `scripts/seed_database.sql` for sample data generation.

## Troubleshooting

### Migration Errors

```bash
# Check current migration version
sqlx migrate info

# Force mark migration as applied (use with caution)
# sqlx migrate force <version>

# Rollback and retry
sqlx migrate revert
sqlx migrate run
```

### Permission Issues

```sql
-- Grant permissions
GRANT ALL PRIVILEGES ON DATABASE llm_benchmark_exchange TO your_user;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO your_user;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO your_user;
```

### Partition Issues

```sql
-- List partitions
SELECT
    parent.relname AS parent,
    child.relname AS partition
FROM pg_inherits
JOIN pg_class parent ON pg_inherits.inhparent = parent.oid
JOIN pg_class child ON pg_inherits.inhrelid = child.oid
WHERE parent.relname IN ('domain_events', 'audit_log')
ORDER BY child.relname;

-- Manually create missing partition
CREATE TABLE domain_events_2027_01 PARTITION OF domain_events
    FOR VALUES FROM ('2027-01-01') TO ('2027-02-01');
```

## References

- [PostgreSQL Documentation](https://www.postgresql.org/docs/)
- [SQLx Documentation](https://github.com/launchbadge/sqlx)
- [SPARC Specification](../plans/LLM-Benchmark-Exchange-SPARC-Complete.md)
