-- ============================================================================
-- Migration: 00010_functions.sql
-- Description: Database functions, triggers, and utilities
-- Author: LLM Benchmark Exchange
-- Created: 2025-11-26
-- ============================================================================

-- ============================================================================
-- UUID v7 GENERATION FUNCTION
-- ============================================================================

-- UUID v7 includes timestamp for better indexing performance
CREATE OR REPLACE FUNCTION uuid_generate_v7()
RETURNS UUID AS $$
DECLARE
    unix_ts_ms BIGINT;
    uuid_bytes BYTEA;
BEGIN
    -- Get current timestamp in milliseconds
    unix_ts_ms := (EXTRACT(EPOCH FROM clock_timestamp()) * 1000)::BIGINT;

    -- Start with timestamp bytes (48 bits)
    uuid_bytes := substring(int8send(unix_ts_ms) from 3);

    -- Add random bytes (80 bits)
    uuid_bytes := uuid_bytes || gen_random_bytes(10);

    -- Set version to 7 (4 bits)
    uuid_bytes := set_byte(uuid_bytes, 6, (get_byte(uuid_bytes, 6) & 15) | 112);

    -- Set variant to RFC 4122 (2 bits)
    uuid_bytes := set_byte(uuid_bytes, 8, (get_byte(uuid_bytes, 8) & 63) | 128);

    RETURN encode(uuid_bytes, 'hex')::UUID;
END;
$$ LANGUAGE plpgsql VOLATILE;

COMMENT ON FUNCTION uuid_generate_v7() IS 'Generate time-ordered UUID v7 for better index performance';

-- ============================================================================
-- UPDATED_AT TRIGGER FUNCTION
-- ============================================================================

CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION update_updated_at_column() IS 'Automatically update updated_at timestamp on row changes';

-- Apply triggers to tables with updated_at column
CREATE TRIGGER update_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_organizations_updated_at
    BEFORE UPDATE ON organizations
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_benchmarks_updated_at
    BEFORE UPDATE ON benchmarks
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_submissions_updated_at
    BEFORE UPDATE ON submissions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_proposals_updated_at
    BEFORE UPDATE ON proposals
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_community_verifications_updated_at
    BEFORE UPDATE ON community_verifications
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_review_comments_updated_at
    BEFORE UPDATE ON review_comments
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_event_subscriptions_updated_at
    BEFORE UPDATE ON event_subscriptions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- BENCHMARK VERSION INCREMENT FUNCTION
-- ============================================================================

CREATE OR REPLACE FUNCTION increment_benchmark_version(
    p_benchmark_id UUID,
    p_version_type VARCHAR,  -- 'major', 'minor', 'patch'
    p_changelog TEXT,
    p_created_by UUID
)
RETURNS UUID AS $$
DECLARE
    v_latest_version RECORD;
    v_new_major INTEGER;
    v_new_minor INTEGER;
    v_new_patch INTEGER;
    v_new_version_id UUID;
BEGIN
    -- Get latest version
    SELECT version_major, version_minor, version_patch, id
    INTO v_latest_version
    FROM benchmark_versions
    WHERE benchmark_id = p_benchmark_id
    ORDER BY version_major DESC, version_minor DESC, version_patch DESC
    LIMIT 1;

    -- Calculate new version numbers
    IF p_version_type = 'major' THEN
        v_new_major := COALESCE(v_latest_version.version_major, 0) + 1;
        v_new_minor := 0;
        v_new_patch := 0;
    ELSIF p_version_type = 'minor' THEN
        v_new_major := COALESCE(v_latest_version.version_major, 0);
        v_new_minor := COALESCE(v_latest_version.version_minor, 0) + 1;
        v_new_patch := 0;
    ELSIF p_version_type = 'patch' THEN
        v_new_major := COALESCE(v_latest_version.version_major, 0);
        v_new_minor := COALESCE(v_latest_version.version_minor, 0);
        v_new_patch := COALESCE(v_latest_version.version_patch, 0) + 1;
    ELSE
        RAISE EXCEPTION 'Invalid version_type: %. Must be major, minor, or patch.', p_version_type;
    END IF;

    -- Create new version (minimal insert - application will update with full data)
    INSERT INTO benchmark_versions (
        benchmark_id,
        version_major,
        version_minor,
        version_patch,
        parent_version_id,
        changelog,
        created_by,
        primary_metric_name,
        primary_metric_description,
        primary_metric_type,
        aggregation_method
    )
    VALUES (
        p_benchmark_id,
        v_new_major,
        v_new_minor,
        v_new_patch,
        v_latest_version.id,
        p_changelog,
        p_created_by,
        'placeholder',
        'placeholder',
        'placeholder',
        '{}'::JSONB
    )
    RETURNING id INTO v_new_version_id;

    RETURN v_new_version_id;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION increment_benchmark_version IS 'Create new benchmark version with automatic version numbering';

