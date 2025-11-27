-- ============================================================================
-- Migration: 00003_benchmarks.sql
-- Description: Benchmark definitions with versioning and metadata
-- Author: LLM Benchmark Exchange
-- Created: 2025-11-26
-- ============================================================================

-- ============================================================================
-- BENCHMARKS TABLE
-- ============================================================================

CREATE TABLE benchmarks (
    -- Primary key
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),

    -- Basic information
    slug VARCHAR(100) NOT NULL UNIQUE,
    name VARCHAR(200) NOT NULL,
    description TEXT NOT NULL,
    long_description TEXT,
    category benchmark_category NOT NULL,
    status benchmark_status NOT NULL DEFAULT 'draft',

    -- Licensing
    license_type VARCHAR(50) NOT NULL,
    license_custom TEXT,

    -- External resources
    documentation_url VARCHAR(255),
    source_url VARCHAR(255),

    -- Citation information
    citation_title VARCHAR(500),
    citation_authors TEXT[],
    citation_venue VARCHAR(255),
    citation_year INTEGER,
    citation_doi VARCHAR(100),
    citation_bibtex TEXT,

    -- Ownership
    created_by UUID NOT NULL REFERENCES users(id),

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ,  -- Soft delete

    -- Constraints
    CONSTRAINT benchmarks_slug_format CHECK (
        slug ~* '^[a-z0-9]+(?:-[a-z0-9]+)*$'
    ),
    CONSTRAINT benchmarks_description_length CHECK (
        LENGTH(description) >= 50
    ),
    CONSTRAINT benchmarks_citation_year_valid CHECK (
        citation_year IS NULL OR (citation_year >= 1950 AND citation_year <= EXTRACT(YEAR FROM NOW()) + 1)
    )
);

-- Indexes
CREATE INDEX idx_benchmarks_slug ON benchmarks(slug) WHERE deleted_at IS NULL;
CREATE INDEX idx_benchmarks_category ON benchmarks(category) WHERE deleted_at IS NULL;
CREATE INDEX idx_benchmarks_status ON benchmarks(status) WHERE deleted_at IS NULL;
CREATE INDEX idx_benchmarks_created_by ON benchmarks(created_by) WHERE deleted_at IS NULL;
CREATE INDEX idx_benchmarks_created_at ON benchmarks(created_at DESC);

-- Full-text search index using trigrams
CREATE INDEX idx_benchmarks_name_trgm ON benchmarks USING gin(name gin_trgm_ops)
    WHERE deleted_at IS NULL;
CREATE INDEX idx_benchmarks_description_trgm ON benchmarks USING gin(description gin_trgm_ops)
    WHERE deleted_at IS NULL;

-- Comments
COMMENT ON TABLE benchmarks IS 'Benchmark definitions with metadata and versioning';
COMMENT ON COLUMN benchmarks.slug IS 'URL-safe unique identifier (e.g., mmlu-pro)';
COMMENT ON COLUMN benchmarks.description IS 'Short description (minimum 50 characters)';
COMMENT ON COLUMN benchmarks.long_description IS 'Extended markdown description with details';
COMMENT ON COLUMN benchmarks.citation_bibtex IS 'BibTeX citation for academic papers';

-- ============================================================================
-- BENCHMARK MAINTAINERS TABLE
-- ============================================================================

