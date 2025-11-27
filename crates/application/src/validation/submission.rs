//! Submission validation rules

use super::{Validatable, ValidationResult, ValidationRules};
use llm_benchmark_domain::submission::{
    SubmissionResults, SubmissionVisibility, TestCaseResult, VerificationLevel,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Create submission request validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSubmissionRequest {
    pub benchmark_id: String,
    pub benchmark_version_id: String,
    pub model_provider: String,
    pub model_name: String,
    pub model_version: Option<String>,
    pub results: SubmissionResultsInput,
    pub visibility: SubmissionVisibility,
}

impl CreateSubmissionRequest {
    pub const MAX_PROVIDER_LENGTH: usize = 100;
    pub const MAX_MODEL_NAME_LENGTH: usize = 200;
}

impl Validatable for CreateSubmissionRequest {
    fn validate_all(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        // Benchmark ID validation
        let benchmark_id_result = ValidationRules::validate_uuid(&self.benchmark_id, "benchmark_id");
        result.merge(benchmark_id_result);

        // Benchmark version ID validation
        let version_id_result = ValidationRules::validate_uuid(&self.benchmark_version_id, "benchmark_version_id");
        result.merge(version_id_result);

        // Model provider validation
        let provider_result = ValidationRules::validate_length(
            &self.model_provider,
            "model_provider",
            Some(1),
            Some(Self::MAX_PROVIDER_LENGTH),
        );
        result.merge(provider_result);

        // Model name validation
        let name_result = ValidationRules::validate_length(
            &self.model_name,
            "model_name",
            Some(1),
            Some(Self::MAX_MODEL_NAME_LENGTH),
        );
        result.merge(name_result);

        // Model version validation if provided
        if let Some(ref version) = self.model_version {
            let version_result = ValidationRules::validate_length(
                version,
                "model_version",
                Some(1),
                Some(100),
            );
            result.merge(version_result);
        }

        // Results validation
        let results_result = self.results.validate_all();
        result.merge(results_result);

        result
    }
}

/// Submission results input validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionResultsInput {
    pub aggregate_score: f64,
    pub metric_scores: HashMap<String, f64>,
    pub test_case_results: Vec<TestCaseResultInput>,
}

impl Validatable for SubmissionResultsInput {
    fn validate_all(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        // Aggregate score validation
        let score_result = ValidationRules::validate_score(self.aggregate_score, 0.0, 1.0);
        result.merge(score_result);

        // Validate metric scores
        for (metric_name, score) in &self.metric_scores {
            if metric_name.is_empty() {
                result.add_field_error("metric_scores", "Metric name cannot be empty");
            }
            if metric_name.len() > 100 {
                result.add_field_error("metric_scores", "Metric name must be 100 characters or less");
            }

            let metric_score_result = ValidationRules::validate_score(*score, 0.0, f64::MAX);
            if !metric_score_result.valid {
                result.add_field_error(
                    format!("metric_scores.{}", metric_name),
                    "Invalid score value",
                );
            }
        }

        // Validate test case results
        if self.test_case_results.is_empty() {
            result.add_field_error("test_case_results", "At least one test case result is required");
        }

        if self.test_case_results.len() > 10000 {
            result.add_field_error(
                "test_case_results",
                "Maximum 10000 test case results allowed",
            );
        }

        for (i, tc) in self.test_case_results.iter().enumerate() {
            let tc_result = tc.validate_all();
            for (field, errors) in tc_result.field_errors {
                for error in errors {
                    result.add_field_error(format!("test_case_results[{}].{}", i, field), error);
                }
            }
        }

        // Verify aggregate score matches test case scores (with tolerance)
        if !self.test_case_results.is_empty() {
            let calculated_avg: f64 = self.test_case_results.iter().map(|tc| tc.score).sum::<f64>()
                / self.test_case_results.len() as f64;

            let tolerance = 0.01;
            if (self.aggregate_score - calculated_avg).abs() > tolerance {
                result.add_object_error(format!(
                    "Aggregate score ({:.4}) does not match calculated average ({:.4}) within tolerance",
                    self.aggregate_score, calculated_avg
                ));
            }
        }

        result
    }
}

/// Test case result input validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCaseResultInput {
    pub test_case_id: String,
    pub passed: bool,
    pub score: f64,
    pub latency_ms: Option<u64>,
    pub tokens_generated: Option<u32>,
}

