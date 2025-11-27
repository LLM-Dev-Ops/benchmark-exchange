# Database Quick Reference

Quick reference guide for common database operations.

## Initial Setup

```bash
# Create database
createdb llm_benchmark_exchange

# Set environment variable
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/llm_benchmark_exchange"

# Run migrations
./migrations/run_migrations.sh

# Validate schema
psql $DATABASE_URL -f migrations/validate_schema.sql
```

## Common Queries

### User Management

```sql
-- Create user
INSERT INTO users (email, username, password_hash, display_name)
VALUES ('user@example.com', 'username', 'hashed_password', 'Display Name')
RETURNING id;

-- Find user by email
SELECT * FROM users WHERE email = 'user@example.com' AND deleted_at IS NULL;

-- Update user role
UPDATE users SET role = 'contributor' WHERE id = 'user-uuid';

-- Soft delete user
UPDATE users SET deleted_at = NOW() WHERE id = 'user-uuid';
```

### Benchmark Management

```sql
-- Create benchmark
INSERT INTO benchmarks (slug, name, description, category, license_type, created_by)
VALUES ('my-benchmark', 'My Benchmark', 'Description...', 'accuracy', 'MIT', 'user-uuid')
RETURNING id;

-- Get active benchmarks
SELECT * FROM benchmarks
WHERE status = 'active' AND deleted_at IS NULL
ORDER BY created_at DESC;

-- Search benchmarks
SELECT * FROM search_benchmarks('machine learning', 20);

-- Get benchmark with latest version
SELECT b.*, bv.*
FROM benchmarks b
JOIN benchmark_versions bv ON bv.benchmark_id = b.id
WHERE b.slug = 'my-benchmark'
ORDER BY bv.version_major DESC, bv.version_minor DESC, bv.version_patch DESC
LIMIT 1;
```

### Submission Management

```sql
-- Create submission
INSERT INTO submissions (
    benchmark_id,
    benchmark_version_id,
    model_provider,
    model_name,
    aggregate_score,
    submitted_by,
    execution_id,
    execution_started_at,
    execution_completed_at,
    execution_duration_seconds,
    executor_version,
    model_parameters_used,
    dataset_checksums
) VALUES (
    'benchmark-uuid',
    'version-uuid',
    'OpenAI',
    'gpt-4',
    0.95,
    'user-uuid',
    'exec-123',
    NOW(),
    NOW(),
    300.5,
    '1.0.0',
    '{"temperature": 0.7}'::JSONB,
    '{"test": "sha256:abc123"}'::JSONB
) RETURNING id;

-- Get top submissions for benchmark
SELECT * FROM get_leaderboard('benchmark-uuid', NULL, 100);

-- Get verified submissions only
SELECT * FROM get_leaderboard('benchmark-uuid', 'platform_verified', 50);
```

### Leaderboard Queries

```sql
-- Get leaderboard (uses cached view)
SELECT *
FROM leaderboard_cache
WHERE benchmark_id = 'benchmark-uuid'
ORDER BY rank_overall
LIMIT 100;

-- Get leaderboard for specific category
SELECT *
FROM leaderboard_cache
WHERE benchmark_category = 'accuracy'
ORDER BY aggregate_score DESC
LIMIT 100;

-- Compare two models across benchmarks
SELECT
    benchmark_name,
    model_name,
    aggregate_score,
    rank_overall
FROM leaderboard_cache
WHERE model_name IN ('gpt-4', 'claude-3')
ORDER BY benchmark_name, aggregate_score DESC;
```

### Statistics

```sql
-- Get benchmark statistics
SELECT * FROM benchmark_stats WHERE benchmark_slug = 'my-benchmark';

-- Get user statistics
SELECT * FROM user_stats WHERE username = 'myusername';

-- Get model comparison for category
SELECT *
FROM model_comparison
WHERE benchmark_category = 'accuracy'
ORDER BY avg_score DESC;

-- Top performers by category
SELECT
    benchmark_category,
    model_provider,
    model_name,
    MAX(avg_score) as best_avg_score
FROM model_comparison
GROUP BY benchmark_category, model_provider, model_name
ORDER BY benchmark_category, best_avg_score DESC;
```

### Governance

