-- ============================================================================
-- Migration: 00007_governance.sql
-- Description: Governance proposals, voting, and reviews
-- Author: LLM Benchmark Exchange
-- Created: 2025-11-26
-- ============================================================================

-- ============================================================================
-- PROPOSALS TABLE
-- ============================================================================

CREATE TABLE proposals (
    -- Primary key
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),

    -- Proposal details
    proposal_type proposal_type NOT NULL,
    title VARCHAR(200) NOT NULL,
    description TEXT NOT NULL,
    rationale TEXT NOT NULL,
    status proposal_status NOT NULL DEFAULT 'draft',

    -- Related benchmark (if applicable)
    benchmark_definition JSONB,  -- For new benchmark proposals
    benchmark_id UUID REFERENCES benchmarks(id),  -- For update/deprecate proposals

    -- Voting configuration
    voting_starts_at TIMESTAMPTZ,
    voting_ends_at TIMESTAMPTZ,
    votes_for INTEGER NOT NULL DEFAULT 0,
    votes_against INTEGER NOT NULL DEFAULT 0,
    votes_abstain INTEGER NOT NULL DEFAULT 0,
    quorum_required INTEGER NOT NULL,
    approval_threshold DOUBLE PRECISION NOT NULL DEFAULT 0.66,

    -- Discussion
    discussion_url VARCHAR(255),
    discussion_closed BOOLEAN NOT NULL DEFAULT FALSE,

    -- Implementation
    implementation_notes TEXT,
    implemented_at TIMESTAMPTZ,
    implemented_by UUID REFERENCES users(id),

    -- Author
    created_by UUID NOT NULL REFERENCES users(id),

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT proposals_valid_threshold CHECK (
        approval_threshold >= 0.5 AND approval_threshold <= 1.0
    ),
    CONSTRAINT proposals_valid_quorum CHECK (
        quorum_required > 0
    ),
    CONSTRAINT proposals_valid_voting_period CHECK (
        voting_starts_at IS NULL OR voting_ends_at IS NULL OR
        voting_starts_at < voting_ends_at
    ),
    CONSTRAINT proposals_valid_votes CHECK (
        votes_for >= 0 AND votes_against >= 0 AND votes_abstain >= 0
    ),
    CONSTRAINT proposals_benchmark_requirement CHECK (
        (proposal_type = 'new_benchmark' AND benchmark_definition IS NOT NULL) OR
        (proposal_type IN ('update_benchmark', 'deprecate_benchmark') AND benchmark_id IS NOT NULL) OR
        (proposal_type = 'policy_change')
    )
);

-- Indexes
CREATE INDEX idx_proposals_status ON proposals(status);
CREATE INDEX idx_proposals_type ON proposals(proposal_type);
CREATE INDEX idx_proposals_created_by ON proposals(created_by);
CREATE INDEX idx_proposals_benchmark ON proposals(benchmark_id) WHERE benchmark_id IS NOT NULL;
CREATE INDEX idx_proposals_created ON proposals(created_at DESC);
CREATE INDEX idx_proposals_updated ON proposals(updated_at DESC);

-- Index for active voting
CREATE INDEX idx_proposals_voting ON proposals(voting_starts_at, voting_ends_at)
    WHERE status = 'voting';

-- Index for pending implementation
CREATE INDEX idx_proposals_implementation ON proposals(status)
    WHERE status = 'approved' AND implemented_at IS NULL;

COMMENT ON TABLE proposals IS 'Governance proposals for platform changes';
COMMENT ON COLUMN proposals.benchmark_definition IS 'JSON definition of proposed benchmark';
COMMENT ON COLUMN proposals.quorum_required IS 'Minimum number of votes needed for validity';
COMMENT ON COLUMN proposals.approval_threshold IS 'Fraction of yes votes needed (0.5-1.0)';
COMMENT ON COLUMN proposals.discussion_url IS 'Link to discussion forum or GitHub issue';

-- ============================================================================
-- VOTES TABLE
-- ============================================================================

CREATE TABLE votes (
    -- Composite primary key
    proposal_id UUID NOT NULL REFERENCES proposals(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id),

    -- Vote details
    vote vote_type NOT NULL,
    voting_power INTEGER NOT NULL DEFAULT 1,  -- For weighted voting systems
    reasoning TEXT,

    -- Timestamp
    voted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (proposal_id, user_id),

    CONSTRAINT votes_valid_power CHECK (voting_power > 0)
);

CREATE INDEX idx_votes_user ON votes(user_id);
CREATE INDEX idx_votes_proposal ON votes(proposal_id);
CREATE INDEX idx_votes_type ON votes(vote);
CREATE INDEX idx_votes_voted_at ON votes(voted_at DESC);

COMMENT ON TABLE votes IS 'User votes on governance proposals';
COMMENT ON COLUMN votes.voting_power IS 'Weight of vote (default 1, may be adjusted by governance rules)';
COMMENT ON COLUMN votes.reasoning IS 'Optional explanation for vote';

-- ============================================================================
-- REVIEWS TABLE
-- ============================================================================

