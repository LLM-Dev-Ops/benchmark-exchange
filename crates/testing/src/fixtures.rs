//! Test fixtures for generating domain entities with realistic data.
//!
//! This module provides functions to create test instances of all domain types
//! with sensible defaults and optional randomization.

use chrono::Utc;
use fake::{
    faker::{
        internet::en::{FreeEmail, Username},
        lorem::en::{Paragraph, Sentence, Word},
        name::en::Name,
    },
    Fake,
};
use llm_benchmark_domain::{
    benchmark::{BenchmarkMetadata, Citation, LicenseType},
    governance::{Proposal, ProposalStatus, ProposalType, Review, ReviewStatus, VotingState},
    identifiers::*,
    submission::{
        ConfidenceInterval, EnvironmentInfo, ExecutionMetadata, HardwareInfo, MetricScore,
        ModelInfo, StatisticalSignificance, Submission, SubmissionResults, SubmissionVisibility,
        SubmitterInfo, TestCaseError, TestCaseErrorType, TestCaseResult, VerificationLevel,
        VerificationStatus,
    },
    user::{Organization, OrganizationMembership, OrganizationRole, OrganizationType, User, UserProfile, UserRole},
    version::SemanticVersion,
};
use std::collections::{HashMap, HashSet};

/// Create a test user with default values
pub fn create_test_user() -> User {
    create_test_user_with_role(UserRole::Registered)
}

/// Create a test user with a specific role
pub fn create_test_user_with_role(role: UserRole) -> User {
    let username: String = Username().fake();
    User {
        id: UserId::new(),
        email: FreeEmail().fake(),
        username: username.clone(),
        display_name: Some(Name().fake()),
        role,
        organizations: vec![],
        created_at: Utc::now(),
        last_active_at: Some(Utc::now()),
        email_verified: true,
        profile: UserProfile {
            bio: Some(Paragraph(2..3).fake()),
            affiliation: Some(format!("{} University", Name().fake::<String>())),
            website: None,
            github_username: Some(username),
            orcid: None,
            public_email: None,
        },
    }
}

/// Create a test admin user
pub fn create_test_admin() -> User {
    create_test_user_with_role(UserRole::Admin)
}

/// Create a test reviewer user
pub fn create_test_reviewer() -> User {
    create_test_user_with_role(UserRole::Reviewer)
}

/// Create a test contributor user
pub fn create_test_contributor() -> User {
    create_test_user_with_role(UserRole::Contributor)
}

/// Create a test organization
pub fn create_test_organization() -> Organization {
    create_test_organization_of_type(OrganizationType::ResearchInstitution)
}

/// Create a test organization of a specific type
pub fn create_test_organization_of_type(org_type: OrganizationType) -> Organization {
    Organization {
        id: OrganizationId::new(),
        name: format!("{} {}", Word().fake::<String>(), match org_type {
            OrganizationType::LlmProvider => "AI",
            OrganizationType::ResearchInstitution => "Research",
            OrganizationType::Enterprise => "Corporation",
            OrganizationType::OpenSource => "Foundation",
            OrganizationType::Individual => "Labs",
        }),
        slug: Word().fake::<String>().to_lowercase(),
        description: Some(Paragraph(1..2).fake()),
        website: None,
        logo_url: None,
        organization_type: org_type,
        verified: true,
        verification_date: Some(Utc::now()),
        created_at: Utc::now(),
    }
}

/// Create a test organization membership
pub fn create_test_membership(org_id: OrganizationId, role: OrganizationRole) -> OrganizationMembership {
    OrganizationMembership {
        organization_id: org_id,
        role,
        joined_at: Utc::now(),
    }
}

/// Create a test benchmark metadata
pub fn create_test_benchmark_metadata() -> BenchmarkMetadata {
    let name: String = Sentence(3..5).fake();
    BenchmarkMetadata {
        slug: name.to_lowercase().replace(' ', "-"),
        name: name.clone(),
        description: Sentence(10..20).fake(),
        long_description: Some(Paragraph(3..5).fake()),
        tags: vec![
            Word().fake::<String>().to_lowercase(),
            Word().fake::<String>().to_lowercase(),
            Word().fake::<String>().to_lowercase(),
        ],
        license: LicenseType::MIT,
        citation: Some(create_test_citation()),
        documentation_url: None,
        source_url: None,
        maintainers: vec![UserId::new()],
    }
}

