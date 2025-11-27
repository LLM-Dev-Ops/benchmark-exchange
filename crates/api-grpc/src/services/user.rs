//! User service implementation

use crate::conversions::datetime_to_timestamp;
use crate::proto::{
    user_service_server::UserService, GetProfileRequest, GetProfileResponse, LoginRequest,
    LoginResponse, RegisterRequest, RegisterResponse, UpdateProfileRequest, UpdateProfileResponse,
};
use tonic::{Request, Response, Status};
use tracing::{info, warn};

/// User service implementation
#[derive(Debug, Clone)]
pub struct UserServiceImpl {
    // TODO: Add application service dependencies
}

impl UserServiceImpl {
    /// Create a new user service
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for UserServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[tonic::async_trait]
impl UserService for UserServiceImpl {
    async fn register(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        let req = request.into_inner();
        info!("Registering new user: {}", req.username);

        // TODO: Call application service to register user
        // Validate email format
        // Check username uniqueness
        // Hash password
        // Create user account
        // Generate JWT tokens
        // Send verification email

        Err(Status::unimplemented("Register not yet implemented"))
    }

    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        let req = request.into_inner();
        info!("Login attempt for: {}", req.email_or_username);

        // TODO: Call application service to login
        // Find user by email or username
        // Verify password
        // Generate JWT tokens
        // Update last_active_at
        // Return user profile and tokens

        Err(Status::unauthenticated("Invalid credentials"))
    }

    async fn get_profile(
        &self,
        request: Request<GetProfileRequest>,
    ) -> Result<Response<GetProfileResponse>, Status> {
        let req = request.into_inner();
        let user_id = req.user_id.unwrap_or_else(|| {
            // Get from auth context
            "current-user".to_string()
        });

        info!("Getting profile for user: {}", user_id);

        // TODO: Call application service to get user profile
        // If user_id is provided, get that user (public profile)
        // Otherwise get current authenticated user (full profile)

        Err(Status::not_found("User not found"))
    }

    async fn update_profile(
        &self,
        request: Request<UpdateProfileRequest>,
    ) -> Result<Response<UpdateProfileResponse>, Status> {
        let req = request.into_inner();
        info!("Updating user profile");

        // TODO: Call application service to update profile
        // Get current user from auth context
        // Update profile fields
        // Validate data
        // Save changes

        Err(Status::unimplemented("Update profile not yet implemented"))
    }
}
