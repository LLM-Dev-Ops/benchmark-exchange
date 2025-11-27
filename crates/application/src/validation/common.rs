//! Common validation utilities and shared validators

use super::{Validatable, ValidationResult, ValidationRules};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Request for creating resources with common fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRequest<T> {
    /// The data to create
    pub data: T,
    /// Request ID for idempotency
    pub request_id: Option<String>,
}

impl<T: Validate> CreateRequest<T> {
    /// Validate the request including nested data
    pub fn validate_request(&self) -> Result<(), validator::ValidationErrors> {
        if let Some(ref id) = self.request_id {
            if id.is_empty() || id.len() > 64 {
                let mut errors = validator::ValidationErrors::new();
                errors.add("request_id", validator::ValidationError::new("length"));
                return Err(errors);
            }
        }
        self.data.validate()
    }
}

/// Request for updating resources with common fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRequest<T> {
    /// The data to update
    pub data: T,
    /// Expected version for optimistic locking
    pub expected_version: Option<i64>,
}

impl<T: Validate> UpdateRequest<T> {
    /// Validate the request including nested data
    pub fn validate_request(&self) -> Result<(), validator::ValidationErrors> {
        self.data.validate()
    }
}

/// Pagination parameters validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub offset: Option<u64>,
    pub limit: Option<u32>,
}

impl PaginationParams {
    pub const MAX_PAGE_SIZE: u32 = 100;
    pub const DEFAULT_PAGE_SIZE: u32 = 20;
}

impl Validatable for PaginationParams {
    fn validate_all(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        if let Some(page) = self.page {
            if page == 0 {
                result.add_field_error("page", "Page must be greater than 0");
            }
        }

        if let Some(page_size) = self.page_size {
            if page_size == 0 {
                result.add_field_error("page_size", "Page size must be greater than 0");
            }
            if page_size > Self::MAX_PAGE_SIZE {
                result.add_field_error(
                    "page_size",
                    format!("Page size must be {} or less", Self::MAX_PAGE_SIZE),
                );
            }
        }

        if let Some(limit) = self.limit {
            if limit == 0 {
                result.add_field_error("limit", "Limit must be greater than 0");
            }
            if limit > Self::MAX_PAGE_SIZE {
                result.add_field_error(
                    "limit",
                    format!("Limit must be {} or less", Self::MAX_PAGE_SIZE),
                );
            }
        }

        result
    }
}

/// Sort order validation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Asc,
    Desc,
}

impl Default for SortOrder {
    fn default() -> Self {
        Self::Desc
    }
}

/// Sort parameters validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortParams {
    pub field: String,
    pub order: SortOrder,
}

impl SortParams {
    pub fn validate_with_allowed_fields(&self, allowed: &[&str]) -> ValidationResult {
        let mut result = ValidationResult::success();

        if !allowed.contains(&self.field.as_str()) {
            result.add_field_error(
                "sort_field",
                format!(
                    "Invalid sort field. Allowed: {}",
                    allowed.join(", ")
                ),
            );
        }

        result
    }
}

/// Date range validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start: Option<chrono::DateTime<chrono::Utc>>,
    pub end: Option<chrono::DateTime<chrono::Utc>>,
}

impl Validatable for DateRange {
    fn validate_all(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        if let (Some(start), Some(end)) = (&self.start, &self.end) {
            if start > end {
                result.add_object_error("Start date must be before end date");
            }
        }

        result
    }
}

/// ID list validation for batch operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdList {
    pub ids: Vec<String>,
}

impl IdList {
    pub const MAX_BATCH_SIZE: usize = 100;
}

impl Validatable for IdList {
    fn validate_all(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        if self.ids.is_empty() {
            result.add_field_error("ids", "At least one ID is required");
            return result;
        }

        if self.ids.len() > Self::MAX_BATCH_SIZE {
            result.add_field_error(
                "ids",
                format!("Maximum {} IDs allowed per batch", Self::MAX_BATCH_SIZE),
            );
        }

        // Validate each ID is a valid UUID
        for (i, id) in self.ids.iter().enumerate() {
            let id_result = ValidationRules::validate_uuid(id, &format!("ids[{}]", i));
            result.merge(id_result);
        }

        result
    }
}

/// Tag list validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagList {
    pub tags: Vec<String>,
}

impl TagList {
    pub const MAX_TAGS: usize = 20;
    pub const MAX_TAG_LENGTH: usize = 50;
}

