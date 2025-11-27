//! Data Transfer Objects (DTOs) for API layer
//!
//! DTOs provide a stable API contract separate from internal domain models.
//! They handle serialization, validation annotations, and API documentation.

mod benchmark;
mod common;
mod submission;
mod user;

pub use benchmark::*;
pub use common::*;
pub use submission::*;
pub use user::*;

use serde::{Deserialize, Serialize};

/// Standard API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// Whether the request was successful
    pub success: bool,
    /// Response data (if successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// Error information (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiError>,
    /// Request metadata
    pub meta: ResponseMeta,
}

impl<T> ApiResponse<T> {
    /// Create a successful response
    pub fn success(data: T, request_id: String) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            meta: ResponseMeta::new(request_id),
        }
    }

    /// Create an error response
    pub fn error(error: ApiError, request_id: String) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(error),
            meta: ResponseMeta::new(request_id),
        }
    }
}

/// API error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    /// Error code
    pub code: String,
    /// Human-readable message
    pub message: String,
    /// Additional error details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// Field-specific errors
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_errors: Option<Vec<FieldError>>,
}

impl ApiError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
            field_errors: None,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn with_field_errors(mut self, errors: Vec<FieldError>) -> Self {
        self.field_errors = Some(errors);
        self
    }
}

/// Field-specific validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldError {
    /// Field name (supports nested paths like "address.city")
    pub field: String,
    /// Error message
    pub message: String,
    /// Error code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

impl FieldError {
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            code: None,
        }
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }
}

/// Response metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMeta {
    /// Unique request identifier for tracing
    pub request_id: String,
    /// Response timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// API version
    pub api_version: String,
}

impl ResponseMeta {
    pub fn new(request_id: String) -> Self {
        Self {
            request_id,
            timestamp: chrono::Utc::now(),
            api_version: "v1".to_string(),
        }
    }
}

/// Paginated response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    /// List of items
    pub items: Vec<T>,
    /// Pagination info
    pub pagination: PaginationInfo,
}

/// Pagination information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationInfo {
    /// Current page (1-indexed)
    pub page: u32,
    /// Items per page
    pub page_size: u32,
    /// Total number of items
    pub total_items: u64,
    /// Total number of pages
    pub total_pages: u32,
    /// Whether there's a next page
    pub has_next: bool,
    /// Whether there's a previous page
    pub has_previous: bool,
}

impl PaginationInfo {
    pub fn new(page: u32, page_size: u32, total_items: u64) -> Self {
        let total_pages = ((total_items as f64) / (page_size as f64)).ceil() as u32;
        Self {
            page,
            page_size,
            total_items,
            total_pages,
            has_next: page < total_pages,
            has_previous: page > 1,
        }
    }
}

/// Links for HATEOAS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Links {
    /// Self link
    #[serde(rename = "self")]
    pub self_link: String,
    /// Next page link
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    /// Previous page link
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev: Option<String>,
    /// First page link
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first: Option<String>,
    /// Last page link
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last: Option<String>,
}

impl Links {
    pub fn new(self_link: impl Into<String>) -> Self {
        Self {
            self_link: self_link.into(),
            next: None,
            prev: None,
            first: None,
            last: None,
        }
    }

    pub fn for_pagination(
        base_url: &str,
        page: u32,
        page_size: u32,
        total_pages: u32,
    ) -> Self {
        let make_url = |p: u32| format!("{}?page={}&page_size={}", base_url, p, page_size);

        Self {
            self_link: make_url(page),
            next: if page < total_pages {
                Some(make_url(page + 1))
            } else {
                None
            },
            prev: if page > 1 {
                Some(make_url(page - 1))
            } else {
                None
            },
            first: Some(make_url(1)),
            last: if total_pages > 0 {
                Some(make_url(total_pages))
            } else {
                None
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_success() {
        let response = ApiResponse::success("test data", "req-123".to_string());
        assert!(response.success);
        assert_eq!(response.data, Some("test data"));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_api_response_error() {
        let error = ApiError::new("NOT_FOUND", "Resource not found");
        let response: ApiResponse<()> = ApiResponse::<()>::error(error, "req-123".to_string());
        assert!(!response.success);
        assert!(response.data.is_none());
        assert!(response.error.is_some());
    }

    #[test]
    fn test_pagination_info() {
        let info = PaginationInfo::new(1, 20, 100);
        assert_eq!(info.total_pages, 5);
        assert!(info.has_next);
        assert!(!info.has_previous);

        let info = PaginationInfo::new(3, 20, 100);
        assert!(info.has_next);
        assert!(info.has_previous);

        let info = PaginationInfo::new(5, 20, 100);
        assert!(!info.has_next);
        assert!(info.has_previous);
    }

    #[test]
    fn test_links_pagination() {
        let links = Links::for_pagination("/api/items", 2, 20, 5);
        assert_eq!(links.self_link, "/api/items?page=2&page_size=20");
        assert_eq!(links.next, Some("/api/items?page=3&page_size=20".to_string()));
        assert_eq!(links.prev, Some("/api/items?page=1&page_size=20".to_string()));
        assert_eq!(links.first, Some("/api/items?page=1&page_size=20".to_string()));
        assert_eq!(links.last, Some("/api/items?page=5&page_size=20".to_string()));
    }
}
