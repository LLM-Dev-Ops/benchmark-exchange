//! Type conversions between domain and protobuf types

use crate::proto;
use chrono::{DateTime, Utc};
use llm_benchmark_domain::*;
use prost_types::Timestamp;
use std::collections::HashMap;

// Timestamp conversions
pub fn datetime_to_timestamp(dt: &DateTime<Utc>) -> Option<Timestamp> {
    Some(Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    })
}

pub fn timestamp_to_datetime(ts: &Timestamp) -> DateTime<Utc> {
    DateTime::from_timestamp(ts.seconds, ts.nanos as u32).unwrap_or_default()
}

// Benchmark category conversions
impl From<BenchmarkCategory> for proto::BenchmarkCategory {
    fn from(cat: BenchmarkCategory) -> Self {
        match cat {
            BenchmarkCategory::Performance => proto::BenchmarkCategory::Performance,
            BenchmarkCategory::Accuracy => proto::BenchmarkCategory::Accuracy,
            BenchmarkCategory::Reliability => proto::BenchmarkCategory::Reliability,
            BenchmarkCategory::Safety => proto::BenchmarkCategory::Safety,
            BenchmarkCategory::Cost => proto::BenchmarkCategory::Cost,
            BenchmarkCategory::Capability => proto::BenchmarkCategory::Capability,
        }
    }
}

impl From<proto::BenchmarkCategory> for BenchmarkCategory {
    fn from(cat: proto::BenchmarkCategory) -> Self {
        match cat {
            proto::BenchmarkCategory::Performance => BenchmarkCategory::Performance,
            proto::BenchmarkCategory::Accuracy => BenchmarkCategory::Accuracy,
            proto::BenchmarkCategory::Reliability => BenchmarkCategory::Reliability,
            proto::BenchmarkCategory::Safety => BenchmarkCategory::Safety,
            proto::BenchmarkCategory::Cost => BenchmarkCategory::Cost,
            proto::BenchmarkCategory::Capability => BenchmarkCategory::Capability,
            _ => BenchmarkCategory::Performance, // Default fallback
        }
    }
}

// Benchmark status conversions
impl From<BenchmarkStatus> for proto::BenchmarkStatus {
    fn from(status: BenchmarkStatus) -> Self {
        match status {
            BenchmarkStatus::Draft => proto::BenchmarkStatus::Draft,
            BenchmarkStatus::UnderReview => proto::BenchmarkStatus::UnderReview,
            BenchmarkStatus::Active => proto::BenchmarkStatus::Active,
            BenchmarkStatus::Deprecated => proto::BenchmarkStatus::Deprecated,
            BenchmarkStatus::Archived => proto::BenchmarkStatus::Archived,
        }
    }
}

impl From<proto::BenchmarkStatus> for BenchmarkStatus {
    fn from(status: proto::BenchmarkStatus) -> Self {
        match status {
            proto::BenchmarkStatus::Draft => BenchmarkStatus::Draft,
            proto::BenchmarkStatus::UnderReview => BenchmarkStatus::UnderReview,
            proto::BenchmarkStatus::Active => BenchmarkStatus::Active,
            proto::BenchmarkStatus::Deprecated => BenchmarkStatus::Deprecated,
            proto::BenchmarkStatus::Archived => BenchmarkStatus::Archived,
            _ => BenchmarkStatus::Draft,
        }
    }
}

// License type conversions
impl From<LicenseType> for proto::LicenseType {
    fn from(license: LicenseType) -> Self {
        match license {
            LicenseType::Apache2 => proto::LicenseType::Apache2,
            LicenseType::MIT => proto::LicenseType::Mit,
            LicenseType::BSD3Clause => proto::LicenseType::Bsd3Clause,
            LicenseType::CC_BY_4_0 => proto::LicenseType::CcBy40,
            LicenseType::CC_BY_SA_4_0 => proto::LicenseType::CcBySa40,
            LicenseType::Custom(_) => proto::LicenseType::Custom,
        }
    }
}

// Verification level conversions
impl From<submission::VerificationLevel> for proto::VerificationLevel {
    fn from(level: submission::VerificationLevel) -> Self {
        match level {
            submission::VerificationLevel::Unverified => proto::VerificationLevel::Unverified,
            submission::VerificationLevel::CommunityVerified => {
                proto::VerificationLevel::CommunityVerified
            }
            submission::VerificationLevel::PlatformVerified => {
                proto::VerificationLevel::PlatformVerified
            }
            submission::VerificationLevel::Audited => proto::VerificationLevel::Audited,
        }
    }
}

impl From<proto::VerificationLevel> for submission::VerificationLevel {
    fn from(level: proto::VerificationLevel) -> Self {
        match level {
            proto::VerificationLevel::Unverified => submission::VerificationLevel::Unverified,
            proto::VerificationLevel::CommunityVerified => {
                submission::VerificationLevel::CommunityVerified
            }
            proto::VerificationLevel::PlatformVerified => {
                submission::VerificationLevel::PlatformVerified
            }
            proto::VerificationLevel::Audited => submission::VerificationLevel::Audited,
            _ => submission::VerificationLevel::Unverified,
        }
    }
}

// Submission visibility conversions
impl From<submission::SubmissionVisibility> for proto::SubmissionVisibility {
    fn from(vis: submission::SubmissionVisibility) -> Self {
        match vis {
            submission::SubmissionVisibility::Public => proto::SubmissionVisibility::Public,
            submission::SubmissionVisibility::Unlisted => proto::SubmissionVisibility::Unlisted,
            submission::SubmissionVisibility::Private => proto::SubmissionVisibility::Private,
        }
    }
}

