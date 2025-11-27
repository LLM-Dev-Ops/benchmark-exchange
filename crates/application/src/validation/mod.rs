//! Validation Framework
//!
//! Provides comprehensive validation for all application inputs including
//! benchmarks, submissions, users, and organizations.

mod benchmark;
mod common;
mod submission;
mod user;

pub use benchmark::*;
pub use common::*;
pub use submission::*;
pub use user::*;

use crate::ApplicationError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

/// Validation result containing all errors
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether validation passed
    pub valid: bool,
    /// Field-level errors
    pub field_errors: HashMap<String, Vec<String>>,
    /// Object-level errors
    pub object_errors: Vec<String>,
}

impl ValidationResult {
    /// Create a successful validation result
    pub fn success() -> Self {
        Self {
            valid: true,
            field_errors: HashMap::new(),
            object_errors: Vec::new(),
        }
    }

    /// Create a failed validation result with a single error
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            valid: false,
            field_errors: HashMap::new(),
            object_errors: vec![message.into()],
        }
    }

    /// Add a field-level error
    pub fn add_field_error(&mut self, field: impl Into<String>, message: impl Into<String>) {
        self.valid = false;
        self.field_errors
            .entry(field.into())
            .or_default()
            .push(message.into());
    }

    /// Add an object-level error
    pub fn add_object_error(&mut self, message: impl Into<String>) {
        self.valid = false;
        self.object_errors.push(message.into());
    }

    /// Merge another validation result into this one
    pub fn merge(&mut self, other: ValidationResult) {
        if !other.valid {
            self.valid = false;
        }

        for (field, errors) in other.field_errors {
            self.field_errors
                .entry(field)
                .or_default()
                .extend(errors);
        }

        self.object_errors.extend(other.object_errors);
    }

    /// Convert to ApplicationError if invalid
    pub fn to_error(&self) -> Option<ApplicationError> {
        if self.valid {
            return None;
        }

        let mut messages = Vec::new();

        for (field, errors) in &self.field_errors {
            for error in errors {
                messages.push(format!("{}: {}", field, error));
            }
        }

        messages.extend(self.object_errors.clone());

        Some(ApplicationError::ValidationFailed(messages.join("; ")))
    }

    /// Ensure validation passed, returning error if not
    pub fn ensure_valid(&self) -> Result<(), ApplicationError> {
        if let Some(err) = self.to_error() {
            Err(err)
        } else {
            Ok(())
        }
    }
}

/// Trait for validatable types
pub trait Validatable {
    /// Validate the type and return a result
    fn validate_all(&self) -> ValidationResult;
}

/// Extension to convert validator errors to our format
pub trait ValidatorExt {
    fn to_validation_result(&self) -> ValidationResult;
}

impl<T: Validate> ValidatorExt for T {
    fn to_validation_result(&self) -> ValidationResult {
        match self.validate() {
            Ok(_) => ValidationResult::success(),
            Err(errors) => {
                let mut result = ValidationResult::success();

                for (field, field_errors) in errors.field_errors() {
                    for error in field_errors {
                        let message = error
                            .message
                            .as_ref()
                            .map(|m| m.to_string())
                            .unwrap_or_else(|| {
                                error
                                    .code
                                    .to_string()
                            });
                        result.add_field_error(field.to_string(), message);
                    }
                }

                result
            }
        }
    }
}

/// Validation context for contextual validation
#[derive(Debug, Clone, Default)]
pub struct ValidationContext {
    /// Current operation being validated
    pub operation: String,
    /// User performing the operation (if any)
    pub user_id: Option<String>,
    /// Whether strict validation is enabled
    pub strict: bool,
    /// Additional context data
    pub data: HashMap<String, serde_json::Value>,
}

impl ValidationContext {
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            user_id: None,
            strict: false,
            data: HashMap::new(),
        }
    }

    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn with_strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }

    pub fn with_data(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.data.insert(key.into(), value);
        self
    }
}

/// Common validation rules
pub struct ValidationRules;

impl ValidationRules {
    /// Validate a slug format
    pub fn validate_slug(slug: &str) -> ValidationResult {
        let mut result = ValidationResult::success();

        if slug.is_empty() {
            result.add_field_error("slug", "Slug cannot be empty");
            return result;
        }

        if slug.len() > 100 {
            result.add_field_error("slug", "Slug must be 100 characters or less");
        }

        if !slug.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            result.add_field_error(
                "slug",
                "Slug must contain only lowercase letters, numbers, and hyphens",
            );
        }

