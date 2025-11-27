-- ============================================================================
-- Migration: 00001_initial_setup.sql
-- Description: Database extensions and enum types
-- Author: LLM Benchmark Exchange
-- Created: 2025-11-26
-- ============================================================================

-- ============================================================================
-- EXTENSIONS
-- ============================================================================

-- Enable UUID generation
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Enable cryptographic functions
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Enable trigram similarity for text search
CREATE EXTENSION IF NOT EXISTS "pg_trgm";

-- ============================================================================
-- ENUM TYPES
-- ============================================================================

-- Benchmark lifecycle status
CREATE TYPE benchmark_status AS ENUM (
    'draft',          -- Initial creation, not yet submitted
    'under_review',   -- Submitted for community review
    'active',         -- Approved and available for submissions
    'deprecated',     -- Still accessible but discouraged
    'archived'        -- Historical, read-only
);

-- Primary benchmark categorization
CREATE TYPE benchmark_category AS ENUM (
    'performance',    -- Speed, throughput, latency
    'accuracy',       -- Correctness, precision, recall
    'reliability',    -- Consistency, robustness
    'safety',         -- Harmful content, bias, toxicity
    'cost',           -- Token usage, API costs
    'capability'      -- Task-specific abilities
);

-- Trust level for submission verification
CREATE TYPE verification_level AS ENUM (
    'unverified',           -- No verification attempted
    'community_verified',   -- Verified by community members
    'platform_verified',    -- Verified by platform infrastructure
    'audited'              -- Professionally audited
);

-- Submission visibility control
CREATE TYPE submission_visibility AS ENUM (
    'public',    -- Visible on public leaderboards
    'unlisted',  -- Accessible via direct link only
    'private'    -- Only visible to submitter and admins
);

-- User permission levels
CREATE TYPE user_role AS ENUM (
    'anonymous',     -- Unauthenticated read-only access
    'registered',    -- Authenticated user, can submit
    'contributor',   -- Can create benchmarks
    'reviewer',      -- Can review proposals
    'admin'         -- Full administrative access
);

-- Organization categorization
CREATE TYPE organization_type AS ENUM (
    'llm_provider',           -- OpenAI, Anthropic, etc.
    'research_institution',   -- Universities, labs
    'enterprise',             -- Companies using LLMs
    'open_source',           -- OSS projects
    'individual'             -- Personal account
);

-- Organization membership roles
CREATE TYPE organization_role AS ENUM (
    'member',  -- Basic member privileges
    'admin',   -- Can manage members and settings
    'owner'    -- Full control including deletion
);

-- Governance proposal lifecycle
CREATE TYPE proposal_status AS ENUM (
    'draft',         -- Being written
    'under_review',  -- Open for comments
    'voting',        -- Active voting period
    'approved',      -- Passed and will be implemented
    'rejected',      -- Failed to pass
    'withdrawn'      -- Cancelled by author
);

-- Types of governance proposals
CREATE TYPE proposal_type AS ENUM (
    'new_benchmark',      -- Propose new benchmark
    'update_benchmark',   -- Modify existing benchmark
    'deprecate_benchmark',-- Mark benchmark as deprecated
    'policy_change'       -- Platform policy modification
);

-- Voting options
CREATE TYPE vote_type AS ENUM (
    'approve',  -- Vote in favor
    'reject',   -- Vote against
    'abstain'   -- Recorded participation without preference
);

-- ============================================================================
-- COMMENTS
-- ============================================================================

COMMENT ON EXTENSION "uuid-ossp" IS 'UUID generation functions';
COMMENT ON EXTENSION "pgcrypto" IS 'Cryptographic functions for password hashing';
COMMENT ON EXTENSION "pg_trgm" IS 'Trigram similarity for fuzzy text search';

COMMENT ON TYPE benchmark_status IS 'Lifecycle states for benchmark definitions';
COMMENT ON TYPE benchmark_category IS 'Primary categorization of benchmarks';
COMMENT ON TYPE verification_level IS 'Trust levels for result verification';
COMMENT ON TYPE submission_visibility IS 'Access control for submissions';
COMMENT ON TYPE user_role IS 'Permission levels for platform users';
COMMENT ON TYPE organization_type IS 'Organization classification';
COMMENT ON TYPE organization_role IS 'Membership roles within organizations';
COMMENT ON TYPE proposal_status IS 'Governance proposal workflow states';
COMMENT ON TYPE proposal_type IS 'Categories of governance proposals';
COMMENT ON TYPE vote_type IS 'Available voting options';