CREATE TABLE benchmark_maintainers (
    benchmark_id UUID NOT NULL REFERENCES benchmarks(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (benchmark_id, user_id)
);

CREATE INDEX idx_benchmark_maintainers_user ON benchmark_maintainers(user_id);
CREATE INDEX idx_benchmark_maintainers_added ON benchmark_maintainers(added_at DESC);

COMMENT ON TABLE benchmark_maintainers IS 'Users with maintainer privileges for benchmarks';

-- ============================================================================
-- BENCHMARK TAGS TABLE
-- ============================================================================

CREATE TABLE benchmark_tags (
    benchmark_id UUID NOT NULL REFERENCES benchmarks(id) ON DELETE CASCADE,
    tag VARCHAR(50) NOT NULL,

    PRIMARY KEY (benchmark_id, tag),

    CONSTRAINT benchmark_tags_format CHECK (
        tag ~* '^[a-z0-9]+(?:-[a-z0-9]+)*$'
    )
);

CREATE INDEX idx_benchmark_tags_tag ON benchmark_tags(tag);

COMMENT ON TABLE benchmark_tags IS 'Flexible tagging for benchmark discovery';
COMMENT ON COLUMN benchmark_tags.tag IS 'Lowercase, hyphen-separated tag (e.g., multi-turn)';

-- ============================================================================
-- BENCHMARK VERSIONS TABLE
-- ============================================================================

CREATE TABLE benchmark_versions (
    -- Primary key
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    benchmark_id UUID NOT NULL REFERENCES benchmarks(id) ON DELETE CASCADE,

    -- Semantic versioning
    version_major INTEGER NOT NULL,
    version_minor INTEGER NOT NULL,
    version_patch INTEGER NOT NULL,
    version_prerelease VARCHAR(50),     -- e.g., 'alpha', 'beta.1', 'rc.2'
    version_build_metadata VARCHAR(50), -- e.g., build number, commit hash

    -- Version history
    parent_version_id UUID REFERENCES benchmark_versions(id),
    changelog TEXT NOT NULL,
    breaking_changes BOOLEAN NOT NULL DEFAULT FALSE,
    migration_notes TEXT,

    -- Primary metric definition (denormalized for performance)
    primary_metric_name VARCHAR(100) NOT NULL,
    primary_metric_description TEXT NOT NULL,
    primary_metric_type VARCHAR(50) NOT NULL, -- 'accuracy', 'f1', 'bleu', 'latency_p95', etc.
    primary_metric_unit VARCHAR(20),          -- 'seconds', 'tokens', 'percent', etc.
    primary_metric_higher_is_better BOOLEAN NOT NULL DEFAULT TRUE,
    primary_metric_range_min DOUBLE PRECISION,
    primary_metric_range_max DOUBLE PRECISION,

    -- Secondary metrics (stored as JSONB array)
    secondary_metrics JSONB NOT NULL DEFAULT '[]',
    -- Example: [{"name": "f1_score", "description": "F1 Score", "type": "f1", "unit": null, "higher_is_better": true}]

    -- Aggregation and scoring
    aggregation_method JSONB NOT NULL,
    -- Example: {"type": "weighted_average", "weights": {"accuracy": 0.6, "f1": 0.4}}
    score_normalization JSONB NOT NULL DEFAULT '{"type": "none"}',
    -- Example: {"type": "min_max", "min": 0, "max": 100} or {"type": "z_score"}

    -- Test requirements
    minimum_test_cases INTEGER NOT NULL DEFAULT 1,
    confidence_level DOUBLE PRECISION NOT NULL DEFAULT 0.95,

    -- Execution configuration
    timeout_per_test_ms INTEGER NOT NULL DEFAULT 30000,
    max_retries INTEGER NOT NULL DEFAULT 3,
    retry_delay_ms INTEGER NOT NULL DEFAULT 1000,
    max_concurrent_requests INTEGER NOT NULL DEFAULT 10,
    rate_limit_per_minute INTEGER,

    -- Model parameters (JSONB for flexibility)
    model_parameters JSONB NOT NULL DEFAULT '{}',
    -- Example: {"temperature": 0.7, "max_tokens": 2048, "top_p": 0.9}

    -- Environment requirements
    environment_requirements JSONB NOT NULL DEFAULT '{}',
    -- Example: {"python_version": ">=3.9", "packages": {"numpy": ">=1.20"}}

    -- Dataset references
    dataset_references JSONB NOT NULL DEFAULT '[]',
    -- Example: [{"name": "MMLU", "url": "https://...", "version": "1.0", "checksum": "sha256:..."}]

    -- Ownership
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT benchmark_versions_semver_unique UNIQUE (
        benchmark_id, version_major, version_minor, version_patch, version_prerelease
    ),
    CONSTRAINT benchmark_versions_valid_confidence CHECK (
        confidence_level >= 0.5 AND confidence_level <= 0.99
    ),
    CONSTRAINT benchmark_versions_valid_timeout CHECK (
        timeout_per_test_ms > 0 AND timeout_per_test_ms <= 600000
    ),
    CONSTRAINT benchmark_versions_valid_retries CHECK (
        max_retries >= 0 AND max_retries <= 10
    ),
    CONSTRAINT benchmark_versions_valid_concurrent CHECK (
        max_concurrent_requests > 0 AND max_concurrent_requests <= 100
    ),
    CONSTRAINT benchmark_versions_valid_metric_range CHECK (
        primary_metric_range_min IS NULL OR primary_metric_range_max IS NULL OR
        primary_metric_range_min < primary_metric_range_max
    )
);

-- Indexes
CREATE INDEX idx_benchmark_versions_benchmark ON benchmark_versions(benchmark_id);
CREATE INDEX idx_benchmark_versions_created ON benchmark_versions(created_at DESC);
CREATE INDEX idx_benchmark_versions_semver ON benchmark_versions(
    benchmark_id, version_major DESC, version_minor DESC, version_patch DESC
);

-- Comments
COMMENT ON TABLE benchmark_versions IS 'Versioned benchmark configurations with evaluation criteria';
COMMENT ON COLUMN benchmark_versions.version_prerelease IS 'Pre-release identifier (alpha, beta, rc)';
COMMENT ON COLUMN benchmark_versions.parent_version_id IS 'Previous version for version history tracking';
COMMENT ON COLUMN benchmark_versions.breaking_changes IS 'Indicates incompatible changes from previous version';
COMMENT ON COLUMN benchmark_versions.aggregation_method IS 'JSON configuration for score aggregation';
COMMENT ON COLUMN benchmark_versions.model_parameters IS 'Default model parameters for submissions';

-- ============================================================================
-- BENCHMARK SUBCATEGORIES TABLE
-- ============================================================================

CREATE TABLE benchmark_subcategories (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    benchmark_version_id UUID NOT NULL REFERENCES benchmark_versions(id) ON DELETE CASCADE,
    parent_category benchmark_category NOT NULL,
    name VARCHAR(100) NOT NULL,
    description TEXT,

    CONSTRAINT benchmark_subcategories_unique UNIQUE (benchmark_version_id, name)
);

CREATE INDEX idx_benchmark_subcategories_version ON benchmark_subcategories(benchmark_version_id);
CREATE INDEX idx_benchmark_subcategories_category ON benchmark_subcategories(parent_category);

COMMENT ON TABLE benchmark_subcategories IS 'Hierarchical subcategories within benchmarks';
COMMENT ON COLUMN benchmark_subcategories.parent_category IS 'Primary category this subcategory belongs to';
