//! Governance service
//!
//! Service for participating in community governance through proposals.

use crate::client::Client;
use crate::error::SdkResult;
use crate::models::{
    CreateProposalRequest, PaginatedResponse, Proposal, ProposalFilter, ProposalSummary,
    VoteRequest, VoteType,
};

/// Service for governance operations
#[derive(Clone)]
pub struct GovernanceService {
    client: Client,
}

impl GovernanceService {
    /// Create a new governance service
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// List proposals with optional filters
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use llm_benchmark_sdk::{Client, ProposalFilter, ProposalStatus};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder().api_key("key").build()?;
    ///
    /// // List all proposals
    /// let proposals = client.governance().list().await?;
    ///
    /// // List only voting proposals
    /// let filter = ProposalFilter {
    ///     status: Some(ProposalStatus::Voting),
    ///     ..Default::default()
    /// };
    /// let voting_proposals = client.governance().list_with_filter(filter).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(&self) -> SdkResult<PaginatedResponse<ProposalSummary>> {
        self.client.get("/api/v1/proposals").await
    }

    /// List proposals with filters
    pub async fn list_with_filter(
        &self,
        filter: ProposalFilter,
    ) -> SdkResult<PaginatedResponse<ProposalSummary>> {
        self.client
            .get_with_query("/api/v1/proposals", &filter)
            .await
    }

    /// Get a proposal by ID
    pub async fn get(&self, id: &str) -> SdkResult<Proposal> {
        self.client
            .get(&format!("/api/v1/proposals/{}", id))
            .await
    }

    /// Create a new proposal
    ///
    /// Requires authentication.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use llm_benchmark_sdk::{Client, CreateProposalRequest, ProposalType};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder().api_key("key").build()?;
    ///
    /// let request = CreateProposalRequest {
    ///     title: "Add new benchmark category".to_string(),
    ///     description: "Proposing to add a new category for code generation benchmarks".to_string(),
    ///     proposal_type: ProposalType::NewBenchmark,
    ///     rationale: "Code generation is becoming increasingly important...".to_string(),
    ///     benchmark_id: None,
    /// };
    ///
    /// let proposal = client.governance().create(request).await?;
    /// println!("Created proposal: {}", proposal.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(&self, request: CreateProposalRequest) -> SdkResult<Proposal> {
        self.client.post("/api/v1/proposals", &request).await
    }

    /// Vote on a proposal
    ///
    /// Requires authentication.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use llm_benchmark_sdk::{Client, VoteType};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder().api_key("key").build()?;
    ///
    /// // Vote to approve
    /// client.governance()
    ///     .vote("proposal-id", VoteType::Approve, Some("Great proposal!"))
    ///     .await?;
    ///
    /// // Vote to reject
    /// client.governance()
    ///     .vote("proposal-id", VoteType::Reject, Some("Needs more detail"))
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn vote(
        &self,
        proposal_id: &str,
        vote: VoteType,
        reason: Option<&str>,
    ) -> SdkResult<VoteResult> {
        let request = VoteRequest {
            vote,
            reason: reason.map(String::from),
        };
        self.client
            .post(&format!("/api/v1/proposals/{}/vote", proposal_id), &request)
            .await
    }

    /// Add a comment to a proposal
    pub async fn comment(&self, proposal_id: &str, message: &str) -> SdkResult<Comment> {
        self.client
            .post(
                &format!("/api/v1/proposals/{}/comments", proposal_id),
                &CommentRequest {
                    message: message.to_string(),
                    reply_to: None,
                },
            )
            .await
    }

    /// Reply to a comment
    pub async fn reply(
        &self,
        proposal_id: &str,
        comment_id: &str,
        message: &str,
    ) -> SdkResult<Comment> {
        self.client
            .post(
                &format!("/api/v1/proposals/{}/comments", proposal_id),
                &CommentRequest {
                    message: message.to_string(),
                    reply_to: Some(comment_id.to_string()),
                },
            )
            .await
    }

    /// Get comments for a proposal
    pub async fn get_comments(&self, proposal_id: &str) -> SdkResult<Vec<Comment>> {
        self.client
            .get(&format!("/api/v1/proposals/{}/comments", proposal_id))
            .await
    }

    /// Withdraw a proposal
    ///
    /// Only the proposal creator can withdraw.
    pub async fn withdraw(&self, proposal_id: &str, reason: Option<&str>) -> SdkResult<Proposal> {
        self.client
            .post(
                &format!("/api/v1/proposals/{}/withdraw", proposal_id),
                &WithdrawRequest {
                    reason: reason.map(String::from),
                },
            )
            .await
    }

    /// Submit a draft proposal for voting
    pub async fn submit_for_voting(&self, proposal_id: &str) -> SdkResult<Proposal> {
        self.client
            .post(
                &format!("/api/v1/proposals/{}/submit", proposal_id),
                &serde_json::json!({}),
            )
            .await
    }

    /// Get voting results for a proposal
    pub async fn get_voting_results(&self, proposal_id: &str) -> SdkResult<VotingResults> {
        self.client
            .get(&format!("/api/v1/proposals/{}/results", proposal_id))
            .await
    }
}

/// Vote result response
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VoteResult {
    /// Whether the vote was recorded
    pub success: bool,
    /// Current vote counts
    pub votes_for: u32,
    /// Votes against
    pub votes_against: u32,
    /// Abstain votes
    pub votes_abstain: u32,
}

/// Comment on a proposal
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Comment {
    /// Comment ID
    pub id: uuid::Uuid,
    /// Author user ID
    pub author_id: uuid::Uuid,
    /// Author username
    pub author_name: String,
    /// Comment message
    pub message: String,
    /// Parent comment ID (for replies)
    pub reply_to: Option<uuid::Uuid>,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Request to add a comment
#[derive(Debug, serde::Serialize)]
struct CommentRequest {
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to: Option<String>,
}

/// Request to withdraw a proposal
#[derive(Debug, serde::Serialize)]
struct WithdrawRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

/// Voting results
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VotingResults {
    /// Total votes
    pub total_votes: u32,
    /// Votes for
    pub votes_for: u32,
    /// Votes against
    pub votes_against: u32,
    /// Abstain votes
    pub votes_abstain: u32,
    /// Approval percentage
    pub approval_percentage: f64,
    /// Whether quorum was reached
    pub quorum_reached: bool,
    /// Whether the proposal passed
    pub passed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_governance_service_creation() {
        let client = Client::builder()
            .base_url("https://api.test.com")
            .api_key("test-key")
            .build()
            .unwrap();

        let _service = client.governance();
        assert!(true);
    }
}
