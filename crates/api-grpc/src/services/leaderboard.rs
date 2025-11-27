//! Leaderboard service implementation

use crate::conversions::datetime_to_timestamp;
use crate::proto::{
    leaderboard_service_server::LeaderboardService, CompareModelsRequest, CompareModelsResponse,
    GetCategoryLeaderboardRequest, GetCategoryLeaderboardResponse, GetLeaderboardRequest,
    GetLeaderboardResponse,
};
use tonic::{Request, Response, Status};
use tracing::{debug, info};

/// Leaderboard service implementation
#[derive(Debug, Clone)]
pub struct LeaderboardServiceImpl {
    // TODO: Add application service dependencies
}

impl LeaderboardServiceImpl {
    /// Create a new leaderboard service
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for LeaderboardServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[tonic::async_trait]
impl LeaderboardService for LeaderboardServiceImpl {
    async fn get_leaderboard(
        &self,
        request: Request<GetLeaderboardRequest>,
    ) -> Result<Response<GetLeaderboardResponse>, Status> {
        let req = request.into_inner();
        info!("Getting leaderboard for benchmark: {}", req.benchmark_id);

        // TODO: Call application service to get leaderboard
        // Fetch submissions for benchmark
        // Filter by verification level
        // Rank by aggregate score
        // Apply pagination

        Ok(Response::new(GetLeaderboardResponse {
            benchmark_id: req.benchmark_id.clone(),
            benchmark_name: "Placeholder Benchmark".to_string(),
            version: req.version.unwrap_or_default(),
            entries: vec![],
            total_entries: 0,
            last_updated: datetime_to_timestamp(&chrono::Utc::now()),
        }))
    }

    async fn get_category_leaderboard(
        &self,
        request: Request<GetCategoryLeaderboardRequest>,
    ) -> Result<Response<GetCategoryLeaderboardResponse>, Status> {
        let req = request.into_inner();
        debug!("Getting category leaderboard");

        // TODO: Call application service to get category leaderboard
        // Fetch all benchmarks in category
        // Get top submissions for each benchmark
        // Aggregate rankings

        Ok(Response::new(GetCategoryLeaderboardResponse {
            category: req.category,
            rankings: vec![],
            last_updated: datetime_to_timestamp(&chrono::Utc::now()),
        }))
    }

    async fn compare_models(
        &self,
        request: Request<CompareModelsRequest>,
    ) -> Result<Response<CompareModelsResponse>, Status> {
        let req = request.into_inner();
        info!(
            "Comparing {} models for benchmark: {}",
            req.submission_ids.len(),
            req.benchmark_id
        );

        // TODO: Call application service to compare models
        // Fetch submissions
        // Calculate score differences
        // Run statistical significance tests if requested
        // Generate comparison matrix

        Ok(Response::new(CompareModelsResponse {
            models: vec![],
            comparisons: vec![],
            benchmark_name: "Placeholder Benchmark".to_string(),
        }))
    }
}
