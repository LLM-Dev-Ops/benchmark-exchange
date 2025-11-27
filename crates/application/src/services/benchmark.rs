//! Benchmark Service
//!
//! Business logic for benchmark management including CRUD operations,
//! versioning, and status transitions.

use super::{
    Authorizer, EventPublisher, PaginatedResult, Pagination, ServiceConfig, ServiceContext,
    ServiceEvent,
};
use crate::validation::{
    CreateBenchmarkRequest, CreateVersionRequest, StatusTransitionRequest, UpdateBenchmarkRequest,
    Validatable,
};
use crate::{ApplicationError, ApplicationResult};
use async_trait::async_trait;
use llm_benchmark_domain::benchmark::{BenchmarkCategory, BenchmarkMetadata, BenchmarkStatus};
use llm_benchmark_domain::identifiers::{BenchmarkId, BenchmarkVersionId, UserId};
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

/// Benchmark data transfer object
#[derive(Debug, Clone)]
pub struct BenchmarkDto {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub category: BenchmarkCategory,
    pub status: BenchmarkStatus,
    pub tags: Vec<String>,
    pub current_version: Option<String>,
    pub submission_count: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Benchmark version data transfer object
#[derive(Debug, Clone)]
pub struct BenchmarkVersionDto {
    pub id: String,
    pub benchmark_id: String,
    pub version: String,
    pub changelog: String,
    pub breaking_changes: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Benchmark query filters
#[derive(Debug, Clone, Default)]
pub struct BenchmarkFilters {
    pub category: Option<BenchmarkCategory>,
    pub status: Option<BenchmarkStatus>,
    pub tags: Option<Vec<String>>,
    pub search: Option<String>,
    pub maintainer_id: Option<String>,
}

/// Benchmark repository trait (to be implemented by infrastructure)
#[async_trait]
pub trait BenchmarkRepositoryPort: Send + Sync {
    async fn create(&self, benchmark: &CreateBenchmarkData) -> Result<String, ApplicationError>;
    async fn get_by_id(&self, id: &str) -> Result<Option<BenchmarkDto>, ApplicationError>;
    async fn get_by_slug(&self, slug: &str) -> Result<Option<BenchmarkDto>, ApplicationError>;
    async fn list(
        &self,
        filters: &BenchmarkFilters,
        pagination: &Pagination,
    ) -> Result<(Vec<BenchmarkDto>, u64), ApplicationError>;
    async fn update(&self, id: &str, update: &UpdateBenchmarkData) -> Result<(), ApplicationError>;
    async fn update_status(&self, id: &str, status: BenchmarkStatus) -> Result<(), ApplicationError>;
    async fn delete(&self, id: &str) -> Result<(), ApplicationError>;
    async fn slug_exists(&self, slug: &str) -> Result<bool, ApplicationError>;
    async fn create_version(&self, version: &CreateVersionData) -> Result<String, ApplicationError>;
    async fn get_versions(&self, benchmark_id: &str) -> Result<Vec<BenchmarkVersionDto>, ApplicationError>;
}

/// Data for creating a benchmark
#[derive(Debug, Clone)]
pub struct CreateBenchmarkData {
    pub name: String,
    pub slug: String,
    pub description: String,
    pub category: BenchmarkCategory,
    pub tags: Vec<String>,
    pub version: String,
    pub creator_id: String,
}

/// Data for updating a benchmark
#[derive(Debug, Clone)]
pub struct UpdateBenchmarkData {
    pub name: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub long_description: Option<String>,
}

/// Data for creating a version
#[derive(Debug, Clone)]
pub struct CreateVersionData {
    pub benchmark_id: String,
    pub version: String,
    pub changelog: String,
    pub breaking_changes: bool,
    pub migration_notes: Option<String>,
    pub creator_id: String,
}

/// Benchmark service implementation
pub struct BenchmarkService<R, A, E>
where
    R: BenchmarkRepositoryPort,
    A: Authorizer,
    E: EventPublisher,
{
    repository: Arc<R>,
    authorizer: Arc<A>,
    event_publisher: Arc<E>,
    config: ServiceConfig,
}

impl<R, A, E> BenchmarkService<R, A, E>
where
    R: BenchmarkRepositoryPort,
    A: Authorizer,
    E: EventPublisher,
{
    pub fn new(
        repository: Arc<R>,
        authorizer: Arc<A>,
        event_publisher: Arc<E>,
        config: ServiceConfig,
    ) -> Self {
        Self {
            repository,
            authorizer,
            event_publisher,
            config,
        }
    }

    /// Create a new benchmark
    #[instrument(skip(self, ctx, request), fields(correlation_id = %ctx.correlation_id))]
    pub async fn create(
        &self,
        ctx: &ServiceContext,
        request: CreateBenchmarkRequest,
    ) -> ApplicationResult<BenchmarkDto> {
        // Validate request
        let validation = request.validate_all();
        validation.ensure_valid()?;

        // Check authorization
        let auth = self.authorizer.can_create_benchmark(ctx).await;
        auth.ensure_allowed()?;

        // Get authenticated user
        let user_id = ctx.require_authenticated()?;

        // Check slug uniqueness
        if self.repository.slug_exists(&request.slug).await? {
            return Err(ApplicationError::Conflict(format!(
                "Benchmark with slug '{}' already exists",
                request.slug
            )));
        }

        // Create benchmark
        let create_data = CreateBenchmarkData {
            name: request.name,
            slug: request.slug,
            description: request.description,
            category: request.category,
            tags: request.tags,
            version: request.version,
            creator_id: user_id.to_string(),
        };

        let id = self.repository.create(&create_data).await?;

        info!(benchmark_id = %id, "Benchmark created");

        // Publish event
        self.event_publisher
            .publish(ServiceEvent::BenchmarkCreated {
                benchmark_id: id.clone(),
            })
            .await?;

        // Fetch and return the created benchmark
        self.repository
            .get_by_id(&id)
            .await?
            .ok_or_else(|| ApplicationError::Internal("Failed to fetch created benchmark".to_string()))
    }

    /// Get a benchmark by ID
    #[instrument(skip(self, ctx), fields(correlation_id = %ctx.correlation_id))]
    pub async fn get_by_id(
        &self,
        ctx: &ServiceContext,
        id: &str,
    ) -> ApplicationResult<Option<BenchmarkDto>> {
        debug!(benchmark_id = %id, "Fetching benchmark");
        self.repository.get_by_id(id).await
    }

    /// Get a benchmark by slug
    #[instrument(skip(self, ctx), fields(correlation_id = %ctx.correlation_id))]
    pub async fn get_by_slug(
        &self,
        ctx: &ServiceContext,
        slug: &str,
    ) -> ApplicationResult<Option<BenchmarkDto>> {
        debug!(slug = %slug, "Fetching benchmark by slug");
        self.repository.get_by_slug(slug).await
    }

    /// List benchmarks with filters and pagination
    #[instrument(skip(self, ctx), fields(correlation_id = %ctx.correlation_id))]
    pub async fn list(
        &self,
        ctx: &ServiceContext,
        filters: BenchmarkFilters,
        pagination: Pagination,
    ) -> ApplicationResult<PaginatedResult<BenchmarkDto>> {
        // Clamp page size
        let pagination = Pagination::new(
            pagination.page.max(1),
            pagination.page_size.min(self.config.max_page_size),
        );

        let (items, total) = self.repository.list(&filters, &pagination).await?;
        Ok(PaginatedResult::new(items, total, &pagination))
    }

    /// Update a benchmark
    #[instrument(skip(self, ctx, request), fields(correlation_id = %ctx.correlation_id))]
    pub async fn update(
        &self,
        ctx: &ServiceContext,
        id: &str,
        request: UpdateBenchmarkRequest,
    ) -> ApplicationResult<BenchmarkDto> {
        // Validate request
        let validation = request.validate_all();
        validation.ensure_valid()?;

        // Check authorization
        let auth = self.authorizer.can_update_benchmark(ctx, id).await;
        auth.ensure_allowed()?;

        // Check benchmark exists
        let existing = self
            .repository
            .get_by_id(id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound(format!("Benchmark not found: {}", id)))?;

        // Update benchmark
        let update_data = UpdateBenchmarkData {
            name: request.name,
            description: request.description,
            tags: request.tags,
            long_description: request.long_description,
        };

        self.repository.update(id, &update_data).await?;

        info!(benchmark_id = %id, "Benchmark updated");

        // Publish event
        self.event_publisher
            .publish(ServiceEvent::BenchmarkUpdated {
                benchmark_id: id.to_string(),
            })
            .await?;

        // Fetch and return updated benchmark
        self.repository
            .get_by_id(id)
            .await?
            .ok_or_else(|| ApplicationError::Internal("Failed to fetch updated benchmark".to_string()))
    }

    /// Transition benchmark status
    #[instrument(skip(self, ctx, request), fields(correlation_id = %ctx.correlation_id))]
    pub async fn transition_status(
        &self,
        ctx: &ServiceContext,
        id: &str,
        request: StatusTransitionRequest,
    ) -> ApplicationResult<BenchmarkDto> {
        // Validate request
        let validation = request.validate_all();
        validation.ensure_valid()?;

        // Check authorization
        let auth = self.authorizer.can_update_benchmark(ctx, id).await;
        auth.ensure_allowed()?;

        // Check benchmark exists and current status matches
        let existing = self
            .repository
            .get_by_id(id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound(format!("Benchmark not found: {}", id)))?;

        if existing.status != request.current_status {
            return Err(ApplicationError::Conflict(format!(
                "Benchmark status has changed. Expected {:?}, got {:?}",
                request.current_status, existing.status
            )));
        }

        // Update status
        self.repository.update_status(id, request.target_status).await?;

        info!(
            benchmark_id = %id,
            from = ?request.current_status,
            to = ?request.target_status,
            "Benchmark status transitioned"
        );

        // Publish event
        self.event_publisher
            .publish(ServiceEvent::BenchmarkStatusChanged {
                benchmark_id: id.to_string(),
                new_status: format!("{:?}", request.target_status),
            })
            .await?;

        // Fetch and return updated benchmark
        self.repository
            .get_by_id(id)
            .await?
            .ok_or_else(|| ApplicationError::Internal("Failed to fetch updated benchmark".to_string()))
    }

    /// Create a new version for a benchmark
    #[instrument(skip(self, ctx, request), fields(correlation_id = %ctx.correlation_id))]
    pub async fn create_version(
        &self,
        ctx: &ServiceContext,
        benchmark_id: &str,
        request: CreateVersionRequest,
    ) -> ApplicationResult<BenchmarkVersionDto> {
        // Validate request
        let validation = request.validate_all();
        validation.ensure_valid()?;

        // Check authorization
        let auth = self.authorizer.can_update_benchmark(ctx, benchmark_id).await;
        auth.ensure_allowed()?;

        // Get authenticated user
        let user_id = ctx.require_authenticated()?;

        // Check benchmark exists
        let existing = self
            .repository
            .get_by_id(benchmark_id)
            .await?
            .ok_or_else(|| {
                ApplicationError::NotFound(format!("Benchmark not found: {}", benchmark_id))
            })?;

        // Create version
        let version_data = CreateVersionData {
            benchmark_id: benchmark_id.to_string(),
            version: request.version,
            changelog: request.changelog,
            breaking_changes: request.breaking_changes,
            migration_notes: request.migration_notes,
            creator_id: user_id.to_string(),
        };

        let version_id = self.repository.create_version(&version_data).await?;

        info!(
            benchmark_id = %benchmark_id,
            version_id = %version_id,
            "Benchmark version created"
        );

        // Publish event
        self.event_publisher
            .publish(ServiceEvent::BenchmarkVersionCreated {
                benchmark_id: benchmark_id.to_string(),
                version_id: version_id.clone(),
            })
            .await?;

        // Get versions and return the new one
        let versions = self.repository.get_versions(benchmark_id).await?;
        versions
            .into_iter()
            .find(|v| v.id == version_id)
            .ok_or_else(|| ApplicationError::Internal("Failed to fetch created version".to_string()))
    }

    /// Get all versions for a benchmark
    #[instrument(skip(self, ctx), fields(correlation_id = %ctx.correlation_id))]
    pub async fn get_versions(
        &self,
        ctx: &ServiceContext,
        benchmark_id: &str,
    ) -> ApplicationResult<Vec<BenchmarkVersionDto>> {
        // Check benchmark exists
        self.repository
            .get_by_id(benchmark_id)
            .await?
            .ok_or_else(|| {
                ApplicationError::NotFound(format!("Benchmark not found: {}", benchmark_id))
            })?;

        self.repository.get_versions(benchmark_id).await
    }

    /// Delete a benchmark (admin only)
    #[instrument(skip(self, ctx), fields(correlation_id = %ctx.correlation_id))]
    pub async fn delete(&self, ctx: &ServiceContext, id: &str) -> ApplicationResult<()> {
        // Check authorization
        let auth = self.authorizer.can_delete_benchmark(ctx, id).await;
        auth.ensure_allowed()?;

        // Check benchmark exists
        self.repository
            .get_by_id(id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound(format!("Benchmark not found: {}", id)))?;

        // Delete benchmark
        self.repository.delete(id).await?;

        info!(benchmark_id = %id, "Benchmark deleted");

        Ok(())
    }

    /// Search benchmarks by text query
    #[instrument(skip(self, ctx), fields(correlation_id = %ctx.correlation_id))]
    pub async fn search(
        &self,
        ctx: &ServiceContext,
        query: &str,
        pagination: Pagination,
    ) -> ApplicationResult<PaginatedResult<BenchmarkDto>> {
        let filters = BenchmarkFilters {
            search: Some(query.to_string()),
            ..Default::default()
        };

        self.list(ctx, filters, pagination).await
    }

    /// Get benchmarks by category
    #[instrument(skip(self, ctx), fields(correlation_id = %ctx.correlation_id))]
    pub async fn get_by_category(
        &self,
        ctx: &ServiceContext,
        category: BenchmarkCategory,
        pagination: Pagination,
    ) -> ApplicationResult<PaginatedResult<BenchmarkDto>> {
        let filters = BenchmarkFilters {
            category: Some(category),
            status: Some(BenchmarkStatus::Active),
            ..Default::default()
        };

        self.list(ctx, filters, pagination).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would go here with mock implementations
}
