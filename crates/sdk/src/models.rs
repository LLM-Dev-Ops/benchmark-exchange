//! SDK data models
//!
//! This module provides the data structures used in API requests and responses.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Re-export domain types for convenience
pub use llm_benchmark_domain::benchmark::{BenchmarkCategory, BenchmarkStatus, LicenseType};
pub use llm_benchmark_domain::governance::{ProposalStatus, ProposalType};
pub use llm_benchmark_domain::submission::{SubmissionVisibility, VerificationLevel};
pub use llm_benchmark_domain::user::UserRole;

/// Paginated response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    /// List of items
    pub items: Vec<T>,
    /// Current page (1-indexed)
    pub page: u32,
    /// Items per page
    pub page_size: u32,
    /// Total number of items
    pub total_items: u64,
    /// Total number of pages
    pub total_pages: u32,
    /// Whether there's a next page
    pub has_next: bool,
    /// Whether there's a previous page
    pub has_previous: bool,
}

impl<T> PaginatedResponse<T> {
    /// Check if the response is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get the number of items in the current page
    pub fn len(&self) -> usize {
        self.items.len()
    }
}

/// Pagination parameters for list requests
#[derive(Debug, Clone, Default, Serialize)]
pub struct PaginationParams {
    /// Page number (1-indexed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    /// Number of items per page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<u32>,
}

impl PaginationParams {
    /// Create new pagination parameters
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the page number
    pub fn page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }

    /// Set the page size
    pub fn page_size(mut self, size: u32) -> Self {
        self.page_size = Some(size);
        self
    }
}

// ============================================================================
// Benchmark Models
// ============================================================================

/// Benchmark summary (used in list responses)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSummary {
    /// Unique identifier
    pub id: Uuid,
    /// URL-friendly identifier
    pub slug: String,
    /// Display name
    pub name: String,
    /// Short description
    pub description: String,
    /// Benchmark category
    pub category: BenchmarkCategory,
    /// Current status
    pub status: BenchmarkStatus,
    /// Number of submissions
    pub submission_count: u64,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Full benchmark details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Benchmark {
    /// Unique identifier
    pub id: Uuid,
    /// URL-friendly identifier
    pub slug: String,
    /// Display name
    pub name: String,
    /// Short description
    pub description: String,
    /// Long description (markdown)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_description: Option<String>,
    /// Benchmark category
    pub category: BenchmarkCategory,
    /// Current status
    pub status: BenchmarkStatus,
    /// Tags
    pub tags: Vec<String>,
    /// License type
    pub license: LicenseType,
    /// Documentation URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_url: Option<String>,
    /// Source code URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    /// Current version
    pub version: String,
    /// Number of submissions
    pub submission_count: u64,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Request to create a benchmark
#[derive(Debug, Clone, Serialize)]
pub struct CreateBenchmarkRequest {
    /// Display name
    pub name: String,
    /// Short description
    pub description: String,
    /// Benchmark category
    pub category: BenchmarkCategory,
    /// Tags
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    /// License type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<LicenseType>,
    /// Long description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_description: Option<String>,
    /// Documentation URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_url: Option<String>,
    /// Source URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
}

impl CreateBenchmarkRequest {
    /// Create a new benchmark request
    pub fn new(name: impl Into<String>, description: impl Into<String>, category: BenchmarkCategory) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            category,
            tags: None,
            license: None,
            long_description: None,
            documentation_url: None,
            source_url: None,
        }
    }

    /// Add tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    /// Set the license
    pub fn with_license(mut self, license: LicenseType) -> Self {
        self.license = Some(license);
        self
    }

    /// Set the long description
    pub fn with_long_description(mut self, desc: impl Into<String>) -> Self {
        self.long_description = Some(desc.into());
        self
    }
}

/// Request to update a benchmark
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateBenchmarkRequest {
    /// Display name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Short description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Tags
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    /// Long description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_description: Option<String>,
    /// Documentation URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_url: Option<String>,
}

/// Filter for listing benchmarks
#[derive(Debug, Clone, Default, Serialize)]
pub struct BenchmarkFilter {
    /// Filter by category
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<BenchmarkCategory>,
    /// Filter by status
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<BenchmarkStatus>,
    /// Search query
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    /// Filter by tag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    /// Pagination
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

impl BenchmarkFilter {
    /// Create a new filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by category
    pub fn category(mut self, category: BenchmarkCategory) -> Self {
        self.category = Some(category);
        self
    }

