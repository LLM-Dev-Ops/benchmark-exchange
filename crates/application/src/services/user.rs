//! User Service
//!
//! Business logic for user management including authentication,
//! profile management, and API key operations.

use super::{EventPublisher, PaginatedResult, Pagination, ServiceConfig, ServiceContext, ServiceEvent};
use crate::validation::{ChangePasswordRequest, CreateApiKeyRequest, CreateUserRequest, UpdateUserRequest, Validatable};
use crate::{ApplicationError, ApplicationResult};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

/// User data transfer object
#[derive(Debug, Clone)]
pub struct UserDto {
    pub id: String,
    pub email: String,
    pub username: String,
    pub display_name: String,
    pub bio: Option<String>,
    pub website: Option<String>,
    pub avatar_url: Option<String>,
    pub is_verified: bool,
    pub is_admin: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// User profile (public view)
#[derive(Debug, Clone)]
pub struct UserProfileDto {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub bio: Option<String>,
    pub website: Option<String>,
    pub avatar_url: Option<String>,
    pub submission_count: u64,
    pub benchmark_count: u64,
    pub joined_at: chrono::DateTime<chrono::Utc>,
}

/// API key data transfer object
#[derive(Debug, Clone)]
pub struct ApiKeyDto {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub scopes: Vec<String>,
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// API key with secret (only returned on creation)
#[derive(Debug, Clone)]
pub struct ApiKeyWithSecretDto {
    pub key: ApiKeyDto,
    pub secret: String,
}

/// User repository trait
#[async_trait]
pub trait UserRepositoryPort: Send + Sync {
    async fn create(&self, user: &CreateUserData) -> Result<String, ApplicationError>;
    async fn get_by_id(&self, id: &str) -> Result<Option<UserDto>, ApplicationError>;
    async fn get_by_email(&self, email: &str) -> Result<Option<UserDto>, ApplicationError>;
    async fn get_by_username(&self, username: &str) -> Result<Option<UserDto>, ApplicationError>;
    async fn update(&self, id: &str, update: &UpdateUserData) -> Result<(), ApplicationError>;
    async fn update_password(&self, id: &str, password_hash: &str) -> Result<(), ApplicationError>;
    async fn verify_password(&self, id: &str, password: &str) -> Result<bool, ApplicationError>;
    async fn delete(&self, id: &str) -> Result<(), ApplicationError>;
    async fn get_profile(&self, id: &str) -> Result<Option<UserProfileDto>, ApplicationError>;
    async fn email_exists(&self, email: &str) -> Result<bool, ApplicationError>;
    async fn username_exists(&self, username: &str) -> Result<bool, ApplicationError>;
    async fn create_api_key(&self, user_id: &str, key: &CreateApiKeyData) -> Result<ApiKeyWithSecretDto, ApplicationError>;
    async fn list_api_keys(&self, user_id: &str) -> Result<Vec<ApiKeyDto>, ApplicationError>;
    async fn revoke_api_key(&self, user_id: &str, key_id: &str) -> Result<(), ApplicationError>;
    async fn verify_api_key(&self, key_secret: &str) -> Result<Option<(String, Vec<String>)>, ApplicationError>;
}

/// Data for creating a user
#[derive(Debug, Clone)]
pub struct CreateUserData {
    pub email: String,
    pub username: String,
    pub display_name: String,
    pub password_hash: Option<String>,
    pub oauth_provider: Option<String>,
    pub oauth_id: Option<String>,
}

/// Data for updating a user
#[derive(Debug, Clone)]
pub struct UpdateUserData {
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub website: Option<String>,
    pub avatar_url: Option<String>,
}

/// Data for creating an API key
#[derive(Debug, Clone)]
pub struct CreateApiKeyData {
    pub name: String,
    pub description: Option<String>,
    pub scopes: Vec<String>,
    pub expires_in_days: Option<u32>,
}

/// Password hasher trait
#[async_trait]
pub trait PasswordHasher: Send + Sync {
    async fn hash(&self, password: &str) -> Result<String, ApplicationError>;
    async fn verify(&self, password: &str, hash: &str) -> Result<bool, ApplicationError>;
}

/// Default password hasher using argon2
pub struct Argon2PasswordHasher;

#[async_trait]
impl PasswordHasher for Argon2PasswordHasher {
    async fn hash(&self, password: &str) -> Result<String, ApplicationError> {
        // In production, use proper argon2 hashing
        // For now, return a placeholder
        Ok(format!("argon2:${}", password))
    }

