//! Validation utilities.
//!
//! This module provides validators for common input types like emails, URLs, and slugs.

use once_cell::sync::Lazy;
use regex::Regex;

// Regex patterns
static SLUG_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-z0-9]+(?:-[a-z0-9]+)*$").unwrap()
});

static EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap()
});

static URL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^https?://[^\s/$.?#].[^\s]*$").unwrap()
});

static UUID_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").unwrap()
});

static ALPHANUMERIC_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z0-9]+$").unwrap()
});

/// Validate a slug (lowercase alphanumeric with hyphens).
///
/// A valid slug:
/// - Contains only lowercase letters, numbers, and hyphens
/// - Starts and ends with an alphanumeric character
/// - Does not contain consecutive hyphens
///
/// # Examples
///
/// ```
/// use common::validation::validate_slug;
///
/// assert!(validate_slug("hello-world").is_ok());
/// assert!(validate_slug("test-123").is_ok());
/// assert!(validate_slug("Hello-World").is_err()); // uppercase not allowed
/// assert!(validate_slug("hello--world").is_err()); // consecutive hyphens
/// ```
pub fn validate_slug(slug: &str) -> Result<(), String> {
    if slug.is_empty() {
        return Err("Slug cannot be empty".to_string());
    }

    if slug.len() > 100 {
        return Err("Slug cannot be longer than 100 characters".to_string());
    }

    if !SLUG_REGEX.is_match(slug) {
        return Err(
            "Slug must contain only lowercase letters, numbers, and hyphens (no consecutive hyphens)".to_string()
        );
    }

    Ok(())
}

/// Validate an email address.
///
/// # Examples
///
/// ```
/// use common::validation::validate_email;
///
/// assert!(validate_email("user@example.com").is_ok());
/// assert!(validate_email("invalid-email").is_err());
/// ```
pub fn validate_email(email: &str) -> Result<(), String> {
    if email.is_empty() {
        return Err("Email cannot be empty".to_string());
    }

    if email.len() > 254 {
        return Err("Email cannot be longer than 254 characters".to_string());
    }

    if !EMAIL_REGEX.is_match(email) {
        return Err("Invalid email format".to_string());
    }

    Ok(())
}

/// Validate a URL.
///
/// # Examples
///
/// ```
/// use common::validation::validate_url;
///
/// assert!(validate_url("https://example.com").is_ok());
/// assert!(validate_url("http://localhost:8080").is_ok());
/// assert!(validate_url("not-a-url").is_err());
/// ```
pub fn validate_url(url: &str) -> Result<(), String> {
    if url.is_empty() {
        return Err("URL cannot be empty".to_string());
    }

    if url.len() > 2048 {
        return Err("URL cannot be longer than 2048 characters".to_string());
    }

    if !URL_REGEX.is_match(url) {
        return Err("Invalid URL format (must start with http:// or https://)".to_string());
    }

    Ok(())
}

/// Validate a UUID (v4 format).
///
/// # Examples
///
/// ```
/// use common::validation::validate_uuid;
///
/// assert!(validate_uuid("550e8400-e29b-41d4-a716-446655440000").is_ok());
/// assert!(validate_uuid("invalid-uuid").is_err());
/// ```
pub fn validate_uuid(uuid: &str) -> Result<(), String> {
    if uuid.is_empty() {
        return Err("UUID cannot be empty".to_string());
    }

    if !UUID_REGEX.is_match(uuid) {
        return Err("Invalid UUID format".to_string());
    }

    Ok(())
}

/// Validate an alphanumeric string.
///
/// # Examples
///
/// ```
/// use common::validation::validate_alphanumeric;
///
/// assert!(validate_alphanumeric("abc123").is_ok());
/// assert!(validate_alphanumeric("abc-123").is_err());
/// ```
pub fn validate_alphanumeric(value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err("Value cannot be empty".to_string());
    }

    if !ALPHANUMERIC_REGEX.is_match(value) {
        return Err("Value must contain only alphanumeric characters".to_string());
    }

    Ok(())
}

/// Validate a string length is within a range.
///
/// # Examples
///
/// ```
/// use common::validation::validate_length;
///
/// assert!(validate_length("hello", 1, 10).is_ok());
/// assert!(validate_length("", 1, 10).is_err());
/// assert!(validate_length("too long string", 1, 5).is_err());
/// ```
pub fn validate_length(value: &str, min: usize, max: usize) -> Result<(), String> {
    let len = value.len();

    if len < min {
        return Err(format!("Value must be at least {} characters long", min));
    }

    if len > max {
        return Err(format!("Value cannot be longer than {} characters", max));
    }

    Ok(())
}

