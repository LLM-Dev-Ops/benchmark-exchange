-- ============================================================================
-- Migration: 00009_materialized_views.sql
-- Description: Materialized views for performance optimization
-- Author: LLM Benchmark Exchange
-- Created: 2025-11-26
-- ============================================================================

-- ============================================================================
-- LEADERBOARD CACHE MATERIALIZED VIEW
-- ============================================================================

CREATE MATERIALIZED VIEW leaderboard_cache AS
SELECT
    s.id AS submission_id,
    s.benchmark_id,
    s.benchmark_version_id,
    b.slug AS benchmark_slug,
    b.name AS benchmark_name,
    b.category AS benchmark_category,
    bv.version_major,
    bv.version_minor,
    bv.version_patch,

    -- Model information
    s.model_provider,
    s.model_name,
    s.model_version,
    s.model_registry_id,

    -- Scores
    s.aggregate_score,
    s.confidence_interval_lower,
    s.confidence_interval_upper,
    s.confidence_level,

    -- Verification
    s.verification_level,
    s.is_official_submission,
    s.is_verified_provider,

    -- Submitter
    s.submitted_by,
    u.username AS submitter_username,
    u.display_name AS submitter_display_name,
    s.organization_id,
    o.name AS organization_name,
    o.slug AS organization_slug,

    -- Ranking (per benchmark)
    ROW_NUMBER() OVER (
        PARTITION BY s.benchmark_id
        ORDER BY s.aggregate_score DESC
    ) AS rank_overall,

    -- Ranking (per benchmark, verified only)
    ROW_NUMBER() OVER (
        PARTITION BY s.benchmark_id
        ORDER BY
            CASE
                WHEN s.verification_level IN ('platform_verified', 'audited')
                THEN s.aggregate_score
                ELSE NULL
            END DESC NULLS LAST
    ) AS rank_verified,

    -- Ranking (per benchmark, official only)
    ROW_NUMBER() OVER (
        PARTITION BY s.benchmark_id
        ORDER BY
            CASE
                WHEN s.is_official_submission
                THEN s.aggregate_score
                ELSE NULL
            END DESC NULLS LAST
    ) AS rank_official,

    -- Timestamps
    s.created_at AS submission_created_at,
    s.execution_started_at,
    s.execution_completed_at

FROM submissions s
JOIN benchmarks b ON b.id = s.benchmark_id
JOIN benchmark_versions bv ON bv.id = s.benchmark_version_id
JOIN users u ON u.id = s.submitted_by
LEFT JOIN organizations o ON o.id = s.organization_id

WHERE
    s.deleted_at IS NULL
    AND b.deleted_at IS NULL
    AND u.deleted_at IS NULL
    AND s.visibility = 'public';

-- Indexes on materialized view
CREATE UNIQUE INDEX idx_leaderboard_cache_submission ON leaderboard_cache(submission_id);
CREATE INDEX idx_leaderboard_cache_benchmark ON leaderboard_cache(benchmark_id, rank_overall);
CREATE INDEX idx_leaderboard_cache_benchmark_verified ON leaderboard_cache(benchmark_id, rank_verified);
CREATE INDEX idx_leaderboard_cache_category ON leaderboard_cache(benchmark_category, aggregate_score DESC);
CREATE INDEX idx_leaderboard_cache_model ON leaderboard_cache(model_provider, model_name);
CREATE INDEX idx_leaderboard_cache_verification ON leaderboard_cache(verification_level);
CREATE INDEX idx_leaderboard_cache_created ON leaderboard_cache(submission_created_at DESC);

COMMENT ON MATERIALIZED VIEW leaderboard_cache IS 'Pre-computed leaderboard data for fast queries';

-- ============================================================================
-- BENCHMARK STATISTICS MATERIALIZED VIEW
-- ============================================================================