-- ============================================================================
-- VOTE COUNT UPDATE FUNCTION
-- ============================================================================

CREATE OR REPLACE FUNCTION update_proposal_vote_counts()
RETURNS TRIGGER AS $$
BEGIN
    -- Recalculate vote counts for the proposal
    UPDATE proposals
    SET
        votes_for = (
            SELECT COUNT(*) FROM votes
            WHERE proposal_id = COALESCE(NEW.proposal_id, OLD.proposal_id)
            AND vote = 'approve'
        ),
        votes_against = (
            SELECT COUNT(*) FROM votes
            WHERE proposal_id = COALESCE(NEW.proposal_id, OLD.proposal_id)
            AND vote = 'reject'
        ),
        votes_abstain = (
            SELECT COUNT(*) FROM votes
            WHERE proposal_id = COALESCE(NEW.proposal_id, OLD.proposal_id)
            AND vote = 'abstain'
        )
    WHERE id = COALESCE(NEW.proposal_id, OLD.proposal_id);

    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION update_proposal_vote_counts() IS 'Automatically update proposal vote counts';

-- Create trigger for vote count updates
CREATE TRIGGER update_vote_counts_on_insert
    AFTER INSERT ON votes
    FOR EACH ROW
    EXECUTE FUNCTION update_proposal_vote_counts();

CREATE TRIGGER update_vote_counts_on_update
    AFTER UPDATE ON votes
    FOR EACH ROW
    EXECUTE FUNCTION update_proposal_vote_counts();

CREATE TRIGGER update_vote_counts_on_delete
    AFTER DELETE ON votes
    FOR EACH ROW
    EXECUTE FUNCTION update_proposal_vote_counts();

-- ============================================================================
-- COMMUNITY VERIFICATION VOTE COUNT UPDATE FUNCTION
-- ============================================================================

CREATE OR REPLACE FUNCTION update_community_verification_votes()
RETURNS TRIGGER AS $$
BEGIN
    -- Recalculate vote counts
    UPDATE community_verifications
    SET
        upvotes = (
            SELECT COUNT(*) FROM verification_votes
            WHERE community_verification_id = COALESCE(NEW.community_verification_id, OLD.community_verification_id)
            AND vote_type = 'upvote'
        ),
        downvotes = (
            SELECT COUNT(*) FROM verification_votes
            WHERE community_verification_id = COALESCE(NEW.community_verification_id, OLD.community_verification_id)
            AND vote_type = 'downvote'
        )
    WHERE id = COALESCE(NEW.community_verification_id, OLD.community_verification_id);

    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION update_community_verification_votes() IS 'Automatically update community verification vote counts';

-- Create trigger for verification vote count updates
CREATE TRIGGER update_verification_votes_on_insert
    AFTER INSERT ON verification_votes
    FOR EACH ROW
    EXECUTE FUNCTION update_community_verification_votes();

CREATE TRIGGER update_verification_votes_on_update
    AFTER UPDATE ON verification_votes
    FOR EACH ROW
    EXECUTE FUNCTION update_community_verification_votes();

CREATE TRIGGER update_verification_votes_on_delete
    AFTER DELETE ON verification_votes
    FOR EACH ROW
    EXECUTE FUNCTION update_community_verification_votes();

-- ============================================================================
-- SOFT DELETE FUNCTION
-- ============================================================================

