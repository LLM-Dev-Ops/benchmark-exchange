//! Benchmark service implementation

use crate::conversions::{datetime_to_timestamp, timestamp_to_datetime};
use crate::proto::{
    benchmark_service_server::BenchmarkService, ApproveBenchmarkRequest, ApproveBenchmarkResponse,
    Benchmark, BenchmarkMetadata, Citation, CreateBenchmarkRequest, CreateBenchmarkResponse,
    GetBenchmarkRequest, GetBenchmarkResponse, ListBenchmarksRequest, ListBenchmarksResponse,
    RejectBenchmarkRequest, RejectBenchmarkResponse, SubmitForReviewRequest,
    SubmitForReviewResponse, UpdateBenchmarkRequest, UpdateBenchmarkResponse,
};
use tonic::{Request, Response, Status};
use tracing::{debug, error, info};

/// Benchmark service implementation
#[derive(Debug, Clone)]
pub struct BenchmarkServiceImpl {
    // TODO: Add application service dependencies
}

impl BenchmarkServiceImpl {
    /// Create a new benchmark service
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for BenchmarkServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[tonic::async_trait]
impl BenchmarkService for BenchmarkServiceImpl {
    async fn create_benchmark(
        &self,
        request: Request<CreateBenchmarkRequest>,
    ) -> Result<Response<CreateBenchmarkResponse>, Status> {
        let req = request.into_inner();
        info!("Creating benchmark: {:?}", req.metadata.as_ref().map(|m| &m.name));

        // TODO: Call application service to create benchmark
        // For now, return a placeholder
        let benchmark = Benchmark {
            id: uuid::Uuid::now_v7().to_string(),
            metadata: req.metadata,
            category: req.category,
            status: crate::proto::BenchmarkStatus::Draft as i32,
            version: req.version,
            version_id: uuid::Uuid::now_v7().to_string(),
            created_at: datetime_to_timestamp(&chrono::Utc::now()),
            updated_at: datetime_to_timestamp(&chrono::Utc::now()),
            created_by: "user-id-placeholder".to_string(),
        };

        Ok(Response::new(CreateBenchmarkResponse {
            benchmark: Some(benchmark),
        }))
    }

    async fn get_benchmark(
        &self,
        request: Request<GetBenchmarkRequest>,
    ) -> Result<Response<GetBenchmarkResponse>, Status> {
        let req = request.into_inner();
        info!("Getting benchmark: {}", req.id);

        // TODO: Call application service to get benchmark
        Err(Status::not_found("Benchmark not found"))
    }

    async fn list_benchmarks(
        &self,
        request: Request<ListBenchmarksRequest>,
    ) -> Result<Response<ListBenchmarksResponse>, Status> {
        let req = request.into_inner();
        debug!("Listing benchmarks with filters");

        // TODO: Call application service to list benchmarks
        Ok(Response::new(ListBenchmarksResponse {
            benchmarks: vec![],
            total_count: 0,
            page: req.page,
            page_size: req.page_size,
        }))
    }

    async fn update_benchmark(
        &self,
        request: Request<UpdateBenchmarkRequest>,
    ) -> Result<Response<UpdateBenchmarkResponse>, Status> {
        let req = request.into_inner();
        info!("Updating benchmark: {}", req.id);

        // TODO: Call application service to update benchmark
        Err(Status::not_found("Benchmark not found"))
    }

    async fn submit_for_review(
        &self,
        request: Request<SubmitForReviewRequest>,
    ) -> Result<Response<SubmitForReviewResponse>, Status> {
        let req = request.into_inner();
        info!("Submitting benchmark for review: {}", req.id);

        // TODO: Call application service to submit benchmark for review
        Err(Status::not_found("Benchmark not found"))
    }

    async fn approve_benchmark(
        &self,
        request: Request<ApproveBenchmarkRequest>,
    ) -> Result<Response<ApproveBenchmarkResponse>, Status> {
        let req = request.into_inner();
        info!("Approving benchmark: {}", req.id);

        // TODO: Call application service to approve benchmark
        // Verify user has reviewer role
        Err(Status::permission_denied("Insufficient permissions"))
    }

    async fn reject_benchmark(
        &self,
        request: Request<RejectBenchmarkRequest>,
    ) -> Result<Response<RejectBenchmarkResponse>, Status> {
        let req = request.into_inner();
        info!("Rejecting benchmark: {} with reason: {}", req.id, req.reason);

        // TODO: Call application service to reject benchmark
        // Verify user has reviewer role
        Err(Status::permission_denied("Insufficient permissions"))
    }
}