/// Validate a number is within a range.
///
/// # Examples
///
/// ```
/// use common::validation::validate_range;
///
/// assert!(validate_range(5, 1, 10).is_ok());
/// assert!(validate_range(0, 1, 10).is_err());
/// assert!(validate_range(11, 1, 10).is_err());
/// ```
pub fn validate_range<T: PartialOrd + std::fmt::Display>(
    value: T,
    min: T,
    max: T,
) -> Result<(), String> {
    if value < min {
        return Err(format!("Value must be at least {}", min));
    }

    if value > max {
        return Err(format!("Value cannot be greater than {}", max));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_slug() {
        // Valid slugs
        assert!(validate_slug("hello-world").is_ok());
        assert!(validate_slug("test-123").is_ok());
        assert!(validate_slug("a").is_ok());
        assert!(validate_slug("123").is_ok());

        // Invalid slugs
        assert!(validate_slug("").is_err());
        assert!(validate_slug("Hello-World").is_err());
        assert!(validate_slug("hello--world").is_err());
        assert!(validate_slug("-hello").is_err());
        assert!(validate_slug("hello-").is_err());
        assert!(validate_slug("hello_world").is_err());
        assert!(validate_slug(&"a".repeat(101)).is_err());
    }

    #[test]
    fn test_validate_email() {
        // Valid emails
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("test.user@example.co.uk").is_ok());
        assert!(validate_email("user+tag@example.com").is_ok());

        // Invalid emails
        assert!(validate_email("").is_err());
        assert!(validate_email("invalid").is_err());
        assert!(validate_email("@example.com").is_err());
        assert!(validate_email("user@").is_err());
        assert!(validate_email("user@example").is_err());
        assert!(validate_email(&format!("{}@example.com", "a".repeat(250))).is_err());
    }

    #[test]
    fn test_validate_url() {
        // Valid URLs
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("http://localhost:8080").is_ok());
        assert!(validate_url("https://example.com/path?query=value").is_ok());

        // Invalid URLs
        assert!(validate_url("").is_err());
        assert!(validate_url("not-a-url").is_err());
        assert!(validate_url("ftp://example.com").is_err());
        assert!(validate_url(&format!("https://{}.com", "a".repeat(2050))).is_err());
    }

    #[test]
    fn test_validate_uuid() {
        // Valid UUIDs
        assert!(validate_uuid("550e8400-e29b-41d4-a716-446655440000").is_ok());
        assert!(validate_uuid("6ba7b810-9dad-11d1-80b4-00c04fd430c8").is_ok());

        // Invalid UUIDs
        assert!(validate_uuid("").is_err());
        assert!(validate_uuid("invalid-uuid").is_err());
        assert!(validate_uuid("550e8400-e29b-41d4-a716").is_err());
        assert!(validate_uuid("550e8400-e29b-41d4-a716-446655440000-extra").is_err());
    }

    #[test]
    fn test_validate_alphanumeric() {
        // Valid
        assert!(validate_alphanumeric("abc123").is_ok());
        assert!(validate_alphanumeric("ABC").is_ok());
        assert!(validate_alphanumeric("123").is_ok());

        // Invalid
        assert!(validate_alphanumeric("").is_err());
        assert!(validate_alphanumeric("abc-123").is_err());
        assert!(validate_alphanumeric("abc 123").is_err());
        assert!(validate_alphanumeric("abc_123").is_err());
    }

    #[test]
    fn test_validate_length() {
        assert!(validate_length("hello", 1, 10).is_ok());
        assert!(validate_length("a", 1, 10).is_ok());
        assert!(validate_length("1234567890", 1, 10).is_ok());

        assert!(validate_length("", 1, 10).is_err());
        assert!(validate_length("12345678901", 1, 10).is_err());
    }

    #[test]
    fn test_validate_range() {
        assert!(validate_range(5, 1, 10).is_ok());
        assert!(validate_range(1, 1, 10).is_ok());
        assert!(validate_range(10, 1, 10).is_ok());

        assert!(validate_range(0, 1, 10).is_err());
        assert!(validate_range(11, 1, 10).is_err());

        // Test with floats
        assert!(validate_range(5.5, 1.0, 10.0).is_ok());
        assert!(validate_range(0.5, 1.0, 10.0).is_err());
    }
}