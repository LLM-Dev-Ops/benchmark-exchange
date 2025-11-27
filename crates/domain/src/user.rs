//! User and organization types.

use crate::identifiers::{OrganizationId, UserId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

/// User account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub email: String,
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub role: UserRole,
    pub organizations: Vec<OrganizationMembership>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_active_at: Option<DateTime<Utc>>,
    pub email_verified: bool,
    pub profile: UserProfile,
}

/// User roles for access control
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    Anonymous = 0,
    Registered = 1,
    Contributor = 2,
    Reviewer = 3,
    Admin = 4,
}

impl UserRole {
    pub fn can_submit_results(&self) -> bool {
        *self >= Self::Registered
    }

    pub fn can_propose_benchmarks(&self) -> bool {
        *self >= Self::Contributor
    }

    pub fn can_vote(&self) -> bool {
        *self >= Self::Contributor
    }

    pub fn can_review(&self) -> bool {
        *self >= Self::Reviewer
    }

    pub fn can_verify(&self) -> bool {
        *self >= Self::Reviewer
    }

    pub fn can_manage_users(&self) -> bool {
        *self >= Self::Admin
    }

    pub fn can_configure_system(&self) -> bool {
        *self >= Self::Admin
    }
}

/// User profile information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affiliation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<Url>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github_username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orcid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_email: Option<String>,
}

/// Organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: OrganizationId,
    pub name: String,
    pub slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<Url>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_url: Option<Url>,
    pub organization_type: OrganizationType,
    pub verified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationType {
    LlmProvider,
    ResearchInstitution,
    Enterprise,
    OpenSource,
    Individual,
}

/// Organization membership
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationMembership {
    pub organization_id: OrganizationId,
    pub role: OrganizationRole,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationRole {
    Member,
    Admin,
    Owner,
}