CREATE MATERIALIZED VIEW benchmark_stats AS
SELECT
    b.id AS benchmark_id,
    b.slug AS benchmark_slug,
    b.name AS benchmark_name,
    b.category,
    b.status,

    -- Latest version
    (
        SELECT CONCAT(version_major, '.', version_minor, '.', version_patch)
        FROM benchmark_versions bv
        WHERE bv.benchmark_id = b.id
        ORDER BY version_major DESC, version_minor DESC, version_patch DESC
        LIMIT 1
    ) AS latest_version,

    -- Submission counts
    COUNT(DISTINCT s.id) AS total_submissions,
    COUNT(DISTINCT s.id) FILTER (
        WHERE s.verification_level IN ('platform_verified', 'audited')
    ) AS verified_submissions,
    COUNT(DISTINCT s.id) FILTER (
        WHERE s.is_official_submission = TRUE
    ) AS official_submissions,

    -- Unique models and providers
    COUNT(DISTINCT s.model_provider || '/' || s.model_name) AS unique_models,
    COUNT(DISTINCT s.model_provider) AS unique_providers,

    -- Unique submitters
    COUNT(DISTINCT s.submitted_by) AS unique_submitters,

    -- Score statistics
    MAX(s.aggregate_score) AS max_score,
    MIN(s.aggregate_score) AS min_score,
    AVG(s.aggregate_score) AS avg_score,
    PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY s.aggregate_score) AS median_score,
    PERCENTILE_CONT(0.75) WITHIN GROUP (ORDER BY s.aggregate_score) AS p75_score,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY s.aggregate_score) AS p95_score,
    STDDEV(s.aggregate_score) AS score_stddev,

    -- Top score (verified)
    (
        SELECT aggregate_score
        FROM submissions s2
        WHERE s2.benchmark_id = b.id
            AND s2.deleted_at IS NULL
            AND s2.visibility = 'public'
            AND s2.verification_level IN ('platform_verified', 'audited')
        ORDER BY aggregate_score DESC
        LIMIT 1
    ) AS top_verified_score,

    -- Timestamps
    MIN(s.created_at) AS first_submission_at,
    MAX(s.created_at) AS latest_submission_at,
    b.created_at AS benchmark_created_at,
    b.updated_at AS benchmark_updated_at

FROM benchmarks b
LEFT JOIN submissions s ON s.benchmark_id = b.id
    AND s.deleted_at IS NULL
    AND s.visibility = 'public'

WHERE b.deleted_at IS NULL

GROUP BY b.id, b.slug, b.name, b.category, b.status, b.created_at, b.updated_at;

-- Indexes on benchmark stats
CREATE UNIQUE INDEX idx_benchmark_stats_id ON benchmark_stats(benchmark_id);
CREATE INDEX idx_benchmark_stats_category ON benchmark_stats(category);
CREATE INDEX idx_benchmark_stats_status ON benchmark_stats(status);
CREATE INDEX idx_benchmark_stats_submissions ON benchmark_stats(total_submissions DESC);
CREATE INDEX idx_benchmark_stats_latest ON benchmark_stats(latest_submission_at DESC NULLS LAST);

COMMENT ON MATERIALIZED VIEW benchmark_stats IS 'Aggregate statistics per benchmark';

-- ============================================================================
-- USER STATISTICS MATERIALIZED VIEW
-- ============================================================================

CREATE MATERIALIZED VIEW user_stats AS
SELECT
    u.id AS user_id,
    u.username,
    u.display_name,
    u.role,

    -- Submission statistics
    COUNT(DISTINCT s.id) AS total_submissions,
    COUNT(DISTINCT s.id) FILTER (
        WHERE s.verification_level IN ('platform_verified', 'audited')
    ) AS verified_submissions,
    COUNT(DISTINCT s.benchmark_id) AS benchmarks_submitted_to,

    -- Best ranks
    MIN(lc.rank_overall) AS best_rank_overall,
    MIN(lc.rank_verified) AS best_rank_verified,

    -- Average score
    AVG(s.aggregate_score) AS avg_score,

    -- Benchmarks created
    COUNT(DISTINCT b.id) AS benchmarks_created,
    COUNT(DISTINCT b.id) FILTER (WHERE b.status = 'active') AS active_benchmarks,

    -- Governance participation
    COUNT(DISTINCT p.id) AS proposals_created,
    COUNT(DISTINCT v.proposal_id) AS votes_cast,
    COUNT(DISTINCT r.id) AS reviews_completed,

    -- Timestamps
    u.created_at AS user_created_at,
    u.last_active_at,
    MAX(s.created_at) AS latest_submission_at

