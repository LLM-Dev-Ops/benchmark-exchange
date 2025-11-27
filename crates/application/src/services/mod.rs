//! Application Services
//!
//! Business logic orchestration layer that coordinates domain operations,
//! repository access, and cross-cutting concerns.

mod benchmark;
mod organization;
mod submission;
mod user;

pub use benchmark::*;
pub use organization::*;
pub use submission::*;
pub use user::*;

use crate::ApplicationError;
use async_trait::async_trait;
use std::sync::Arc;

/// Service configuration
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// Maximum page size for list operations
    pub max_page_size: u32,
    /// Default page size for list operations
    pub default_page_size: u32,
    /// Enable caching
    pub cache_enabled: bool,
    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            max_page_size: 100,
            default_page_size: 20,
            cache_enabled: true,
            cache_ttl_seconds: 300,
        }
    }
}

/// Pagination parameters for list operations
#[derive(Debug, Clone)]
pub struct Pagination {
    pub page: u32,
    pub page_size: u32,
}

impl Pagination {
    pub fn new(page: u32, page_size: u32) -> Self {
        Self { page, page_size }
    }

    pub fn offset(&self) -> u64 {
        ((self.page.saturating_sub(1)) * self.page_size) as u64
    }

    pub fn limit(&self) -> u32 {
        self.page_size
    }
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 20,
        }
    }
}

/// Paginated result
#[derive(Debug, Clone)]
pub struct PaginatedResult<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

impl<T> PaginatedResult<T> {
    pub fn new(items: Vec<T>, total: u64, pagination: &Pagination) -> Self {
        let total_pages = ((total as f64) / (pagination.page_size as f64)).ceil() as u32;
        Self {
            items,
            total,
            page: pagination.page,
            page_size: pagination.page_size,
            total_pages,
        }
    }

    pub fn empty(pagination: &Pagination) -> Self {
        Self {
            items: vec![],
            total: 0,
            page: pagination.page,
            page_size: pagination.page_size,
            total_pages: 0,
        }
    }

    pub fn has_next_page(&self) -> bool {
        self.page < self.total_pages
    }

    pub fn has_previous_page(&self) -> bool {
        self.page > 1
    }
}

/// Service context for request handling
#[derive(Debug, Clone)]
pub struct ServiceContext {
    /// The authenticated user ID (if any)
    pub user_id: Option<String>,
    /// Request correlation ID for tracing
    pub correlation_id: String,
    /// Organization context (if any)
    pub organization_id: Option<String>,
    /// Whether the user has admin privileges
    pub is_admin: bool,
}

impl ServiceContext {
    pub fn anonymous(correlation_id: String) -> Self {
        Self {
            user_id: None,
            correlation_id,
            organization_id: None,
            is_admin: false,
        }
    }

    pub fn authenticated(user_id: String, correlation_id: String) -> Self {
        Self {
            user_id: Some(user_id),
            correlation_id,
            organization_id: None,
            is_admin: false,
        }
    }

    pub fn with_organization(mut self, org_id: String) -> Self {
        self.organization_id = Some(org_id);
        self
    }

    pub fn with_admin(mut self) -> Self {
        self.is_admin = true;
        self
    }

    pub fn require_authenticated(&self) -> Result<&str, ApplicationError> {
        self.user_id
            .as_deref()
            .ok_or_else(|| ApplicationError::Unauthorized("Authentication required".to_string()))
    }

    pub fn require_admin(&self) -> Result<(), ApplicationError> {
        if !self.is_admin {
            return Err(ApplicationError::Forbidden(
                "Admin privileges required".to_string(),
            ));
        }
        Ok(())
    }
}

/// Service event for event-driven architecture
#[derive(Debug, Clone)]
pub enum ServiceEvent {
    // Benchmark events
    BenchmarkCreated { benchmark_id: String },
    BenchmarkUpdated { benchmark_id: String },
    BenchmarkStatusChanged { benchmark_id: String, new_status: String },
    BenchmarkVersionCreated { benchmark_id: String, version_id: String },

    // Submission events
    SubmissionCreated { submission_id: String },
    SubmissionVerified { submission_id: String, level: String },
    SubmissionScoreUpdated { submission_id: String },

    // User events
    UserCreated { user_id: String },
    UserUpdated { user_id: String },
    UserPasswordChanged { user_id: String },

    // Organization events
    OrganizationCreated { organization_id: String },
    OrganizationMemberAdded { organization_id: String, user_id: String },
    OrganizationMemberRemoved { organization_id: String, user_id: String },
}

/// Event publisher trait for service events
#[async_trait]
pub trait EventPublisher: Send + Sync {
    async fn publish(&self, event: ServiceEvent) -> Result<(), ApplicationError>;
}

/// No-op event publisher for testing
pub struct NoOpEventPublisher;

#[async_trait]
impl EventPublisher for NoOpEventPublisher {
    async fn publish(&self, _event: ServiceEvent) -> Result<(), ApplicationError> {
        Ok(())
    }
}