```sql
-- Create proposal
INSERT INTO proposals (
    proposal_type,
    title,
    description,
    rationale,
    created_by,
    quorum_required,
    approval_threshold
) VALUES (
    'new_benchmark',
    'Proposal Title',
    'Description...',
    'Rationale...',
    'user-uuid',
    100,
    0.66
) RETURNING id;

-- Cast vote
INSERT INTO votes (proposal_id, user_id, vote)
VALUES ('proposal-uuid', 'user-uuid', 'approve')
ON CONFLICT (proposal_id, user_id) DO UPDATE
SET vote = EXCLUDED.vote, voted_at = NOW();

-- Get active proposals
SELECT * FROM proposals
WHERE status = 'voting'
    AND voting_starts_at <= NOW()
    AND voting_ends_at >= NOW()
ORDER BY voting_ends_at;
```

## Maintenance Operations

### Materialized View Refresh

```sql
-- Refresh all views (scheduled daily)
SELECT refresh_all_materialized_views();

-- Refresh leaderboard only (after submission)
SELECT refresh_leaderboard_cache();

-- Manual concurrent refresh
REFRESH MATERIALIZED VIEW CONCURRENTLY leaderboard_cache;
REFRESH MATERIALIZED VIEW CONCURRENTLY benchmark_stats;
REFRESH MATERIALIZED VIEW CONCURRENTLY user_stats;
REFRESH MATERIALIZED VIEW CONCURRENTLY model_comparison;
```

### Partition Management

```sql
-- Create next month's partitions
SELECT create_next_month_partitions();

-- Cleanup old partitions (2-year retention)
SELECT cleanup_old_partitions(24);

-- Manually create partition
CREATE TABLE domain_events_2027_01 PARTITION OF domain_events
    FOR VALUES FROM ('2027-01-01') TO ('2027-02-01');

CREATE TABLE audit_log_2027_01 PARTITION OF audit_log
    FOR VALUES FROM ('2027-01-01') TO ('2027-02-01');
```

### Performance Optimization

```sql
-- Analyze tables
ANALYZE users;
ANALYZE benchmarks;
ANALYZE submissions;

-- Vacuum and analyze
VACUUM ANALYZE submissions;
VACUUM ANALYZE test_case_results;

-- Reindex (if needed)
REINDEX TABLE submissions;
REINDEX INDEX idx_submissions_leaderboard;

-- Check index usage
SELECT
    schemaname,
    tablename,
    indexname,
    idx_scan as index_scans,
    idx_tup_read as tuples_read,
    idx_tup_fetch as tuples_fetched
FROM pg_stat_user_indexes
WHERE schemaname = 'public'
ORDER BY idx_scan DESC;

-- Find unused indexes
SELECT
    schemaname,
    tablename,
    indexname
FROM pg_stat_user_indexes
WHERE schemaname = 'public'
    AND idx_scan = 0
    AND indexname NOT LIKE '%_pkey'
ORDER BY tablename, indexname;
```

### Monitoring

```sql
-- Check database size
SELECT pg_size_pretty(pg_database_size('llm_benchmark_exchange'));

-- Check table sizes
SELECT
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS total_size,
    pg_size_pretty(pg_relation_size(schemaname||'.'||tablename)) AS table_size,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename) - pg_relation_size(schemaname||'.'||tablename)) AS index_size
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC
LIMIT 20;

-- Check active connections
SELECT
    datname,
    count(*) as connections
FROM pg_stat_activity
GROUP BY datname;

-- Check long-running queries
SELECT
    pid,
    now() - pg_stat_activity.query_start AS duration,
    query,
    state
FROM pg_stat_activity
WHERE state != 'idle'
    AND now() - pg_stat_activity.query_start > interval '5 minutes'
ORDER BY duration DESC;

-- Check partition counts
SELECT
    parent.relname AS parent_table,
    COUNT(child.relname) as partition_count,
    pg_size_pretty(SUM(pg_total_relation_size(child.oid))) as total_size
FROM pg_inherits
JOIN pg_class parent ON pg_inherits.inhparent = parent.oid
JOIN pg_class child ON pg_inherits.inhrelid = child.oid
WHERE parent.relname IN ('domain_events', 'audit_log')
GROUP BY parent.relname;
```

## Backup and Restore

### Backup

```bash
# Full database backup
pg_dump llm_benchmark_exchange > backup.sql

# Compressed backup
pg_dump llm_benchmark_exchange | gzip > backup.sql.gz

# Custom format (supports parallel restore)
pg_dump -Fc llm_benchmark_exchange > backup.dump

# Schema only
pg_dump --schema-only llm_benchmark_exchange > schema.sql

# Data only
pg_dump --data-only llm_benchmark_exchange > data.sql

# Specific tables
pg_dump -t users -t organizations llm_benchmark_exchange > users_orgs.sql
```

