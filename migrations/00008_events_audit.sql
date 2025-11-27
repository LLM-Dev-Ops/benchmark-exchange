-- ============================================================================
-- Migration: 00008_events_audit.sql
-- Description: Event sourcing and audit logging with partitioning
-- Author: LLM Benchmark Exchange
-- Created: 2025-11-26
-- ============================================================================

-- ============================================================================
-- DOMAIN EVENTS TABLE (Outbox Pattern)
-- ============================================================================

CREATE TABLE domain_events (
    -- Primary key
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),

    -- Event identification
    event_type VARCHAR(100) NOT NULL,
    aggregate_type VARCHAR(50) NOT NULL,
    aggregate_id UUID NOT NULL,

    -- Event data
    payload JSONB NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,

    -- Event correlation (for distributed tracing)
    correlation_id UUID,  -- Groups related events
    causation_id UUID,    -- Points to event that caused this one

    -- Actor
    actor_id UUID REFERENCES users(id),
    actor_type VARCHAR(50),  -- 'user', 'system', 'service'

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ,

    -- Partitioning key
    partition_key VARCHAR(50) NOT NULL DEFAULT 'default',

    -- Constraints
    CONSTRAINT domain_events_valid_version CHECK (version > 0)
) PARTITION BY RANGE (created_at);

-- Create monthly partitions for 2025
CREATE TABLE domain_events_2025_01 PARTITION OF domain_events
    FOR VALUES FROM ('2025-01-01') TO ('2025-02-01');

CREATE TABLE domain_events_2025_02 PARTITION OF domain_events
    FOR VALUES FROM ('2025-02-01') TO ('2025-03-01');

CREATE TABLE domain_events_2025_03 PARTITION OF domain_events
    FOR VALUES FROM ('2025-03-01') TO ('2025-04-01');

CREATE TABLE domain_events_2025_04 PARTITION OF domain_events
    FOR VALUES FROM ('2025-04-01') TO ('2025-05-01');

CREATE TABLE domain_events_2025_05 PARTITION OF domain_events
    FOR VALUES FROM ('2025-05-01') TO ('2025-06-01');

CREATE TABLE domain_events_2025_06 PARTITION OF domain_events
    FOR VALUES FROM ('2025-06-01') TO ('2025-07-01');

CREATE TABLE domain_events_2025_07 PARTITION OF domain_events
    FOR VALUES FROM ('2025-07-01') TO ('2025-08-01');

CREATE TABLE domain_events_2025_08 PARTITION OF domain_events
    FOR VALUES FROM ('2025-08-01') TO ('2025-09-01');

CREATE TABLE domain_events_2025_09 PARTITION OF domain_events
    FOR VALUES FROM ('2025-09-01') TO ('2025-10-01');

CREATE TABLE domain_events_2025_10 PARTITION OF domain_events
    FOR VALUES FROM ('2025-10-01') TO ('2025-11-01');

CREATE TABLE domain_events_2025_11 PARTITION OF domain_events
    FOR VALUES FROM ('2025-11-01') TO ('2025-12-01');

CREATE TABLE domain_events_2025_12 PARTITION OF domain_events
    FOR VALUES FROM ('2025-12-01') TO ('2026-01-01');

-- Create partitions for 2026
CREATE TABLE domain_events_2026_01 PARTITION OF domain_events
    FOR VALUES FROM ('2026-01-01') TO ('2026-02-01');

CREATE TABLE domain_events_2026_02 PARTITION OF domain_events
    FOR VALUES FROM ('2026-02-01') TO ('2026-03-01');

CREATE TABLE domain_events_2026_03 PARTITION OF domain_events
    FOR VALUES FROM ('2026-03-01') TO ('2026-04-01');

CREATE TABLE domain_events_2026_04 PARTITION OF domain_events
    FOR VALUES FROM ('2026-04-01') TO ('2026-05-01');

CREATE TABLE domain_events_2026_05 PARTITION OF domain_events
    FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');

CREATE TABLE domain_events_2026_06 PARTITION OF domain_events
    FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');

CREATE TABLE domain_events_2026_07 PARTITION OF domain_events
    FOR VALUES FROM ('2026-07-01') TO ('2026-08-01');

CREATE TABLE domain_events_2026_08 PARTITION OF domain_events
    FOR VALUES FROM ('2026-08-01') TO ('2026-09-01');

CREATE TABLE domain_events_2026_09 PARTITION OF domain_events
    FOR VALUES FROM ('2026-09-01') TO ('2026-10-01');

CREATE TABLE domain_events_2026_10 PARTITION OF domain_events
    FOR VALUES FROM ('2026-10-01') TO ('2026-11-01');

CREATE TABLE domain_events_2026_11 PARTITION OF domain_events
    FOR VALUES FROM ('2026-11-01') TO ('2026-12-01');

