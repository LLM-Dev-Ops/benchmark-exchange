//! Submission types for benchmark results.

use crate::evaluation::ModelParameters;
use crate::identifiers::{BenchmarkId, BenchmarkVersionId, ModelId, OrganizationId, SubmissionId, UserId, VerificationId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

/// Benchmark result submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Submission {
    pub id: SubmissionId,
    pub benchmark_id: BenchmarkId,
    pub benchmark_version_id: BenchmarkVersionId,
    pub model_info: ModelInfo,
    pub submitter: SubmitterInfo,
    pub results: SubmissionResults,
    pub execution_metadata: ExecutionMetadata,
    pub verification_status: VerificationStatus,
    pub visibility: SubmissionVisibility,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<ModelId>,
    pub provider: String,
    pub model_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_endpoint: Option<String>,
    pub is_official: bool,
}

/// Submitter information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitterInfo {
    pub user_id: UserId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<OrganizationId>,
    pub is_verified_provider: bool,
}

/// Submission results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionResults {
    pub aggregate_score: f64,
    pub metric_scores: HashMap<String, MetricScore>,
    pub test_case_results: Vec<TestCaseResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_interval: Option<ConfidenceInterval>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statistical_significance: Option<StatisticalSignificance>,
}

/// Individual metric score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricScore {
    pub value: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_values: Option<Vec<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub std_dev: Option<f64>,
}

/// Individual test case result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCaseResult {
    pub test_case_id: String,
    pub passed: bool,
    pub score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_generated: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<TestCaseError>,
}

/// Test case execution error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCaseError {
    pub error_type: TestCaseErrorType,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestCaseErrorType {
    Timeout,
    RateLimited,
    ModelError,
    InvalidOutput,
    EvaluationError,
}

/// Confidence interval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceInterval {
    pub lower: f64,
    pub upper: f64,
    pub confidence_level: f64,
}

/// Statistical significance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticalSignificance {
    pub p_value: f64,
    pub effect_size: f64,
    pub sample_size: usize,
    pub test_used: String,
}

/// Execution metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    pub execution_id: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub duration_seconds: f64,
    pub environment: EnvironmentInfo,
    pub model_parameters_used: ModelParameters,
    pub dataset_checksums: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub random_seed: Option<u64>,
    pub executor_version: String,
}

/// Environment information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    pub platform: String,
    pub architecture: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub python_version: Option<String>,
    pub package_versions: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hardware: Option<HardwareInfo>,
}

/// Hardware information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub cpu: String,
    pub cpu_cores: u32,
    pub memory_gb: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_memory_gb: Option<u32>,
}

/// Verification status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationStatus {
    pub level: VerificationLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_by: Option<VerifiedBy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_details: Option<VerificationDetails>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationLevel {
    Unverified,
    CommunityVerified,
    PlatformVerified,
    Audited,
}

impl VerificationLevel {
    pub fn rank(&self) -> u8 {
        match self {
            Self::Unverified => 0,
            Self::CommunityVerified => 1,
            Self::PlatformVerified => 2,
            Self::Audited => 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VerifiedBy {
    CommunityMember { user_id: UserId },
    Platform { verification_id: VerificationId },
    Auditor { auditor_name: String, attestation_url: Option<Url> },
}

/// Verification details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationDetails {
    pub reproduced_score: f64,
    pub score_variance: f64,
    pub environment_match: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Submission visibility
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionVisibility {
    Public,
    Unlisted,
    Private,
}
