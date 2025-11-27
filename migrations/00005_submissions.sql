-- ============================================================================
-- Migration: 00005_submissions.sql
-- Description: Benchmark submissions and results
-- Author: LLM Benchmark Exchange
-- Created: 2025-11-26
-- ============================================================================

-- ============================================================================
-- SUBMISSIONS TABLE
-- ============================================================================

CREATE TABLE submissions (
    -- Primary key
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),

    -- Benchmark reference
    benchmark_id UUID NOT NULL REFERENCES benchmarks(id),
    benchmark_version_id UUID NOT NULL REFERENCES benchmark_versions(id),

    -- Model information
    model_registry_id UUID,  -- Reference to LLM-Registry (external)
    model_provider VARCHAR(100) NOT NULL,
    model_name VARCHAR(200) NOT NULL,
    model_version VARCHAR(100),
    model_api_endpoint VARCHAR(255),
    is_official_submission BOOLEAN NOT NULL DEFAULT FALSE,

    -- Submitter information
    submitted_by UUID NOT NULL REFERENCES users(id),
    organization_id UUID REFERENCES organizations(id),
    is_verified_provider BOOLEAN NOT NULL DEFAULT FALSE,

    -- Aggregate results
    aggregate_score DOUBLE PRECISION NOT NULL,
    confidence_interval_lower DOUBLE PRECISION,
    confidence_interval_upper DOUBLE PRECISION,
    confidence_level DOUBLE PRECISION,
    statistical_significance JSONB,
    -- Example: {"p_value": 0.001, "effect_size": 0.5, "baseline_id": "uuid"}

    -- Verification status
    verification_level verification_level NOT NULL DEFAULT 'unverified',
    verified_at TIMESTAMPTZ,
    verified_by_type VARCHAR(50),  -- 'community', 'platform', 'auditor'
    verified_by_id UUID,
    verified_by_name VARCHAR(200),
    verification_attestation_url VARCHAR(255),
    verification_reproduced_score DOUBLE PRECISION,
    verification_score_variance DOUBLE PRECISION,
    verification_environment_match BOOLEAN,
    verification_notes TEXT,

    -- Execution metadata
    execution_id VARCHAR(100) NOT NULL,
    execution_started_at TIMESTAMPTZ NOT NULL,
    execution_completed_at TIMESTAMPTZ NOT NULL,
    execution_duration_seconds DOUBLE PRECISION NOT NULL,
    executor_version VARCHAR(50) NOT NULL,
    random_seed BIGINT,

    -- Environment information
    environment_platform VARCHAR(50),      -- 'linux', 'darwin', 'windows'
    environment_architecture VARCHAR(50),  -- 'x86_64', 'arm64'
    environment_container_image VARCHAR(255),
    environment_container_digest VARCHAR(100),
    environment_python_version VARCHAR(20),
    environment_package_versions JSONB,
    -- Example: {"numpy": "1.24.3", "torch": "2.0.1"}
    environment_hardware JSONB,
    -- Example: {"cpu": "Intel Xeon", "gpu": "NVIDIA A100", "memory_gb": 64}

    -- Model parameters used
    model_parameters_used JSONB NOT NULL,
    -- Example: {"temperature": 0.7, "max_tokens": 2048, "top_p": 0.95}

    -- Dataset checksums for reproducibility
    dataset_checksums JSONB NOT NULL,
    -- Example: {"train": "sha256:abc123...", "test": "sha256:def456..."}

    -- Visibility control
    visibility submission_visibility NOT NULL DEFAULT 'public',

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ,  -- Soft delete

    -- Constraints
    CONSTRAINT submissions_valid_score CHECK (aggregate_score >= 0),
    CONSTRAINT submissions_valid_duration CHECK (execution_duration_seconds > 0),
    CONSTRAINT submissions_valid_confidence_interval CHECK (
        (confidence_interval_lower IS NULL AND confidence_interval_upper IS NULL) OR
        (confidence_interval_lower IS NOT NULL AND confidence_interval_upper IS NOT NULL AND
         confidence_interval_lower <= aggregate_score AND aggregate_score <= confidence_interval_upper)
    ),
    CONSTRAINT submissions_valid_verification CHECK (
        (verification_level = 'unverified' AND verified_at IS NULL) OR
        (verification_level != 'unverified' AND verified_at IS NOT NULL)
    )
);

