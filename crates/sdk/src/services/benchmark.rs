//! Benchmark service
//!
//! Service for managing benchmarks.

use crate::client::Client;
use crate::error::SdkResult;
use crate::models::{
    Benchmark, BenchmarkFilter, BenchmarkSummary, CreateBenchmarkRequest, PaginatedResponse,
    UpdateBenchmarkRequest,
};

/// Service for benchmark operations
#[derive(Clone)]
pub struct BenchmarkService {
    client: Client,
}

impl BenchmarkService {
    /// Create a new benchmark service
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// List benchmarks with optional filters
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use llm_benchmark_sdk::{Client, BenchmarkCategory, BenchmarkFilter};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder().api_key("key").build()?;
    ///
    /// // List all benchmarks
    /// let benchmarks = client.benchmarks().list().await?;
    ///
    /// // List with filters
    /// let filter = BenchmarkFilter::new()
    ///     .category(BenchmarkCategory::Performance)
    ///     .page_size(10);
    /// let benchmarks = client.benchmarks().list_with_filter(filter).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(&self) -> SdkResult<PaginatedResponse<BenchmarkSummary>> {
        self.client.get("/api/v1/benchmarks").await
    }

    /// List benchmarks with filters
    pub async fn list_with_filter(
        &self,
        filter: BenchmarkFilter,
    ) -> SdkResult<PaginatedResponse<BenchmarkSummary>> {
        self.client.get_with_query("/api/v1/benchmarks", &filter).await
    }

    /// Get a benchmark by ID or slug
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use llm_benchmark_sdk::Client;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder().api_key("key").build()?;
    ///
    /// // Get by slug
    /// let benchmark = client.benchmarks().get("mmlu").await?;
    ///
    /// // Get by UUID
    /// let benchmark = client.benchmarks().get("550e8400-e29b-41d4-a716-446655440000").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(&self, id_or_slug: &str) -> SdkResult<Benchmark> {
        self.client
            .get(&format!("/api/v1/benchmarks/{}", id_or_slug))
            .await
    }

    /// Create a new benchmark
    ///
    /// Requires authentication.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use llm_benchmark_sdk::{Client, CreateBenchmarkRequest, BenchmarkCategory};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder().api_key("key").build()?;
    ///
    /// let request = CreateBenchmarkRequest::new(
    ///     "My Benchmark",
    ///     "A benchmark for testing X",
    ///     BenchmarkCategory::Accuracy,
    /// )
    /// .with_tags(vec!["nlp".to_string(), "testing".to_string()]);
    ///
    /// let benchmark = client.benchmarks().create(request).await?;
    /// println!("Created benchmark: {}", benchmark.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(&self, request: CreateBenchmarkRequest) -> SdkResult<Benchmark> {
        self.client.post("/api/v1/benchmarks", &request).await
    }

    /// Update an existing benchmark
    ///
    /// Requires authentication and ownership/admin privileges.
    pub async fn update(&self, id: &str, request: UpdateBenchmarkRequest) -> SdkResult<Benchmark> {
        self.client
            .patch(&format!("/api/v1/benchmarks/{}", id), &request)
            .await
    }

    /// Submit a benchmark for review
    ///
    /// Transitions a draft benchmark to the review process.
    pub async fn submit_for_review(&self, id: &str) -> SdkResult<Benchmark> {
        self.client
            .post(
                &format!("/api/v1/benchmarks/{}/submit-for-review", id),
                &serde_json::json!({}),
            )
            .await
    }

    /// Get benchmark versions
    ///
    /// Returns the version history for a benchmark.
    pub async fn get_versions(&self, id: &str) -> SdkResult<Vec<BenchmarkVersion>> {
        self.client
            .get(&format!("/api/v1/benchmarks/{}/versions", id))
            .await
    }

    /// Get benchmark statistics
    pub async fn get_stats(&self, id: &str) -> SdkResult<BenchmarkStats> {
        self.client
            .get(&format!("/api/v1/benchmarks/{}/stats", id))
            .await
    }
}

/// Benchmark version information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BenchmarkVersion {
    /// Version string
    pub version: String,
    /// Release date
    pub released_at: chrono::DateTime<chrono::Utc>,
    /// Changelog
    pub changelog: Option<String>,
}

/// Benchmark statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BenchmarkStats {
    /// Total submissions
    pub total_submissions: u64,
    /// Unique models
    pub unique_models: u64,
    /// Average score
    pub average_score: f64,
    /// Score distribution
    pub score_distribution: Vec<ScoreBucket>,
}

/// Score distribution bucket
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScoreBucket {
    /// Lower bound (inclusive)
    pub min: f64,
    /// Upper bound (exclusive)
    pub max: f64,
    /// Count of submissions in this bucket
    pub count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_benchmark_service_creation() {
        let client = Client::builder()
            .base_url("https://api.test.com")
            .api_key("test-key")
            .build()
            .unwrap();

        let service = client.benchmarks();
        // Service is created, actual API calls would require a running server
        assert!(true);
    }
}
