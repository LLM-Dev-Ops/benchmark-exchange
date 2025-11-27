//! Benchmark DTOs for API layer

use llm_benchmark_domain::benchmark::{BenchmarkCategory, BenchmarkStatus};
use serde::{Deserialize, Serialize};

/// Benchmark response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResponse {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_description: Option<String>,
    pub category: BenchmarkCategory,
    pub status: BenchmarkStatus,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_version: Option<VersionSummary>,
    pub submission_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    pub maintainers: Vec<MaintainerSummary>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Benchmark summary for list views
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSummary {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub category: BenchmarkCategory,
    pub status: BenchmarkStatus,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_version: Option<String>,
    pub submission_count: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Version summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionSummary {
    pub id: String,
    pub version: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Maintainer summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintainerSummary {
    pub id: String,
    pub username: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
}

/// Create benchmark request DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBenchmarkDto {
    pub name: String,
    pub slug: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_description: Option<String>,
    pub category: BenchmarkCategory,
    #[serde(default)]
    pub tags: Vec<String>,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evaluation_criteria: Option<EvaluationCriteriaDto>,
}

/// Update benchmark request DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBenchmarkDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
}

/// Benchmark version response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkVersionResponse {
    pub id: String,
    pub benchmark_id: String,
    pub version: String,
    pub changelog: String,
    pub breaking_changes: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migration_notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_version_id: Option<String>,
    pub submission_count: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub created_by: MaintainerSummary,
}

/// Create version request DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVersionDto {
    pub version: String,
    pub changelog: String,
    #[serde(default)]
    pub breaking_changes: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migration_notes: Option<String>,
}

/// Status transition request DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusTransitionDto {
    pub target_status: BenchmarkStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Evaluation criteria DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationCriteriaDto {
    pub primary_metric: MetricDefinitionDto,
    #[serde(default)]
    pub secondary_metrics: Vec<MetricDefinitionDto>,
    #[serde(default = "default_aggregation")]
    pub aggregation_method: String,
    #[serde(default = "default_normalization")]
    pub score_normalization: String,
    #[serde(default = "default_min_test_cases")]
    pub minimum_test_cases: usize,
    #[serde(default = "default_confidence")]
    pub confidence_level: f64,
}

fn default_aggregation() -> String {
    "mean".to_string()
}

fn default_normalization() -> String {
    "none".to_string()
}

fn default_min_test_cases() -> usize {
    10
}

fn default_confidence() -> f64 {
    0.95
}

/// Metric definition DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDefinitionDto {
    pub name: String,
    pub description: String,
    pub metric_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default = "default_true")]
    pub higher_is_better: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<MetricRangeDto>,
}

fn default_true() -> bool {
    true
}

/// Metric range DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricRangeDto {
    pub min: f64,
    pub max: f64,
}

/// Benchmark list query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkListQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<BenchmarkCategory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<BenchmarkStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maintainer_id: Option<String>,
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<String>,
}

fn default_page() -> u32 {
    1
}

fn default_page_size() -> u32 {
    20
}

/// Benchmark statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkStats {
    pub total_submissions: u64,
    pub unique_models: u64,
    pub unique_providers: u64,
    pub avg_score: f64,
    pub highest_score: f64,
    pub lowest_score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_distribution: Option<Vec<ScoreDistributionBucket>>,
    pub submissions_this_week: u64,
    pub submissions_this_month: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreDistributionBucket {
    pub min_score: f64,
    pub max_score: f64,
    pub count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_response_serialization() {
        let response = BenchmarkSummary {
            id: "uuid".to_string(),
            name: "Test Benchmark".to_string(),
            slug: "test-benchmark".to_string(),
            description: "A test benchmark".to_string(),
            category: BenchmarkCategory::Accuracy,
            status: BenchmarkStatus::Active,
            tags: vec!["test".to_string()],
            current_version: Some("1.0.0".to_string()),
            submission_count: 100,
            created_at: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test-benchmark"));
        assert!(json.contains("accuracy"));
    }

    #[test]
    fn test_create_benchmark_defaults() {
        let json = r#"{
            "name": "Test",
            "slug": "test",
            "description": "Test description",
            "category": "accuracy",
            "version": "1.0.0"
        }"#;

        let dto: CreateBenchmarkDto = serde_json::from_str(json).unwrap();
        assert!(dto.tags.is_empty());
        assert!(dto.long_description.is_none());
    }

    #[test]
    fn test_evaluation_criteria_defaults() {
        let json = r#"{
            "primary_metric": {
                "name": "accuracy",
                "description": "Accuracy metric",
                "metric_type": "accuracy"
            }
        }"#;

        let dto: EvaluationCriteriaDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.aggregation_method, "mean");
        assert_eq!(dto.confidence_level, 0.95);
        assert_eq!(dto.minimum_test_cases, 10);
    }
}