impl Validatable for TestCaseResultInput {
    fn validate_all(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        // Test case ID validation
        if self.test_case_id.is_empty() {
            result.add_field_error("test_case_id", "Test case ID cannot be empty");
        }
        if self.test_case_id.len() > 200 {
            result.add_field_error("test_case_id", "Test case ID must be 200 characters or less");
        }

        // Score validation
        let score_result = ValidationRules::validate_score(self.score, 0.0, 1.0);
        result.merge(score_result);

        // Latency validation
        if let Some(latency) = self.latency_ms {
            if latency > 600_000 {
                // 10 minutes max
                result.add_field_error("latency_ms", "Latency cannot exceed 600000ms (10 minutes)");
            }
        }

        // Tokens validation
        if let Some(tokens) = self.tokens_generated {
            if tokens > 100_000 {
                result.add_field_error("tokens_generated", "Tokens generated cannot exceed 100000");
            }
        }

        // Score consistency with passed flag
        if self.passed && self.score < 0.5 {
            result.add_object_error("Test case marked as passed but score is below 0.5");
        }
        if !self.passed && self.score >= 1.0 {
            result.add_object_error("Test case marked as failed but score is 1.0");
        }

        result
    }
}

/// Update submission request validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSubmissionRequest {
    pub visibility: Option<SubmissionVisibility>,
    pub notes: Option<String>,
}

impl Validatable for UpdateSubmissionRequest {
    fn validate_all(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        // Notes validation if provided
        if let Some(ref notes) = self.notes {
            let notes_result = ValidationRules::validate_length(
                notes,
                "notes",
                None,
                Some(5000),
            );
            result.merge(notes_result);
        }

        result
    }
}

/// Verification request validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationRequest {
    pub submission_id: String,
    pub verification_level: VerificationLevel,
    pub reproduced_score: Option<f64>,
    pub score_variance: Option<f64>,
    pub environment_match: Option<bool>,
    pub notes: Option<String>,
}

impl Validatable for VerificationRequest {
    fn validate_all(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        // Submission ID validation
        let id_result = ValidationRules::validate_uuid(&self.submission_id, "submission_id");
        result.merge(id_result);

        // For platform verification and above, require detailed results
        match self.verification_level {
            VerificationLevel::PlatformVerified | VerificationLevel::Audited => {
                if self.reproduced_score.is_none() {
                    result.add_field_error(
                        "reproduced_score",
                        "Reproduced score is required for platform verification",
                    );
                }
                if self.score_variance.is_none() {
                    result.add_field_error(
                        "score_variance",
                        "Score variance is required for platform verification",
                    );
                }
                if self.environment_match.is_none() {
                    result.add_field_error(
                        "environment_match",
                        "Environment match flag is required for platform verification",
                    );
                }
            }
            _ => {}
        }

        // Reproduced score validation
        if let Some(score) = self.reproduced_score {
            let score_result = ValidationRules::validate_score(score, 0.0, 1.0);
            result.merge(score_result);
        }

        // Score variance validation
        if let Some(variance) = self.score_variance {
            if variance < 0.0 {
                result.add_field_error("score_variance", "Score variance cannot be negative");
            }
            if variance > 1.0 {
                result.add_field_error("score_variance", "Score variance cannot exceed 1.0");
            }
        }

        // Notes validation
        if let Some(ref notes) = self.notes {
            let notes_result = ValidationRules::validate_length(
                notes,
                "notes",
                None,
                Some(5000),
            );
            result.merge(notes_result);
        }

        result
    }
}

/// Submission query filters validation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubmissionQueryFilters {
    pub benchmark_id: Option<String>,
    pub model_provider: Option<String>,
    pub model_name: Option<String>,
    pub submitter_id: Option<String>,
    pub verification_level: Option<VerificationLevel>,
    pub visibility: Option<SubmissionVisibility>,
    pub min_score: Option<f64>,
    pub max_score: Option<f64>,
}

impl Validatable for SubmissionQueryFilters {
    fn validate_all(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        // Benchmark ID validation
        if let Some(ref id) = self.benchmark_id {
            let id_result = ValidationRules::validate_uuid(id, "benchmark_id");
            result.merge(id_result);
        }

        // Submitter ID validation
        if let Some(ref id) = self.submitter_id {
            let id_result = ValidationRules::validate_uuid(id, "submitter_id");
            result.merge(id_result);
        }

        // Score range validation
        if let Some(min) = self.min_score {
            let score_result = ValidationRules::validate_score(min, 0.0, 1.0);
            if !score_result.valid {
                result.add_field_error("min_score", "Invalid minimum score");
            }
        }

        if let Some(max) = self.max_score {
            let score_result = ValidationRules::validate_score(max, 0.0, 1.0);
            if !score_result.valid {
                result.add_field_error("max_score", "Invalid maximum score");
            }
        }

        // Validate min <= max
        if let (Some(min), Some(max)) = (self.min_score, self.max_score) {
            if min > max {
                result.add_object_error("Minimum score cannot be greater than maximum score");
            }
        }

        result
    }
}

/// Leaderboard query validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardQuery {
    pub benchmark_id: String,
    pub benchmark_version_id: Option<String>,
    pub limit: Option<u32>,
    pub min_verification_level: Option<VerificationLevel>,
}

impl LeaderboardQuery {
    pub const MAX_LIMIT: u32 = 100;
    pub const DEFAULT_LIMIT: u32 = 50;
}