    /// Filter by status
    pub fn status(mut self, status: BenchmarkStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Search by query
    pub fn query(mut self, query: impl Into<String>) -> Self {
        self.query = Some(query.into());
        self
    }

    /// Set page
    pub fn page(mut self, page: u32) -> Self {
        self.pagination.page = Some(page);
        self
    }

    /// Set page size
    pub fn page_size(mut self, size: u32) -> Self {
        self.pagination.page_size = Some(size);
        self
    }
}

// ============================================================================
// Submission Models
// ============================================================================

/// Submission summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionSummary {
    /// Unique identifier
    pub id: Uuid,
    /// Benchmark ID
    pub benchmark_id: Uuid,
    /// Model name
    pub model_name: String,
    /// Model version
    pub model_version: String,
    /// Aggregate score
    pub score: f64,
    /// Verification level
    pub verification_level: VerificationLevel,
    /// Visibility
    pub visibility: SubmissionVisibility,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Full submission details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Submission {
    /// Unique identifier
    pub id: Uuid,
    /// Benchmark ID
    pub benchmark_id: Uuid,
    /// Benchmark name
    pub benchmark_name: String,
    /// Model information
    pub model: ModelInfo,
    /// Results
    pub results: SubmissionResults,
    /// Verification status
    pub verification: VerificationInfo,
    /// Visibility
    pub visibility: SubmissionVisibility,
    /// Submitter information
    pub submitter: SubmitterInfo,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model name
    pub name: String,
    /// Model version
    pub version: String,
    /// Provider (e.g., OpenAI, Anthropic)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Model family
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    /// Number of parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_count: Option<u64>,
}

/// Submission results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionResults {
    /// Aggregate score (0.0 - 1.0)
    pub aggregate_score: f64,
    /// Metric-specific scores
    pub metrics: std::collections::HashMap<String, f64>,
    /// Test case results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_case_results: Option<Vec<TestCaseResult>>,
}

/// Test case result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCaseResult {
    /// Test case ID
    pub test_case_id: String,
    /// Whether the test passed
    pub passed: bool,
    /// Score for this test case
    pub score: f64,
    /// Execution time in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_time_ms: Option<u64>,
}

/// Verification information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationInfo {
    /// Verification level
    pub level: VerificationLevel,
    /// Verification timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_at: Option<DateTime<Utc>>,
    /// Verifier user ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_by: Option<Uuid>,
}

/// Submitter information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitterInfo {
    /// User ID
    pub user_id: Uuid,
    /// Username
    pub username: String,
    /// Organization (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
}

/// Request to create a submission
#[derive(Debug, Clone, Serialize)]
pub struct CreateSubmissionRequest {
    /// Benchmark ID or slug
    pub benchmark_id: String,
    /// Model name
    pub model_name: String,
    /// Model version
    pub model_version: String,
    /// Results
    pub results: SubmissionResults,
    /// Model provider
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Visibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<SubmissionVisibility>,
    /// Notes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Filter for listing submissions
#[derive(Debug, Clone, Default, Serialize)]
pub struct SubmissionFilter {
    /// Filter by benchmark
    #[serde(skip_serializing_if = "Option::is_none")]
    pub benchmark_id: Option<String>,
    /// Filter by model name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Filter by verification level
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_level: Option<VerificationLevel>,
    /// Pagination
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

// ============================================================================
// Leaderboard Models
// ============================================================================

/// Leaderboard entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    /// Rank position
    pub rank: u32,
    /// Submission ID
    pub submission_id: Uuid,
    /// Model name
    pub model_name: String,
    /// Model version
    pub model_version: String,
    /// Provider
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Aggregate score
    pub score: f64,
    /// Metric scores
    pub metrics: std::collections::HashMap<String, f64>,
    /// Verification level
    pub verification_level: VerificationLevel,
    /// Submission timestamp
    pub submitted_at: DateTime<Utc>,
}

/// Leaderboard response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Leaderboard {
    /// Benchmark ID
    pub benchmark_id: Uuid,
    /// Benchmark name
    pub benchmark_name: String,
    /// Entries
    pub entries: Vec<LeaderboardEntry>,
    /// Last updated
    pub updated_at: DateTime<Utc>,
}

