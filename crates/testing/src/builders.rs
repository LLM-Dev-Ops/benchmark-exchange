//! Fluent builder pattern for constructing test data.
//!
//! This module provides builder structs for creating complex domain entities
//! with a fluent API for customization.

use chrono::Utc;
use llm_benchmark_domain::{
    benchmark::{BenchmarkCategory, BenchmarkMetadata, LicenseType},
    governance::{Proposal, ProposalStatus, ProposalType, VotingState},
    identifiers::*,
    submission::{
        Submission, SubmissionVisibility, VerificationLevel,
        VerificationStatus,
    },
    user::{Organization, OrganizationType, User, UserProfile, UserRole},
};
use std::collections::HashSet;

use crate::fixtures::{
    create_test_execution_metadata, create_test_model_info, create_test_submission_results,
    create_test_submitter_info,
};

/// Builder for creating User test instances
#[derive(Clone)]
pub struct UserBuilder {
    id: UserId,
    email: String,
    username: String,
    display_name: Option<String>,
    role: UserRole,
    email_verified: bool,
}

impl UserBuilder {
    pub fn new() -> Self {
        Self {
            id: UserId::new(),
            email: "test@example.com".to_string(),
            username: "testuser".to_string(),
            display_name: Some("Test User".to_string()),
            role: UserRole::Registered,
            email_verified: true,
        }
    }

    pub fn with_id(mut self, id: UserId) -> Self {
        self.id = id;
        self
    }

    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email = email.into();
        self
    }

    pub fn with_username(mut self, username: impl Into<String>) -> Self {
        self.username = username.into();
        self
    }

    pub fn with_display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = Some(name.into());
        self
    }

    pub fn with_role(mut self, role: UserRole) -> Self {
        self.role = role;
        self
    }

    pub fn admin(mut self) -> Self {
        self.role = UserRole::Admin;
        self
    }

    pub fn reviewer(mut self) -> Self {
        self.role = UserRole::Reviewer;
        self
    }

    pub fn contributor(mut self) -> Self {
        self.role = UserRole::Contributor;
        self
    }

    pub fn unverified(mut self) -> Self {
        self.email_verified = false;
        self
    }

    pub fn build(self) -> User {
        User {
            id: self.id,
            email: self.email,
            username: self.username,
            display_name: self.display_name,
            role: self.role,
            organizations: vec![],
            created_at: Utc::now(),
            last_active_at: Some(Utc::now()),
            email_verified: self.email_verified,
            profile: UserProfile {
                bio: None,
                affiliation: None,
                website: None,
                github_username: None,
                orcid: None,
                public_email: None,
            },
        }
    }
}

impl Default for UserBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating Organization test instances
#[derive(Clone)]
pub struct OrganizationBuilder {
    id: OrganizationId,
    name: String,
    slug: String,
    organization_type: OrganizationType,
    verified: bool,
}

impl OrganizationBuilder {
    pub fn new() -> Self {
        Self {
            id: OrganizationId::new(),
            name: "Test Organization".to_string(),
            slug: "test-org".to_string(),
            organization_type: OrganizationType::ResearchInstitution,
            verified: true,
        }
    }

