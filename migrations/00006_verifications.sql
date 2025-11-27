-- ============================================================================
-- Migration: 00006_verifications.sql
-- Description: Verification workflows and attestations
-- Author: LLM Benchmark Exchange
-- Created: 2025-11-26
-- ============================================================================

-- ============================================================================
-- VERIFICATIONS TABLE
-- ============================================================================

CREATE TABLE verifications (
    -- Primary key
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),

    -- Submission being verified
    submission_id UUID NOT NULL REFERENCES submissions(id),

    -- Verification workflow
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    -- 'pending', 'in_progress', 'completed', 'failed', 'cancelled'

    -- Verifier information
    verifier_type VARCHAR(50) NOT NULL,  -- 'community', 'platform', 'auditor'
    verifier_id UUID,  -- User ID or system ID
    verifier_name VARCHAR(200),

    -- Timing
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,

    -- Results
    success BOOLEAN,
    reproduced_score DOUBLE PRECISION,
    score_variance DOUBLE PRECISION,
    environment_match BOOLEAN,
    confidence DOUBLE PRECISION,  -- Confidence in verification result (0-1)

    -- Details and notes
    details TEXT,
    failure_reason TEXT,

    -- Attestation
    attestation_url VARCHAR(255),
    attestation_signature TEXT,  -- Cryptographic signature
    attestation_metadata JSONB,

    -- Metadata
    verification_config JSONB,
    -- Example: {"attempts": 3, "timeout_per_attempt": 300, "tolerance": 0.01}

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT verifications_valid_status CHECK (
        status IN ('pending', 'in_progress', 'completed', 'failed', 'cancelled')
    ),
    CONSTRAINT verifications_valid_confidence CHECK (
        confidence IS NULL OR (confidence >= 0 AND confidence <= 1)
    ),
    CONSTRAINT verifications_completed_requirements CHECK (
        (status = 'completed' AND completed_at IS NOT NULL AND success IS NOT NULL) OR
        (status != 'completed')
    ),
    CONSTRAINT verifications_score_variance_check CHECK (
        score_variance IS NULL OR score_variance >= 0
    )
);

-- Indexes
CREATE INDEX idx_verifications_submission ON verifications(submission_id);
CREATE INDEX idx_verifications_status ON verifications(status);
CREATE INDEX idx_verifications_verifier ON verifications(verifier_type, verifier_id);
CREATE INDEX idx_verifications_started ON verifications(started_at DESC);
CREATE INDEX idx_verifications_completed ON verifications(completed_at DESC)
    WHERE completed_at IS NOT NULL;
CREATE INDEX idx_verifications_pending ON verifications(created_at)
    WHERE status = 'pending';

COMMENT ON TABLE verifications IS 'Verification attempts for submitted results';
COMMENT ON COLUMN verifications.verifier_type IS 'Type of verifier: community, platform, or external auditor';
COMMENT ON COLUMN verifications.score_variance IS 'Absolute difference between original and reproduced score';
COMMENT ON COLUMN verifications.environment_match IS 'Whether execution environment matched original submission';
COMMENT ON COLUMN verifications.confidence IS 'Confidence level in verification result (0-1)';
COMMENT ON COLUMN verifications.attestation_signature IS 'Cryptographic signature of verification';

-- ============================================================================
-- VERIFICATION ATTEMPTS TABLE
-- ============================================================================

CREATE TABLE verification_attempts (
    -- Primary key
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    verification_id UUID NOT NULL REFERENCES verifications(id) ON DELETE CASCADE,

    -- Attempt number (1-indexed)
    attempt_number INTEGER NOT NULL,

    -- Results
    score DOUBLE PRECISION NOT NULL,
    execution_time_ms INTEGER NOT NULL,
    environment_hash VARCHAR(64),
    success BOOLEAN NOT NULL,

    -- Error information
    error TEXT,
    error_details JSONB,

    -- Raw output (optional)
    raw_output_url VARCHAR(255),
    logs_url VARCHAR(255),

    -- Timing
    started_at TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT verification_attempts_unique UNIQUE (verification_id, attempt_number),
    CONSTRAINT verification_attempts_valid_number CHECK (attempt_number > 0),
    CONSTRAINT verification_attempts_valid_time CHECK (execution_time_ms > 0),
    CONSTRAINT verification_attempts_timing CHECK (completed_at >= started_at)
);

CREATE INDEX idx_verification_attempts_verification ON verification_attempts(verification_id);
CREATE INDEX idx_verification_attempts_number ON verification_attempts(verification_id, attempt_number);
CREATE INDEX idx_verification_attempts_success ON verification_attempts(success);
CREATE INDEX idx_verification_attempts_created ON verification_attempts(created_at DESC);

COMMENT ON TABLE verification_attempts IS 'Individual attempts within a verification process';
COMMENT ON COLUMN verification_attempts.attempt_number IS 'Sequential attempt number (1, 2, 3, ...)';
COMMENT ON COLUMN verification_attempts.environment_hash IS 'Hash of environment configuration for reproducibility';

-- ============================================================================
-- VERIFICATION STEPS TABLE
-- ============================================================================

