-- ============================================================================
-- Migration: 00004_test_cases.sql
-- Description: Test cases and evaluation definitions
-- Author: LLM Benchmark Exchange
-- Created: 2025-11-26
-- ============================================================================

-- ============================================================================
-- TEST CASES TABLE
-- ============================================================================

CREATE TABLE test_cases (
    -- Primary key
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    benchmark_version_id UUID NOT NULL REFERENCES benchmark_versions(id) ON DELETE CASCADE,

    -- Identifiers
    test_case_id VARCHAR(100) NOT NULL,  -- Human-readable ID within benchmark
    name VARCHAR(200) NOT NULL,
    description TEXT,

    -- Weighting and difficulty
    weight DOUBLE PRECISION NOT NULL DEFAULT 1.0,
    difficulty VARCHAR(20),  -- 'easy', 'medium', 'hard', 'expert'

    -- Input configuration
    prompt_template TEXT NOT NULL,
    variables JSONB NOT NULL DEFAULT '{}',
    -- Example: {"context": "...", "question": "...", "choices": ["A", "B", "C", "D"]}

    system_prompt TEXT,
    few_shot_examples JSONB NOT NULL DEFAULT '[]',
    -- Example: [{"input": "...", "output": "..."}, ...]

    input_format VARCHAR(50) NOT NULL DEFAULT 'plain_text',
    -- 'plain_text', 'json', 'markdown', 'code', 'multi_modal'
    input_format_config JSONB,
    -- Format-specific configuration

    -- Expected output
    reference_output TEXT,
    acceptable_outputs TEXT[],  -- Array of acceptable answers
    output_schema JSONB,        -- JSON schema for structured outputs
    output_constraints JSONB NOT NULL DEFAULT '[]',
    -- Example: [{"type": "max_length", "value": 100}, {"type": "required_keywords", "value": ["foo", "bar"]}]

    -- Evaluation method
    evaluation_method JSONB NOT NULL,
    -- Example: {"type": "exact_match"} or {"type": "llm_judge", "judge_prompt": "...", "judge_model": "gpt-4"}

    -- Ordering
    sort_order INTEGER NOT NULL DEFAULT 0,

    -- Timestamp
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT test_cases_unique_id UNIQUE (benchmark_version_id, test_case_id),
    CONSTRAINT test_cases_valid_weight CHECK (weight > 0 AND weight <= 1),
    CONSTRAINT test_cases_valid_difficulty CHECK (
        difficulty IS NULL OR difficulty IN ('easy', 'medium', 'hard', 'expert')
    )
);

-- Indexes
CREATE INDEX idx_test_cases_version ON test_cases(benchmark_version_id);
CREATE INDEX idx_test_cases_difficulty ON test_cases(difficulty);
CREATE INDEX idx_test_cases_sort ON test_cases(benchmark_version_id, sort_order);
CREATE INDEX idx_test_cases_test_id ON test_cases(benchmark_version_id, test_case_id);

-- GIN indexes for JSONB columns
CREATE INDEX idx_test_cases_variables ON test_cases USING gin(variables);
CREATE INDEX idx_test_cases_evaluation ON test_cases USING gin(evaluation_method);

-- Comments
COMMENT ON TABLE test_cases IS 'Individual test cases within benchmark versions';
COMMENT ON COLUMN test_cases.test_case_id IS 'Human-readable identifier unique within benchmark version';
COMMENT ON COLUMN test_cases.weight IS 'Relative weight for aggregation (0 < weight <= 1)';
COMMENT ON COLUMN test_cases.prompt_template IS 'Template string with variable placeholders';
COMMENT ON COLUMN test_cases.variables IS 'JSON object with values to substitute in template';
COMMENT ON COLUMN test_cases.few_shot_examples IS 'Array of example input/output pairs';
COMMENT ON COLUMN test_cases.acceptable_outputs IS 'Array of correct answers for exact matching';
COMMENT ON COLUMN test_cases.output_schema IS 'JSON Schema for validating structured outputs';
COMMENT ON COLUMN test_cases.evaluation_method IS 'JSON configuration for evaluation strategy';
COMMENT ON COLUMN test_cases.sort_order IS 'Display order within benchmark (lower = earlier)';

