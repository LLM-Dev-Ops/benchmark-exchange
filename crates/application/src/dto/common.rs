//! Common DTOs shared across the API

use serde::{Deserialize, Serialize};

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: HealthStatus,
    pub version: String,
    pub uptime_seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checks: Option<Vec<ComponentHealth>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub name: String,
    pub status: HealthStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Statistics response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsResponse {
    pub total_benchmarks: u64,
    pub active_benchmarks: u64,
    pub total_submissions: u64,
    pub total_users: u64,
    pub total_organizations: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_stats: Option<Vec<CategoryStats>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryStats {
    pub category: String,
    pub benchmark_count: u64,
    pub submission_count: u64,
}

/// Search request DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    #[serde(default)]
    pub filters: SearchFilters,
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

fn default_page() -> u32 {
    1
}

fn default_page_size() -> u32 {
    20
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchFilters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_type: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_from: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_to: Option<chrono::DateTime<chrono::Utc>>,
}

/// Search result DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub resource_type: String,
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub url: String,
    pub score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlights: Option<Vec<SearchHighlight>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHighlight {
    pub field: String,
    pub fragments: Vec<String>,
}

/// Bulk operation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkRequest<T> {
    pub items: Vec<T>,
    #[serde(default)]
    pub options: BulkOptions,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BulkOptions {
    /// Continue processing on errors
    #[serde(default)]
    pub continue_on_error: bool,
    /// Return detailed results for each item
    #[serde(default = "default_true")]
    pub detailed_results: bool,
}

fn default_true() -> bool {
    true
}

/// Bulk operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkResult<T> {
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<BulkItemResult<T>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkItemResult<T> {
    pub index: usize,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Sort options for list queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortOption {
    pub field: String,
    #[serde(default)]
    pub direction: SortDirection,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    Asc,
    #[default]
    Desc,
}

/// Export request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRequest {
    pub format: ExportFormat,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Json,
    Csv,
    Xlsx,
}

/// Export response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResponse {
    pub download_url: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub format: ExportFormat,
    pub record_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response() {
        let response = HealthResponse {
            status: HealthStatus::Healthy,
            version: "1.0.0".to_string(),
            uptime_seconds: 3600,
            checks: Some(vec![ComponentHealth {
                name: "database".to_string(),
                status: HealthStatus::Healthy,
                latency_ms: Some(5),
                message: None,
            }]),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("healthy"));
    }

    #[test]
    fn test_search_request_defaults() {
        let json = r#"{"query": "test"}"#;
        let request: SearchRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.page, 1);
        assert_eq!(request.page_size, 20);
    }

    #[test]
    fn test_bulk_result() {
        let result: BulkResult<String> = BulkResult {
            total: 3,
            successful: 2,
            failed: 1,
            results: Some(vec![
                BulkItemResult {
                    index: 0,
                    success: true,
                    data: Some("created".to_string()),
                    error: None,
                },
                BulkItemResult {
                    index: 1,
                    success: true,
                    data: Some("created".to_string()),
                    error: None,
                },
                BulkItemResult {
                    index: 2,
                    success: false,
                    data: None,
                    error: Some("Validation failed".to_string()),
                },
            ]),
        };

        assert_eq!(result.successful, 2);
        assert_eq!(result.failed, 1);
    }
}
