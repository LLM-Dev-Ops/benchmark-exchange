//! Pagination and sorting utilities.
//!
//! This module provides types and utilities for handling paginated API responses,
//! sorting, and date range filtering.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Default page number (1-indexed)
const DEFAULT_PAGE: u32 = 1;

/// Default items per page
const DEFAULT_PER_PAGE: u32 = 20;

/// Maximum items per page
const MAX_PER_PAGE: u32 = 100;

/// Pagination parameters for API requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    pub page: u32,

    /// Items per page
    #[serde(default = "default_per_page")]
    pub per_page: u32,
}

fn default_page() -> u32 {
    DEFAULT_PAGE
}

fn default_per_page() -> u32 {
    DEFAULT_PER_PAGE
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: DEFAULT_PAGE,
            per_page: DEFAULT_PER_PAGE,
        }
    }
}

impl PaginationParams {
    /// Create new pagination parameters.
    pub fn new(page: u32, per_page: u32) -> Self {
        let page = if page == 0 { DEFAULT_PAGE } else { page };
        let per_page = if per_page == 0 {
            DEFAULT_PER_PAGE
        } else {
            per_page.min(MAX_PER_PAGE)
        };

        Self { page, per_page }
    }

    /// Calculate the offset for database queries (0-indexed).
    pub fn offset(&self) -> u32 {
        (self.page - 1) * self.per_page
    }

    /// Get the limit for database queries.
    pub fn limit(&self) -> u32 {
        self.per_page
    }

    /// Validate pagination parameters.
    pub fn validate(&self) -> Result<(), String> {
        if self.page == 0 {
            return Err("Page number must be greater than 0".to_string());
        }
        if self.per_page == 0 {
            return Err("Items per page must be greater than 0".to_string());
        }
        if self.per_page > MAX_PER_PAGE {
            return Err(format!(
                "Items per page cannot exceed {}",
                MAX_PER_PAGE
            ));
        }
        Ok(())
    }
}

/// Sort direction for query results.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    /// Ascending order
    Asc,
    /// Descending order
    Desc,
}

impl Default for SortDirection {
    fn default() -> Self {
        Self::Asc
    }
}

impl fmt::Display for SortDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Asc => write!(f, "ASC"),
            Self::Desc => write!(f, "DESC"),
        }
    }
}

impl From<&str> for SortDirection {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "desc" | "descending" => Self::Desc,
            _ => Self::Asc,
        }
    }
}

/// Sort parameters for API requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortParams {
    /// Field to sort by
    pub field: String,

    /// Sort direction
    #[serde(default)]
    pub direction: SortDirection,
}

impl SortParams {
    /// Create new sort parameters.
    pub fn new(field: impl Into<String>, direction: SortDirection) -> Self {
        Self {
            field: field.into(),
            direction,
        }
    }

    /// Create ascending sort parameters.
    pub fn asc(field: impl Into<String>) -> Self {
        Self::new(field, SortDirection::Asc)
    }

    /// Create descending sort parameters.
    pub fn desc(field: impl Into<String>) -> Self {
        Self::new(field, SortDirection::Desc)
    }
}

impl Default for SortParams {
    fn default() -> Self {
        Self {
            field: "created_at".to_string(),
            direction: SortDirection::Desc,
        }
    }
}

/// Paginated result wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResult<T> {
    /// The items for the current page
    pub items: Vec<T>,

    /// Current page number (1-indexed)
    pub page: u32,

    /// Items per page
    pub per_page: u32,

    /// Total number of items across all pages
    pub total: u64,

    /// Total number of pages
    pub total_pages: u32,

    /// Whether there is a next page
    pub has_next: bool,

    /// Whether there is a previous page
    pub has_prev: bool,
}

impl<T> PaginatedResult<T> {
    /// Create a new paginated result.
    pub fn new(items: Vec<T>, page: u32, per_page: u32, total: u64) -> Self {
        let total_pages = ((total as f64) / (per_page as f64)).ceil() as u32;
        let has_next = page < total_pages;
        let has_prev = page > 1;

        Self {
            items,
            page,
            per_page,
            total,
            total_pages,
            has_next,
            has_prev,
        }
    }

    /// Create from pagination parameters and total count.
    pub fn from_params(items: Vec<T>, params: &PaginationParams, total: u64) -> Self {
        Self::new(items, params.page, params.per_page, total)
    }

    /// Map the items to a different type.
    pub fn map<U, F>(self, f: F) -> PaginatedResult<U>
    where
        F: FnMut(T) -> U,
    {
        PaginatedResult {
            items: self.items.into_iter().map(f).collect(),
            page: self.page,
            per_page: self.per_page,
            total: self.total,
            total_pages: self.total_pages,
            has_next: self.has_next,
            has_prev: self.has_prev,
        }
    }
}

