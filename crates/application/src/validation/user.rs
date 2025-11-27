//! User and Organization validation rules

use super::{Validatable, ValidationResult, ValidationRules};
use serde::{Deserialize, Serialize};

/// Create user request validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub username: String,
    pub display_name: String,
    pub password: Option<String>,
    pub oauth_provider: Option<String>,
    pub oauth_id: Option<String>,
}

impl CreateUserRequest {
    pub const MIN_USERNAME_LENGTH: usize = 3;
    pub const MAX_USERNAME_LENGTH: usize = 50;
    pub const MIN_DISPLAY_NAME_LENGTH: usize = 1;
    pub const MAX_DISPLAY_NAME_LENGTH: usize = 100;
    pub const MIN_PASSWORD_LENGTH: usize = 12;
    pub const MAX_PASSWORD_LENGTH: usize = 128;
}

impl Validatable for CreateUserRequest {
    fn validate_all(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        // Email validation
        let email_result = ValidationRules::validate_email(&self.email);
        result.merge(email_result);

        // Username validation
        let username_result = ValidationRules::validate_length(
            &self.username,
            "username",
            Some(Self::MIN_USERNAME_LENGTH),
            Some(Self::MAX_USERNAME_LENGTH),
        );
        result.merge(username_result);

        // Username format (alphanumeric, underscore, hyphen)
        if !self.username.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
            result.add_field_error(
                "username",
                "Username must contain only letters, numbers, underscores, and hyphens",
            );
        }

        // Username cannot start with a number
        if self.username.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            result.add_field_error("username", "Username cannot start with a number");
        }

        // Display name validation
        let display_name_result = ValidationRules::validate_length(
            &self.display_name,
            "display_name",
            Some(Self::MIN_DISPLAY_NAME_LENGTH),
            Some(Self::MAX_DISPLAY_NAME_LENGTH),
        );
        result.merge(display_name_result);

        // Either password or OAuth must be provided
        if self.password.is_none() && self.oauth_provider.is_none() {
            result.add_object_error("Either password or OAuth credentials must be provided");
        }

        // Password validation if provided
        if let Some(ref password) = self.password {
            let password_result = validate_password(password);
            result.merge(password_result);
        }

        // OAuth validation if provided
        if let Some(ref provider) = self.oauth_provider {
            let valid_providers = ["google", "github", "microsoft", "gitlab"];
            if !valid_providers.contains(&provider.to_lowercase().as_str()) {
                result.add_field_error(
                    "oauth_provider",
                    format!("Invalid OAuth provider. Allowed: {}", valid_providers.join(", ")),
                );
            }

            if self.oauth_id.is_none() {
                result.add_field_error("oauth_id", "OAuth ID is required when provider is specified");
            }
        }

        result
    }
}

/// Validate password strength
pub fn validate_password(password: &str) -> ValidationResult {
    let mut result = ValidationResult::success();

    // Length check
    if password.len() < CreateUserRequest::MIN_PASSWORD_LENGTH {
        result.add_field_error(
            "password",
            format!(
                "Password must be at least {} characters",
                CreateUserRequest::MIN_PASSWORD_LENGTH
            ),
        );
    }

    if password.len() > CreateUserRequest::MAX_PASSWORD_LENGTH {
        result.add_field_error(
            "password",
            format!(
                "Password must be {} characters or less",
                CreateUserRequest::MAX_PASSWORD_LENGTH
            ),
        );
    }

    // Complexity requirements
    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_special = password.chars().any(|c| !c.is_alphanumeric());

    if !has_uppercase {
        result.add_field_error("password", "Password must contain at least one uppercase letter");
    }
    if !has_lowercase {
        result.add_field_error("password", "Password must contain at least one lowercase letter");
    }
    if !has_digit {
        result.add_field_error("password", "Password must contain at least one digit");
    }
    if !has_special {
        result.add_field_error("password", "Password must contain at least one special character");
    }

    // Common password check (simplified - in production, use a proper dictionary)
    let common_passwords = [
        "password123",
        "123456789012",
        "qwertyuiopas",
        "letmein12345",
    ];
    if common_passwords.iter().any(|p| password.to_lowercase().contains(p)) {
        result.add_field_error("password", "Password is too common");
    }

    result
}

/// Update user request validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub website: Option<String>,
    pub avatar_url: Option<String>,
}

impl UpdateUserRequest {
    pub const MAX_BIO_LENGTH: usize = 500;
}

impl Validatable for UpdateUserRequest {
    fn validate_all(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        if let Some(ref display_name) = self.display_name {
            let name_result = ValidationRules::validate_length(
                display_name,
                "display_name",
                Some(1),
                Some(CreateUserRequest::MAX_DISPLAY_NAME_LENGTH),
            );
            result.merge(name_result);
        }

        if let Some(ref bio) = self.bio {
            let bio_result = ValidationRules::validate_length(
                bio,
                "bio",
                None,
                Some(Self::MAX_BIO_LENGTH),
            );
            result.merge(bio_result);
        }

        if let Some(ref website) = self.website {
            let url_result = ValidationRules::validate_url(website);
            result.merge(url_result);
        }

        if let Some(ref avatar_url) = self.avatar_url {
            let url_result = ValidationRules::validate_url(avatar_url);
            result.merge(url_result);
        }

        result
    }
}

