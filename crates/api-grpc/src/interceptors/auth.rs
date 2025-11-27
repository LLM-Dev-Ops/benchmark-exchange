//! Authentication interceptor for gRPC requests

use tonic::{Request, Status};
use tracing::{debug, warn};

/// Authentication interceptor
#[derive(Clone)]
pub struct AuthInterceptor {
    // TODO: Add JWT validator, token store, etc.
}

impl AuthInterceptor {
    /// Create a new authentication interceptor
    pub fn new() -> Self {
        Self {}
    }

    /// Intercept and validate authentication
    pub fn intercept<T>(&self, req: Request<T>) -> Result<Request<T>, Status> {
        // Extract authorization header
        let token = match req.metadata().get("authorization") {
            Some(t) => match t.to_str() {
                Ok(token_str) => {
                    if token_str.starts_with("Bearer ") {
                        Some(token_str[7..].to_string())
                    } else {
                        None
                    }
                }
                Err(_) => None,
            },
            None => None,
        };

        // Check for x-grpc-method metadata to determine endpoint
        let method = req
            .metadata()
            .get("x-grpc-method")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        // Public endpoints that don't require authentication
        let is_public_endpoint = method.contains("Register")
            || method.contains("Login")
            || method.contains("GetLeaderboard")
            || method.contains("ListBenchmarks")
            || method.is_empty(); // Allow if method metadata not set

        if token.is_none() && !is_public_endpoint {
            warn!("Unauthenticated request to protected endpoint");
            return Err(Status::unauthenticated("Missing authentication token"));
        }

        // TODO: Validate JWT token
        // For now, we'll just pass through
        if let Some(_token_value) = token {
            debug!("Authenticated request with token");
            // Store user context in request extensions for downstream use
            // req.extensions_mut().insert(UserContext { user_id, role });
        }

        Ok(req)
    }
}

impl Default for AuthInterceptor {
    fn default() -> Self {
        Self::new()
    }
}

/// User context extracted from authentication
#[derive(Debug, Clone)]
pub struct UserContext {
    pub user_id: String,
    pub role: String,
}
