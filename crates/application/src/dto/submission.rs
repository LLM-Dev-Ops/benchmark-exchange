//! Submission DTOs for API layer

use llm_benchmark_domain::submission::{SubmissionVisibility, VerificationLevel};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Submission response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionResponse {
    pub id: String,
    pub benchmark: SubmissionBenchmarkInfo,
    pub model: ModelInfoDto,
    pub submitter: SubmitterInfoDto,
    pub results: ResultsSummaryDto,
    pub verification: VerificationInfoDto,
    pub visibility: SubmissionVisibility,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Benchmark info for submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionBenchmarkInfo {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub version_id: String,
    pub version: String,
}

/// Model information DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfoDto {
    pub provider: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_endpoint: Option<String>,
    pub is_official: bool,
}

/// Submitter information DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitterInfoDto {
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<OrganizationSummaryDto>,
    pub is_verified_provider: bool,
}

/// Organization summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationSummaryDto {
    pub id: String,
    pub name: String,
    pub slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_url: Option<String>,
}

/// Results summary DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultsSummaryDto {
    pub aggregate_score: f64,
    pub metric_scores: HashMap<String, MetricScoreDto>,
    pub test_cases_passed: u32,
    pub test_cases_total: u32,
    pub pass_rate: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_interval: Option<ConfidenceIntervalDto>,
}

/// Metric score DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricScoreDto {
    pub value: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub std_dev: Option<f64>,
}

/// Confidence interval DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceIntervalDto {
    pub lower: f64,
    pub upper: f64,
    pub confidence_level: f64,
}

/// Verification info DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationInfoDto {
    pub level: VerificationLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_by: Option<VerifierInfoDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<VerificationDetailsDto>,
}

/// Verifier info DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VerifierInfoDto {
    CommunityMember { user_id: String, username: String },
    Platform { verification_id: String },
    Auditor { name: String, attestation_url: Option<String> },
}

/// Verification details DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationDetailsDto {
    pub reproduced_score: f64,
    pub score_variance: f64,
    pub environment_match: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Create submission request DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSubmissionDto {
    pub benchmark_id: String,
    pub benchmark_version_id: String,
    pub model: CreateModelInfoDto,
    pub results: CreateResultsDto,
    #[serde(default = "default_visibility")]
    pub visibility: SubmissionVisibility,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_metadata: Option<ExecutionMetadataDto>,
}

fn default_visibility() -> SubmissionVisibility {
    SubmissionVisibility::Public
}

/// Model info for creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateModelInfoDto {
    pub provider: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_endpoint: Option<String>,
}

/// Results for creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateResultsDto {
    pub aggregate_score: f64,
    pub metric_scores: HashMap<String, f64>,
    pub test_case_results: Vec<TestCaseResultDto>,
}

/// Test case result DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCaseResultDto {
    pub test_case_id: String,
    pub passed: bool,
    pub score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_generated: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<TestCaseErrorDto>,
}

/// Test case error DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCaseErrorDto {
    pub error_type: String,
    pub message: String,
}

/// Execution metadata DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadataDto {
    pub execution_id: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: chrono::DateTime<chrono::Utc>,
    pub duration_seconds: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<EnvironmentInfoDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_parameters: Option<ModelParametersDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub random_seed: Option<u64>,
    pub executor_version: String,
}

/// Environment info DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentInfoDto {
    pub platform: String,
    pub architecture: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub python_version: Option<String>,
    #[serde(default)]
    pub package_versions: HashMap<String, String>,
}

/// Model parameters DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelParametersDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub stop_sequences: Vec<String>,
}

/// Update submission request DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSubmissionDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<SubmissionVisibility>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Verification request DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifySubmissionDto {
    pub verification_level: VerificationLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reproduced_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_variance: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment_match: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Leaderboard entry DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntryDto {
    pub rank: u32,
    pub submission_id: String,
    pub model: LeaderboardModelDto,
    pub score: f64,
    pub verification_level: VerificationLevel,
    pub submitter: LeaderboardSubmitterDto,
    pub submitted_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rank_change: Option<i32>,
}

/// Model info for leaderboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardModelDto {
    pub provider: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Submitter info for leaderboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardSubmitterDto {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
}

/// Leaderboard query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardQueryDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub benchmark_version_id: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_verification_level: Option<VerificationLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

fn default_limit() -> u32 {
    50
}

/// Submission list query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionListQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub benchmark_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submitter_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_level: Option<VerificationLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<SubmissionVisibility>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_score: Option<f64>,
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

fn default_page() -> u32 {
    1
}

fn default_page_size() -> u32 {
    20
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_submission_dto_serialization() {
        let model = LeaderboardModelDto {
            provider: "openai".to_string(),
            name: "gpt-4".to_string(),
            version: Some("2024-01-01".to_string()),
        };

        let json = serde_json::to_string(&model).unwrap();
        assert!(json.contains("gpt-4"));
        assert!(json.contains("openai"));
    }

    #[test]
    fn test_create_submission_defaults() {
        let json = r#"{
            "benchmark_id": "uuid1",
            "benchmark_version_id": "uuid2",
            "model": {
                "provider": "openai",
                "name": "gpt-4"
            },
            "results": {
                "aggregate_score": 0.85,
                "metric_scores": {"accuracy": 0.85},
                "test_case_results": []
            }
        }"#;

        let dto: CreateSubmissionDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.visibility, SubmissionVisibility::Public);
    }

    #[test]
    fn test_verification_info_variants() {
        let community = VerifierInfoDto::CommunityMember {
            user_id: "user-123".to_string(),
            username: "reviewer".to_string(),
        };

        let json = serde_json::to_string(&community).unwrap();
        assert!(json.contains("community_member"));

        let platform = VerifierInfoDto::Platform {
            verification_id: "ver-123".to_string(),
        };

        let json = serde_json::to_string(&platform).unwrap();
        assert!(json.contains("platform"));
    }
}
