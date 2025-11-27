//! SDK error types
//!
//! This module defines the error types used throughout the SDK.

use std::fmt;
use thiserror::Error;

/// Result type alias for SDK operations
pub type SdkResult<T> = Result<T, SdkError>;

/// SDK error type
#[derive(Error, Debug)]
pub enum SdkError {
    /// Authentication failed or credentials are invalid
    #[error("Authentication failed: {message}")]
    Unauthorized {
        /// Error message
        message: String,
        /// HTTP status code
        status_code: u16,
    },

    /// Access to the resource is forbidden
    #[error("Access forbidden: {message}")]
    Forbidden {
        /// Error message
        message: String,
        /// Resource that was denied
        resource: Option<String>,
    },

    /// Resource was not found
    #[error("Resource not found: {resource_type} '{resource_id}'")]
    NotFound {
        /// Type of resource (e.g., "benchmark", "submission")
        resource_type: String,
        /// ID of the resource
        resource_id: String,
    },

    /// Request validation failed
    #[error("Validation failed: {message}")]
    ValidationError {
        /// Error message
        message: String,
        /// Field-specific errors
        field_errors: Vec<FieldError>,
    },

    /// Conflict with existing resource
    #[error("Conflict: {message}")]
    Conflict {
        /// Error message
        message: String,
    },

    /// Rate limit exceeded
    #[error("Rate limit exceeded. Retry after {retry_after:?} seconds")]
    RateLimited {
        /// Seconds until rate limit resets
        retry_after: Option<u64>,
    },

    /// Request timeout
    #[error("Request timed out after {duration:?}")]
    Timeout {
        /// Duration before timeout
        duration: std::time::Duration,
    },

    /// Network error
    #[error("Network error: {message}")]
    NetworkError {
        /// Error message
        message: String,
        /// Underlying error
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Server error
    #[error("Server error ({status_code}): {message}")]
    ServerError {
        /// HTTP status code
        status_code: u16,
        /// Error message
        message: String,
    },

    /// API returned an unexpected response
    #[error("Invalid API response: {message}")]
    InvalidResponse {
        /// Error message
        message: String,
    },

    /// Configuration error
    #[error("Configuration error: {message}")]
    ConfigError {
        /// Error message
        message: String,
    },

    /// Serialization/deserialization error
    #[error("Serialization error: {message}")]
    SerializationError {
        /// Error message
        message: String,
        /// Underlying error
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Generic API error
    #[error("API error: {code} - {message}")]
    ApiError {
        /// Error code
        code: String,
        /// Error message
        message: String,
        /// Additional details
        details: Option<serde_json::Value>,
    },
}

impl SdkError {
    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            SdkError::Timeout { .. } => true,
            SdkError::NetworkError { .. } => true,
            SdkError::RateLimited { .. } => true,
            SdkError::ServerError { status_code, .. } => *status_code >= 500,
            _ => false,
        }
    }

    /// Get the HTTP status code if available
    pub fn status_code(&self) -> Option<u16> {
        match self {
            SdkError::Unauthorized { status_code, .. } => Some(*status_code),
            SdkError::ServerError { status_code, .. } => Some(*status_code),
            SdkError::NotFound { .. } => Some(404),
            SdkError::Forbidden { .. } => Some(403),
            SdkError::ValidationError { .. } => Some(400),
            SdkError::Conflict { .. } => Some(409),
            SdkError::RateLimited { .. } => Some(429),
            _ => None,
        }
    }

    /// Create a not found error
    pub fn not_found(resource_type: impl Into<String>, resource_id: impl Into<String>) -> Self {
        SdkError::NotFound {
            resource_type: resource_type.into(),
            resource_id: resource_id.into(),
        }
    }

    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        SdkError::ValidationError {
            message: message.into(),
            field_errors: Vec::new(),
        }
    }

    /// Create a validation error with field errors
    pub fn validation_with_fields(
        message: impl Into<String>,
        field_errors: Vec<FieldError>,
    ) -> Self {
        SdkError::ValidationError {
            message: message.into(),
            field_errors,
        }
    }
}

/// Field-specific validation error
#[derive(Debug, Clone)]
pub struct FieldError {
    /// Field name (supports nested paths like "model.version")
    pub field: String,
    /// Error message
    pub message: String,
    /// Error code
    pub code: Option<String>,
}

impl FieldError {
    /// Create a new field error
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            code: None,
        }
    }

    /// Create a field error with a code
    pub fn with_code(
        field: impl Into<String>,
        message: impl Into<String>,
        code: impl Into<String>,
    ) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            code: Some(code.into()),
        }
    }
}

impl fmt::Display for FieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

/// Convert from reqwest errors
impl From<reqwest::Error> for SdkError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            SdkError::Timeout {
                duration: std::time::Duration::from_secs(30),
            }
        } else if err.is_connect() {
            SdkError::NetworkError {
                message: "Connection failed".to_string(),
                source: Some(Box::new(err)),
            }
        } else if let Some(status) = err.status() {
            let status_code = status.as_u16();
            match status_code {
                401 => SdkError::Unauthorized {
                    message: "Authentication required".to_string(),
                    status_code,
                },
                403 => SdkError::Forbidden {
                    message: "Access denied".to_string(),
                    resource: None,
                },
                404 => SdkError::NotFound {
                    resource_type: "Resource".to_string(),
                    resource_id: "unknown".to_string(),
                },
                429 => SdkError::RateLimited { retry_after: None },
                500..=599 => SdkError::ServerError {
                    status_code,
                    message: format!("Server error: {}", err),
                },
                _ => SdkError::NetworkError {
                    message: err.to_string(),
                    source: Some(Box::new(err)),
                },
            }
        } else {
            SdkError::NetworkError {
                message: err.to_string(),
                source: Some(Box::new(err)),
            }
        }
    }
}

/// Convert from JSON errors
impl From<serde_json::Error> for SdkError {
    fn from(err: serde_json::Error) -> Self {
        SdkError::SerializationError {
            message: err.to_string(),
            source: Some(Box::new(err)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_is_retryable() {
        assert!(SdkError::Timeout {
            duration: std::time::Duration::from_secs(30)
        }
        .is_retryable());
        assert!(SdkError::RateLimited { retry_after: Some(60) }.is_retryable());
        assert!(SdkError::ServerError {
            status_code: 503,
            message: "Service unavailable".to_string()
        }
        .is_retryable());

        assert!(!SdkError::Unauthorized {
            message: "Invalid token".to_string(),
            status_code: 401
        }
        .is_retryable());
        assert!(!SdkError::NotFound {
            resource_type: "benchmark".to_string(),
            resource_id: "abc".to_string()
        }
        .is_retryable());
    }

    #[test]
    fn test_error_status_code() {
        assert_eq!(
            SdkError::Unauthorized {
                message: "".to_string(),
                status_code: 401
            }
            .status_code(),
            Some(401)
        );
        assert_eq!(
            SdkError::NotFound {
                resource_type: "".to_string(),
                resource_id: "".to_string()
            }
            .status_code(),
            Some(404)
        );
        assert_eq!(
            SdkError::Timeout {
                duration: std::time::Duration::from_secs(1)
            }
            .status_code(),
            None
        );
    }

    #[test]
    fn test_field_error() {
        let err = FieldError::new("email", "Invalid email format");
        assert_eq!(err.field, "email");
        assert_eq!(err.message, "Invalid email format");
        assert!(err.code.is_none());

        let err = FieldError::with_code("email", "Invalid email format", "INVALID_EMAIL");
        assert_eq!(err.code, Some("INVALID_EMAIL".to_string()));
    }
}