CREATE TABLE domain_events_2026_12 PARTITION OF domain_events
    FOR VALUES FROM ('2026-12-01') TO ('2027-01-01');

-- Indexes on domain_events
CREATE INDEX idx_domain_events_type ON domain_events(event_type);
CREATE INDEX idx_domain_events_aggregate ON domain_events(aggregate_type, aggregate_id);
CREATE INDEX idx_domain_events_correlation ON domain_events(correlation_id)
    WHERE correlation_id IS NOT NULL;
CREATE INDEX idx_domain_events_causation ON domain_events(causation_id)
    WHERE causation_id IS NOT NULL;
CREATE INDEX idx_domain_events_actor ON domain_events(actor_id)
    WHERE actor_id IS NOT NULL;
CREATE INDEX idx_domain_events_created ON domain_events(created_at DESC);

-- Partial index for unprocessed events (outbox pattern)
CREATE INDEX idx_domain_events_unprocessed ON domain_events(created_at)
    WHERE processed_at IS NULL;

-- GIN index for JSONB payload
CREATE INDEX idx_domain_events_payload ON domain_events USING gin(payload);

COMMENT ON TABLE domain_events IS 'Event sourcing log with outbox pattern for async processing';
COMMENT ON COLUMN domain_events.event_type IS 'Type of event (e.g., BenchmarkCreated, SubmissionVerified)';
COMMENT ON COLUMN domain_events.aggregate_type IS 'Type of aggregate (e.g., Benchmark, Submission)';
COMMENT ON COLUMN domain_events.aggregate_id IS 'ID of the aggregate this event belongs to';
COMMENT ON COLUMN domain_events.correlation_id IS 'Groups related events across aggregates';
COMMENT ON COLUMN domain_events.causation_id IS 'ID of event that caused this event';
COMMENT ON COLUMN domain_events.processed_at IS 'When event was processed (NULL = pending)';
COMMENT ON COLUMN domain_events.partition_key IS 'Partitioning hint for horizontal scaling';

-- ============================================================================
-- AUDIT LOG TABLE
-- ============================================================================

CREATE TABLE audit_log (
    -- Primary key
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),

    -- Action details
    action VARCHAR(100) NOT NULL,
    resource_type VARCHAR(50) NOT NULL,
    resource_id UUID,

    -- Actor information
    actor_id UUID REFERENCES users(id),
    actor_type VARCHAR(50) NOT NULL DEFAULT 'user',  -- 'user', 'system', 'service', 'anonymous'
    actor_ip INET,
    actor_user_agent TEXT,

    -- Request context
    request_id VARCHAR(100),
    session_id VARCHAR(100),
    api_key_id UUID,

    -- Changes
    old_values JSONB,
    new_values JSONB,

    -- Additional metadata
    metadata JSONB,

    -- Timestamp
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
) PARTITION BY RANGE (created_at);

-- Create monthly partitions for 2025 audit log
CREATE TABLE audit_log_2025_01 PARTITION OF audit_log
    FOR VALUES FROM ('2025-01-01') TO ('2025-02-01');

CREATE TABLE audit_log_2025_02 PARTITION OF audit_log
    FOR VALUES FROM ('2025-02-01') TO ('2025-03-01');

CREATE TABLE audit_log_2025_03 PARTITION OF audit_log
    FOR VALUES FROM ('2025-03-01') TO ('2025-04-01');

CREATE TABLE audit_log_2025_04 PARTITION OF audit_log
    FOR VALUES FROM ('2025-04-01') TO ('2025-05-01');

CREATE TABLE audit_log_2025_05 PARTITION OF audit_log
    FOR VALUES FROM ('2025-05-01') TO ('2025-06-01');

CREATE TABLE audit_log_2025_06 PARTITION OF audit_log
    FOR VALUES FROM ('2025-06-01') TO ('2025-07-01');

CREATE TABLE audit_log_2025_07 PARTITION OF audit_log
    FOR VALUES FROM ('2025-07-01') TO ('2025-08-01');

CREATE TABLE audit_log_2025_08 PARTITION OF audit_log
    FOR VALUES FROM ('2025-08-01') TO ('2025-09-01');

CREATE TABLE audit_log_2025_09 PARTITION OF audit_log
    FOR VALUES FROM ('2025-09-01') TO ('2025-10-01');

CREATE TABLE audit_log_2025_10 PARTITION OF audit_log
    FOR VALUES FROM ('2025-10-01') TO ('2025-11-01');

CREATE TABLE audit_log_2025_11 PARTITION OF audit_log
    FOR VALUES FROM ('2025-11-01') TO ('2025-12-01');

CREATE TABLE audit_log_2025_12 PARTITION OF audit_log
    FOR VALUES FROM ('2025-12-01') TO ('2026-01-01');

