//! Repository implementations for data persistence.
//!
//! This module provides PostgreSQL-backed implementations of repository traits
//! defined in the domain layer.

mod benchmark_repository;
mod submission_repository;
mod user_repository;
mod organization_repository;

pub use benchmark_repository::*;
pub use submission_repository::*;
pub use user_repository::*;
pub use organization_repository::*;

use async_trait::async_trait;
use llm_benchmark_common::pagination::{PaginatedResult, PaginationParams, SortParams};

/// Common repository trait for CRUD operations.
#[async_trait]
pub trait Repository<T, ID> {
    /// Create a new entity.
    async fn create(&self, entity: &T) -> crate::Result<ID>;

    /// Find an entity by its ID.
    async fn find_by_id(&self, id: ID) -> crate::Result<Option<T>>;

    /// Update an existing entity.
    async fn update(&self, entity: &T) -> crate::Result<()>;

    /// Delete an entity by its ID.
    async fn delete(&self, id: ID) -> crate::Result<bool>;

    /// Check if an entity exists.
    async fn exists(&self, id: ID) -> crate::Result<bool>;
}