/// Model comparison result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelComparison {
    /// Benchmark ID
    pub benchmark_id: Uuid,
    /// First model entry
    pub model1: LeaderboardEntry,
    /// Second model entry
    pub model2: LeaderboardEntry,
    /// Score difference (model1 - model2)
    pub score_diff: f64,
    /// Metric differences
    pub metric_diffs: std::collections::HashMap<String, f64>,
}

// ============================================================================
// Governance Models
// ============================================================================

/// Proposal summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalSummary {
    /// Unique identifier
    pub id: Uuid,
    /// Title
    pub title: String,
    /// Proposal type
    pub proposal_type: ProposalType,
    /// Current status
    pub status: ProposalStatus,
    /// Creator user ID
    pub created_by: Uuid,
    /// Votes for
    pub votes_for: u32,
    /// Votes against
    pub votes_against: u32,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Full proposal details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    /// Unique identifier
    pub id: Uuid,
    /// Title
    pub title: String,
    /// Description
    pub description: String,
    /// Proposal type
    pub proposal_type: ProposalType,
    /// Current status
    pub status: ProposalStatus,
    /// Rationale
    pub rationale: String,
    /// Related benchmark ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub benchmark_id: Option<Uuid>,
    /// Creator user ID
    pub created_by: Uuid,
    /// Voting information
    pub voting: VotingInfo,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Voting information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VotingInfo {
    /// Voting start time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starts_at: Option<DateTime<Utc>>,
    /// Voting end time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ends_at: Option<DateTime<Utc>>,
    /// Votes for
    pub votes_for: u32,
    /// Votes against
    pub votes_against: u32,
    /// Abstain votes
    pub votes_abstain: u32,
    /// Required quorum
    pub quorum_required: u32,
    /// Approval threshold (0.0 - 1.0)
    pub approval_threshold: f64,
}

/// Request to create a proposal
#[derive(Debug, Clone, Serialize)]
pub struct CreateProposalRequest {
    /// Title
    pub title: String,
    /// Description
    pub description: String,
    /// Proposal type
    pub proposal_type: ProposalType,
    /// Rationale
    pub rationale: String,
    /// Related benchmark ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub benchmark_id: Option<String>,
}

/// Vote type
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VoteType {
    /// Vote in favor
    Approve,
    /// Vote against
    Reject,
    /// Abstain from voting
    Abstain,
}

/// Request to vote on a proposal
#[derive(Debug, Clone, Serialize)]
pub struct VoteRequest {
    /// Vote type
    pub vote: VoteType,
    /// Optional reason
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Filter for listing proposals
#[derive(Debug, Clone, Default, Serialize)]
pub struct ProposalFilter {
    /// Filter by status
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ProposalStatus>,
    /// Filter by type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposal_type: Option<ProposalType>,
    /// Pagination
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paginated_response() {
        let response: PaginatedResponse<String> = PaginatedResponse {
            items: vec!["a".to_string(), "b".to_string()],
            page: 1,
            page_size: 20,
            total_items: 2,
            total_pages: 1,
            has_next: false,
            has_previous: false,
        };

        assert_eq!(response.len(), 2);
        assert!(!response.is_empty());
    }

    #[test]
    fn test_benchmark_filter() {
        let filter = BenchmarkFilter::new()
            .category(BenchmarkCategory::Performance)
            .status(BenchmarkStatus::Active)
            .page(2)
            .page_size(50);

        assert_eq!(filter.category, Some(BenchmarkCategory::Performance));
        assert_eq!(filter.status, Some(BenchmarkStatus::Active));
        assert_eq!(filter.pagination.page, Some(2));
        assert_eq!(filter.pagination.page_size, Some(50));
    }

    #[test]
    fn test_create_benchmark_request() {
        let request = CreateBenchmarkRequest::new(
            "Test Benchmark",
            "A test benchmark",
            BenchmarkCategory::Accuracy,
        )
        .with_tags(vec!["nlp".to_string(), "test".to_string()])
        .with_license(LicenseType::MIT);

        assert_eq!(request.name, "Test Benchmark");
        assert_eq!(request.tags, Some(vec!["nlp".to_string(), "test".to_string()]));
        assert_eq!(request.license, Some(LicenseType::MIT));
    }
}
