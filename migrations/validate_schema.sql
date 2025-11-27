-- ============================================================================
-- Schema Validation Script
-- Description: Verify that all migrations have been applied correctly
-- Usage: psql -d llm_benchmark_exchange -f validate_schema.sql
-- ============================================================================

\echo '╔════════════════════════════════════════════════════════════╗'
\echo '║       LLM Benchmark Exchange - Schema Validation          ║'
\echo '╚════════════════════════════════════════════════════════════╝'
\echo ''

-- Check PostgreSQL version
\echo '==> Checking PostgreSQL version...'
SELECT version();
\echo ''

-- Check installed extensions
\echo '==> Checking installed extensions...'
SELECT extname, extversion
FROM pg_extension
WHERE extname IN ('uuid-ossp', 'pgcrypto', 'pg_trgm')
ORDER BY extname;
\echo ''

-- Count enum types
\echo '==> Checking enum types...'
SELECT typname as enum_name, COUNT(enumlabel) as value_count
FROM pg_type t
JOIN pg_enum e ON t.oid = e.enumtypid
WHERE typname IN (
    'benchmark_status',
    'benchmark_category',
    'verification_level',
    'submission_visibility',
    'user_role',
    'organization_type',
    'organization_role',
    'proposal_status',
    'proposal_type',
    'vote_type'
)
GROUP BY typname
ORDER BY typname;
\echo ''

-- Count tables
\echo '==> Checking tables...'
SELECT
    schemaname,
    COUNT(*) as table_count
FROM pg_tables
WHERE schemaname = 'public'
GROUP BY schemaname;
\echo ''

-- List all tables
\echo '==> Listing all tables...'
SELECT tablename
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY tablename;
\echo ''

-- Count indexes
\echo '==> Checking indexes...'
SELECT
    schemaname,
    COUNT(*) as index_count
FROM pg_indexes
WHERE schemaname = 'public'
GROUP BY schemaname;
\echo ''

-- Check materialized views
\echo '==> Checking materialized views...'
SELECT matviewname
FROM pg_matviews
WHERE schemaname = 'public'
ORDER BY matviewname;
\echo ''

-- Check functions
\echo '==> Checking custom functions...'
SELECT
    p.proname as function_name,
    pg_get_function_arguments(p.oid) as arguments
FROM pg_proc p
JOIN pg_namespace n ON p.pronamespace = n.oid
WHERE n.nspname = 'public'
    AND p.proname IN (
        'uuid_generate_v7',
        'update_updated_at_column',
        'increment_benchmark_version',
        'update_proposal_vote_counts',
        'update_community_verification_votes',
        'soft_delete',
        'calculate_aggregate_score',
        'search_benchmarks',
        'get_leaderboard',
        'create_next_month_partitions',
        'cleanup_old_partitions',
        'refresh_all_materialized_views',
        'refresh_leaderboard_cache'
    )
ORDER BY p.proname;
\echo ''

-- Check triggers
\echo '==> Checking triggers...'
SELECT
    tgname as trigger_name,
    tgrelid::regclass as table_name
FROM pg_trigger
WHERE tgisinternal = false
ORDER BY tgrelid::regclass::text, tgname;
\echo ''

-- Check partitions
\echo '==> Checking partitions...'
SELECT
    parent.relname AS parent_table,
    COUNT(child.relname) as partition_count
FROM pg_inherits
JOIN pg_class parent ON pg_inherits.inhparent = parent.oid
JOIN pg_class child ON pg_inherits.inhrelid = child.oid
WHERE parent.relname IN ('domain_events', 'audit_log')
GROUP BY parent.relname
ORDER BY parent.relname;
\echo ''

-- List domain_events partitions
\echo '==> Domain Events Partitions:'
SELECT
    child.relname as partition_name,
    pg_get_expr(child.relpartbound, child.oid) as partition_bound
FROM pg_inherits
JOIN pg_class parent ON pg_inherits.inhparent = parent.oid
JOIN pg_class child ON pg_inherits.inhrelid = child.oid
WHERE parent.relname = 'domain_events'
ORDER BY child.relname;
\echo ''