/// Authorization result
#[derive(Debug, Clone)]
pub struct AuthorizationResult {
    pub allowed: bool,
    pub reason: Option<String>,
}

impl AuthorizationResult {
    pub fn allow() -> Self {
        Self {
            allowed: true,
            reason: None,
        }
    }

    pub fn deny(reason: impl Into<String>) -> Self {
        Self {
            allowed: false,
            reason: Some(reason.into()),
        }
    }

    pub fn ensure_allowed(&self) -> Result<(), ApplicationError> {
        if self.allowed {
            Ok(())
        } else {
            Err(ApplicationError::Forbidden(
                self.reason
                    .clone()
                    .unwrap_or_else(|| "Access denied".to_string()),
            ))
        }
    }
}

/// Authorization trait for services
#[async_trait]
pub trait Authorizer: Send + Sync {
    async fn can_create_benchmark(&self, ctx: &ServiceContext) -> AuthorizationResult;
    async fn can_update_benchmark(&self, ctx: &ServiceContext, benchmark_id: &str) -> AuthorizationResult;
    async fn can_delete_benchmark(&self, ctx: &ServiceContext, benchmark_id: &str) -> AuthorizationResult;
    async fn can_create_submission(&self, ctx: &ServiceContext, benchmark_id: &str) -> AuthorizationResult;
    async fn can_verify_submission(&self, ctx: &ServiceContext, submission_id: &str) -> AuthorizationResult;
    async fn can_manage_organization(&self, ctx: &ServiceContext, org_id: &str) -> AuthorizationResult;
}

/// Default authorizer implementation
pub struct DefaultAuthorizer;

#[async_trait]
impl Authorizer for DefaultAuthorizer {
    async fn can_create_benchmark(&self, ctx: &ServiceContext) -> AuthorizationResult {
        if ctx.user_id.is_some() {
            AuthorizationResult::allow()
        } else {
            AuthorizationResult::deny("Authentication required to create benchmarks")
        }
    }

    async fn can_update_benchmark(&self, ctx: &ServiceContext, _benchmark_id: &str) -> AuthorizationResult {
        // In a real implementation, check if user is a maintainer
        if ctx.user_id.is_some() {
            AuthorizationResult::allow()
        } else {
            AuthorizationResult::deny("Authentication required to update benchmarks")
        }
    }

    async fn can_delete_benchmark(&self, ctx: &ServiceContext, _benchmark_id: &str) -> AuthorizationResult {
        if ctx.is_admin {
            AuthorizationResult::allow()
        } else {
            AuthorizationResult::deny("Admin privileges required to delete benchmarks")
        }
    }

    async fn can_create_submission(&self, ctx: &ServiceContext, _benchmark_id: &str) -> AuthorizationResult {
        if ctx.user_id.is_some() {
            AuthorizationResult::allow()
        } else {
            AuthorizationResult::deny("Authentication required to create submissions")
        }
    }

    async fn can_verify_submission(&self, ctx: &ServiceContext, _submission_id: &str) -> AuthorizationResult {
        // Platform verification requires special privileges
        if ctx.is_admin || ctx.user_id.is_some() {
            AuthorizationResult::allow()
        } else {
            AuthorizationResult::deny("Authentication required to verify submissions")
        }
    }

    async fn can_manage_organization(&self, ctx: &ServiceContext, _org_id: &str) -> AuthorizationResult {
        // In a real implementation, check if user is an admin/owner of the org
        if ctx.user_id.is_some() {
            AuthorizationResult::allow()
        } else {
            AuthorizationResult::deny("Authentication required to manage organizations")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination() {
        let pagination = Pagination::new(1, 20);
        assert_eq!(pagination.offset(), 0);
        assert_eq!(pagination.limit(), 20);

        let pagination = Pagination::new(3, 10);
        assert_eq!(pagination.offset(), 20);
        assert_eq!(pagination.limit(), 10);
    }

    #[test]
    fn test_paginated_result() {
        let items = vec![1, 2, 3];
        let pagination = Pagination::new(1, 10);
        let result = PaginatedResult::new(items, 25, &pagination);

        assert_eq!(result.total, 25);
        assert_eq!(result.page, 1);
        assert_eq!(result.total_pages, 3);
        assert!(result.has_next_page());
        assert!(!result.has_previous_page());
    }

    #[test]
    fn test_service_context() {
        let ctx = ServiceContext::anonymous("corr-123".to_string());
        assert!(ctx.user_id.is_none());
        assert!(ctx.require_authenticated().is_err());

        let ctx = ServiceContext::authenticated("user-123".to_string(), "corr-123".to_string());
        assert!(ctx.require_authenticated().is_ok());
        assert!(ctx.require_admin().is_err());

        let ctx = ctx.with_admin();
        assert!(ctx.require_admin().is_ok());
    }

    #[test]
    fn test_authorization_result() {
        let allowed = AuthorizationResult::allow();
        assert!(allowed.ensure_allowed().is_ok());

        let denied = AuthorizationResult::deny("Not allowed");
        assert!(denied.ensure_allowed().is_err());
    }
}