-- Indexes for common query patterns
CREATE INDEX idx_submissions_benchmark ON submissions(benchmark_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_submissions_benchmark_version ON submissions(benchmark_version_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_submissions_model ON submissions(model_provider, model_name) WHERE deleted_at IS NULL;
CREATE INDEX idx_submissions_user ON submissions(submitted_by) WHERE deleted_at IS NULL;
CREATE INDEX idx_submissions_org ON submissions(organization_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_submissions_verification ON submissions(verification_level) WHERE deleted_at IS NULL;
CREATE INDEX idx_submissions_visibility ON submissions(visibility) WHERE deleted_at IS NULL;
CREATE INDEX idx_submissions_score ON submissions(aggregate_score DESC) WHERE deleted_at IS NULL;
CREATE INDEX idx_submissions_created ON submissions(created_at DESC) WHERE deleted_at IS NULL;
CREATE INDEX idx_submissions_execution_id ON submissions(execution_id);

-- Composite index for leaderboard queries
CREATE INDEX idx_submissions_leaderboard ON submissions(
    benchmark_id,
    verification_level,
    visibility,
    aggregate_score DESC
) WHERE deleted_at IS NULL;

-- Composite index for model comparison
CREATE INDEX idx_submissions_model_benchmark ON submissions(
    model_provider,
    model_name,
    benchmark_id,
    aggregate_score DESC
) WHERE deleted_at IS NULL AND visibility = 'public';

-- Comments
COMMENT ON TABLE submissions IS 'Benchmark execution results submitted by users';
COMMENT ON COLUMN submissions.model_registry_id IS 'Reference to model in LLM Registry (if available)';
COMMENT ON COLUMN submissions.is_official_submission IS 'Submission from official model provider';
COMMENT ON COLUMN submissions.is_verified_provider IS 'Submitter is verified as legitimate provider';
COMMENT ON COLUMN submissions.execution_id IS 'Unique identifier for this execution run';
COMMENT ON COLUMN submissions.random_seed IS 'Random seed for reproducibility';
COMMENT ON COLUMN submissions.dataset_checksums IS 'SHA-256 checksums of datasets used';

-- ============================================================================
-- METRIC SCORES TABLE
-- ============================================================================

CREATE TABLE metric_scores (
    -- Primary key
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    submission_id UUID NOT NULL REFERENCES submissions(id) ON DELETE CASCADE,

    -- Metric identification
    metric_name VARCHAR(100) NOT NULL,

    -- Score data
    value DOUBLE PRECISION NOT NULL,
    unit VARCHAR(20),
    raw_values DOUBLE PRECISION[],  -- Individual measurements
    std_dev DOUBLE PRECISION,
    min_value DOUBLE PRECISION,
    max_value DOUBLE PRECISION,
    median_value DOUBLE PRECISION,
    percentile_95 DOUBLE PRECISION,

    -- Metadata
    sample_count INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT metric_scores_unique UNIQUE (submission_id, metric_name),
    CONSTRAINT metric_scores_valid_std_dev CHECK (std_dev IS NULL OR std_dev >= 0)
);

CREATE INDEX idx_metric_scores_submission ON metric_scores(submission_id);
CREATE INDEX idx_metric_scores_metric ON metric_scores(metric_name);
CREATE INDEX idx_metric_scores_value ON metric_scores(value DESC);

COMMENT ON TABLE metric_scores IS 'Individual metric scores for submissions';
COMMENT ON COLUMN metric_scores.raw_values IS 'Array of individual measurements for statistical analysis';
COMMENT ON COLUMN metric_scores.std_dev IS 'Standard deviation of raw values';

-- ============================================================================
-- TEST CASE RESULTS TABLE
-- ============================================================================

CREATE TABLE test_case_results (
    -- Primary key
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    submission_id UUID NOT NULL REFERENCES submissions(id) ON DELETE CASCADE,

    -- Test case reference
    test_case_id VARCHAR(100) NOT NULL,

    -- Result
    passed BOOLEAN NOT NULL,
    score DOUBLE PRECISION NOT NULL,

    -- Performance metrics
    latency_ms INTEGER,
    tokens_generated INTEGER,
    tokens_input INTEGER,
    cost_usd DOUBLE PRECISION,

    -- Error information
    error_type VARCHAR(50),
    error_message TEXT,
    error_details JSONB,

    -- Output (optional - may be large)
    actual_output TEXT,
    actual_output_truncated BOOLEAN DEFAULT FALSE,

    -- Evaluation details
    evaluation_details JSONB,
    -- Example: {"judge_score": 0.9, "judge_reasoning": "...", "matched_output_id": "uuid"}

    -- Timestamp
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT test_case_results_unique UNIQUE (submission_id, test_case_id),
    CONSTRAINT test_case_results_valid_score CHECK (score >= 0 AND score <= 1),
    CONSTRAINT test_case_results_valid_latency CHECK (latency_ms IS NULL OR latency_ms > 0),
    CONSTRAINT test_case_results_valid_tokens CHECK (
        (tokens_generated IS NULL OR tokens_generated >= 0) AND
        (tokens_input IS NULL OR tokens_input >= 0)
    ),
    CONSTRAINT test_case_results_valid_cost CHECK (cost_usd IS NULL OR cost_usd >= 0)
);

CREATE INDEX idx_test_case_results_submission ON test_case_results(submission_id);
CREATE INDEX idx_test_case_results_test_case ON test_case_results(test_case_id);
CREATE INDEX idx_test_case_results_passed ON test_case_results(submission_id, passed);
CREATE INDEX idx_test_case_results_score ON test_case_results(score DESC);
CREATE INDEX idx_test_case_results_error ON test_case_results(error_type)
    WHERE error_type IS NOT NULL;

-- GIN index for JSONB evaluation details
CREATE INDEX idx_test_case_results_eval_details ON test_case_results USING gin(evaluation_details);

COMMENT ON TABLE test_case_results IS 'Individual test case results within submissions';
COMMENT ON COLUMN test_case_results.score IS 'Normalized score between 0 and 1';
COMMENT ON COLUMN test_case_results.actual_output IS 'Model output (may be truncated for large outputs)';
COMMENT ON COLUMN test_case_results.evaluation_details IS 'JSON details from evaluation method';

-- ============================================================================
-- EXECUTION METADATA TABLE (Alternative to inline JSONB)
-- ============================================================================

CREATE TABLE execution_metadata (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    submission_id UUID NOT NULL REFERENCES submissions(id) ON DELETE CASCADE,

    -- Metadata key-value pairs
    metadata_key VARCHAR(100) NOT NULL,
    metadata_value JSONB NOT NULL,

    -- Categorization
    category VARCHAR(50) NOT NULL,  -- 'environment', 'model_config', 'execution', 'custom'

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT execution_metadata_unique UNIQUE (submission_id, metadata_key)
);

CREATE INDEX idx_execution_metadata_submission ON execution_metadata(submission_id);
CREATE INDEX idx_execution_metadata_category ON execution_metadata(category);
CREATE INDEX idx_execution_metadata_key ON execution_metadata(metadata_key);

COMMENT ON TABLE execution_metadata IS 'Extensible metadata for submissions';
COMMENT ON COLUMN execution_metadata.category IS 'Logical grouping of metadata';