impl From<proto::SubmissionVisibility> for submission::SubmissionVisibility {
    fn from(vis: proto::SubmissionVisibility) -> Self {
        match vis {
            proto::SubmissionVisibility::Public => submission::SubmissionVisibility::Public,
            proto::SubmissionVisibility::Unlisted => submission::SubmissionVisibility::Unlisted,
            proto::SubmissionVisibility::Private => submission::SubmissionVisibility::Private,
            _ => submission::SubmissionVisibility::Public,
        }
    }
}

// User role conversions
impl From<user::UserRole> for proto::UserRole {
    fn from(role: user::UserRole) -> Self {
        match role {
            user::UserRole::Anonymous => proto::UserRole::Anonymous,
            user::UserRole::Registered => proto::UserRole::Registered,
            user::UserRole::Contributor => proto::UserRole::Contributor,
            user::UserRole::Reviewer => proto::UserRole::Reviewer,
            user::UserRole::Admin => proto::UserRole::Admin,
        }
    }
}

impl From<proto::UserRole> for user::UserRole {
    fn from(role: proto::UserRole) -> Self {
        match role {
            proto::UserRole::Anonymous => user::UserRole::Anonymous,
            proto::UserRole::Registered => user::UserRole::Registered,
            proto::UserRole::Contributor => user::UserRole::Contributor,
            proto::UserRole::Reviewer => user::UserRole::Reviewer,
            proto::UserRole::Admin => user::UserRole::Admin,
            _ => user::UserRole::Anonymous,
        }
    }
}

// Organization type conversions
impl From<user::OrganizationType> for proto::OrganizationType {
    fn from(org_type: user::OrganizationType) -> Self {
        match org_type {
            user::OrganizationType::LlmProvider => proto::OrganizationType::LlmProvider,
            user::OrganizationType::ResearchInstitution => {
                proto::OrganizationType::ResearchInstitution
            }
            user::OrganizationType::Enterprise => proto::OrganizationType::Enterprise,
            user::OrganizationType::OpenSource => proto::OrganizationType::OpenSource,
            user::OrganizationType::Individual => proto::OrganizationType::Individual,
        }
    }
}

// Proposal type conversions
impl From<governance::ProposalType> for proto::ProposalType {
    fn from(ptype: governance::ProposalType) -> Self {
        match ptype {
            governance::ProposalType::NewBenchmark => proto::ProposalType::NewBenchmark,
            governance::ProposalType::UpdateBenchmark => proto::ProposalType::UpdateBenchmark,
            governance::ProposalType::DeprecateBenchmark => {
                proto::ProposalType::DeprecateBenchmark
            }
            governance::ProposalType::PolicyChange => proto::ProposalType::PolicyChange,
        }
    }
}

impl From<proto::ProposalType> for governance::ProposalType {
    fn from(ptype: proto::ProposalType) -> Self {
        match ptype {
            proto::ProposalType::NewBenchmark => governance::ProposalType::NewBenchmark,
            proto::ProposalType::UpdateBenchmark => governance::ProposalType::UpdateBenchmark,
            proto::ProposalType::DeprecateBenchmark => {
                governance::ProposalType::DeprecateBenchmark
            }
            proto::ProposalType::PolicyChange => governance::ProposalType::PolicyChange,
            _ => governance::ProposalType::NewBenchmark,
        }
    }
}

// Proposal status conversions
impl From<governance::ProposalStatus> for proto::ProposalStatus {
    fn from(status: governance::ProposalStatus) -> Self {
        match status {
            governance::ProposalStatus::Draft => proto::ProposalStatus::Draft,
            governance::ProposalStatus::UnderReview => proto::ProposalStatus::UnderReview,
            governance::ProposalStatus::Voting => proto::ProposalStatus::Voting,
            governance::ProposalStatus::Approved => proto::ProposalStatus::Approved,
            governance::ProposalStatus::Rejected => proto::ProposalStatus::Rejected,
            governance::ProposalStatus::Withdrawn => proto::ProposalStatus::Withdrawn,
        }
    }
}

impl From<proto::ProposalStatus> for governance::ProposalStatus {
    fn from(status: proto::ProposalStatus) -> Self {
        match status {
            proto::ProposalStatus::Draft => governance::ProposalStatus::Draft,
            proto::ProposalStatus::UnderReview => governance::ProposalStatus::UnderReview,
            proto::ProposalStatus::Voting => governance::ProposalStatus::Voting,
            proto::ProposalStatus::Approved => governance::ProposalStatus::Approved,
            proto::ProposalStatus::Rejected => governance::ProposalStatus::Rejected,
            proto::ProposalStatus::Withdrawn => governance::ProposalStatus::Withdrawn,
            _ => governance::ProposalStatus::Draft,
        }
    }
}

// Vote conversions
impl From<governance::Vote> for proto::Vote {
    fn from(vote: governance::Vote) -> Self {
        match vote {
            governance::Vote::Approve => proto::Vote::Approve,
            governance::Vote::Reject => proto::Vote::Reject,
            governance::Vote::Abstain => proto::Vote::Abstain,
        }
    }
}

impl From<proto::Vote> for governance::Vote {
    fn from(vote: proto::Vote) -> Self {
        match vote {
            proto::Vote::Approve => governance::Vote::Approve,
            proto::Vote::Reject => governance::Vote::Reject,
            proto::Vote::Abstain => governance::Vote::Abstain,
            _ => governance::Vote::Abstain,
        }
    }
}
