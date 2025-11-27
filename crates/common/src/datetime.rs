//! DateTime utilities.
//!
//! This module provides helper functions for working with dates and times.

use chrono::{DateTime, NaiveDateTime, Utc};

/// Get the current UTC time.
///
/// # Examples
///
/// ```
/// use common::datetime::now_utc;
///
/// let now = now_utc();
/// println!("Current time: {}", now);
/// ```
pub fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

/// Parse a datetime string into a UTC DateTime.
///
/// Supports multiple common formats:
/// - ISO 8601 (RFC 3339): "2023-12-01T12:30:45Z"
/// - ISO 8601 with offset: "2023-12-01T12:30:45+00:00"
/// - ISO 8601 with timezone: "2023-12-01T12:30:45-05:00"
///
/// # Arguments
///
/// * `datetime_str` - The datetime string to parse
///
/// # Examples
///
/// ```
/// use common::datetime::parse_datetime;
///
/// let dt = parse_datetime("2023-12-01T12:30:45Z").expect("Failed to parse");
/// println!("Parsed: {}", dt);
/// ```
pub fn parse_datetime(datetime_str: &str) -> Result<DateTime<Utc>, String> {
    // Try parsing as RFC 3339 (ISO 8601)
    DateTime::parse_from_rfc3339(datetime_str)
        .map(|dt| dt.with_timezone(&Utc))
        .or_else(|_| {
            // Try parsing as RFC 2822
            DateTime::parse_from_rfc2822(datetime_str)
                .map(|dt| dt.with_timezone(&Utc))
        })
        .or_else(|_| {
            // Try parsing as naive datetime and assume UTC
            NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S")
                .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
        })
        .or_else(|_| {
            // Try parsing as naive datetime with 'T' separator
            NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%dT%H:%M:%S")
                .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
        })
        .map_err(|e| format!("Failed to parse datetime '{}': {}", datetime_str, e))
}

/// Format a DateTime as an ISO 8601 / RFC 3339 string.
///
/// # Arguments
///
/// * `datetime` - The datetime to format
///
/// # Examples
///
/// ```
/// use common::datetime::{now_utc, format_datetime};
///
/// let now = now_utc();
/// let formatted = format_datetime(&now);
/// println!("Formatted: {}", formatted);
/// ```
pub fn format_datetime(datetime: &DateTime<Utc>) -> String {
    datetime.to_rfc3339()
}

/// Format a DateTime for display (human-readable).
///
/// # Arguments
///
/// * `datetime` - The datetime to format
///
/// # Examples
///
/// ```
/// use common::datetime::{now_utc, format_datetime_display};
///
/// let now = now_utc();
/// let formatted = format_datetime_display(&now);
/// println!("Display: {}", formatted);
/// ```
pub fn format_datetime_display(datetime: &DateTime<Utc>) -> String {
    datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

/// Calculate the duration between two datetimes in seconds.
///
/// # Arguments
///
/// * `start` - The start datetime
/// * `end` - The end datetime
///
/// # Examples
///
/// ```
/// use common::datetime::{now_utc, duration_seconds};
/// use chrono::Duration;
///
/// let start = now_utc();
/// let end = start + Duration::hours(2);
/// let seconds = duration_seconds(&start, &end);
/// assert_eq!(seconds, 7200);
/// ```
pub fn duration_seconds(start: &DateTime<Utc>, end: &DateTime<Utc>) -> i64 {
    (*end - *start).num_seconds()
}

/// Check if a datetime is in the past.
///
/// # Arguments
///
/// * `datetime` - The datetime to check
///
/// # Examples
///
/// ```
/// use common::datetime::{now_utc, is_past};
/// use chrono::Duration;
///
/// let past = now_utc() - Duration::hours(1);
/// assert!(is_past(&past));
///
/// let future = now_utc() + Duration::hours(1);
/// assert!(!is_past(&future));
/// ```
pub fn is_past(datetime: &DateTime<Utc>) -> bool {
    datetime < &now_utc()
}

/// Check if a datetime is in the future.
///
/// # Arguments
///
/// * `datetime` - The datetime to check
///
/// # Examples
///
/// ```
/// use common::datetime::{now_utc, is_future};
/// use chrono::Duration;
///
/// let future = now_utc() + Duration::hours(1);
/// assert!(is_future(&future));
///
/// let past = now_utc() - Duration::hours(1);
/// assert!(!is_future(&past));
/// ```
pub fn is_future(datetime: &DateTime<Utc>) -> bool {
    datetime > &now_utc()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Duration};

    #[test]
    fn test_now_utc() {
        let now = now_utc();
        assert!(now <= Utc::now());
    }

    #[test]
    fn test_parse_datetime_rfc3339() {
        let result = parse_datetime("2023-12-01T12:30:45Z");
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.year(), 2023);
        assert_eq!(dt.month(), 12);
        assert_eq!(dt.day(), 1);
    }

    #[test]
    fn test_parse_datetime_with_offset() {
        let result = parse_datetime("2023-12-01T12:30:45+00:00");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_datetime_naive() {
        let result = parse_datetime("2023-12-01 12:30:45");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_datetime_invalid() {
        let result = parse_datetime("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_format_datetime() {
        let dt = DateTime::parse_from_rfc3339("2023-12-01T12:30:45Z")
            .unwrap()
            .with_timezone(&Utc);
        let formatted = format_datetime(&dt);
        assert!(formatted.contains("2023-12-01"));
        assert!(formatted.contains("12:30:45"));
    }

    #[test]
    fn test_format_datetime_display() {
        let dt = DateTime::parse_from_rfc3339("2023-12-01T12:30:45Z")
            .unwrap()
            .with_timezone(&Utc);
        let formatted = format_datetime_display(&dt);
        assert_eq!(formatted, "2023-12-01 12:30:45 UTC");
    }

    #[test]
    fn test_duration_seconds() {
        let start = now_utc();
        let end = start + Duration::hours(2);
        let seconds = duration_seconds(&start, &end);
        assert_eq!(seconds, 7200);
    }

    #[test]
    fn test_is_past() {
        let past = now_utc() - Duration::hours(1);
        assert!(is_past(&past));

        let future = now_utc() + Duration::hours(1);
        assert!(!is_past(&future));
    }

    #[test]
    fn test_is_future() {
        let future = now_utc() + Duration::hours(1);
        assert!(is_future(&future));

        let past = now_utc() - Duration::hours(1);
        assert!(!is_future(&past));
    }
}