/// Change password request validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

impl Validatable for ChangePasswordRequest {
    fn validate_all(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        // Current password must not be empty
        if self.current_password.is_empty() {
            result.add_field_error("current_password", "Current password is required");
        }

        // New password validation
        let password_result = validate_password(&self.new_password);
        result.merge(password_result);

        // New password must be different from current
        if self.current_password == self.new_password {
            result.add_object_error("New password must be different from current password");
        }

        result
    }
}

/// Create organization request validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrganizationRequest {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub website: Option<String>,
    pub contact_email: Option<String>,
}

impl CreateOrganizationRequest {
    pub const MIN_NAME_LENGTH: usize = 2;
    pub const MAX_NAME_LENGTH: usize = 100;
    pub const MAX_DESCRIPTION_LENGTH: usize = 1000;
}

impl Validatable for CreateOrganizationRequest {
    fn validate_all(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        // Name validation
        let name_result = ValidationRules::validate_length(
            &self.name,
            "name",
            Some(Self::MIN_NAME_LENGTH),
            Some(Self::MAX_NAME_LENGTH),
        );
        result.merge(name_result);

        // Slug validation
        let slug_result = ValidationRules::validate_slug(&self.slug);
        result.merge(slug_result);

        // Description validation
        if let Some(ref description) = self.description {
            let desc_result = ValidationRules::validate_length(
                description,
                "description",
                None,
                Some(Self::MAX_DESCRIPTION_LENGTH),
            );
            result.merge(desc_result);
        }

        // Website validation
        if let Some(ref website) = self.website {
            let url_result = ValidationRules::validate_url(website);
            result.merge(url_result);
        }

        // Contact email validation
        if let Some(ref email) = self.contact_email {
            let email_result = ValidationRules::validate_email(email);
            result.merge(email_result);
        }

        result
    }
}

/// Update organization request validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateOrganizationRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub website: Option<String>,
    pub contact_email: Option<String>,
    pub logo_url: Option<String>,
}

impl Validatable for UpdateOrganizationRequest {
    fn validate_all(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        if let Some(ref name) = self.name {
            let name_result = ValidationRules::validate_length(
                name,
                "name",
                Some(CreateOrganizationRequest::MIN_NAME_LENGTH),
                Some(CreateOrganizationRequest::MAX_NAME_LENGTH),
            );
            result.merge(name_result);
        }

        if let Some(ref description) = self.description {
            let desc_result = ValidationRules::validate_length(
                description,
                "description",
                None,
                Some(CreateOrganizationRequest::MAX_DESCRIPTION_LENGTH),
            );
            result.merge(desc_result);
        }

        if let Some(ref website) = self.website {
            let url_result = ValidationRules::validate_url(website);
            result.merge(url_result);
        }

        if let Some(ref email) = self.contact_email {
            let email_result = ValidationRules::validate_email(email);
            result.merge(email_result);
        }

        if let Some(ref logo_url) = self.logo_url {
            let url_result = ValidationRules::validate_url(logo_url);
            result.merge(url_result);
        }

        result
    }
}

/// Add organization member request validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddMemberRequest {
    pub user_id: String,
    pub role: OrganizationRole,
}

/// Organization roles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationRole {
    Owner,
    Admin,
    Member,
    Viewer,
}

impl Validatable for AddMemberRequest {
    fn validate_all(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        // User ID validation
        let id_result = ValidationRules::validate_uuid(&self.user_id, "user_id");
        result.merge(id_result);

        result
    }
}

/// API key creation request validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub description: Option<String>,
    pub scopes: Vec<String>,
    pub expires_in_days: Option<u32>,
}

impl CreateApiKeyRequest {
    pub const MAX_NAME_LENGTH: usize = 100;
    pub const MAX_DESCRIPTION_LENGTH: usize = 500;
    pub const MAX_SCOPES: usize = 50;
    pub const MAX_EXPIRY_DAYS: u32 = 365;
}

