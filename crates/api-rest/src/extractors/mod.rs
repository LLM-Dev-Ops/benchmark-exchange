//! Custom Axum extractors.
//!
//! This module provides reusable extractors for common patterns
//! like authentication, pagination, and validated JSON payloads.

pub mod auth;
pub mod pagination;
pub mod validated_json;

pub use auth::AuthenticatedUser;
pub use pagination::Pagination;
pub use validated_json::ValidatedJson;
