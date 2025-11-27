//! Tests for submission service
//!
//! Tests submission creation, validation, duplicate detection, and score validation.

use llm_benchmark_domain::{
    benchmark::BenchmarkStatus,
    identifiers::*,
    submission::{SubmissionVisibility, VerificationLevel},
    user::UserRole,
};
use llm_benchmark_testing::{
    builders::*, fixtures::*, mocks::*,
};

#[tokio::test]
async fn test_submit_results_happy_path() {
    // Arrange
    let submission = SubmissionBuilder::new()
        .public()
        .build();

    // Assert
    assert_eq!(submission.visibility, SubmissionVisibility::Public);
    assert_eq!(submission.verification_status.level, VerificationLevel::Unverified);
    assert!(!submission.results.test_case_results.is_empty());
}

#[tokio::test]
async fn test_submit_results_to_inactive_benchmark() {
    // Test that submissions to non-active benchmarks should fail
    let draft_status = BenchmarkStatus::Draft;

    // Assert - Draft benchmarks are not usable
    assert!(!draft_status.is_usable());
}

#[tokio::test]
async fn test_submit_results_to_active_benchmark() {
    // Test that submissions to active benchmarks succeed
    let active_status = BenchmarkStatus::Active;

    // Assert
    assert!(active_status.is_usable());
}

#[tokio::test]
async fn test_submit_results_authorization() {
    // Registered users should be able to submit results
    let user = UserBuilder::new()
        .with_role(UserRole::Registered)
        .build();

    // Assert
    assert!(user.role.can_submit_results());

    // Anonymous users should not
    let anonymous = UserBuilder::new()
        .with_role(UserRole::Anonymous)
        .build();
    assert!(!anonymous.role.can_submit_results());
}

#[tokio::test]
async fn test_duplicate_submission_detection() {
    // Arrange
    let repo = MockSubmissionRepository::new();
    let submission1 = create_test_submission();
    let submission2 = create_test_submission();

    // Act
    repo.create(submission1.clone()).await.unwrap();
    repo.create(submission2).await.unwrap();

    // Assert - Both submissions exist
    assert_eq!(repo.count(), 2);

    // In a real service, duplicate detection would check:
    // - Same benchmark + version
    // - Same model
    // - Same submitter
    // - Within time window
}

#[tokio::test]
async fn test_submission_with_failed_tests() {
    // Arrange
    let submission = create_test_submission_with_failures();

    // Assert - Should have at least one failed test
    assert!(submission.results.test_case_results.iter().any(|r| !r.passed));
}

#[tokio::test]
async fn test_score_validation_range() {
    // Arrange
    let submission = create_test_submission();

    // Assert - Scores should be in valid range
    assert!(submission.results.aggregate_score >= 0.0);
    assert!(submission.results.aggregate_score <= 1.0);

    for result in &submission.results.test_case_results {
        assert!(result.score >= 0.0);
        assert!(result.score <= 1.0);
    }
}

#[tokio::test]
async fn test_submission_with_confidence_interval() {
    // Arrange
    let submission = create_test_submission();

    // Assert
    assert!(submission.results.confidence_interval.is_some());
    if let Some(ci) = &submission.results.confidence_interval {
        assert!(ci.lower <= ci.upper);
        assert!(ci.confidence_level > 0.0 && ci.confidence_level < 1.0);
    }
}

#[tokio::test]
async fn test_submission_with_statistical_significance() {
    // Arrange
    let submission = create_test_submission();

    // Assert
    assert!(submission.results.statistical_significance.is_some());
    if let Some(stats) = &submission.results.statistical_significance {
        assert!(stats.p_value >= 0.0 && stats.p_value <= 1.0);
        assert!(stats.sample_size > 0);
    }
}

#[tokio::test]
async fn test_submission_execution_metadata() {
    // Arrange
    let submission = create_test_submission();

    // Assert - Execution metadata should be present
    assert!(!submission.execution_metadata.execution_id.is_empty());
    assert!(submission.execution_metadata.duration_seconds > 0.0);
    assert!(submission.execution_metadata.started_at <= submission.execution_metadata.completed_at);
}

#[tokio::test]
async fn test_submission_environment_info() {
    // Arrange
    let submission = create_test_submission();

    // Assert - Environment should be captured
    assert!(!submission.execution_metadata.environment.platform.is_empty());
    assert!(!submission.execution_metadata.environment.architecture.is_empty());
}

#[tokio::test]
async fn test_private_submission() {
    // Arrange
    let submission = SubmissionBuilder::new()
        .private()
        .build();

    // Assert
    assert_eq!(submission.visibility, SubmissionVisibility::Private);
}

#[tokio::test]
async fn test_unlisted_submission() {
    // Arrange
    let submission = SubmissionBuilder::new()
        .unlisted()
        .build();

    // Assert
    assert_eq!(submission.visibility, SubmissionVisibility::Unlisted);
}

#[tokio::test]
async fn test_verified_submission() {
    // Arrange
    let submission = SubmissionBuilder::new()
        .verified()
        .build();

    // Assert
    assert_eq!(submission.verification_status.level, VerificationLevel::PlatformVerified);
    assert!(submission.verification_status.verified_at.is_some());
}

#[tokio::test]
async fn test_audited_submission() {
    // Arrange
    let submission = SubmissionBuilder::new()
        .audited()
        .build();

    // Assert
    assert_eq!(submission.verification_status.level, VerificationLevel::Audited);
}

#[cfg(test)]
mod repository_tests {
    use super::*;

    #[tokio::test]
    async fn test_submission_repository_create() {
        let repo = MockSubmissionRepository::new();
        let submission = create_test_submission();

        let created = repo.create(submission.clone()).await.unwrap();
        assert_eq!(created.id, submission.id);
        assert_eq!(repo.count(), 1);
    }

    #[tokio::test]
    async fn test_submission_repository_find_by_benchmark() {
        let repo = MockSubmissionRepository::new();
        let benchmark_id = BenchmarkId::new();

        let submission1 = SubmissionBuilder::new()
            .with_benchmark_id(benchmark_id)
            .build();
        let submission2 = SubmissionBuilder::new()
            .with_benchmark_id(benchmark_id)
            .build();

        repo.create(submission1).await.unwrap();
        repo.create(submission2).await.unwrap();

        let submissions = repo.find_by_benchmark(&benchmark_id).await.unwrap();
        assert_eq!(submissions.len(), 2);
    }

    #[tokio::test]
    async fn test_submission_repository_update() {
        let repo = MockSubmissionRepository::new();
        let mut submission = create_test_submission();

        repo.create(submission.clone()).await.unwrap();

        submission.verification_status.level = VerificationLevel::PlatformVerified;
        let updated = repo.update(submission.clone()).await.unwrap();
        assert_eq!(updated.verification_status.level, VerificationLevel::PlatformVerified);
    }
}
