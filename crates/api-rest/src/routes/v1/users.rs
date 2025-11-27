//! User and authentication endpoints.

use crate::{
    error::{ApiError, ApiResult},
    extractors::{AuthenticatedUser, ValidatedJson},
    responses::{ApiResponse, Created, NoContent},
    state::AppState,
};
use axum::{
    extract::{Path, State},
    routing::{get, patch, post, put},
    Json, Router,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use llm_benchmark_domain::{identifiers::UserId, user::UserRole};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// User registration request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RegisterRequest {
    #[validate(email)]
    pub email: String,

    #[validate(length(min = 3, max = 50))]
    pub username: String,

    #[validate(length(min = 8, max = 100))]
    pub password: String,
}

/// User login request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,

    #[validate(length(min = 1))]
    pub password: String,
}

/// Authentication response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
    pub expires_at: String,
}

/// User response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserResponse {
    pub id: UserId,
    pub email: String,
    pub username: String,
    pub display_name: Option<String>,
    pub role: UserRole,
    pub created_at: String,
    pub email_verified: bool,
}

/// Update user profile request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateProfileRequest {
    #[validate(length(max = 100))]
    pub display_name: Option<String>,

    #[validate(length(max = 500))]
    pub bio: Option<String>,

    #[validate(length(max = 200))]
    pub affiliation: Option<String>,

    #[validate(url)]
    pub website: Option<String>,
}

/// Update user role request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateRoleRequest {
    pub role: UserRole,

    #[validate(length(max = 500))]
    pub reason: Option<String>,
}

/// User routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/users/me", get(get_current_user).put(update_profile))
        .route("/users/:id", get(get_user))
        .route("/users/:id/role", patch(update_user_role))
}

/// Register new user
///
/// Create a new user account.
#[utoipa::path(
    post,
    path = "/auth/register",
    tag = "users",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered", body = AuthResponse),
        (status = 400, description = "Invalid request"),
        (status = 409, description = "User already exists"),
    )
)]
async fn register(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<RegisterRequest>,
) -> ApiResult<Created<AuthResponse>> {
    // In production: Check if user exists, hash password, create user in database
    let user_id = UserId::new();
    let now = Utc::now();

    // Create JWT token
    let claims = crate::extractors::auth::Claims {
        sub: user_id.to_string(),
        role: UserRole::Registered,
        exp: (now + Duration::hours(24)).timestamp() as usize,
        iat: now.timestamp() as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret().as_bytes()),
    )
    .map_err(|e| ApiError::Internal(format!("Failed to create token: {}", e)))?;

    let user = UserResponse {
        id: user_id,
        email: req.email,
        username: req.username,
        display_name: None,
        role: UserRole::Registered,
        created_at: now.to_rfc3339(),
        email_verified: false,
    };

    let response = AuthResponse {
        token,
        user,
        expires_at: (now + Duration::hours(24)).to_rfc3339(),
    };

    Ok(Created(response))
}

/// Login
///
/// Authenticate and receive a JWT token.
#[utoipa::path(
    post,
    path = "/auth/login",
    tag = "users",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = AuthResponse),
        (status = 401, description = "Invalid credentials"),
    )
)]
async fn login(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<LoginRequest>,
) -> ApiResult<Json<ApiResponse<AuthResponse>>> {
    // In production: Verify credentials against database
    // For now, return a mock response
    let user_id = UserId::new();
    let now = Utc::now();

    let claims = crate::extractors::auth::Claims {
        sub: user_id.to_string(),
        role: UserRole::Registered,
        exp: (now + Duration::hours(24)).timestamp() as usize,
        iat: now.timestamp() as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret().as_bytes()),
    )
    .map_err(|e| ApiError::Internal(format!("Failed to create token: {}", e)))?;

    let user = UserResponse {
        id: user_id,
        email: req.email,
        username: "testuser".to_string(),
        display_name: None,
        role: UserRole::Registered,
        created_at: now.to_rfc3339(),
        email_verified: true,
    };

    let response = AuthResponse {
        token,
        user,
        expires_at: (now + Duration::hours(24)).to_rfc3339(),
    };

    Ok(Json(ApiResponse::success(response)))
}

/// Get current user
///
/// Retrieve information about the currently authenticated user.
#[utoipa::path(
    get,
    path = "/users/me",
    tag = "users",
    responses(
        (status = 200, description = "Current user", body = UserResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
async fn get_current_user(
    user: AuthenticatedUser,
) -> ApiResult<Json<ApiResponse<UserResponse>>> {
    // In production: Fetch full user details from database
    let user_response = UserResponse {
        id: user.user_id,
        email: "user@example.com".to_string(),
        username: "user".to_string(),
        display_name: None,
        role: user.role,
        created_at: Utc::now().to_rfc3339(),
        email_verified: true,
    };

    Ok(Json(ApiResponse::success(user_response)))
}

/// Update profile
///
/// Update the current user's profile information.
#[utoipa::path(
    put,
    path = "/users/me",
    tag = "users",
    request_body = UpdateProfileRequest,
    responses(
        (status = 200, description = "Profile updated", body = UserResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
async fn update_profile(
    user: AuthenticatedUser,
    ValidatedJson(_req): ValidatedJson<UpdateProfileRequest>,
) -> ApiResult<Json<ApiResponse<UserResponse>>> {
    // In production: Update user profile in database
    let user_response = UserResponse {
        id: user.user_id,
        email: "user@example.com".to_string(),
        username: "user".to_string(),
        display_name: None,
        role: user.role,
        created_at: Utc::now().to_rfc3339(),
        email_verified: true,
    };

    Ok(Json(ApiResponse::success(user_response)))
}

/// Get user by ID
///
/// Retrieve public information about a specific user.
#[utoipa::path(
    get,
    path = "/users/{id}",
    tag = "users",
    params(
        ("id" = Uuid, Path, description = "User ID"),
    ),
    responses(
        (status = 200, description = "User information", body = UserResponse),
        (status = 404, description = "User not found"),
    )
)]
async fn get_user(
    State(_state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<UserResponse>>> {
    let _user_id = UserId::from(id);

    // In production: Query database
    Err(ApiError::NotFound)
}

/// Update user role
///
/// Update a user's role. Requires admin privileges.
#[utoipa::path(
    patch,
    path = "/users/{id}/role",
    tag = "users",
    params(
        ("id" = Uuid, Path, description = "User ID"),
    ),
    request_body = UpdateRoleRequest,
    responses(
        (status = 200, description = "Role updated"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "User not found"),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
async fn update_user_role(
    State(_state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
    ValidatedJson(_req): ValidatedJson<UpdateRoleRequest>,
) -> ApiResult<NoContent> {
    if !user.is_admin() {
        return Err(ApiError::BadRequest(
            "Insufficient permissions to update user roles".to_string(),
        ));
    }

    let _target_user_id = UserId::from(id);

    // In production: Update user role in database
    Err(ApiError::NotFound)
}
