//! Domain event types for event-driven architecture.

use crate::benchmark::{BenchmarkCategory, BenchmarkStatus};
use crate::governance::{ProposalOutcome, ProposalType, Vote};
use crate::identifiers::{BenchmarkId, ProposalId, SubmissionId, UserId, VerificationId};
use crate::submission::{ModelInfo, VerificationLevel};
use crate::version::SemanticVersion;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Domain event envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEvent {
    pub id: Uuid,
    pub event_type: String,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub version: u32,
    pub metadata: EventMetadata,
}

/// Event metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub causation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<UserId>,
}

/// Benchmark events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BenchmarkEvent {
    BenchmarkCreated {
        benchmark_id: BenchmarkId,
        name: String,
        category: BenchmarkCategory,
        created_by: UserId,
    },
    BenchmarkUpdated {
        benchmark_id: BenchmarkId,
        version: SemanticVersion,
        changelog: String,
        updated_by: UserId,
    },
    BenchmarkStatusChanged {
        benchmark_id: BenchmarkId,
        old_status: BenchmarkStatus,
        new_status: BenchmarkStatus,
        changed_by: UserId,
        reason: Option<String>,
    },
    BenchmarkDeprecated {
        benchmark_id: BenchmarkId,
        reason: String,
        successor_id: Option<BenchmarkId>,
        deprecated_by: UserId,
    },
}

/// Submission events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SubmissionEvent {
    ResultsSubmitted {
        submission_id: SubmissionId,
        benchmark_id: BenchmarkId,
        model_info: ModelInfo,
        aggregate_score: f64,
        submitted_by: UserId,
    },
    VerificationStarted {
        submission_id: SubmissionId,
        verification_id: VerificationId,
    },
    VerificationCompleted {
        submission_id: SubmissionId,
        verification_id: VerificationId,
        old_level: VerificationLevel,
        new_level: VerificationLevel,
        reproduced_score: f64,
    },
    VerificationFailed {
        submission_id: SubmissionId,
        verification_id: VerificationId,
        reason: String,
    },
}

/// Governance events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GovernanceEvent {
    ProposalCreated {
        proposal_id: ProposalId,
        proposal_type: ProposalType,
        title: String,
        created_by: UserId,
    },
    VoteCast {
        proposal_id: ProposalId,
        voter_id: UserId,
        vote: Vote,
    },
    ProposalFinalized {
        proposal_id: ProposalId,
        outcome: ProposalOutcome,
        votes_for: u32,
        votes_against: u32,
        votes_abstain: u32,
    },
}
