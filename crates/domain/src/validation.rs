//! Validation result types for the LLM Benchmark Exchange domain.
//!
//! This module provides structures for representing validation results,
//! including errors, warnings, and informational messages.

use serde::{Deserialize, Serialize};

/// Result of a validation operation
///
/// Contains a boolean indicating overall validity and lists of issues
/// categorized by severity (errors, warnings).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationResult {
    /// Whether the validation passed (no errors)
    pub valid: bool,

    /// List of validation errors (block operation)
    pub errors: Vec<ValidationIssue>,

    /// List of validation warnings (don't block operation)
    pub warnings: Vec<ValidationIssue>,
}

impl ValidationResult {
    /// Create a new successful validation result
    pub fn success() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Create a new failed validation result with a single error
    pub fn error(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            valid: false,
            errors: vec![ValidationIssue {
                path: path.into(),
                message: message.into(),
                severity: IssueSeverity::Error,
            }],
            warnings: Vec::new(),
        }
    }

    /// Create a new validation result with errors
    pub fn with_errors(errors: Vec<ValidationIssue>) -> Self {
        Self {
            valid: errors.is_empty(),
            errors,
            warnings: Vec::new(),
        }
    }

    /// Add an error to this validation result
    pub fn add_error(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.valid = false;
        self.errors.push(ValidationIssue {
            path: path.into(),
            message: message.into(),
            severity: IssueSeverity::Error,
        });
    }

    /// Add a warning to this validation result
    pub fn add_warning(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.warnings.push(ValidationIssue {
            path: path.into(),
            message: message.into(),
            severity: IssueSeverity::Warning,
        });
    }

    /// Add an info message to this validation result
    pub fn add_info(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.warnings.push(ValidationIssue {
            path: path.into(),
            message: message.into(),
            severity: IssueSeverity::Info,
        });
    }

    /// Merge another validation result into this one
    pub fn merge(&mut self, other: ValidationResult) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
        self.valid = self.errors.is_empty();
    }

    /// Check if there are any warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get total count of all issues
    pub fn total_issues(&self) -> usize {
        self.errors.len() + self.warnings.len()
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::success()
    }
}

/// Individual validation issue
///
/// Represents a single validation problem with its location (path),
/// description (message), and severity level.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationIssue {
    /// Path to the field or location that failed validation
    ///
    /// Examples: "metadata.name", "test_cases[0].input", "evaluation_criteria"
    pub path: String,

    /// Human-readable description of the validation issue
    pub message: String,

    /// Severity level of this issue
    pub severity: IssueSeverity,
}

impl ValidationIssue {
    /// Create a new error-level validation issue
    pub fn error(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
            severity: IssueSeverity::Error,
        }
    }

    /// Create a new warning-level validation issue
    pub fn warning(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
            severity: IssueSeverity::Warning,
        }
    }

    /// Create a new info-level validation issue
    pub fn info(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
            severity: IssueSeverity::Info,
        }
    }
}

/// Severity level of a validation issue
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    /// Critical error that blocks the operation
    Error,

    /// Warning that doesn't block the operation but should be addressed
    Warning,

    /// Informational message
    Info,
}

impl IssueSeverity {
    /// Check if this severity level is blocking
    pub fn is_blocking(&self) -> bool {
        matches!(self, Self::Error)
    }

    /// Get a numeric priority for this severity (higher = more severe)
    pub fn priority(&self) -> u8 {
        match self {
            Self::Error => 3,
            Self::Warning => 2,
            Self::Info => 1,
        }
    }
}

impl PartialOrd for IssueSeverity {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.priority().cmp(&other.priority()))
    }
}

impl Ord for IssueSeverity {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority().cmp(&other.priority())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result_success() {
        let result = ValidationResult::success();
        assert!(result.valid);
        assert!(result.errors.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_validation_result_error() {
        let result = ValidationResult::error("field", "Invalid value");
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].path, "field");
        assert_eq!(result.errors[0].message, "Invalid value");
    }

    #[test]
    fn test_validation_result_add_error() {
        let mut result = ValidationResult::success();
        assert!(result.valid);

        result.add_error("field1", "Error 1");
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);

        result.add_error("field2", "Error 2");
        assert_eq!(result.errors.len(), 2);
    }

    #[test]
    fn test_validation_result_add_warning() {
        let mut result = ValidationResult::success();
        result.add_warning("field", "This is not recommended");

        assert!(result.valid); // Warnings don't affect validity
        assert!(result.has_warnings());
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_validation_result_merge() {
        let mut result1 = ValidationResult::success();
        result1.add_error("field1", "Error 1");

        let mut result2 = ValidationResult::success();
        result2.add_error("field2", "Error 2");
        result2.add_warning("field3", "Warning");

        result1.merge(result2);

        assert!(!result1.valid);
        assert_eq!(result1.errors.len(), 2);
        assert_eq!(result1.warnings.len(), 1);
    }

    #[test]
    fn test_issue_severity_ordering() {
        assert!(IssueSeverity::Error > IssueSeverity::Warning);
        assert!(IssueSeverity::Warning > IssueSeverity::Info);
        assert!(IssueSeverity::Error > IssueSeverity::Info);
    }

    #[test]
    fn test_issue_severity_blocking() {
        assert!(IssueSeverity::Error.is_blocking());
        assert!(!IssueSeverity::Warning.is_blocking());
        assert!(!IssueSeverity::Info.is_blocking());
    }

    #[test]
    fn test_validation_issue_constructors() {
        let error = ValidationIssue::error("path", "message");
        assert_eq!(error.severity, IssueSeverity::Error);

        let warning = ValidationIssue::warning("path", "message");
        assert_eq!(warning.severity, IssueSeverity::Warning);

        let info = ValidationIssue::info("path", "message");
        assert_eq!(info.severity, IssueSeverity::Info);
    }

    #[test]
    fn test_serialization() {
        let mut result = ValidationResult::success();
        result.add_error("field", "error message");
        result.add_warning("field2", "warning message");

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ValidationResult = serde_json::from_str(&json).unwrap();

        assert_eq!(result, deserialized);
    }
}
