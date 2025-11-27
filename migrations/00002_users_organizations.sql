-- ============================================================================
-- Migration: 00002_users_organizations.sql
-- Description: User accounts and organization management
-- Author: LLM Benchmark Exchange
-- Created: 2025-11-26
-- ============================================================================

-- ============================================================================
-- USERS TABLE
-- ============================================================================

CREATE TABLE users (
    -- Primary key using UUID v7 for time-ordered IDs
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),

    -- Authentication
    email VARCHAR(255) NOT NULL UNIQUE,
    username VARCHAR(50) NOT NULL UNIQUE,
    display_name VARCHAR(100),
    password_hash VARCHAR(255) NOT NULL,
    role user_role NOT NULL DEFAULT 'registered',

    -- Email verification
    email_verified BOOLEAN NOT NULL DEFAULT FALSE,
    email_verification_token VARCHAR(255),

    -- Password reset
    password_reset_token VARCHAR(255),
    password_reset_expires TIMESTAMPTZ,

    -- Profile information
    bio TEXT,
    affiliation VARCHAR(255),
    website VARCHAR(255),
    github_username VARCHAR(50),
    orcid VARCHAR(20),  -- Open Researcher and Contributor ID
    public_email VARCHAR(255),

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_active_at TIMESTAMPTZ,
    deleted_at TIMESTAMPTZ,  -- Soft delete

    -- Constraints
    CONSTRAINT users_email_format CHECK (
        email ~* '^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$'
    ),
    CONSTRAINT users_username_format CHECK (
        username ~* '^[a-z0-9_-]{3,50}$'
    )
);

-- Indexes for common query patterns
CREATE INDEX idx_users_email ON users(email) WHERE deleted_at IS NULL;
CREATE INDEX idx_users_username ON users(username) WHERE deleted_at IS NULL;
CREATE INDEX idx_users_role ON users(role) WHERE deleted_at IS NULL;
CREATE INDEX idx_users_created_at ON users(created_at DESC);
CREATE INDEX idx_users_last_active ON users(last_active_at DESC) WHERE deleted_at IS NULL;
CREATE INDEX idx_users_email_verification ON users(email_verification_token)
    WHERE email_verification_token IS NOT NULL;
CREATE INDEX idx_users_password_reset ON users(password_reset_token)
    WHERE password_reset_token IS NOT NULL AND password_reset_expires > NOW();

-- Comments
COMMENT ON TABLE users IS 'Platform user accounts with authentication and profile data';
COMMENT ON COLUMN users.id IS 'UUID v7 primary key with time-ordering';
COMMENT ON COLUMN users.email IS 'Unique email address for login and notifications';
COMMENT ON COLUMN users.username IS 'Unique URL-safe username (lowercase, alphanumeric, hyphens, underscores)';
COMMENT ON COLUMN users.password_hash IS 'Bcrypt hash of user password';
COMMENT ON COLUMN users.orcid IS 'Open Researcher and Contributor ID for academic attribution';
COMMENT ON COLUMN users.deleted_at IS 'Soft delete timestamp - NULL means active account';

-- ============================================================================
-- ORGANIZATIONS TABLE
-- ============================================================================

CREATE TABLE organizations (
    -- Primary key
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),

    -- Basic information
    name VARCHAR(100) NOT NULL,
    slug VARCHAR(100) NOT NULL UNIQUE,
    description TEXT,
    website VARCHAR(255),
    logo_url VARCHAR(255),
    organization_type organization_type NOT NULL,

    -- Verification
    verified BOOLEAN NOT NULL DEFAULT FALSE,
    verification_date TIMESTAMPTZ,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ,  -- Soft delete

    -- Constraints
    CONSTRAINT organizations_slug_format CHECK (
        slug ~* '^[a-z0-9]+(?:-[a-z0-9]+)*$'
    )
);

-- Indexes
CREATE INDEX idx_organizations_slug ON organizations(slug) WHERE deleted_at IS NULL;
CREATE INDEX idx_organizations_type ON organizations(organization_type) WHERE deleted_at IS NULL;
CREATE INDEX idx_organizations_verified ON organizations(verified) WHERE deleted_at IS NULL;
CREATE INDEX idx_organizations_created_at ON organizations(created_at DESC);

-- Comments
COMMENT ON TABLE organizations IS 'Organizations that can submit benchmarks and results';
COMMENT ON COLUMN organizations.slug IS 'URL-safe unique identifier (lowercase, hyphens only)';
COMMENT ON COLUMN organizations.verified IS 'Indicates official/verified organization status';
COMMENT ON COLUMN organizations.verification_date IS 'When organization was verified by platform admins';

-- ============================================================================
-- ORGANIZATION MEMBERS TABLE
-- ============================================================================

CREATE TABLE organization_members (
    -- Composite primary key
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Role and metadata
    role organization_role NOT NULL DEFAULT 'member',
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (organization_id, user_id)
);

-- Indexes
CREATE INDEX idx_org_members_user ON organization_members(user_id);
CREATE INDEX idx_org_members_role ON organization_members(role);
CREATE INDEX idx_org_members_joined ON organization_members(joined_at DESC);

-- Comments
COMMENT ON TABLE organization_members IS 'Junction table for organization membership with roles';
COMMENT ON COLUMN organization_members.role IS 'Member privileges: member, admin, or owner';
COMMENT ON COLUMN organization_members.joined_at IS 'When user joined the organization';

-- ============================================================================
-- CONSTRAINTS
-- ============================================================================

-- Ensure at least one owner per organization (handled via application logic)
-- Organizations must have at least one owner before first member can be added
