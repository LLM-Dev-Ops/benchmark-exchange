//! Submission service implementation

use crate::conversions::datetime_to_timestamp;
use crate::proto::{
    submission_service_server::SubmissionService, GetSubmissionRequest, GetSubmissionResponse,
    ListSubmissionsRequest, ListSubmissionsResponse, RequestVerificationRequest,
    RequestVerificationResponse, SubmitResultsRequest, SubmitResultsResponse,
};
use tonic::{Request, Response, Status};
use tracing::{debug, info};

/// Submission service implementation
#[derive(Debug, Clone)]
pub struct SubmissionServiceImpl {
    // TODO: Add application service dependencies
}

impl SubmissionServiceImpl {
    /// Create a new submission service
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for SubmissionServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[tonic::async_trait]
impl SubmissionService for SubmissionServiceImpl {
    async fn submit_results(
        &self,
        request: Request<SubmitResultsRequest>,
    ) -> Result<Response<SubmitResultsResponse>, Status> {
        let req = request.into_inner();
        info!(
            "Submitting results for benchmark: {}",
            req.benchmark_id
        );

        // TODO: Call application service to submit results
        // Validate submission data
        // Store execution metadata
        // Calculate aggregate score

        Err(Status::unimplemented("Submit results not yet implemented"))
    }

    async fn get_submission(
        &self,
        request: Request<GetSubmissionRequest>,
    ) -> Result<Response<GetSubmissionResponse>, Status> {
        let req = request.into_inner();
        info!("Getting submission: {}", req.id);

        // TODO: Call application service to get submission
        Err(Status::not_found("Submission not found"))
    }

    async fn list_submissions(
        &self,
        request: Request<ListSubmissionsRequest>,
    ) -> Result<Response<ListSubmissionsResponse>, Status> {
        let req = request.into_inner();
        debug!("Listing submissions with filters");

        // TODO: Call application service to list submissions
        // Apply filters for benchmark_id, model_id, user_id
        // Apply verification level filter
        // Apply pagination

        Ok(Response::new(ListSubmissionsResponse {
            submissions: vec![],
            total_count: 0,
            page: req.page,
            page_size: req.page_size,
        }))
    }

    async fn request_verification(
        &self,
        request: Request<RequestVerificationRequest>,
    ) -> Result<Response<RequestVerificationResponse>, Status> {
        let req = request.into_inner();
        info!("Requesting verification for submission: {}", req.submission_id);

        // TODO: Call application service to request verification
        // Create verification task
        // Enqueue for verification worker

        Ok(Response::new(RequestVerificationResponse {
            verification_id: uuid::Uuid::now_v7().to_string(),
            status: "pending".to_string(),
        }))
    }
}
