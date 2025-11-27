//! Governance endpoints.

use crate::{
    error::{ApiError, ApiResult},
    extractors::{AuthenticatedUser, Pagination, ValidatedJson},
    responses::{ApiResponse, Created, NoContent, PaginatedResponse},
    state::AppState,
};
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use llm_benchmark_domain::{
    governance::{ProposalStatus, ProposalType, Vote},
    identifiers::ProposalId,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Proposal list item
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ProposalListItem {
    pub id: ProposalId,
    pub title: String,
    pub proposal_type: ProposalType,
    pub status: ProposalStatus,
    pub proposer: String,
    pub created_at: String,
    pub voting_ends_at: Option<String>,
    pub votes_for: u32,
    pub votes_against: u32,
}

/// Proposal detail response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ProposalDetail {
    pub id: ProposalId,
    pub title: String,
    pub description: String,
    pub proposal_type: ProposalType,
    pub status: ProposalStatus,
    pub proposer: String,
    pub created_at: String,
    pub voting_starts_at: String,
    pub voting_ends_at: String,
    pub votes_for: u32,
    pub votes_against: u32,
    pub votes_abstain: u32,
    pub quorum_required: u32,
    pub approval_threshold: f64,
}

/// Create proposal request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateProposalRequest {
    #[validate(length(min = 1, max = 200))]
    pub title: String,

    #[validate(length(min = 1, max = 5000))]
    pub description: String,

    pub proposal_type: ProposalType,

    pub voting_duration_days: u32,
}

/// Vote request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct VoteRequest {
    pub vote: Vote,

    #[validate(length(max = 1000))]
    pub comment: Option<String>,
}

/// Comment request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CommentRequest {
    #[validate(length(min = 1, max = 2000))]
    pub content: String,
}

/// Comment response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CommentDetail {
    pub id: String,
    pub proposal_id: ProposalId,
    pub author: String,
    pub content: String,
    pub created_at: String,
}

/// Governance routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/proposals", get(list_proposals).post(create_proposal))
        .route("/proposals/:id", get(get_proposal))
        .route("/proposals/:id/vote", post(vote_on_proposal))
        .route("/proposals/:id/comments", post(add_comment))
}

/// List proposals
///
/// Retrieve a paginated list of governance proposals.
#[utoipa::path(
    get,
    path = "/proposals",
    tag = "governance",
    params(
        ("page" = Option<u32>, Query, description = "Page number"),
        ("per_page" = Option<u32>, Query, description = "Items per page"),
        ("status" = Option<String>, Query, description = "Filter by status"),
    ),
    responses(
        (status = 200, description = "List of proposals", body = PaginatedResponse<ProposalListItem>)
    )
)]
async fn list_proposals(
    State(_state): State<AppState>,
    pagination: Pagination,
) -> ApiResult<Json<PaginatedResponse<ProposalListItem>>> {
    // In production: Query database with filters
    let items = vec![];
    let total = 0;

    let result = llm_benchmark_common::pagination::PaginatedResult::from_params(
        items,
        &pagination.params,
        total,
    );

    Ok(Json(result.into()))
}

/// Create proposal
///
/// Create a new governance proposal. Requires contributor role.
#[utoipa::path(
    post,
    path = "/proposals",
    tag = "governance",
    request_body = CreateProposalRequest,
    responses(
        (status = 201, description = "Proposal created", body = ProposalDetail),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
async fn create_proposal(
    State(_state): State<AppState>,
    user: AuthenticatedUser,
    ValidatedJson(req): ValidatedJson<CreateProposalRequest>,
) -> ApiResult<Created<ProposalDetail>> {
    if !user.can_vote() {
        return Err(ApiError::BadRequest(
            "Insufficient permissions to create proposals".to_string(),
        ));
    }

    // In production: Create proposal in database
    let now = chrono::Utc::now();
    let voting_ends = now + chrono::Duration::days(req.voting_duration_days as i64);

    let proposal = ProposalDetail {
        id: ProposalId::new(),
        title: req.title,
        description: req.description,
        proposal_type: req.proposal_type,
        status: ProposalStatus::Draft,
        proposer: user.user_id.to_string(),
        created_at: now.to_rfc3339(),
        voting_starts_at: now.to_rfc3339(),
        voting_ends_at: voting_ends.to_rfc3339(),
        votes_for: 0,
        votes_against: 0,
        votes_abstain: 0,
        quorum_required: 100,
        approval_threshold: 0.6,
    };

    Ok(Created(proposal))
}

/// Get proposal
///
/// Retrieve detailed information about a specific proposal.
#[utoipa::path(
    get,
    path = "/proposals/{id}",
    tag = "governance",
    params(
        ("id" = Uuid, Path, description = "Proposal ID"),
    ),
    responses(
        (status = 200, description = "Proposal details", body = ProposalDetail),
        (status = 404, description = "Proposal not found"),
    )
)]
async fn get_proposal(
    State(_state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<ProposalDetail>>> {
    let _proposal_id = ProposalId::from(id);

    // In production: Query database
    Err(ApiError::NotFound)
}

/// Vote on proposal
///
/// Cast a vote on a governance proposal.
#[utoipa::path(
    post,
    path = "/proposals/{id}/vote",
    tag = "governance",
    params(
        ("id" = Uuid, Path, description = "Proposal ID"),
    ),
    request_body = VoteRequest,
    responses(
        (status = 200, description = "Vote recorded"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Proposal not found"),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
async fn vote_on_proposal(
    State(_state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
    ValidatedJson(_req): ValidatedJson<VoteRequest>,
) -> ApiResult<NoContent> {
    if !user.can_vote() {
        return Err(ApiError::BadRequest(
            "Insufficient permissions to vote".to_string(),
        ));
    }

    let _proposal_id = ProposalId::from(id);

    // In production: Record vote in database
    Err(ApiError::NotFound)
}

/// Add comment
///
/// Add a comment to a proposal.
#[utoipa::path(
    post,
    path = "/proposals/{id}/comments",
    tag = "governance",
    params(
        ("id" = Uuid, Path, description = "Proposal ID"),
    ),
    request_body = CommentRequest,
    responses(
        (status = 201, description = "Comment added", body = CommentDetail),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Proposal not found"),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
async fn add_comment(
    State(_state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<CommentRequest>,
) -> ApiResult<Created<CommentDetail>> {
    let proposal_id = ProposalId::from(id);

    // In production: Create comment in database
    let comment = CommentDetail {
        id: Uuid::new_v4().to_string(),
        proposal_id,
        author: user.user_id.to_string(),
        content: req.content,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    Ok(Created(comment))
}
