//! Authentication extractor.

use crate::{error::ApiError, state::AppState};
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header, request::Parts},
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use llm_benchmark_domain::{identifiers::UserId, user::UserRole};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Claims stored in JWT token
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,

    /// User role
    pub role: UserRole,

    /// Expiration time (as UTC timestamp)
    pub exp: usize,

    /// Issued at (as UTC timestamp)
    pub iat: usize,
}

impl Claims {
    /// Get user ID from claims
    pub fn user_id(&self) -> Result<UserId, ApiError> {
        Uuid::parse_str(&self.sub)
            .map(UserId::from)
            .map_err(|_| ApiError::InvalidToken("Invalid user ID in token".to_string()))
    }
}

/// Authenticated user information extracted from JWT
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    /// User ID
    pub user_id: UserId,

    /// User role
    pub role: UserRole,

    /// Original claims
    pub claims: Claims,
}

impl AuthenticatedUser {
    /// Check if user has required role
    pub fn has_role(&self, required_role: UserRole) -> bool {
        self.role >= required_role
    }

    /// Check if user can submit results
    pub fn can_submit_results(&self) -> bool {
        self.role.can_submit_results()
    }

    /// Check if user can propose benchmarks
    pub fn can_propose_benchmarks(&self) -> bool {
        self.role.can_propose_benchmarks()
    }

    /// Check if user can vote
    pub fn can_vote(&self) -> bool {
        self.role.can_vote()
    }

    /// Check if user can review
    pub fn can_review(&self) -> bool {
        self.role.can_review()
    }

    /// Check if user is admin
    pub fn is_admin(&self) -> bool {
        self.role.can_manage_users()
    }
}

#[async_trait]
impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Extract Authorization header
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or(ApiError::Unauthorized)?;

        // Extract token from "Bearer <token>"
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| ApiError::InvalidToken("Invalid authorization header format".to_string()))?;

        // Decode and validate token
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(state.jwt_secret().as_bytes()),
            &Validation::default(),
        )
        .map_err(|e| ApiError::InvalidToken(format!("Token validation failed: {}", e)))?;

        let claims = token_data.claims;
        let user_id = claims.user_id()?;

        Ok(Self {
            user_id,
            role: claims.role,
            claims,
        })
    }
}

/// Optional authenticated user (allows anonymous access)
#[derive(Debug, Clone)]
pub struct MaybeAuthenticatedUser(pub Option<AuthenticatedUser>);

impl MaybeAuthenticatedUser {
    /// Get the user if authenticated
    pub fn user(&self) -> Option<&AuthenticatedUser> {
        self.0.as_ref()
    }

    /// Check if user is authenticated
    pub fn is_authenticated(&self) -> bool {
        self.0.is_some()
    }
}

#[async_trait]
impl FromRequestParts<AppState> for MaybeAuthenticatedUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        match AuthenticatedUser::from_request_parts(parts, state).await {
            Ok(user) => Ok(Self(Some(user))),
            Err(_) => Ok(Self(None)),
        }
    }
}