impl Validatable for CreateApiKeyRequest {
    fn validate_all(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        // Name validation
        let name_result = ValidationRules::validate_length(
            &self.name,
            "name",
            Some(1),
            Some(Self::MAX_NAME_LENGTH),
        );
        result.merge(name_result);

        // Description validation
        if let Some(ref description) = self.description {
            let desc_result = ValidationRules::validate_length(
                description,
                "description",
                None,
                Some(Self::MAX_DESCRIPTION_LENGTH),
            );
            result.merge(desc_result);
        }

        // Scopes validation
        if self.scopes.is_empty() {
            result.add_field_error("scopes", "At least one scope is required");
        }

        if self.scopes.len() > Self::MAX_SCOPES {
            result.add_field_error(
                "scopes",
                format!("Maximum {} scopes allowed", Self::MAX_SCOPES),
            );
        }

        // Validate individual scopes
        let valid_scopes = [
            "read:benchmarks",
            "write:benchmarks",
            "read:submissions",
            "write:submissions",
            "read:users",
            "write:users",
            "read:organizations",
            "write:organizations",
            "admin",
        ];

        for scope in &self.scopes {
            if !valid_scopes.contains(&scope.as_str()) {
                result.add_field_error(
                    "scopes",
                    format!("Invalid scope: {}", scope),
                );
            }
        }

        // Expiry validation
        if let Some(days) = self.expires_in_days {
            if days == 0 {
                result.add_field_error("expires_in_days", "Expiry must be at least 1 day");
            }
            if days > Self::MAX_EXPIRY_DAYS {
                result.add_field_error(
                    "expires_in_days",
                    format!("Expiry cannot exceed {} days", Self::MAX_EXPIRY_DAYS),
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
    fn test_create_user_validation() {
        let valid = CreateUserRequest {
            email: "user@example.com".to_string(),
            username: "testuser".to_string(),
            display_name: "Test User".to_string(),
            password: Some("SecureP@ssw0rd!".to_string()),
            oauth_provider: None,
            oauth_id: None,
        };
        assert!(valid.validate_all().valid);

        let invalid_email = CreateUserRequest {
            email: "invalid-email".to_string(),
            ..valid.clone()
        };
        assert!(!invalid_email.validate_all().valid);

        let invalid_username = CreateUserRequest {
            username: "123invalid".to_string(), // Starts with number
            ..valid.clone()
        };
        assert!(!invalid_username.validate_all().valid);

        let no_auth = CreateUserRequest {
            password: None,
            oauth_provider: None,
            oauth_id: None,
            ..valid.clone()
        };
        assert!(!no_auth.validate_all().valid);
    }

    #[test]
    fn test_password_validation() {
        // Valid password
        let valid = validate_password("SecureP@ssw0rd!");
        assert!(valid.valid);

        // Too short
        let too_short = validate_password("Short1!");
        assert!(!too_short.valid);

        // Missing uppercase
        let no_upper = validate_password("securep@ssw0rd!");
        assert!(!no_upper.valid);

        // Missing lowercase
        let no_lower = validate_password("SECUREP@SSW0RD!");
        assert!(!no_lower.valid);

        // Missing digit
        let no_digit = validate_password("SecureP@ssword!");
        assert!(!no_digit.valid);

        // Missing special
        let no_special = validate_password("SecurePassword1");
        assert!(!no_special.valid);
    }

    #[test]
    fn test_create_organization_validation() {
        let valid = CreateOrganizationRequest {
            name: "Test Organization".to_string(),
            slug: "test-org".to_string(),
            description: Some("A test organization".to_string()),
            website: Some("https://example.com".to_string()),
            contact_email: Some("contact@example.com".to_string()),
        };
        assert!(valid.validate_all().valid);

        let invalid_slug = CreateOrganizationRequest {
            slug: "Invalid Slug!".to_string(),
            ..valid.clone()
        };
        assert!(!invalid_slug.validate_all().valid);
    }

    #[test]
    fn test_add_member_validation() {
        let valid = AddMemberRequest {
            user_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            role: OrganizationRole::Member,
        };
        assert!(valid.validate_all().valid);

        let invalid_id = AddMemberRequest {
            user_id: "not-a-uuid".to_string(),
            role: OrganizationRole::Member,
        };
        assert!(!invalid_id.validate_all().valid);
    }

    #[test]
    fn test_create_api_key_validation() {
        let valid = CreateApiKeyRequest {
            name: "My API Key".to_string(),
            description: Some("For testing".to_string()),
            scopes: vec!["read:benchmarks".to_string(), "read:submissions".to_string()],
            expires_in_days: Some(30),
        };
        assert!(valid.validate_all().valid);

        let empty_scopes = CreateApiKeyRequest {
            scopes: vec![],
            ..valid.clone()
        };
        assert!(!empty_scopes.validate_all().valid);

        let invalid_scope = CreateApiKeyRequest {
            scopes: vec!["invalid:scope".to_string()],
            ..valid.clone()
        };
        assert!(!invalid_scope.validate_all().valid);

        let too_long_expiry = CreateApiKeyRequest {
            expires_in_days: Some(500),
            ..valid.clone()
        };
        assert!(!too_long_expiry.validate_all().valid);
    }

    #[test]
    fn test_change_password_validation() {
        let valid = ChangePasswordRequest {
            current_password: "OldP@ssw0rd!".to_string(),
            new_password: "NewSecureP@ss1!".to_string(),
        };
        assert!(valid.validate_all().valid);

        let same_password = ChangePasswordRequest {
            current_password: "SameP@ssw0rd!".to_string(),
            new_password: "SameP@ssw0rd!".to_string(),
        };
        assert!(!same_password.validate_all().valid);
    }
}
