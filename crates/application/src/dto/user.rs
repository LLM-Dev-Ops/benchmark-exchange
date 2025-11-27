//! User and Organization DTOs for API layer

use serde::{Deserialize, Serialize};

/// User response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    pub is_verified: bool,
    pub stats: UserStatsDto,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// User stats DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStatsDto {
    pub submission_count: u64,
    pub benchmark_count: u64,
    pub organization_count: u64,
}

/// User profile response (self view with email)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfileResponse {
    pub id: String,
    pub email: String,
    pub username: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    pub is_verified: bool,
    pub is_admin: bool,
    pub stats: UserStatsDto,
    pub organizations: Vec<UserOrganizationDto>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// User's organization membership
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserOrganizationDto {
    pub organization_id: String,
    pub organization_name: String,
    pub organization_slug: String,
    pub role: String,
    pub joined_at: chrono::DateTime<chrono::Utc>,
}

/// Register user request DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterUserDto {
    pub email: String,
    pub username: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth_token: Option<String>,
}

/// Update user profile request DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
}

/// Change password request DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePasswordDto {
    pub current_password: String,
    pub new_password: String,
}

/// Login request DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginDto {
    pub email: String,
    pub password: String,
}

/// Login response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub user: UserResponse,
}

/// Refresh token request DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenDto {
    pub refresh_token: String,
}

/// API key response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyResponse {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub scopes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// API key with secret (returned only on creation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyWithSecretResponse {
    pub key: ApiKeyResponse,
    /// The secret key (only shown once)
    pub secret: String,
}

/// Create API key request DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKeyDto {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub scopes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in_days: Option<u32>,
}

/// Organization response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationResponse {
    pub id: String,
    pub name: String,
    pub slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_url: Option<String>,
    pub is_verified: bool,
    pub stats: OrganizationStatsDto,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Organization stats DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationStatsDto {
    pub member_count: u64,
    pub submission_count: u64,
    pub benchmark_count: u64,
}

/// Create organization request DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrganizationDto {
    pub name: String,
    pub slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_email: Option<String>,
}

/// Update organization request DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateOrganizationDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_url: Option<String>,
}

/// Organization member response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationMemberResponse {
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    pub role: String,
    pub joined_at: chrono::DateTime<chrono::Utc>,
}

/// Add organization member request DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddMemberDto {
    pub user_id: String,
    pub role: String,
}

/// Update member role request DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMemberRoleDto {
    pub role: String,
}

/// Invite member request DTO (via email)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteMemberDto {
    pub email: String,
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Invitation response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationResponse {
    pub id: String,
    pub organization_id: String,
    pub email: String,
    pub role: String,
    pub status: InvitationStatus,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InvitationStatus {
    Pending,
    Accepted,
    Declined,
    Expired,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_response_serialization() {
        let user = UserResponse {
            id: "user-123".to_string(),
            username: "testuser".to_string(),
            display_name: "Test User".to_string(),
            bio: Some("A test user".to_string()),
            website: Some("https://example.com".to_string()),
            avatar_url: None,
            is_verified: true,
            stats: UserStatsDto {
                submission_count: 10,
                benchmark_count: 2,
                organization_count: 1,
            },
            created_at: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("testuser"));
        assert!(json.contains("submission_count"));
    }

    #[test]
    fn test_login_dto() {
        let json = r#"{"email": "user@example.com", "password": "secret"}"#;
        let dto: LoginDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.email, "user@example.com");
    }

    #[test]
    fn test_create_api_key() {
        let json = r#"{
            "name": "My Key",
            "scopes": ["read:benchmarks", "read:submissions"]
        }"#;

        let dto: CreateApiKeyDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.name, "My Key");
        assert_eq!(dto.scopes.len(), 2);
        assert!(dto.expires_in_days.is_none());
    }

    #[test]
    fn test_organization_response() {
        let org = OrganizationResponse {
            id: "org-123".to_string(),
            name: "Test Org".to_string(),
            slug: "test-org".to_string(),
            description: Some("A test organization".to_string()),
            website: None,
            contact_email: None,
            logo_url: None,
            is_verified: false,
            stats: OrganizationStatsDto {
                member_count: 5,
                submission_count: 20,
                benchmark_count: 3,
            },
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&org).unwrap();
        assert!(json.contains("test-org"));
        assert!(json.contains("member_count"));
    }
}
