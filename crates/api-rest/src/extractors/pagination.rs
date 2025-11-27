//! Pagination extractor.

use crate::error::ApiError;
use axum::{
    async_trait,
    extract::{FromRequestParts, Query},
    http::request::Parts,
};
use llm_benchmark_common::pagination::{PaginationParams, SortDirection, SortParams};
use serde::Deserialize;

/// Query parameters for pagination
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    pub page: u32,

    /// Items per page
    #[serde(default = "default_per_page")]
    pub per_page: u32,

    /// Sort field
    #[serde(default)]
    pub sort_by: Option<String>,

    /// Sort direction (asc/desc)
    #[serde(default)]
    pub sort_dir: Option<String>,
}

fn default_page() -> u32 {
    1
}

fn default_per_page() -> u32 {
    20
}

/// Extracted pagination parameters
#[derive(Debug, Clone)]
pub struct Pagination {
    /// Pagination parameters
    pub params: PaginationParams,

    /// Sort parameters (if provided)
    pub sort: Option<SortParams>,
}

impl Pagination {
    /// Create new pagination with default sort
    pub fn new(params: PaginationParams) -> Self {
        Self { params, sort: None }
    }

    /// Create with sort parameters
    pub fn with_sort(params: PaginationParams, sort: SortParams) -> Self {
        Self {
            params,
            sort: Some(sort),
        }
    }

    /// Get offset for database queries
    pub fn offset(&self) -> u32 {
        self.params.offset()
    }

    /// Get limit for database queries
    pub fn limit(&self) -> u32 {
        self.params.limit()
    }

    /// Get sort field or default
    pub fn sort_field(&self, default: &str) -> String {
        self.sort
            .as_ref()
            .map(|s| s.field.clone())
            .unwrap_or_else(|| default.to_string())
    }

    /// Get sort direction or default
    pub fn sort_direction(&self) -> SortDirection {
        self.sort
            .as_ref()
            .map(|s| s.direction)
            .unwrap_or(SortDirection::Desc)
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for Pagination
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<PaginationQuery>::from_request_parts(parts, state)
            .await
            .map_err(|e| ApiError::BadRequest(format!("Invalid pagination parameters: {}", e)))?;

        let params = PaginationParams::new(query.page, query.per_page);

        // Validate pagination parameters
        params
            .validate()
            .map_err(|e| ApiError::BadRequest(format!("Invalid pagination: {}", e)))?;

        // Parse sort parameters if provided
        let sort = match (query.sort_by, query.sort_dir) {
            (Some(field), Some(dir)) => {
                let direction = match dir.to_lowercase().as_str() {
                    "desc" | "descending" => SortDirection::Desc,
                    _ => SortDirection::Asc,
                };
                Some(SortParams::new(field, direction))
            }
            (Some(field), None) => Some(SortParams::desc(field)),
            _ => None,
        };

        Ok(Self { params, sort })
    }
}
