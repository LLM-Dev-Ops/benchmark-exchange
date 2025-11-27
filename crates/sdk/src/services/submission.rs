//! Submission service
//!
//! Service for managing benchmark submissions.

use crate::client::Client;
use crate::error::SdkResult;
use crate::models::{
    CreateSubmissionRequest, PaginatedResponse, Submission, SubmissionFilter, SubmissionSummary,
    VerificationLevel,
};

/// Service for submission operations
#[derive(Clone)]
pub struct SubmissionService {
    client: Client,
}

impl SubmissionService {
    /// Create a new submission service
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// List submissions with optional filters
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use llm_benchmark_sdk::{Client, SubmissionFilter};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder().api_key("key").build()?;
    ///
    /// // List all submissions
    /// let submissions = client.submissions().list().await?;
    ///
    /// // List with filter
    /// let filter = SubmissionFilter {
    ///     benchmark_id: Some("mmlu".to_string()),
    ///     ..Default::default()
    /// };
    /// let submissions = client.submissions().list_with_filter(filter).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(&self) -> SdkResult<PaginatedResponse<SubmissionSummary>> {
        self.client.get("/api/v1/submissions").await
    }

    /// List submissions with filters
    pub async fn list_with_filter(
        &self,
        filter: SubmissionFilter,
    ) -> SdkResult<PaginatedResponse<SubmissionSummary>> {
        self.client
            .get_with_query("/api/v1/submissions", &filter)
            .await
    }

    /// Get a submission by ID
    pub async fn get(&self, id: &str) -> SdkResult<Submission> {
        self.client
            .get(&format!("/api/v1/submissions/{}", id))
            .await
    }

    /// Create a new submission
    ///
    /// Requires authentication.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use llm_benchmark_sdk::{Client, CreateSubmissionRequest, SubmissionResults};
    /// use std::collections::HashMap;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder().api_key("key").build()?;
    ///
    /// let results = SubmissionResults {
    ///     aggregate_score: 0.95,
    ///     metrics: HashMap::from([("accuracy".to_string(), 0.95)]),
    ///     test_case_results: None,
    /// };
    ///
    /// let request = CreateSubmissionRequest {
    ///     benchmark_id: "mmlu".to_string(),
    ///     model_name: "gpt-4".to_string(),
    ///     model_version: "0613".to_string(),
    ///     results,
    ///     provider: Some("OpenAI".to_string()),
    ///     visibility: None,
    ///     notes: None,
    /// };
    ///
    /// let submission = client.submissions().create(request).await?;
    /// println!("Created submission: {}", submission.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(&self, request: CreateSubmissionRequest) -> SdkResult<Submission> {
        self.client.post("/api/v1/submissions", &request).await
    }

    /// Request verification for a submission
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use llm_benchmark_sdk::{Client, VerificationLevel};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder().api_key("key").build()?;
    ///
    /// client.submissions()
    ///     .request_verification("submission-id", VerificationLevel::PlatformVerified)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn request_verification(
        &self,
        id: &str,
        level: VerificationLevel,
    ) -> SdkResult<Submission> {
        self.client
            .post(
                &format!("/api/v1/submissions/{}/request-verification", id),
                &serde_json::json!({ "level": level }),
            )
            .await
    }

    /// Update submission visibility
    pub async fn update_visibility(
        &self,
        id: &str,
        visibility: crate::models::SubmissionVisibility,
    ) -> SdkResult<Submission> {
        self.client
            .patch(
                &format!("/api/v1/submissions/{}", id),
                &serde_json::json!({ "visibility": visibility }),
            )
            .await
    }

    /// Delete a submission
    ///
    /// Only the submitter can delete their own submissions.
    pub async fn delete(&self, id: &str) -> SdkResult<()> {
        self.client
            .delete(&format!("/api/v1/submissions/{}", id))
            .await
    }

    /// List submissions for a specific benchmark
    pub async fn list_for_benchmark(
        &self,
        benchmark_id: &str,
    ) -> SdkResult<PaginatedResponse<SubmissionSummary>> {
        self.list_with_filter(SubmissionFilter {
            benchmark_id: Some(benchmark_id.to_string()),
            ..Default::default()
        })
        .await
    }

    /// List submissions for a specific model
    pub async fn list_for_model(
        &self,
        model_name: &str,
    ) -> SdkResult<PaginatedResponse<SubmissionSummary>> {
        self.list_with_filter(SubmissionFilter {
            model: Some(model_name.to_string()),
            ..Default::default()
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_submission_service_creation() {
        let client = Client::builder()
            .base_url("https://api.test.com")
            .api_key("test-key")
            .build()
            .unwrap();

        let _service = client.submissions();
        assert!(true);
    }
}
