//! Governance types for community decision-making.

use crate::benchmark::BenchmarkStatus;
use crate::identifiers::{BenchmarkId, ProposalId, UserId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Governance proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: ProposalId,
    pub proposal_type: ProposalType,
    pub title: String,
    pub description: String,
    pub created_by: UserId,
    pub status: ProposalStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub benchmark_id: Option<BenchmarkId>,
    pub rationale: String,
    pub voting: VotingState,
    pub reviews: Vec<Review>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalType {
    NewBenchmark,
    UpdateBenchmark,
    DeprecateBenchmark,
    PolicyChange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    Draft,
    UnderReview,
    Voting,
    Approved,
    Rejected,
    Withdrawn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VotingState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voting_starts: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voting_ends: Option<DateTime<Utc>>,
    pub votes_for: u32,
    pub votes_against: u32,
    pub votes_abstain: u32,
    pub voters: HashSet<UserId>,
    pub quorum_required: u32,
    pub approval_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    pub reviewer_id: UserId,
    pub status: ReviewStatus,
    pub comments: Vec<ReviewComment>,
    pub submitted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    Pending,
    Approved,
    RequestChanges,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewComment {
    pub content: String,
    pub line_references: Vec<LineReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineReference {
    pub file: String,
    pub start_line: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalOutcome {
    Approved,
    Rejected,
    QuorumNotMet,
    Expired,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Vote {
    Approve,
    Reject,
    Abstain,
}