FROM users u
LEFT JOIN submissions s ON s.submitted_by = u.id AND s.deleted_at IS NULL
LEFT JOIN leaderboard_cache lc ON lc.submission_id = s.id
LEFT JOIN benchmarks b ON b.created_by = u.id AND b.deleted_at IS NULL
LEFT JOIN proposals p ON p.created_by = u.id
LEFT JOIN votes v ON v.user_id = u.id
LEFT JOIN reviews r ON r.reviewer_id = u.id AND r.status = 'completed'

WHERE u.deleted_at IS NULL

GROUP BY u.id, u.username, u.display_name, u.role, u.created_at, u.last_active_at;

-- Indexes on user stats
CREATE UNIQUE INDEX idx_user_stats_id ON user_stats(user_id);
CREATE INDEX idx_user_stats_submissions ON user_stats(total_submissions DESC);
CREATE INDEX idx_user_stats_benchmarks ON user_stats(benchmarks_created DESC);
CREATE INDEX idx_user_stats_best_rank ON user_stats(best_rank_overall NULLS LAST);
CREATE INDEX idx_user_stats_active ON user_stats(last_active_at DESC NULLS LAST);

COMMENT ON MATERIALIZED VIEW user_stats IS 'Aggregate statistics per user';

-- ============================================================================
-- MODEL COMPARISON VIEW
-- ============================================================================

CREATE MATERIALIZED VIEW model_comparison AS
SELECT
    s.model_provider,
    s.model_name,
    s.model_version,
    s.model_registry_id,
    b.category AS benchmark_category,

    -- Aggregate metrics across category
    COUNT(DISTINCT s.benchmark_id) AS benchmarks_in_category,
    COUNT(DISTINCT s.id) AS total_submissions,
    AVG(s.aggregate_score) AS avg_score,
    MIN(s.aggregate_score) AS min_score,
    MAX(s.aggregate_score) AS max_score,
    STDDEV(s.aggregate_score) AS score_stddev,

    -- Best rank in category
    MIN(lc.rank_overall) AS best_rank,

    -- Verification stats
    COUNT(DISTINCT s.id) FILTER (
        WHERE s.verification_level IN ('platform_verified', 'audited')
    ) AS verified_count,

    -- Cost efficiency (if available)
    AVG((s.aggregate_score::NUMERIC / NULLIF(
        (SELECT SUM((tcr.cost_usd)::NUMERIC)
         FROM test_case_results tcr
         WHERE tcr.submission_id = s.id),
        0
    ))) AS score_per_dollar,

    -- Latest submission
    MAX(s.created_at) AS latest_submission_at

FROM submissions s
JOIN benchmarks b ON b.id = s.benchmark_id
LEFT JOIN leaderboard_cache lc ON lc.submission_id = s.id

WHERE
    s.deleted_at IS NULL
    AND b.deleted_at IS NULL
    AND s.visibility = 'public'

GROUP BY
    s.model_provider,
    s.model_name,
    s.model_version,
    s.model_registry_id,
    b.category;

-- Indexes on model comparison
CREATE INDEX idx_model_comparison_model ON model_comparison(model_provider, model_name);
CREATE INDEX idx_model_comparison_category ON model_comparison(benchmark_category);
CREATE INDEX idx_model_comparison_avg_score ON model_comparison(avg_score DESC);
CREATE INDEX idx_model_comparison_latest ON model_comparison(latest_submission_at DESC);

COMMENT ON MATERIALIZED VIEW model_comparison IS 'Model performance comparison across categories';

-- ============================================================================
-- REFRESH FUNCTIONS
-- ============================================================================

-- Function to refresh all materialized views
CREATE OR REPLACE FUNCTION refresh_all_materialized_views()
RETURNS void AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY leaderboard_cache;
    REFRESH MATERIALIZED VIEW CONCURRENTLY benchmark_stats;
    REFRESH MATERIALIZED VIEW CONCURRENTLY user_stats;
    REFRESH MATERIALIZED VIEW CONCURRENTLY model_comparison;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION refresh_all_materialized_views() IS 'Refresh all materialized views concurrently';

-- Function to refresh leaderboard cache only
CREATE OR REPLACE FUNCTION refresh_leaderboard_cache()
RETURNS void AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY leaderboard_cache;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION refresh_leaderboard_cache() IS 'Refresh leaderboard cache materialized view';