    pub fn with_id(mut self, id: OrganizationId) -> Self {
        self.id = id;
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn with_slug(mut self, slug: impl Into<String>) -> Self {
        self.slug = slug.into();
        self
    }

    pub fn with_type(mut self, org_type: OrganizationType) -> Self {
        self.organization_type = org_type;
        self
    }

    pub fn llm_provider(mut self) -> Self {
        self.organization_type = OrganizationType::LlmProvider;
        self
    }

    pub fn unverified(mut self) -> Self {
        self.verified = false;
        self
    }

    pub fn build(self) -> Organization {
        Organization {
            id: self.id,
            name: self.name,
            slug: self.slug,
            description: Some("A test organization".to_string()),
            website: None,
            logo_url: None,
            organization_type: self.organization_type,
            verified: self.verified,
            verification_date: if self.verified { Some(Utc::now()) } else { None },
            created_at: Utc::now(),
        }
    }
}

impl Default for OrganizationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating BenchmarkMetadata test instances
#[derive(Clone)]
pub struct BenchmarkBuilder {
    name: String,
    slug: String,
    description: String,
    category: BenchmarkCategory,
    tags: Vec<String>,
    license: LicenseType,
    maintainers: Vec<UserId>,
}

impl BenchmarkBuilder {
    pub fn new() -> Self {
        Self {
            name: "Test Benchmark".to_string(),
            slug: "test-benchmark".to_string(),
            description: "A test benchmark".to_string(),
            category: BenchmarkCategory::Performance,
            tags: vec![],
            license: LicenseType::MIT,
            maintainers: vec![UserId::new()],
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        self.slug = name.to_lowercase().replace(' ', "-");
        self.name = name;
        self
    }

    pub fn with_slug(mut self, slug: impl Into<String>) -> Self {
        self.slug = slug.into();
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_category(mut self, category: BenchmarkCategory) -> Self {
        self.category = category;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn with_license(mut self, license: LicenseType) -> Self {
        self.license = license;
        self
    }

    pub fn with_maintainer(mut self, user_id: UserId) -> Self {
        self.maintainers.push(user_id);
        self
    }

    pub fn build(self) -> BenchmarkMetadata {
        BenchmarkMetadata {
            name: self.name,
            slug: self.slug,
            description: self.description,
            long_description: None,
            tags: self.tags,
            license: self.license,
            citation: None,
            documentation_url: None,
            source_url: None,
            maintainers: self.maintainers,
        }
    }
}

impl Default for BenchmarkBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating Submission test instances
#[derive(Clone)]
pub struct SubmissionBuilder {
    id: SubmissionId,
    benchmark_id: BenchmarkId,
    benchmark_version_id: BenchmarkVersionId,
    visibility: SubmissionVisibility,
    verification_level: VerificationLevel,
}

impl SubmissionBuilder {
    pub fn new() -> Self {
        Self {
            id: SubmissionId::new(),
            benchmark_id: BenchmarkId::new(),
            benchmark_version_id: BenchmarkVersionId::new(),
            visibility: SubmissionVisibility::Public,
            verification_level: VerificationLevel::Unverified,
        }
    }

    pub fn with_id(mut self, id: SubmissionId) -> Self {
        self.id = id;
        self
    }

    pub fn with_benchmark_id(mut self, id: BenchmarkId) -> Self {
        self.benchmark_id = id;
        self
    }

    pub fn with_benchmark_version_id(mut self, id: BenchmarkVersionId) -> Self {
        self.benchmark_version_id = id;
        self
    }

    pub fn public(mut self) -> Self {
        self.visibility = SubmissionVisibility::Public;
        self
    }

    pub fn private(mut self) -> Self {
        self.visibility = SubmissionVisibility::Private;
        self
    }

    pub fn unlisted(mut self) -> Self {
        self.visibility = SubmissionVisibility::Unlisted;
        self
    }

    pub fn verified(mut self) -> Self {
        self.verification_level = VerificationLevel::PlatformVerified;
        self
    }

    pub fn audited(mut self) -> Self {
        self.verification_level = VerificationLevel::Audited;
        self
    }

    pub fn build(self) -> Submission {
        Submission {
            id: self.id,
            benchmark_id: self.benchmark_id,
            benchmark_version_id: self.benchmark_version_id,
            model_info: create_test_model_info(),
            submitter: create_test_submitter_info(),
            results: create_test_submission_results(),
            execution_metadata: create_test_execution_metadata(),
            verification_status: VerificationStatus {
                level: self.verification_level,
                verified_at: if self.verification_level != VerificationLevel::Unverified {
                    Some(Utc::now())
                } else {
                    None
                },
                verified_by: None,
                verification_details: None,
            },
            visibility: self.visibility,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

impl Default for SubmissionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating Proposal test instances
#[derive(Clone)]
pub struct ProposalBuilder {
    id: ProposalId,
    proposal_type: ProposalType,
    title: String,
    description: String,
    created_by: UserId,
    status: ProposalStatus,
    benchmark_id: Option<BenchmarkId>,
}

impl ProposalBuilder {
    pub fn new() -> Self {
        Self {
            id: ProposalId::new(),
            proposal_type: ProposalType::NewBenchmark,
            title: "Test Proposal".to_string(),
            description: "A test proposal".to_string(),
            created_by: UserId::new(),
            status: ProposalStatus::Draft,
            benchmark_id: Some(BenchmarkId::new()),
        }
    }

    pub fn with_id(mut self, id: ProposalId) -> Self {
        self.id = id;
        self
    }

    pub fn with_type(mut self, proposal_type: ProposalType) -> Self {
        self.proposal_type = proposal_type;
        self
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_creator(mut self, user_id: UserId) -> Self {
        self.created_by = user_id;
        self
    }

    pub fn with_status(mut self, status: ProposalStatus) -> Self {
        self.status = status;
        self
    }

    pub fn with_benchmark_id(mut self, id: BenchmarkId) -> Self {
        self.benchmark_id = Some(id);
        self
    }

    pub fn under_review(mut self) -> Self {
        self.status = ProposalStatus::UnderReview;
        self
    }

    pub fn voting(mut self) -> Self {
        self.status = ProposalStatus::Voting;
        self
    }

    pub fn approved(mut self) -> Self {
        self.status = ProposalStatus::Approved;
        self
    }

    pub fn rejected(mut self) -> Self {
        self.status = ProposalStatus::Rejected;
        self
    }

    pub fn build(self) -> Proposal {
        Proposal {
            id: self.id,
            proposal_type: self.proposal_type,
            title: self.title,
            description: self.description,
            created_by: self.created_by,
            status: self.status,
            benchmark_id: self.benchmark_id,
            rationale: "Test rationale".to_string(),
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
}

impl Default for ProposalBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_builder() {
        let user = UserBuilder::new()
            .with_username("alice")
            .with_email("alice@example.com")
            .admin()
            .build();

        assert_eq!(user.username, "alice");
        assert_eq!(user.email, "alice@example.com");
        assert_eq!(user.role, UserRole::Admin);
    }

    #[test]
    fn test_organization_builder() {
        let org = OrganizationBuilder::new()
            .with_name("Acme Corp")
            .llm_provider()
            .build();

        assert_eq!(org.name, "Acme Corp");
        assert_eq!(org.organization_type, OrganizationType::LlmProvider);
    }

    #[test]
    fn test_benchmark_builder() {
        let benchmark = BenchmarkBuilder::new()
            .with_name("My Benchmark")
            .with_category(BenchmarkCategory::Accuracy)
            .with_tag("nlp")
            .build();

        assert_eq!(benchmark.name, "My Benchmark");
        assert!(benchmark.tags.contains(&"nlp".to_string()));
    }

    #[test]
    fn test_submission_builder() {
        let submission = SubmissionBuilder::new()
            .private()
            .verified()
            .build();

        assert_eq!(submission.visibility, SubmissionVisibility::Private);
        assert_eq!(
            submission.verification_status.level,
            VerificationLevel::PlatformVerified
        );
    }

    #[test]
    fn test_proposal_builder() {
        let proposal = ProposalBuilder::new()
            .with_title("New Feature")
            .voting()
            .build();

        assert_eq!(proposal.title, "New Feature");
        assert_eq!(proposal.status, ProposalStatus::Voting);
    }
}