        if slug.starts_with('-') || slug.ends_with('-') {
            result.add_field_error("slug", "Slug cannot start or end with a hyphen");
        }

        if slug.contains("--") {
            result.add_field_error("slug", "Slug cannot contain consecutive hyphens");
        }

        result
    }

    /// Validate an email address
    pub fn validate_email(email: &str) -> ValidationResult {
        let mut result = ValidationResult::success();

        if email.is_empty() {
            result.add_field_error("email", "Email cannot be empty");
            return result;
        }

        // Basic email validation
        let parts: Vec<&str> = email.split('@').collect();
        if parts.len() != 2 {
            result.add_field_error("email", "Invalid email format");
            return result;
        }

        let (local, domain) = (parts[0], parts[1]);

        if local.is_empty() || domain.is_empty() {
            result.add_field_error("email", "Invalid email format");
            return result;
        }

        if !domain.contains('.') {
            result.add_field_error("email", "Invalid email domain");
        }

        if email.len() > 254 {
            result.add_field_error("email", "Email must be 254 characters or less");
        }

        result
    }

    /// Validate a URL
    pub fn validate_url(url: &str) -> ValidationResult {
        let mut result = ValidationResult::success();

        if url.is_empty() {
            result.add_field_error("url", "URL cannot be empty");
            return result;
        }

        match url::Url::parse(url) {
            Ok(parsed) => {
                if !["http", "https"].contains(&parsed.scheme()) {
                    result.add_field_error("url", "URL must use HTTP or HTTPS scheme");
                }
            }
            Err(_) => {
                result.add_field_error("url", "Invalid URL format");
            }
        }

        result
    }

    /// Validate a semantic version string
    pub fn validate_semver(version: &str) -> ValidationResult {
        let mut result = ValidationResult::success();

        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() != 3 {
            result.add_field_error("version", "Version must be in format major.minor.patch");
            return result;
        }

        for (i, part) in parts.iter().enumerate() {
            if part.parse::<u32>().is_err() {
                let component = match i {
                    0 => "major",
                    1 => "minor",
                    _ => "patch",
                };
                result.add_field_error("version", format!("Invalid {} version component", component));
            }
        }

        result
    }

    /// Validate a score is within range
    pub fn validate_score(score: f64, min: f64, max: f64) -> ValidationResult {
        let mut result = ValidationResult::success();

        if score.is_nan() {
            result.add_field_error("score", "Score cannot be NaN");
            return result;
        }

        if score.is_infinite() {
            result.add_field_error("score", "Score cannot be infinite");
            return result;
        }

        if score < min || score > max {
            result.add_field_error(
                "score",
                format!("Score must be between {} and {}", min, max),
            );
        }

        result
    }

    /// Validate a string length
    pub fn validate_length(
        value: &str,
        field: &str,
        min: Option<usize>,
        max: Option<usize>,
    ) -> ValidationResult {
        let mut result = ValidationResult::success();

        if let Some(min_len) = min {
            if value.len() < min_len {
                result.add_field_error(
                    field,
                    format!("Must be at least {} characters", min_len),
                );
            }
        }

        if let Some(max_len) = max {
            if value.len() > max_len {
                result.add_field_error(
                    field,
                    format!("Must be {} characters or less", max_len),
                );
            }
        }

        result
    }

    /// Validate a list size
    pub fn validate_list_size<T>(
        list: &[T],
        field: &str,
        min: Option<usize>,
        max: Option<usize>,
    ) -> ValidationResult {
        let mut result = ValidationResult::success();

        if let Some(min_size) = min {
            if list.len() < min_size {
                result.add_field_error(field, format!("Must have at least {} items", min_size));
            }
        }

        if let Some(max_size) = max {
            if list.len() > max_size {
                result.add_field_error(field, format!("Must have {} items or less", max_size));
            }
        }

        result
    }

    /// Validate JSON content is valid
    pub fn validate_json(content: &str, field: &str) -> ValidationResult {
        let mut result = ValidationResult::success();

        if serde_json::from_str::<serde_json::Value>(content).is_err() {
            result.add_field_error(field, "Invalid JSON format");
        }

        result
    }

    /// Validate a UUID format
    pub fn validate_uuid(value: &str, field: &str) -> ValidationResult {
        let mut result = ValidationResult::success();

        if uuid::Uuid::parse_str(value).is_err() {
            result.add_field_error(field, "Invalid UUID format");
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result_success() {
        let result = ValidationResult::success();
        assert!(result.valid);
        assert!(result.field_errors.is_empty());
        assert!(result.object_errors.is_empty());
        assert!(result.to_error().is_none());
    }

    #[test]
    fn test_validation_result_error() {
        let result = ValidationResult::error("Test error");
        assert!(!result.valid);
        assert!(result.object_errors.contains(&"Test error".to_string()));
        assert!(result.to_error().is_some());
    }

    #[test]
    fn test_validation_result_field_error() {
        let mut result = ValidationResult::success();
        result.add_field_error("name", "Required");
        assert!(!result.valid);
        assert!(result.field_errors.contains_key("name"));
    }

    #[test]
    fn test_validation_result_merge() {
        let mut result1 = ValidationResult::success();
        result1.add_field_error("field1", "Error 1");

        let mut result2 = ValidationResult::success();
        result2.add_field_error("field2", "Error 2");

        result1.merge(result2);
        assert!(!result1.valid);
        assert!(result1.field_errors.contains_key("field1"));
        assert!(result1.field_errors.contains_key("field2"));
    }

    #[test]
    fn test_validate_slug() {
        assert!(ValidationRules::validate_slug("valid-slug-123").valid);
        assert!(!ValidationRules::validate_slug("Invalid_Slug").valid);
        assert!(!ValidationRules::validate_slug("-invalid").valid);
        assert!(!ValidationRules::validate_slug("invalid-").valid);
        assert!(!ValidationRules::validate_slug("invalid--slug").valid);
        assert!(!ValidationRules::validate_slug("").valid);
    }

    #[test]
    fn test_validate_email() {
        assert!(ValidationRules::validate_email("user@example.com").valid);
        assert!(ValidationRules::validate_email("user.name@example.co.uk").valid);
        assert!(!ValidationRules::validate_email("invalid").valid);
        assert!(!ValidationRules::validate_email("@example.com").valid);
        assert!(!ValidationRules::validate_email("user@").valid);
        assert!(!ValidationRules::validate_email("").valid);
    }

    #[test]
    fn test_validate_url() {
        assert!(ValidationRules::validate_url("https://example.com").valid);
        assert!(ValidationRules::validate_url("http://localhost:8080/path").valid);
        assert!(!ValidationRules::validate_url("ftp://example.com").valid);
        assert!(!ValidationRules::validate_url("not-a-url").valid);
        assert!(!ValidationRules::validate_url("").valid);
    }

    #[test]
    fn test_validate_semver() {
        assert!(ValidationRules::validate_semver("1.0.0").valid);
        assert!(ValidationRules::validate_semver("0.1.0").valid);
        assert!(ValidationRules::validate_semver("12.34.56").valid);
        assert!(!ValidationRules::validate_semver("1.0").valid);
        assert!(!ValidationRules::validate_semver("v1.0.0").valid);
        assert!(!ValidationRules::validate_semver("1.0.0.0").valid);
    }

    #[test]
    fn test_validate_score() {
        assert!(ValidationRules::validate_score(0.5, 0.0, 1.0).valid);
        assert!(ValidationRules::validate_score(0.0, 0.0, 1.0).valid);
        assert!(ValidationRules::validate_score(1.0, 0.0, 1.0).valid);
        assert!(!ValidationRules::validate_score(-0.1, 0.0, 1.0).valid);
        assert!(!ValidationRules::validate_score(1.1, 0.0, 1.0).valid);
        assert!(!ValidationRules::validate_score(f64::NAN, 0.0, 1.0).valid);
        assert!(!ValidationRules::validate_score(f64::INFINITY, 0.0, 1.0).valid);
    }

    #[test]
    fn test_validate_length() {
        assert!(ValidationRules::validate_length("hello", "field", Some(1), Some(10)).valid);
        assert!(!ValidationRules::validate_length("", "field", Some(1), None).valid);
        assert!(!ValidationRules::validate_length("too long", "field", None, Some(5)).valid);
    }

    #[test]
    fn test_validate_json() {
        assert!(ValidationRules::validate_json("{\"key\": \"value\"}", "field").valid);
        assert!(ValidationRules::validate_json("[1, 2, 3]", "field").valid);
        assert!(!ValidationRules::validate_json("not json", "field").valid);
    }

    #[test]
    fn test_validate_uuid() {
        assert!(ValidationRules::validate_uuid("550e8400-e29b-41d4-a716-446655440000", "field").valid);
        assert!(!ValidationRules::validate_uuid("not-a-uuid", "field").valid);
    }
}