    async fn verify(&self, password: &str, hash: &str) -> Result<bool, ApplicationError> {
        // In production, use proper argon2 verification
        Ok(hash == format!("argon2:${}", password))
    }
}

/// User service implementation
pub struct UserService<R, E, H>
where
    R: UserRepositoryPort,
    E: EventPublisher,
    H: PasswordHasher,
{
    repository: Arc<R>,
    event_publisher: Arc<E>,
    password_hasher: Arc<H>,
    config: ServiceConfig,
}

impl<R, E, H> UserService<R, E, H>
where
    R: UserRepositoryPort,
    E: EventPublisher,
    H: PasswordHasher,
{
    pub fn new(
        repository: Arc<R>,
        event_publisher: Arc<E>,
        password_hasher: Arc<H>,
        config: ServiceConfig,
    ) -> Self {
        Self {
            repository,
            event_publisher,
            password_hasher,
            config,
        }
    }

    /// Register a new user
    #[instrument(skip(self, request), fields(email = %request.email))]
    pub async fn register(&self, request: CreateUserRequest) -> ApplicationResult<UserDto> {
        // Validate request
        let validation = request.validate_all();
        validation.ensure_valid()?;

        // Check email uniqueness
        if self.repository.email_exists(&request.email).await? {
            return Err(ApplicationError::Conflict(
                "Email already registered".to_string(),
            ));
        }

        // Check username uniqueness
        if self.repository.username_exists(&request.username).await? {
            return Err(ApplicationError::Conflict(
                "Username already taken".to_string(),
            ));
        }

        // Hash password if provided
        let password_hash = if let Some(ref password) = request.password {
            Some(self.password_hasher.hash(password).await?)
        } else {
            None
        };

        // Create user
        let create_data = CreateUserData {
            email: request.email,
            username: request.username,
            display_name: request.display_name,
            password_hash,
            oauth_provider: request.oauth_provider,
            oauth_id: request.oauth_id,
        };

        let id = self.repository.create(&create_data).await?;

        info!(user_id = %id, "User registered");

        // Publish event
        self.event_publisher
            .publish(ServiceEvent::UserCreated { user_id: id.clone() })
            .await?;

        // Fetch and return created user
        self.repository
            .get_by_id(&id)
            .await?
            .ok_or_else(|| ApplicationError::Internal("Failed to fetch created user".to_string()))
    }

    /// Get user by ID
    #[instrument(skip(self, ctx), fields(correlation_id = %ctx.correlation_id))]
    pub async fn get_by_id(
        &self,
        ctx: &ServiceContext,
        id: &str,
    ) -> ApplicationResult<Option<UserDto>> {
        self.repository.get_by_id(id).await
    }

    /// Get user by email
    #[instrument(skip(self))]
    pub async fn get_by_email(&self, email: &str) -> ApplicationResult<Option<UserDto>> {
        self.repository.get_by_email(email).await
    }

    /// Get user by username
    #[instrument(skip(self))]
    pub async fn get_by_username(&self, username: &str) -> ApplicationResult<Option<UserDto>> {
        self.repository.get_by_username(username).await
    }

    /// Get public user profile
    #[instrument(skip(self, ctx), fields(correlation_id = %ctx.correlation_id))]
    pub async fn get_profile(
        &self,
        ctx: &ServiceContext,
        id: &str,
    ) -> ApplicationResult<Option<UserProfileDto>> {
        self.repository.get_profile(id).await
    }

    /// Update user profile
    #[instrument(skip(self, ctx, request), fields(correlation_id = %ctx.correlation_id))]
    pub async fn update(
        &self,
        ctx: &ServiceContext,
        id: &str,
        request: UpdateUserRequest,
    ) -> ApplicationResult<UserDto> {
        // Validate request
        let validation = request.validate_all();
        validation.ensure_valid()?;

        // Check authorization
        let user_id = ctx.require_authenticated()?;
        if user_id != id && !ctx.is_admin {
            return Err(ApplicationError::Forbidden(
                "You can only update your own profile".to_string(),
            ));
        }

        // Check user exists
        self.repository
            .get_by_id(id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound(format!("User not found: {}", id)))?;

        // Update user
        let update_data = UpdateUserData {
            display_name: request.display_name,
            bio: request.bio,
            website: request.website,
            avatar_url: request.avatar_url,
        };

        self.repository.update(id, &update_data).await?;

        info!(user_id = %id, "User updated");

        // Publish event
        self.event_publisher
            .publish(ServiceEvent::UserUpdated {
                user_id: id.to_string(),
            })
            .await?;

        // Fetch and return updated user
        self.repository
            .get_by_id(id)
            .await?
            .ok_or_else(|| ApplicationError::Internal("Failed to fetch updated user".to_string()))
    }

    /// Change user password
    #[instrument(skip(self, ctx, request), fields(correlation_id = %ctx.correlation_id))]
    pub async fn change_password(
        &self,
        ctx: &ServiceContext,
        id: &str,
        request: ChangePasswordRequest,
    ) -> ApplicationResult<()> {
        // Validate request
        let validation = request.validate_all();
        validation.ensure_valid()?;

        // Check authorization
        let user_id = ctx.require_authenticated()?;
        if user_id != id {
            return Err(ApplicationError::Forbidden(
                "You can only change your own password".to_string(),
            ));
        }

        // Verify current password
        let is_valid = self
            .repository
            .verify_password(id, &request.current_password)
            .await?;

        if !is_valid {
            return Err(ApplicationError::Unauthorized(
                "Current password is incorrect".to_string(),
            ));
        }

        // Hash new password
        let new_hash = self.password_hasher.hash(&request.new_password).await?;

        // Update password
        self.repository.update_password(id, &new_hash).await?;

        info!(user_id = %id, "User password changed");

        // Publish event
        self.event_publisher
            .publish(ServiceEvent::UserPasswordChanged {
                user_id: id.to_string(),
            })
            .await?;

        Ok(())
    }

    /// Authenticate user with password
    #[instrument(skip(self, password))]
    pub async fn authenticate(
        &self,
        email: &str,
        password: &str,
    ) -> ApplicationResult<UserDto> {
        // Get user by email
        let user = self
            .repository
            .get_by_email(email)
            .await?
            .ok_or_else(|| ApplicationError::Unauthorized("Invalid credentials".to_string()))?;

        // Verify password
        let is_valid = self.repository.verify_password(&user.id, password).await?;

        if !is_valid {
            return Err(ApplicationError::Unauthorized(
                "Invalid credentials".to_string(),
            ));
        }

        debug!(user_id = %user.id, "User authenticated");

        Ok(user)
    }

    /// Create an API key for a user
    #[instrument(skip(self, ctx, request), fields(correlation_id = %ctx.correlation_id))]
    pub async fn create_api_key(
        &self,
        ctx: &ServiceContext,
        request: CreateApiKeyRequest,
    ) -> ApplicationResult<ApiKeyWithSecretDto> {
        // Validate request
        let validation = request.validate_all();
        validation.ensure_valid()?;

        // Get authenticated user
        let user_id = ctx.require_authenticated()?;

        // Create API key
        let key_data = CreateApiKeyData {
            name: request.name,
            description: request.description,
            scopes: request.scopes,
            expires_in_days: request.expires_in_days,
        };

        let key = self.repository.create_api_key(user_id, &key_data).await?;

        info!(user_id = %user_id, key_id = %key.key.id, "API key created");

        Ok(key)
    }

    /// List API keys for a user
    #[instrument(skip(self, ctx), fields(correlation_id = %ctx.correlation_id))]
    pub async fn list_api_keys(&self, ctx: &ServiceContext) -> ApplicationResult<Vec<ApiKeyDto>> {
        let user_id = ctx.require_authenticated()?;
        self.repository.list_api_keys(user_id).await
    }

    /// Revoke an API key
    #[instrument(skip(self, ctx), fields(correlation_id = %ctx.correlation_id))]
    pub async fn revoke_api_key(
        &self,
        ctx: &ServiceContext,
        key_id: &str,
    ) -> ApplicationResult<()> {
        let user_id = ctx.require_authenticated()?;
        self.repository.revoke_api_key(user_id, key_id).await?;

        info!(user_id = %user_id, key_id = %key_id, "API key revoked");

        Ok(())
    }

    /// Verify an API key and return user ID and scopes
    #[instrument(skip(self, key_secret))]
    pub async fn verify_api_key(
        &self,
        key_secret: &str,
    ) -> ApplicationResult<Option<(String, Vec<String>)>> {
        self.repository.verify_api_key(key_secret).await
    }

    /// Delete a user account
    #[instrument(skip(self, ctx), fields(correlation_id = %ctx.correlation_id))]
    pub async fn delete(&self, ctx: &ServiceContext, id: &str) -> ApplicationResult<()> {
        // Check authorization
        let user_id = ctx.require_authenticated()?;
        if user_id != id && !ctx.is_admin {
            return Err(ApplicationError::Forbidden(
                "You can only delete your own account".to_string(),
            ));
        }

        // Check user exists
        self.repository
            .get_by_id(id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound(format!("User not found: {}", id)))?;

        // Delete user
        self.repository.delete(id).await?;

        info!(user_id = %id, "User deleted");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would go here with mock implementations
}