impl Validatable for TagList {
    fn validate_all(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        if self.tags.len() > Self::MAX_TAGS {
            result.add_field_error(
                "tags",
                format!("Maximum {} tags allowed", Self::MAX_TAGS),
            );
        }

        for (i, tag) in self.tags.iter().enumerate() {
            if tag.is_empty() {
                result.add_field_error(format!("tags[{}]", i), "Tag cannot be empty");
            }

            if tag.len() > Self::MAX_TAG_LENGTH {
                result.add_field_error(
                    format!("tags[{}]", i),
                    format!("Tag must be {} characters or less", Self::MAX_TAG_LENGTH),
                );
            }

            // Tags should be lowercase and alphanumeric with hyphens
            if !tag.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
                result.add_field_error(
                    format!("tags[{}]", i),
                    "Tag must contain only lowercase letters, numbers, and hyphens",
                );
            }
        }

        result
    }
}

/// Search query validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub fields: Option<Vec<String>>,
    pub fuzzy: bool,
}

impl SearchQuery {
    pub const MIN_QUERY_LENGTH: usize = 2;
    pub const MAX_QUERY_LENGTH: usize = 200;
}

impl Validatable for SearchQuery {
    fn validate_all(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        if self.query.len() < Self::MIN_QUERY_LENGTH {
            result.add_field_error(
                "query",
                format!("Query must be at least {} characters", Self::MIN_QUERY_LENGTH),
            );
        }

        if self.query.len() > Self::MAX_QUERY_LENGTH {
            result.add_field_error(
                "query",
                format!("Query must be {} characters or less", Self::MAX_QUERY_LENGTH),
            );
        }

        // Check for dangerous patterns (SQL injection, etc.)
        let dangerous_patterns = ["--", ";", "/*", "*/", "xp_", "exec(", "drop ", "delete "];
        for pattern in dangerous_patterns {
            if self.query.to_lowercase().contains(pattern) {
                result.add_field_error("query", "Query contains invalid characters");
                break;
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_validation() {
        let valid = PaginationParams {
            page: Some(1),
            page_size: Some(20),
            offset: None,
            limit: None,
        };
        assert!(valid.validate_all().valid);

        let invalid_page = PaginationParams {
            page: Some(0),
            page_size: Some(20),
            offset: None,
            limit: None,
        };
        assert!(!invalid_page.validate_all().valid);

        let too_large = PaginationParams {
            page: Some(1),
            page_size: Some(200),
            offset: None,
            limit: None,
        };
        assert!(!too_large.validate_all().valid);
    }

    #[test]
    fn test_date_range_validation() {
        use chrono::Utc;

        let valid = DateRange {
            start: Some(Utc::now()),
            end: Some(Utc::now() + chrono::Duration::days(1)),
        };
        assert!(valid.validate_all().valid);

        let invalid = DateRange {
            start: Some(Utc::now() + chrono::Duration::days(1)),
            end: Some(Utc::now()),
        };
        assert!(!invalid.validate_all().valid);
    }

    #[test]
    fn test_id_list_validation() {
        let valid = IdList {
            ids: vec!["550e8400-e29b-41d4-a716-446655440000".to_string()],
        };
        assert!(valid.validate_all().valid);

        let empty = IdList { ids: vec![] };
        assert!(!empty.validate_all().valid);

        let invalid_uuid = IdList {
            ids: vec!["not-a-uuid".to_string()],
        };
        assert!(!invalid_uuid.validate_all().valid);
    }

    #[test]
    fn test_tag_list_validation() {
        let valid = TagList {
            tags: vec!["rust".to_string(), "benchmark".to_string()],
        };
        assert!(valid.validate_all().valid);

        let invalid = TagList {
            tags: vec!["Invalid Tag!".to_string()],
        };
        assert!(!invalid.validate_all().valid);
    }

    #[test]
    fn test_search_query_validation() {
        let valid = SearchQuery {
            query: "test query".to_string(),
            fields: None,
            fuzzy: false,
        };
        assert!(valid.validate_all().valid);

        let too_short = SearchQuery {
            query: "a".to_string(),
            fields: None,
            fuzzy: false,
        };
        assert!(!too_short.validate_all().valid);

        let dangerous = SearchQuery {
            query: "test; drop table".to_string(),
            fields: None,
            fuzzy: false,
        };
        assert!(!dangerous.validate_all().valid);
    }
}