-- List audit_log partitions
\echo '==> Audit Log Partitions:'
SELECT
    child.relname as partition_name,
    pg_get_expr(child.relpartbound, child.oid) as partition_bound
FROM pg_inherits
JOIN pg_class parent ON pg_inherits.inhparent = parent.oid
JOIN pg_class child ON pg_inherits.inhrelid = child.oid
WHERE parent.relname = 'audit_log'
ORDER BY child.relname;
\echo ''

-- Check foreign key constraints
\echo '==> Checking foreign key constraints...'
SELECT
    conrelid::regclass AS table_name,
    COUNT(*) as fk_count
FROM pg_constraint
WHERE contype = 'f'
    AND connamespace = 'public'::regnamespace
GROUP BY conrelid
ORDER BY table_name;
\echo ''

-- Check table sizes
\echo '==> Checking table sizes...'
SELECT
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC
LIMIT 20;
\echo ''

-- Verify critical tables exist
\echo '==> Verifying critical tables exist...'
DO $$
DECLARE
    tables TEXT[] := ARRAY[
        'users',
        'organizations',
        'organization_members',
        'benchmarks',
        'benchmark_versions',
        'benchmark_maintainers',
        'benchmark_tags',
        'benchmark_subcategories',
        'test_cases',
        'test_case_tags',
        'expected_outputs',
        'output_constraints',
        'submissions',
        'metric_scores',
        'test_case_results',
        'execution_metadata',
        'verifications',
        'verification_attempts',
        'verification_steps',
        'community_verifications',
        'verification_votes',
        'proposals',
        'votes',
        'reviews',
        'review_comments',
        'proposal_amendments',
        'domain_events',
        'audit_log',
        'event_subscriptions'
    ];
    tbl TEXT;
    missing_count INTEGER := 0;
BEGIN
    FOREACH tbl IN ARRAY tables
    LOOP
        IF NOT EXISTS (SELECT 1 FROM pg_tables WHERE tablename = tbl AND schemaname = 'public') THEN
            RAISE NOTICE 'Missing table: %', tbl;
            missing_count := missing_count + 1;
        END IF;
    END LOOP;

    IF missing_count = 0 THEN
        RAISE NOTICE 'All critical tables exist ✓';
    ELSE
        RAISE NOTICE 'Missing % table(s)', missing_count;
    END IF;
END $$;
\echo ''

-- Verify indexes on critical columns
\echo '==> Verifying critical indexes...'
DO $$
DECLARE
    missing_count INTEGER := 0;
BEGIN
    -- Check users email index
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_users_email') THEN
        RAISE NOTICE 'Missing index: idx_users_email';
        missing_count := missing_count + 1;
    END IF;

    -- Check benchmarks slug index
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_benchmarks_slug') THEN
        RAISE NOTICE 'Missing index: idx_benchmarks_slug';
        missing_count := missing_count + 1;
    END IF;

    -- Check submissions leaderboard index
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_submissions_leaderboard') THEN
        RAISE NOTICE 'Missing index: idx_submissions_leaderboard';
        missing_count := missing_count + 1;
    END IF;

    IF missing_count = 0 THEN
        RAISE NOTICE 'All critical indexes exist ✓';
    ELSE
        RAISE NOTICE 'Missing % index(es)', missing_count;
    END IF;
END $$;
\echo ''

-- Check for tables without primary keys
\echo '==> Checking for tables without primary keys...'
SELECT tablename
FROM pg_tables t
WHERE schemaname = 'public'
    AND NOT EXISTS (
        SELECT 1
        FROM pg_constraint c
        WHERE c.conrelid = (schemaname||'.'||tablename)::regclass
            AND c.contype = 'p'
    )
ORDER BY tablename;
\echo ''

-- Summary
\echo '╔════════════════════════════════════════════════════════════╗'
\echo '║                    Validation Complete                     ║'
\echo '╚════════════════════════════════════════════════════════════╝'
\echo ''
\echo 'Next steps:'
\echo '  1. Review any warnings or missing items above'
\echo '  2. Create next month partitions: SELECT create_next_month_partitions();'
\echo '  3. Refresh materialized views: SELECT refresh_all_materialized_views();'
\echo '  4. Run seed data script if needed'
\echo ''