-- ============================================================================
-- TEST CASE TAGS TABLE
-- ============================================================================

CREATE TABLE test_case_tags (
    test_case_id UUID NOT NULL REFERENCES test_cases(id) ON DELETE CASCADE,
    tag VARCHAR(50) NOT NULL,

    PRIMARY KEY (test_case_id, tag),

    CONSTRAINT test_case_tags_format CHECK (
        tag ~* '^[a-z0-9]+(?:-[a-z0-9]+)*$'
    )
);

CREATE INDEX idx_test_case_tags_tag ON test_case_tags(tag);

COMMENT ON TABLE test_case_tags IS 'Flexible tagging for test case categorization';
COMMENT ON COLUMN test_case_tags.tag IS 'Lowercase, hyphen-separated tag';

-- ============================================================================
-- EXPECTED OUTPUTS TABLE (Alternative to inline storage)
-- ============================================================================

CREATE TABLE expected_outputs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    test_case_id UUID NOT NULL REFERENCES test_cases(id) ON DELETE CASCADE,

    -- Output content
    output_text TEXT NOT NULL,
    output_type VARCHAR(50) NOT NULL DEFAULT 'exact',  -- 'exact', 'pattern', 'semantic'

    -- Matching configuration
    match_config JSONB NOT NULL DEFAULT '{}',
    -- Example: {"case_sensitive": false, "normalize_whitespace": true}

    -- Priority for multiple acceptable outputs
    priority INTEGER NOT NULL DEFAULT 0,

    -- Metadata
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT expected_outputs_valid_type CHECK (
        output_type IN ('exact', 'pattern', 'semantic', 'fuzzy')
    )
);

CREATE INDEX idx_expected_outputs_test_case ON expected_outputs(test_case_id);
CREATE INDEX idx_expected_outputs_priority ON expected_outputs(test_case_id, priority DESC);

COMMENT ON TABLE expected_outputs IS 'Separate storage for expected outputs with matching rules';
COMMENT ON COLUMN expected_outputs.output_type IS 'Type of matching: exact, regex pattern, semantic similarity, fuzzy';
COMMENT ON COLUMN expected_outputs.priority IS 'Higher priority outputs checked first';

-- ============================================================================
-- OUTPUT CONSTRAINTS TABLE
-- ============================================================================

CREATE TABLE output_constraints (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    test_case_id UUID NOT NULL REFERENCES test_cases(id) ON DELETE CASCADE,

    -- Constraint definition
    constraint_type VARCHAR(50) NOT NULL,
    -- 'max_length', 'min_length', 'regex', 'json_schema', 'required_keywords', 'forbidden_keywords'

    constraint_config JSONB NOT NULL,
    -- Example: {"value": 100} for max_length, {"pattern": "^[A-D]$"} for regex

    -- Enforcement
    is_required BOOLEAN NOT NULL DEFAULT TRUE,
    severity VARCHAR(20) NOT NULL DEFAULT 'error',  -- 'error', 'warning', 'info'

    -- Metadata
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT output_constraints_valid_severity CHECK (
        severity IN ('error', 'warning', 'info')
    )
);

CREATE INDEX idx_output_constraints_test_case ON output_constraints(test_case_id);
CREATE INDEX idx_output_constraints_type ON output_constraints(constraint_type);
CREATE INDEX idx_output_constraints_required ON output_constraints(test_case_id, is_required)
    WHERE is_required = TRUE;

COMMENT ON TABLE output_constraints IS 'Validation rules for test case outputs';
COMMENT ON COLUMN output_constraints.constraint_type IS 'Type of validation to perform';
COMMENT ON COLUMN output_constraints.is_required IS 'Whether constraint must pass for test to succeed';
COMMENT ON COLUMN output_constraints.severity IS 'Impact level: error (fail), warning (flag), info (log)';