CREATE TABLE verification_steps (
    -- Primary key
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    verification_id UUID NOT NULL REFERENCES verifications(id) ON DELETE CASCADE,

    -- Step information
    step_number INTEGER NOT NULL,
    step_name VARCHAR(100) NOT NULL,
    step_description TEXT,

    -- Status
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    -- 'pending', 'in_progress', 'completed', 'failed', 'skipped'

    -- Results
    success BOOLEAN,
    output JSONB,
    error_message TEXT,

    -- Timing
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    duration_ms INTEGER,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT verification_steps_unique UNIQUE (verification_id, step_number),
    CONSTRAINT verification_steps_valid_status CHECK (
        status IN ('pending', 'in_progress', 'completed', 'failed', 'skipped')
    ),
    CONSTRAINT verification_steps_valid_duration CHECK (
        duration_ms IS NULL OR duration_ms >= 0
    )
);

CREATE INDEX idx_verification_steps_verification ON verification_steps(verification_id);
CREATE INDEX idx_verification_steps_number ON verification_steps(verification_id, step_number);
CREATE INDEX idx_verification_steps_status ON verification_steps(status);

COMMENT ON TABLE verification_steps IS 'Granular steps in verification process for transparency';
COMMENT ON COLUMN verification_steps.step_number IS 'Sequential step number within verification';
COMMENT ON COLUMN verification_steps.output IS 'JSON output from step execution';

-- ============================================================================
-- COMMUNITY VERIFICATIONS TABLE
-- ============================================================================

CREATE TABLE community_verifications (
    -- Primary key
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),

    -- Submission being verified
    submission_id UUID NOT NULL REFERENCES submissions(id),

    -- Verifier (must be a user)
    verifier_id UUID NOT NULL REFERENCES users(id),

    -- Verification claim
    reproduced BOOLEAN NOT NULL,
    reproduced_score DOUBLE PRECISION,
    score_difference DOUBLE PRECISION,

    -- Environment details
    environment_description TEXT NOT NULL,
    environment_hash VARCHAR(64),
    environment_details JSONB,

    -- Evidence
    evidence_url VARCHAR(255),
    evidence_description TEXT,
    reproduction_notes TEXT NOT NULL,

    -- Trust signals
    upvotes INTEGER NOT NULL DEFAULT 0,
    downvotes INTEGER NOT NULL DEFAULT 0,
    flagged BOOLEAN NOT NULL DEFAULT FALSE,
    flag_reason TEXT,

    -- Status
    reviewed BOOLEAN NOT NULL DEFAULT FALSE,
    review_status VARCHAR(20),  -- 'approved', 'rejected', 'needs_review'
    reviewed_by UUID REFERENCES users(id),
    reviewed_at TIMESTAMPTZ,
    review_notes TEXT,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT community_verifications_valid_votes CHECK (
        upvotes >= 0 AND downvotes >= 0
    ),
    CONSTRAINT community_verifications_valid_review CHECK (
        (reviewed = FALSE) OR
        (reviewed = TRUE AND review_status IS NOT NULL AND reviewed_by IS NOT NULL)
    )
);

CREATE INDEX idx_community_verifications_submission ON community_verifications(submission_id);
CREATE INDEX idx_community_verifications_verifier ON community_verifications(verifier_id);
CREATE INDEX idx_community_verifications_reproduced ON community_verifications(reproduced);
CREATE INDEX idx_community_verifications_flagged ON community_verifications(flagged)
    WHERE flagged = TRUE;
CREATE INDEX idx_community_verifications_review_status ON community_verifications(review_status)
    WHERE reviewed = TRUE;
CREATE INDEX idx_community_verifications_created ON community_verifications(created_at DESC);

-- Index for top verifications
CREATE INDEX idx_community_verifications_votes ON community_verifications(
    submission_id,
    (upvotes - downvotes) DESC
) WHERE flagged = FALSE;

COMMENT ON TABLE community_verifications IS 'Community-submitted verification attempts';
COMMENT ON COLUMN community_verifications.reproduced IS 'Whether verifier successfully reproduced result';
COMMENT ON COLUMN community_verifications.score_difference IS 'Difference from original submission';
COMMENT ON COLUMN community_verifications.upvotes IS 'Community endorsement votes';
COMMENT ON COLUMN community_verifications.flagged IS 'Marked for moderator review';

-- ============================================================================
-- VERIFICATION VOTES TABLE
-- ============================================================================

CREATE TABLE verification_votes (
    community_verification_id UUID NOT NULL REFERENCES community_verifications(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id),
    vote_type VARCHAR(10) NOT NULL,  -- 'upvote', 'downvote'
    voted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (community_verification_id, user_id),

    CONSTRAINT verification_votes_valid_type CHECK (
        vote_type IN ('upvote', 'downvote')
    )
);

CREATE INDEX idx_verification_votes_user ON verification_votes(user_id);
CREATE INDEX idx_verification_votes_voted ON verification_votes(voted_at DESC);

COMMENT ON TABLE verification_votes IS 'User votes on community verifications';