impl Validatable for LeaderboardQuery {
    fn validate_all(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        // Benchmark ID validation
        let id_result = ValidationRules::validate_uuid(&self.benchmark_id, "benchmark_id");
        result.merge(id_result);

        // Benchmark version ID validation if provided
        if let Some(ref version_id) = self.benchmark_version_id {
            let version_result = ValidationRules::validate_uuid(version_id, "benchmark_version_id");
            result.merge(version_result);
        }

        // Limit validation
        if let Some(limit) = self.limit {
            if limit == 0 {
                result.add_field_error("limit", "Limit must be greater than 0");
            }
            if limit > Self::MAX_LIMIT {
                result.add_field_error(
                    "limit",
                    format!("Limit cannot exceed {}", Self::MAX_LIMIT),
                );
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_submission_validation() {
        let valid = CreateSubmissionRequest {
            benchmark_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            benchmark_version_id: "550e8400-e29b-41d4-a716-446655440001".to_string(),
            model_provider: "openai".to_string(),
            model_name: "gpt-4".to_string(),
            model_version: Some("2024-01-01".to_string()),
            results: SubmissionResultsInput {
                aggregate_score: 0.85,
                metric_scores: HashMap::from([("accuracy".to_string(), 0.85)]),
                test_case_results: vec![
                    TestCaseResultInput {
                        test_case_id: "tc-1".to_string(),
                        passed: true,
                        score: 0.85,
                        latency_ms: Some(100),
                        tokens_generated: Some(50),
                    },
                ],
            },
            visibility: SubmissionVisibility::Public,
        };
        assert!(valid.validate_all().valid);

        let invalid_benchmark_id = CreateSubmissionRequest {
            benchmark_id: "not-a-uuid".to_string(),
            ..valid.clone()
        };
        assert!(!invalid_benchmark_id.validate_all().valid);
    }

    #[test]
    fn test_submission_results_validation() {
        let valid = SubmissionResultsInput {
            aggregate_score: 0.8,
            metric_scores: HashMap::from([("accuracy".to_string(), 0.8)]),
            test_case_results: vec![
                TestCaseResultInput {
                    test_case_id: "tc-1".to_string(),
                    passed: true,
                    score: 0.8,
                    latency_ms: Some(100),
                    tokens_generated: Some(50),
                },
            ],
        };
        assert!(valid.validate_all().valid);

        let invalid_score = SubmissionResultsInput {
            aggregate_score: 1.5, // Invalid: > 1.0
            ..valid.clone()
        };
        assert!(!invalid_score.validate_all().valid);

        let mismatched_score = SubmissionResultsInput {
            aggregate_score: 0.5, // Doesn't match test case avg (0.8)
            ..valid.clone()
        };
        assert!(!mismatched_score.validate_all().valid);
    }

    #[test]
    fn test_test_case_result_validation() {
        let valid = TestCaseResultInput {
            test_case_id: "tc-1".to_string(),
            passed: true,
            score: 0.85,
            latency_ms: Some(100),
            tokens_generated: Some(50),
        };
        assert!(valid.validate_all().valid);

        let passed_but_low_score = TestCaseResultInput {
            test_case_id: "tc-1".to_string(),
            passed: true,
            score: 0.2, // Inconsistent with passed=true
            latency_ms: None,
            tokens_generated: None,
        };
        assert!(!passed_but_low_score.validate_all().valid);
    }

    #[test]
    fn test_verification_request_validation() {
        let community = VerificationRequest {
            submission_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            verification_level: VerificationLevel::CommunityVerified,
            reproduced_score: None,
            score_variance: None,
            environment_match: None,
            notes: Some("Looks good".to_string()),
        };
        assert!(community.validate_all().valid);

        let platform_missing_fields = VerificationRequest {
            submission_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            verification_level: VerificationLevel::PlatformVerified,
            reproduced_score: None, // Required for platform verification
            score_variance: None,
            environment_match: None,
            notes: None,
        };
        assert!(!platform_missing_fields.validate_all().valid);

        let platform_complete = VerificationRequest {
            submission_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            verification_level: VerificationLevel::PlatformVerified,
            reproduced_score: Some(0.85),
            score_variance: Some(0.02),
            environment_match: Some(true),
            notes: Some("Verified successfully".to_string()),
        };
        assert!(platform_complete.validate_all().valid);
    }

    #[test]
    fn test_leaderboard_query_validation() {
        let valid = LeaderboardQuery {
            benchmark_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            benchmark_version_id: None,
            limit: Some(50),
            min_verification_level: None,
        };
        assert!(valid.validate_all().valid);

        let invalid_limit = LeaderboardQuery {
            benchmark_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            benchmark_version_id: None,
            limit: Some(200), // Exceeds max
            min_verification_level: None,
        };
        assert!(!invalid_limit.validate_all().valid);
    }
}