CREATE OR REPLACE FUNCTION soft_delete(
    p_table_name TEXT,
    p_record_id UUID
)
RETURNS BOOLEAN AS $$
DECLARE
    v_sql TEXT;
    v_result BOOLEAN;
BEGIN
    -- Validate table name to prevent SQL injection
    IF p_table_name NOT IN (
        'users', 'organizations', 'benchmarks', 'submissions'
    ) THEN
        RAISE EXCEPTION 'Invalid table name for soft delete: %', p_table_name;
    END IF;

    -- Execute soft delete
    v_sql := format(
        'UPDATE %I SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL RETURNING TRUE',
        p_table_name
    );

    EXECUTE v_sql INTO v_result USING p_record_id;

    RETURN COALESCE(v_result, FALSE);
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION soft_delete IS 'Soft delete a record by setting deleted_at timestamp';

-- ============================================================================
-- CALCULATE AGGREGATE SCORE FUNCTION
-- ============================================================================

CREATE OR REPLACE FUNCTION calculate_aggregate_score(
    p_benchmark_version_id UUID,
    p_test_case_results JSONB  -- Array of {test_case_id, score}
)
RETURNS DOUBLE PRECISION AS $$
DECLARE
    v_aggregation_method JSONB;
    v_method_type TEXT;
    v_total_score DOUBLE PRECISION := 0;
    v_total_weight DOUBLE PRECISION := 0;
    v_test_case RECORD;
    v_result JSONB;
BEGIN
    -- Get aggregation method
    SELECT aggregation_method INTO v_aggregation_method
    FROM benchmark_versions
    WHERE id = p_benchmark_version_id;

    v_method_type := v_aggregation_method->>'type';

    -- Calculate based on method type
    IF v_method_type = 'weighted_average' THEN
        -- Sum weighted scores
        FOR v_result IN SELECT * FROM jsonb_array_elements(p_test_case_results)
        LOOP
            SELECT tc.weight, (v_result->>'score')::DOUBLE PRECISION
            INTO v_test_case
            FROM test_cases tc
            WHERE tc.benchmark_version_id = p_benchmark_version_id
                AND tc.test_case_id = v_result->>'test_case_id';

            IF FOUND THEN
                v_total_score := v_total_score + (v_test_case.weight * (v_result->>'score')::DOUBLE PRECISION);
                v_total_weight := v_total_weight + v_test_case.weight;
            END IF;
        END LOOP;

        -- Return weighted average
        IF v_total_weight > 0 THEN
            RETURN v_total_score / v_total_weight;
        ELSE
            RETURN 0;
        END IF;

    ELSIF v_method_type = 'simple_average' THEN
        -- Simple average
        SELECT AVG((value->>'score')::DOUBLE PRECISION)
        INTO v_total_score
        FROM jsonb_array_elements(p_test_case_results) AS value;

        RETURN COALESCE(v_total_score, 0);

    ELSE
        RAISE EXCEPTION 'Unsupported aggregation method: %', v_method_type;
    END IF;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION calculate_aggregate_score IS 'Calculate aggregate score based on benchmark aggregation method';

-- ============================================================================
-- SEARCH BENCHMARKS FUNCTION
-- ============================================================================

