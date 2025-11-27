//! Serialization utilities.
//!
//! This module provides custom serializers and deserializers for common types.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::time::Duration;

/// Serialize a Duration as seconds.
///
/// # Examples
///
/// ```
/// use serde::{Serialize, Deserialize};
/// use std::time::Duration;
///
/// #[derive(Serialize, Deserialize)]
/// struct Config {
///     #[serde(serialize_with = "common::serialization::serialize_duration_as_seconds")]
///     #[serde(deserialize_with = "common::serialization::deserialize_duration_from_seconds")]
///     timeout: Duration,
/// }
/// ```
pub fn serialize_duration_as_seconds<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u64(duration.as_secs())
}

/// Deserialize a Duration from seconds.
pub fn deserialize_duration_from_seconds<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let seconds = u64::deserialize(deserializer)?;
    Ok(Duration::from_secs(seconds))
}

/// Serialize a Duration as milliseconds.
///
/// # Examples
///
/// ```
/// use serde::{Serialize, Deserialize};
/// use std::time::Duration;
///
/// #[derive(Serialize, Deserialize)]
/// struct Config {
///     #[serde(serialize_with = "common::serialization::serialize_duration_as_millis")]
///     #[serde(deserialize_with = "common::serialization::deserialize_duration_from_millis")]
///     timeout: Duration,
/// }
/// ```
pub fn serialize_duration_as_millis<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u128(duration.as_millis())
}

/// Deserialize a Duration from milliseconds.
pub fn deserialize_duration_from_millis<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let millis = u64::deserialize(deserializer)?;
    Ok(Duration::from_millis(millis))
}

/// Serialize an Option<Duration> as seconds, skipping if None.
pub fn serialize_optional_duration_as_seconds<S>(
    duration: &Option<Duration>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match duration {
        Some(d) => serializer.serialize_some(&d.as_secs()),
        None => serializer.serialize_none(),
    }
}

/// Deserialize an Option<Duration> from seconds.
pub fn deserialize_optional_duration_from_seconds<'de, D>(
    deserializer: D,
) -> Result<Option<Duration>, D::Error>
where
    D: Deserializer<'de>,
{
    let seconds = Option::<u64>::deserialize(deserializer)?;
    Ok(seconds.map(Duration::from_secs))
}

/// Skip serializing if the value is the default.
///
/// # Examples
///
/// ```
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct Config {
///     #[serde(default, skip_serializing_if = "common::serialization::is_default")]
///     optional_field: String,
/// }
/// ```
pub fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    value == &T::default()
}

/// Skip serializing if the value is false.
pub fn is_false(value: &bool) -> bool {
    !value
}

/// Skip serializing if the value is zero.
pub fn is_zero<T: Default + PartialEq + From<u8>>(value: &T) -> bool {
    value == &T::from(0)
}

/// Serialize a string as lowercase.
pub fn serialize_lowercase<S>(value: &str, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_lowercase())
}

/// Serialize a string as uppercase.
pub fn serialize_uppercase<S>(value: &str, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::time::Duration;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestDuration {
        #[serde(serialize_with = "serialize_duration_as_seconds")]
        #[serde(deserialize_with = "deserialize_duration_from_seconds")]
        timeout: Duration,
    }

    #[test]
    fn test_duration_serialization() {
        let test = TestDuration {
            timeout: Duration::from_secs(60),
        };

        let json = serde_json::to_string(&test).unwrap();
        assert_eq!(json, r#"{"timeout":60}"#);

        let deserialized: TestDuration = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, test);
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestDurationMillis {
        #[serde(serialize_with = "serialize_duration_as_millis")]
        #[serde(deserialize_with = "deserialize_duration_from_millis")]
        timeout: Duration,
    }

    #[test]
    fn test_duration_millis_serialization() {
        let test = TestDurationMillis {
            timeout: Duration::from_millis(1500),
        };

        let json = serde_json::to_string(&test).unwrap();
        assert_eq!(json, r#"{"timeout":1500}"#);

        let deserialized: TestDurationMillis = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, test);
    }

    #[test]
    fn test_is_default() {
        assert!(is_default(&String::new()));
        assert!(!is_default(&"hello".to_string()));
        assert!(is_default(&0u32));
        assert!(!is_default(&42u32));
    }

    #[test]
    fn test_is_false() {
        assert!(is_false(&false));
        assert!(!is_false(&true));
    }

    #[test]
    fn test_is_zero() {
        assert!(is_zero(&0u32));
        assert!(!is_zero(&42u32));
        assert!(is_zero(&0i64));
        assert!(!is_zero(&-1i64));
    }

    #[derive(Debug, Serialize, PartialEq)]
    struct TestSkip {
        #[serde(skip_serializing_if = "is_default")]
        optional: String,
        #[serde(skip_serializing_if = "is_false")]
        flag: bool,
        required: String,
    }

    #[test]
    fn test_skip_serializing() {
        let test = TestSkip {
            optional: String::new(),
            flag: false,
            required: "value".to_string(),
        };

        let json = serde_json::to_string(&test).unwrap();
        assert_eq!(json, r#"{"required":"value"}"#);

        let test_with_values = TestSkip {
            optional: "present".to_string(),
            flag: true,
            required: "value".to_string(),
        };

        let json = serde_json::to_string(&test_with_values).unwrap();
        assert_eq!(json, r#"{"optional":"present","flag":true,"required":"value"}"#);
    }
}