/// Create a test citation
pub fn create_test_citation() -> Citation {
    Citation {
        title: Sentence(5..10).fake(),
        authors: vec![
            Name().fake(),
            Name().fake(),
            Name().fake(),
        ],
        venue: Some("Conference on Test Fixtures".to_string()),
        year: 2024,
        doi: Some("10.1000/test.citation".to_string()),
        bibtex: None,
    }
}

/// Create a test semantic version
pub fn create_test_version() -> SemanticVersion {
    SemanticVersion::new(1, 0, 0)
}

/// Create a test semantic version with prerelease
pub fn create_test_prerelease_version() -> SemanticVersion {
    SemanticVersion::with_prerelease(1, 0, 0, "alpha.1")
}

/// Create a test model info
pub fn create_test_model_info() -> ModelInfo {
    ModelInfo {
        model_id: Some(ModelId::new()),
        provider: format!("{} AI", Word().fake::<String>()),
        model_name: format!("gpt-{}", Word().fake::<String>()),
        model_version: Some("1.0".to_string()),
        api_endpoint: Some("https://api.example.com/v1".to_string()),
        is_official: true,
    }
}

/// Create a test submitter info
pub fn create_test_submitter_info() -> SubmitterInfo {
    SubmitterInfo {
        user_id: UserId::new(),
        organization_id: Some(OrganizationId::new()),
        is_verified_provider: false,
    }
}

/// Create test submission results
pub fn create_test_submission_results() -> SubmissionResults {
    let mut metric_scores = HashMap::new();
    metric_scores.insert(
        "accuracy".to_string(),
        MetricScore {
            value: 0.92,
            unit: Some("%".to_string()),
            raw_values: Some(vec![0.91, 0.92, 0.93]),
            std_dev: Some(0.01),
        },
    );
    metric_scores.insert(
        "latency".to_string(),
        MetricScore {
            value: 150.0,
            unit: Some("ms".to_string()),
            raw_values: Some(vec![145.0, 150.0, 155.0]),
            std_dev: Some(5.0),
        },
    );

    SubmissionResults {
        aggregate_score: 0.92,
        metric_scores,
        test_case_results: vec![
            TestCaseResult {
                test_case_id: "test_case_1".to_string(),
                passed: true,
                score: 0.95,
                latency_ms: Some(150),
                tokens_generated: Some(50),
                error: None,
            },
            TestCaseResult {
                test_case_id: "test_case_2".to_string(),
                passed: true,
                score: 0.89,
                latency_ms: Some(155),
                tokens_generated: Some(48),
                error: None,
            },
        ],
        confidence_interval: Some(ConfidenceInterval {
            lower: 0.90,
            upper: 0.94,
            confidence_level: 0.95,
        }),
        statistical_significance: Some(StatisticalSignificance {
            p_value: 0.01,
            effect_size: 0.5,
            sample_size: 100,
            test_used: "t-test".to_string(),
        }),
    }
}

/// Create test execution metadata
pub fn create_test_execution_metadata() -> ExecutionMetadata {
    let started = Utc::now();
    let completed = started + chrono::Duration::minutes(30);

    let mut dataset_checksums = HashMap::new();
    dataset_checksums.insert("dataset.json".to_string(), "sha256:abc123".to_string());

    let mut package_versions = HashMap::new();
    package_versions.insert("transformers".to_string(), "4.30.0".to_string());
    package_versions.insert("torch".to_string(), "2.0.1".to_string());

    ExecutionMetadata {
        execution_id: uuid::Uuid::new_v4().to_string(),
        started_at: started,
        completed_at: completed,
        duration_seconds: 1800.0,
        environment: EnvironmentInfo {
            platform: "linux".to_string(),
            architecture: "x86_64".to_string(),
            container_image: Some("ubuntu:22.04".to_string()),
            container_digest: Some("sha256:def456".to_string()),
            python_version: Some("3.11.0".to_string()),
            package_versions,
            hardware: Some(HardwareInfo {
                cpu: "Intel Xeon".to_string(),
                cpu_cores: 16,
                memory_gb: 64,
                gpu: Some("NVIDIA A100".to_string()),
                gpu_memory_gb: Some(40),
            }),
        },
        model_parameters_used: llm_benchmark_domain::evaluation::ModelParameters {
            temperature: Some(0.7),
            max_tokens: Some(1024),
            top_p: Some(0.9),
            top_k: None,
            stop_sequences: vec![],
            random_seed: Some(42),
            additional_params: HashMap::new(),
        },
        dataset_checksums,
        random_seed: Some(42),
        executor_version: "1.0.0".to_string(),
    }
}