-- Create partitions for 2026 audit log
CREATE TABLE audit_log_2026_01 PARTITION OF audit_log
    FOR VALUES FROM ('2026-01-01') TO ('2026-02-01');

CREATE TABLE audit_log_2026_02 PARTITION OF audit_log
    FOR VALUES FROM ('2026-02-01') TO ('2026-03-01');

CREATE TABLE audit_log_2026_03 PARTITION OF audit_log
    FOR VALUES FROM ('2026-03-01') TO ('2026-04-01');

CREATE TABLE audit_log_2026_04 PARTITION OF audit_log
    FOR VALUES FROM ('2026-04-01') TO ('2026-05-01');

CREATE TABLE audit_log_2026_05 PARTITION OF audit_log
    FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');

CREATE TABLE audit_log_2026_06 PARTITION OF audit_log
    FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');

CREATE TABLE audit_log_2026_07 PARTITION OF audit_log
    FOR VALUES FROM ('2026-07-01') TO ('2026-08-01');

CREATE TABLE audit_log_2026_08 PARTITION OF audit_log
    FOR VALUES FROM ('2026-08-01') TO ('2026-09-01');

CREATE TABLE audit_log_2026_09 PARTITION OF audit_log
    FOR VALUES FROM ('2026-09-01') TO ('2026-10-01');

CREATE TABLE audit_log_2026_10 PARTITION OF audit_log
    FOR VALUES FROM ('2026-10-01') TO ('2026-11-01');

CREATE TABLE audit_log_2026_11 PARTITION OF audit_log
    FOR VALUES FROM ('2026-11-01') TO ('2026-12-01');

CREATE TABLE audit_log_2026_12 PARTITION OF audit_log
    FOR VALUES FROM ('2026-12-01') TO ('2027-01-01');

-- Indexes on audit_log
CREATE INDEX idx_audit_log_resource ON audit_log(resource_type, resource_id);
CREATE INDEX idx_audit_log_actor ON audit_log(actor_id) WHERE actor_id IS NOT NULL;
CREATE INDEX idx_audit_log_action ON audit_log(action);
CREATE INDEX idx_audit_log_created ON audit_log(created_at DESC);
CREATE INDEX idx_audit_log_actor_ip ON audit_log(actor_ip) WHERE actor_ip IS NOT NULL;
CREATE INDEX idx_audit_log_request ON audit_log(request_id) WHERE request_id IS NOT NULL;

-- GIN indexes for JSONB
CREATE INDEX idx_audit_log_old_values ON audit_log USING gin(old_values);
CREATE INDEX idx_audit_log_new_values ON audit_log USING gin(new_values);
CREATE INDEX idx_audit_log_metadata ON audit_log USING gin(metadata);

COMMENT ON TABLE audit_log IS 'Comprehensive audit trail for compliance and security';
COMMENT ON COLUMN audit_log.action IS 'Action performed (e.g., create, update, delete, login)';
COMMENT ON COLUMN audit_log.resource_type IS 'Type of resource affected';
COMMENT ON COLUMN audit_log.actor_ip IS 'IP address of actor';
COMMENT ON COLUMN audit_log.old_values IS 'Values before change';
COMMENT ON COLUMN audit_log.new_values IS 'Values after change';
COMMENT ON COLUMN audit_log.metadata IS 'Additional context (request headers, query params, etc.)';

-- ============================================================================
-- EVENT SUBSCRIPTIONS TABLE
-- ============================================================================

CREATE TABLE event_subscriptions (
    -- Primary key
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),

    -- Subscription details
    subscriber_name VARCHAR(100) NOT NULL UNIQUE,
    event_types VARCHAR(100)[],  -- NULL means all events
    aggregate_types VARCHAR(50)[],  -- NULL means all aggregates

    -- Delivery
    delivery_endpoint VARCHAR(255),
    delivery_method VARCHAR(50) NOT NULL DEFAULT 'webhook',  -- 'webhook', 'queue', 'stream'
    delivery_config JSONB,

    -- Status
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    last_event_id UUID,
    last_delivered_at TIMESTAMPTZ,
    failure_count INTEGER NOT NULL DEFAULT 0,
    max_failures INTEGER NOT NULL DEFAULT 10,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT event_subscriptions_valid_method CHECK (
        delivery_method IN ('webhook', 'queue', 'stream', 'database')
    )
);

CREATE INDEX idx_event_subscriptions_enabled ON event_subscriptions(enabled)
    WHERE enabled = TRUE;
CREATE INDEX idx_event_subscriptions_last_delivered ON event_subscriptions(last_delivered_at);

COMMENT ON TABLE event_subscriptions IS 'Event delivery subscriptions for integrations';
COMMENT ON COLUMN event_subscriptions.event_types IS 'Filter by event types (NULL = all)';
COMMENT ON COLUMN event_subscriptions.failure_count IS 'Consecutive delivery failures';
