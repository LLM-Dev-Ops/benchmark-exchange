//! Leaderboard service
//!
//! Service for viewing benchmark leaderboards and comparing models.

use crate::client::Client;
use crate::error::SdkResult;
use crate::models::{Leaderboard, LeaderboardEntry, ModelComparison, VerificationLevel};

/// Service for leaderboard operations
#[derive(Clone)]
pub struct LeaderboardService {
    client: Client,
}

impl LeaderboardService {
    /// Create a new leaderboard service
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Get the leaderboard for a benchmark
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use llm_benchmark_sdk::Client;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder().api_key("key").build()?;
    ///
    /// let leaderboard = client.leaderboards().get("mmlu").await?;
    /// for entry in &leaderboard.entries {
    ///     println!("#{} {} - {:.2}%", entry.rank, entry.model_name, entry.score * 100.0);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(&self, benchmark_id: &str) -> SdkResult<Leaderboard> {
        self.client
            .get(&format!("/api/v1/leaderboards/{}", benchmark_id))
            .await
    }

    /// Get the leaderboard with options
    pub async fn get_with_options(
        &self,
        benchmark_id: &str,
        options: LeaderboardOptions,
    ) -> SdkResult<Leaderboard> {
        self.client
            .get_with_query(&format!("/api/v1/leaderboards/{}", benchmark_id), &options)
            .await
    }

    /// Get top N entries
    pub async fn top(&self, benchmark_id: &str, n: u32) -> SdkResult<Vec<LeaderboardEntry>> {
        let leaderboard = self
            .get_with_options(
                benchmark_id,
                LeaderboardOptions {
                    limit: Some(n),
                    ..Default::default()
                },
            )
            .await?;
        Ok(leaderboard.entries)
    }

    /// Compare two models on a benchmark
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use llm_benchmark_sdk::Client;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder().api_key("key").build()?;
    ///
    /// let comparison = client.leaderboards()
    ///     .compare("mmlu", "gpt-4", "claude-3")
    ///     .await?;
    ///
    /// println!("Score difference: {:.2}%", comparison.score_diff * 100.0);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn compare(
        &self,
        benchmark_id: &str,
        model1: &str,
        model2: &str,
    ) -> SdkResult<ModelComparison> {
        self.client
            .get_with_query(
                &format!("/api/v1/leaderboards/{}/compare", benchmark_id),
                &CompareQuery {
                    model1: model1.to_string(),
                    model2: model2.to_string(),
                },
            )
            .await
    }

    /// Get the rank of a specific submission
    pub async fn get_rank(&self, benchmark_id: &str, submission_id: &str) -> SdkResult<RankInfo> {
        self.client
            .get(&format!(
                "/api/v1/leaderboards/{}/rank/{}",
                benchmark_id, submission_id
            ))
            .await
    }

    /// Export leaderboard data
    ///
    /// Returns the leaderboard in a format suitable for export.
    pub async fn export(&self, benchmark_id: &str) -> SdkResult<LeaderboardExport> {
        self.client
            .get(&format!("/api/v1/leaderboards/{}/export", benchmark_id))
            .await
    }
}

/// Options for leaderboard queries
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct LeaderboardOptions {
    /// Maximum number of entries
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Offset for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    /// Only include verified submissions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_only: Option<bool>,
    /// Minimum verification level
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_verification: Option<VerificationLevel>,
}

impl LeaderboardOptions {
    /// Create new options
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the limit
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set the offset
    pub fn offset(mut self, offset: u32) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Only include verified submissions
    pub fn verified_only(mut self) -> Self {
        self.verified_only = Some(true);
        self
    }

    /// Set minimum verification level
    pub fn min_verification(mut self, level: VerificationLevel) -> Self {
        self.min_verification = Some(level);
        self
    }
}

/// Query for comparing models
#[derive(Debug, serde::Serialize)]
struct CompareQuery {
    model1: String,
    model2: String,
}

/// Rank information for a submission
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RankInfo {
    /// Current rank
    pub rank: u32,
    /// Total entries
    pub total_entries: u64,
    /// Percentile (0.0 - 100.0)
    pub percentile: f64,
    /// Score
    pub score: f64,
    /// Entries above
    pub entries_above: u64,
    /// Entries below
    pub entries_below: u64,
}

/// Leaderboard export data
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LeaderboardExport {
    /// Benchmark information
    pub benchmark_id: uuid::Uuid,
    /// Benchmark name
    pub benchmark_name: String,
    /// Export timestamp
    pub exported_at: chrono::DateTime<chrono::Utc>,
    /// Entries
    pub entries: Vec<LeaderboardEntry>,
    /// Metadata
    pub metadata: ExportMetadata,
}

/// Export metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExportMetadata {
    /// Total entries in leaderboard
    pub total_entries: u64,
    /// Export format version
    pub format_version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leaderboard_options() {
        let options = LeaderboardOptions::new()
            .limit(10)
            .offset(20)
            .verified_only();

        assert_eq!(options.limit, Some(10));
        assert_eq!(options.offset, Some(20));
        assert_eq!(options.verified_only, Some(true));
    }
}