### Restore

```bash
# From SQL file
psql llm_benchmark_exchange < backup.sql

# From compressed file
gunzip -c backup.sql.gz | psql llm_benchmark_exchange

# From custom format
pg_restore -d llm_benchmark_exchange backup.dump

# Parallel restore (4 jobs)
pg_restore -j 4 -d llm_benchmark_exchange backup.dump

# Clean and restore
pg_restore -c -d llm_benchmark_exchange backup.dump
```

## Useful Admin Commands

```sql
-- List all tables
\dt

-- Describe table
\d users
\d+ submissions

-- List indexes
\di

-- List views
\dv

-- List materialized views
\dm

-- List functions
\df

-- List enums
\dT+

-- Show table sizes
\dt+

-- Execute SQL file
\i migrations/00001_initial_setup.sql

-- Toggle timing
\timing

-- Set output format
\x auto  -- Expanded display auto

-- Export query results to CSV
\copy (SELECT * FROM leaderboard_cache LIMIT 100) TO 'leaderboard.csv' CSV HEADER

-- Show current settings
SHOW all;
SHOW max_connections;
SHOW shared_buffers;
```

## Common Troubleshooting

### Reset Auto-increment Sequences

```sql
-- Reset user ID sequence
SELECT setval('users_id_seq', (SELECT MAX(id) FROM users));

-- For UUID columns, this doesn't apply (UUIDs don't use sequences)
```

### Fix Materialized View Issues

```sql
-- Drop and recreate if needed
DROP MATERIALIZED VIEW IF EXISTS leaderboard_cache CASCADE;
-- Then re-run migration 00009

-- If concurrent refresh fails, try non-concurrent
REFRESH MATERIALIZED VIEW leaderboard_cache;
```

### Check for Locks

```sql
-- See blocked queries
SELECT
    blocked_locks.pid AS blocked_pid,
    blocked_activity.usename AS blocked_user,
    blocking_locks.pid AS blocking_pid,
    blocking_activity.usename AS blocking_user,
    blocked_activity.query AS blocked_statement,
    blocking_activity.query AS blocking_statement
FROM pg_catalog.pg_locks blocked_locks
JOIN pg_catalog.pg_stat_activity blocked_activity ON blocked_activity.pid = blocked_locks.pid
JOIN pg_catalog.pg_locks blocking_locks
    ON blocking_locks.locktype = blocked_locks.locktype
    AND blocking_locks.database IS NOT DISTINCT FROM blocked_locks.database
    AND blocking_locks.relation IS NOT DISTINCT FROM blocked_locks.relation
    AND blocking_locks.page IS NOT DISTINCT FROM blocked_locks.page
    AND blocking_locks.tuple IS NOT DISTINCT FROM blocked_locks.tuple
    AND blocking_locks.virtualxid IS NOT DISTINCT FROM blocked_locks.virtualxid
    AND blocking_locks.transactionid IS NOT DISTINCT FROM blocked_locks.transactionid
    AND blocking_locks.classid IS NOT DISTINCT FROM blocked_locks.classid
    AND blocking_locks.objid IS NOT DISTINCT FROM blocked_locks.objid
    AND blocking_locks.objsubid IS NOT DISTINCT FROM blocked_locks.objsubid
    AND blocking_locks.pid != blocked_locks.pid
JOIN pg_catalog.pg_stat_activity blocking_activity ON blocking_activity.pid = blocking_locks.pid
WHERE NOT blocked_locks.granted;

-- Kill blocking query (use with caution)
-- SELECT pg_terminate_backend(pid);
```

## Environment-Specific Configs

### Development

```bash
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/llm_benchmark_exchange"
```

### Testing

```bash
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/llm_benchmark_exchange_test"
```

### Production

```bash
export DATABASE_URL="postgresql://user:password@db.example.com:5432/llm_benchmark_exchange?sslmode=require"
```

## Performance Tips

1. **Use materialized views** for expensive queries (leaderboards, stats)
2. **Refresh materialized views** on a schedule (not on every request)
3. **Use connection pooling** (configured in sqlx.toml)
4. **Add indexes** for common query patterns
5. **Use EXPLAIN ANALYZE** to optimize slow queries
6. **Partition large tables** (events, audit logs)
7. **Regular VACUUM** to reclaim space
8. **Monitor slow query log**
9. **Use prepared statements** for repeated queries
10. **Consider read replicas** for read-heavy workloads