CREATE TABLE reviews (
    -- Primary key
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),

    -- Proposal being reviewed
    proposal_id UUID NOT NULL REFERENCES proposals(id) ON DELETE CASCADE,

    -- Reviewer
    reviewer_id UUID NOT NULL REFERENCES users(id),

    -- Review status
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    -- 'pending', 'in_progress', 'completed', 'declined'

    -- Decision
    decision VARCHAR(20),  -- 'approve', 'request_changes', 'reject'
    summary TEXT,

    -- Timestamps
    assigned_at TIMESTAMPTZ,
    submitted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT reviews_valid_status CHECK (
        status IN ('pending', 'in_progress', 'completed', 'declined')
    ),
    CONSTRAINT reviews_valid_decision CHECK (
        decision IS NULL OR decision IN ('approve', 'request_changes', 'reject')
    ),
    CONSTRAINT reviews_completed_requirements CHECK (
        (status = 'completed' AND decision IS NOT NULL AND summary IS NOT NULL AND submitted_at IS NOT NULL) OR
        (status != 'completed')
    )
);

CREATE INDEX idx_reviews_proposal ON reviews(proposal_id);
CREATE INDEX idx_reviews_reviewer ON reviews(reviewer_id);
CREATE INDEX idx_reviews_status ON reviews(status);
CREATE INDEX idx_reviews_decision ON reviews(decision) WHERE decision IS NOT NULL;
CREATE INDEX idx_reviews_submitted ON reviews(submitted_at DESC) WHERE submitted_at IS NOT NULL;

-- Index for pending reviews
CREATE INDEX idx_reviews_pending ON reviews(created_at)
    WHERE status IN ('pending', 'in_progress');

COMMENT ON TABLE reviews IS 'Expert reviews of governance proposals';
COMMENT ON COLUMN reviews.decision IS 'Final recommendation: approve, request changes, or reject';
COMMENT ON COLUMN reviews.summary IS 'Overall review summary';

-- ============================================================================
-- REVIEW COMMENTS TABLE
-- ============================================================================

CREATE TABLE review_comments (
    -- Primary key
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),

    -- Review this comment belongs to
    review_id UUID NOT NULL REFERENCES reviews(id) ON DELETE CASCADE,

    -- Comment content
    content TEXT NOT NULL,

    -- Location (for inline comments on benchmark definitions)
    file_path VARCHAR(255),
    line_start INTEGER,
    line_end INTEGER,

    -- Threading
    parent_comment_id UUID REFERENCES review_comments(id) ON DELETE CASCADE,

    -- Severity/category
    comment_type VARCHAR(20) NOT NULL DEFAULT 'comment',
    -- 'comment', 'suggestion', 'question', 'issue', 'praise'

    -- Resolution
    resolved BOOLEAN NOT NULL DEFAULT FALSE,
    resolved_by UUID REFERENCES users(id),
    resolved_at TIMESTAMPTZ,

    -- Timestamp
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT review_comments_valid_type CHECK (
        comment_type IN ('comment', 'suggestion', 'question', 'issue', 'praise')
    ),
    CONSTRAINT review_comments_valid_lines CHECK (
        (line_start IS NULL AND line_end IS NULL) OR
        (line_start IS NOT NULL AND line_end IS NOT NULL AND line_start <= line_end)
    )
);

CREATE INDEX idx_review_comments_review ON review_comments(review_id);
CREATE INDEX idx_review_comments_parent ON review_comments(parent_comment_id)
    WHERE parent_comment_id IS NOT NULL;
CREATE INDEX idx_review_comments_type ON review_comments(comment_type);
CREATE INDEX idx_review_comments_resolved ON review_comments(resolved);
CREATE INDEX idx_review_comments_created ON review_comments(created_at DESC);

-- Index for unresolved issues
CREATE INDEX idx_review_comments_unresolved ON review_comments(review_id)
    WHERE resolved = FALSE AND comment_type = 'issue';

COMMENT ON TABLE review_comments IS 'Detailed comments within reviews';
COMMENT ON COLUMN review_comments.file_path IS 'Path within benchmark definition (for inline comments)';
COMMENT ON COLUMN review_comments.line_start IS 'Starting line number for inline comment';
COMMENT ON COLUMN review_comments.comment_type IS 'Category: comment, suggestion, question, issue, or praise';

-- ============================================================================
-- PROPOSAL AMENDMENTS TABLE
-- ============================================================================

CREATE TABLE proposal_amendments (
    -- Primary key
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),

    -- Proposal being amended
    proposal_id UUID NOT NULL REFERENCES proposals(id) ON DELETE CASCADE,

    -- Amendment author
    created_by UUID NOT NULL REFERENCES users(id),

    -- Amendment content
    title VARCHAR(200) NOT NULL,
    description TEXT NOT NULL,
    changes JSONB NOT NULL,
    -- Example: {"field": "quorum_required", "old_value": 100, "new_value": 50}

    -- Status
    status VARCHAR(20) NOT NULL DEFAULT 'proposed',
    -- 'proposed', 'accepted', 'rejected', 'withdrawn'

    -- Acceptance
    accepted_by UUID REFERENCES users(id),
    accepted_at TIMESTAMPTZ,
    rejection_reason TEXT,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT proposal_amendments_valid_status CHECK (
        status IN ('proposed', 'accepted', 'rejected', 'withdrawn')
    )
);

CREATE INDEX idx_proposal_amendments_proposal ON proposal_amendments(proposal_id);
CREATE INDEX idx_proposal_amendments_created_by ON proposal_amendments(created_by);
CREATE INDEX idx_proposal_amendments_status ON proposal_amendments(status);
CREATE INDEX idx_proposal_amendments_created ON proposal_amendments(created_at DESC);

COMMENT ON TABLE proposal_amendments IS 'Amendments to proposals during review';
COMMENT ON COLUMN proposal_amendments.changes IS 'JSON description of changes made';