/// Create a test submission
pub fn create_test_submission() -> Submission {
    Submission {
        id: SubmissionId::new(),
        benchmark_id: BenchmarkId::new(),
        benchmark_version_id: BenchmarkVersionId::new(),
        model_info: create_test_model_info(),
        submitter: create_test_submitter_info(),
        results: create_test_submission_results(),
        execution_metadata: create_test_execution_metadata(),
        verification_status: VerificationStatus {
            level: VerificationLevel::Unverified,
            verified_at: None,
            verified_by: None,
            verification_details: None,
        },
        visibility: SubmissionVisibility::Public,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

/// Create a test submission with failed test cases
pub fn create_test_submission_with_failures() -> Submission {
    let mut submission = create_test_submission();
    submission.results.test_case_results.push(TestCaseResult {
        test_case_id: "test_case_3".to_string(),
        passed: false,
        score: 0.0,
        latency_ms: None,
        tokens_generated: None,
        error: Some(TestCaseError {
            error_type: TestCaseErrorType::Timeout,
            message: "Request timed out after 30s".to_string(),
        }),
    });
    submission
}

/// Create a test proposal
pub fn create_test_proposal() -> Proposal {
    Proposal {
        id: ProposalId::new(),
        proposal_type: ProposalType::NewBenchmark,
        title: Sentence(5..8).fake(),
        description: Paragraph(3..5).fake(),
        created_by: UserId::new(),
        status: ProposalStatus::Draft,
        benchmark_id: Some(BenchmarkId::new()),
        rationale: Paragraph(2..4).fake(),
        voting: VotingState {
            voting_starts: None,
            voting_ends: None,
            votes_for: 0,
            votes_against: 0,
            votes_abstain: 0,
            voters: HashSet::new(),
            quorum_required: 10,
            approval_threshold: 0.66,
        },
        reviews: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

/// Create a test proposal with voting in progress
pub fn create_test_proposal_voting() -> Proposal {
    let mut proposal = create_test_proposal();
    proposal.status = ProposalStatus::Voting;
    proposal.voting.voting_starts = Some(Utc::now() - chrono::Duration::days(1));
    proposal.voting.voting_ends = Some(Utc::now() + chrono::Duration::days(6));
    proposal.voting.votes_for = 15;
    proposal.voting.votes_against = 3;
    proposal.voting.votes_abstain = 2;
    proposal
}

/// Create a test review
pub fn create_test_review(reviewer_id: UserId, status: ReviewStatus) -> Review {
    Review {
        reviewer_id,
        status,
        comments: vec![],
        submitted_at: Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_user() {
        let user = create_test_user();
        assert_eq!(user.role, UserRole::Registered);
        assert!(user.email_verified);
        assert!(!user.email.is_empty());
        assert!(!user.username.is_empty());
    }

    #[test]
    fn test_create_admin() {
        let admin = create_test_admin();
        assert_eq!(admin.role, UserRole::Admin);
    }

    #[test]
    fn test_create_organization() {
        let org = create_test_organization();
        assert!(org.verified);
        assert!(!org.name.is_empty());
    }

    #[test]
    fn test_create_submission() {
        let submission = create_test_submission();
        assert_eq!(submission.results.test_case_results.len(), 2);
        assert!(submission.results.confidence_interval.is_some());
    }

    #[test]
    fn test_create_submission_with_failures() {
        let submission = create_test_submission_with_failures();
        assert!(submission.results.test_case_results.iter().any(|r| !r.passed));
    }

    #[test]
    fn test_create_proposal() {
        let proposal = create_test_proposal();
        assert_eq!(proposal.status, ProposalStatus::Draft);
        assert_eq!(proposal.proposal_type, ProposalType::NewBenchmark);
    }

    #[test]
    fn test_create_proposal_voting() {
        let proposal = create_test_proposal_voting();
        assert_eq!(proposal.status, ProposalStatus::Voting);
        assert!(proposal.voting.votes_for > 0);
    }
}