CREATE OR REPLACE FUNCTION search_benchmarks(
    p_query TEXT,
    p_limit INTEGER DEFAULT 20
)
RETURNS TABLE (
    benchmark_id UUID,
    slug VARCHAR,
    name VARCHAR,
    description TEXT,
    category benchmark_category,
    similarity REAL
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        b.id,
        b.slug,
        b.name,
        b.description,
        b.category,
        GREATEST(
            similarity(b.name, p_query),
            similarity(b.description, p_query)
        ) AS similarity
    FROM benchmarks b
    WHERE
        b.deleted_at IS NULL
        AND (
            b.name % p_query
            OR b.description % p_query
            OR b.slug % p_query
        )
    ORDER BY similarity DESC
    LIMIT p_limit;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION search_benchmarks IS 'Full-text search for benchmarks using trigram similarity';

-- ============================================================================
-- GET LEADERBOARD FUNCTION
-- ============================================================================

CREATE OR REPLACE FUNCTION get_leaderboard(
    p_benchmark_id UUID,
    p_verification_level verification_level DEFAULT NULL,
    p_limit INTEGER DEFAULT 100
)
RETURNS TABLE (
    rank BIGINT,
    submission_id UUID,
    model_provider VARCHAR,
    model_name VARCHAR,
    model_version VARCHAR,
    aggregate_score DOUBLE PRECISION,
    verification_level verification_level,
    is_official_submission BOOLEAN,
    submitter_username VARCHAR,
    organization_name VARCHAR,
    submission_created_at TIMESTAMPTZ
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        lc.rank_overall,
        lc.submission_id,
        lc.model_provider,
        lc.model_name,
        lc.model_version,
        lc.aggregate_score,
        lc.verification_level,
        lc.is_official_submission,
        lc.submitter_username,
        lc.organization_name,
        lc.submission_created_at
    FROM leaderboard_cache lc
    WHERE
        lc.benchmark_id = p_benchmark_id
        AND (p_verification_level IS NULL OR lc.verification_level = p_verification_level)
    ORDER BY lc.rank_overall
    LIMIT p_limit;
END;
$$ LANGUAGE plpgsql STABLE;

COMMENT ON FUNCTION get_leaderboard IS 'Get leaderboard entries for a benchmark with optional filtering';

-- ============================================================================
-- PARTITION MAINTENANCE FUNCTION
-- ============================================================================

CREATE OR REPLACE FUNCTION create_next_month_partitions()
RETURNS void AS $$
DECLARE
    v_next_month DATE;
    v_month_after DATE;
    v_partition_name TEXT;
    v_sql TEXT;
BEGIN
    v_next_month := DATE_TRUNC('month', NOW() + INTERVAL '1 month');
    v_month_after := v_next_month + INTERVAL '1 month';

    -- Create domain_events partition
    v_partition_name := 'domain_events_' || TO_CHAR(v_next_month, 'YYYY_MM');

    IF NOT EXISTS (
        SELECT 1 FROM pg_class WHERE relname = v_partition_name
    ) THEN
        v_sql := format(
            'CREATE TABLE %I PARTITION OF domain_events FOR VALUES FROM (%L) TO (%L)',
            v_partition_name,
            v_next_month,
            v_month_after
        );
        EXECUTE v_sql;
        RAISE NOTICE 'Created partition: %', v_partition_name;
    END IF;

    -- Create audit_log partition
    v_partition_name := 'audit_log_' || TO_CHAR(v_next_month, 'YYYY_MM');

    IF NOT EXISTS (
        SELECT 1 FROM pg_class WHERE relname = v_partition_name
    ) THEN
        v_sql := format(
            'CREATE TABLE %I PARTITION OF audit_log FOR VALUES FROM (%L) TO (%L)',
            v_partition_name,
            v_next_month,
            v_month_after
        );
        EXECUTE v_sql;
        RAISE NOTICE 'Created partition: %', v_partition_name;
    END IF;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION create_next_month_partitions() IS 'Create partitions for the next month (run monthly)';

-- ============================================================================
-- CLEANUP OLD PARTITIONS FUNCTION
-- ============================================================================

CREATE OR REPLACE FUNCTION cleanup_old_partitions(
    p_retention_months INTEGER DEFAULT 24
)
RETURNS void AS $$
DECLARE
    v_cutoff_date DATE;
    v_partition RECORD;
BEGIN
    v_cutoff_date := DATE_TRUNC('month', NOW() - (p_retention_months || ' months')::INTERVAL);

    -- Drop old domain_events partitions
    FOR v_partition IN
        SELECT tablename
        FROM pg_tables
        WHERE schemaname = 'public'
            AND tablename LIKE 'domain_events_%'
            AND tablename < 'domain_events_' || TO_CHAR(v_cutoff_date, 'YYYY_MM')
    LOOP
        EXECUTE 'DROP TABLE IF EXISTS ' || v_partition.tablename;
        RAISE NOTICE 'Dropped partition: %', v_partition.tablename;
    END LOOP;

    -- Note: Keep audit_log for compliance (7 years typical requirement)
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION cleanup_old_partitions IS 'Drop partitions older than retention period';