/// Date range filter for queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    /// Start date (inclusive)
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub start: Option<chrono::DateTime<chrono::Utc>>,

    /// End date (inclusive)
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub end: Option<chrono::DateTime<chrono::Utc>>,
}

impl DateRange {
    /// Create a new date range.
    pub fn new(
        start: Option<chrono::DateTime<chrono::Utc>>,
        end: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Self {
        Self { start, end }
    }

    /// Validate the date range.
    pub fn validate(&self) -> Result<(), String> {
        if let (Some(start), Some(end)) = (self.start, self.end) {
            if start > end {
                return Err("Start date must be before or equal to end date".to_string());
            }
        }
        Ok(())
    }

    /// Check if a date is within this range.
    pub fn contains(&self, date: &chrono::DateTime<chrono::Utc>) -> bool {
        let after_start = self.start.map_or(true, |start| date >= &start);
        let before_end = self.end.map_or(true, |end| date <= &end);
        after_start && before_end
    }
}

impl Default for DateRange {
    fn default() -> Self {
        Self {
            start: None,
            end: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_params_default() {
        let params = PaginationParams::default();
        assert_eq!(params.page, 1);
        assert_eq!(params.per_page, 20);
        assert_eq!(params.offset(), 0);
        assert_eq!(params.limit(), 20);
    }

    #[test]
    fn test_pagination_params_new() {
        let params = PaginationParams::new(2, 50);
        assert_eq!(params.page, 2);
        assert_eq!(params.per_page, 50);
        assert_eq!(params.offset(), 50);
        assert_eq!(params.limit(), 50);
    }

    #[test]
    fn test_pagination_params_zero_page() {
        let params = PaginationParams::new(0, 20);
        assert_eq!(params.page, 1);
    }

    #[test]
    fn test_pagination_params_max_per_page() {
        let params = PaginationParams::new(1, 200);
        assert_eq!(params.per_page, 100);
    }

    #[test]
    fn test_pagination_params_validation() {
        let valid = PaginationParams::new(1, 20);
        assert!(valid.validate().is_ok());

        let mut invalid = PaginationParams { page: 0, per_page: 20 };
        assert!(invalid.validate().is_err());

        invalid = PaginationParams { page: 1, per_page: 0 };
        assert!(invalid.validate().is_err());

        invalid = PaginationParams { page: 1, per_page: 101 };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_sort_direction() {
        assert_eq!(SortDirection::from("asc"), SortDirection::Asc);
        assert_eq!(SortDirection::from("desc"), SortDirection::Desc);
        assert_eq!(SortDirection::from("descending"), SortDirection::Desc);
        assert_eq!(SortDirection::from("invalid"), SortDirection::Asc);
    }

    #[test]
    fn test_sort_params() {
        let sort = SortParams::asc("name");
        assert_eq!(sort.field, "name");
        assert_eq!(sort.direction, SortDirection::Asc);

        let sort = SortParams::desc("created_at");
        assert_eq!(sort.field, "created_at");
        assert_eq!(sort.direction, SortDirection::Desc);
    }

    #[test]
    fn test_paginated_result() {
        let items = vec![1, 2, 3, 4, 5];
        let result = PaginatedResult::new(items, 2, 5, 25);

        assert_eq!(result.page, 2);
        assert_eq!(result.per_page, 5);
        assert_eq!(result.total, 25);
        assert_eq!(result.total_pages, 5);
        assert!(result.has_next);
        assert!(result.has_prev);
    }

    #[test]
    fn test_paginated_result_map() {
        let items = vec![1, 2, 3];
        let result = PaginatedResult::new(items, 1, 3, 10);
        let mapped = result.map(|x| x * 2);

        assert_eq!(mapped.items, vec![2, 4, 6]);
        assert_eq!(mapped.total, 10);
    }

    #[test]
    fn test_date_range_validation() {
        use chrono::Utc;

        let now = Utc::now();
        let future = now + chrono::Duration::days(1);

        let valid = DateRange::new(Some(now), Some(future));
        assert!(valid.validate().is_ok());

        let invalid = DateRange::new(Some(future), Some(now));
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_date_range_contains() {
        use chrono::Utc;

        let now = Utc::now();
        let past = now - chrono::Duration::days(1);
        let future = now + chrono::Duration::days(1);

        let range = DateRange::new(Some(past), Some(future));
        assert!(range.contains(&now));
        assert!(!range.contains(&(past - chrono::Duration::seconds(1))));
        assert!(!range.contains(&(future + chrono::Duration::seconds(1))));
    }
}
