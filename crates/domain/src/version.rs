//! Semantic versioning implementation for benchmarks.
//!
//! This module provides a strict implementation of semantic versioning (SemVer 2.0.0)
//! for benchmark versioning with proper ordering, parsing, and compatibility checking.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};

/// Error type for version parsing failures
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum VersionParseError {
    /// Invalid version format
    #[error("Invalid version format")]
    InvalidFormat,

    /// Invalid component value
    #[error("Invalid component value: {0}")]
    InvalidComponent(String),
}

/// Semantic version with strict ordering
///
/// Follows SemVer 2.0.0 specification:
/// - MAJOR version for incompatible API changes
/// - MINOR version for backwards-compatible functionality additions
/// - PATCH version for backwards-compatible bug fixes
/// - Optional prerelease identifier (e.g., "alpha", "beta.1", "rc.2")
/// - Optional build metadata (e.g., "build.123", "sha.abc123")
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SemanticVersion {
    /// Major version number (breaking changes)
    pub major: u32,

    /// Minor version number (backwards-compatible additions)
    pub minor: u32,

    /// Patch version number (backwards-compatible fixes)
    pub patch: u32,

    /// Optional prerelease identifier (e.g., "alpha", "beta.1")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prerelease: Option<String>,

    /// Optional build metadata (e.g., "build.123")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_metadata: Option<String>,
}

impl SemanticVersion {
    /// Create a new semantic version with major, minor, and patch numbers
    ///
    /// # Example
    /// ```
    /// # use llm_benchmark_domain::version::SemanticVersion;
    /// let version = SemanticVersion::new(1, 2, 3);
    /// assert_eq!(version.to_string(), "1.2.3");
    /// ```
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            prerelease: None,
            build_metadata: None,
        }
    }

    /// Create a new semantic version with prerelease identifier
    ///
    /// # Example
    /// ```
    /// # use llm_benchmark_domain::version::SemanticVersion;
    /// let version = SemanticVersion::with_prerelease(1, 0, 0, "alpha.1");
    /// assert_eq!(version.to_string(), "1.0.0-alpha.1");
    /// ```
    pub fn with_prerelease(major: u32, minor: u32, patch: u32, prerelease: impl Into<String>) -> Self {
        Self {
            major,
            minor,
            patch,
            prerelease: Some(prerelease.into()),
            build_metadata: None,
        }
    }

    /// Parse a version string into a SemanticVersion
    ///
    /// Supports formats:
    /// - "1.2.3"
    /// - "1.2.3-alpha"
    /// - "1.2.3-alpha+build.123"
    /// - "1.2.3+build.123"
    ///
    /// # Example
    /// ```
    /// # use llm_benchmark_domain::version::SemanticVersion;
    /// let version = SemanticVersion::parse("1.2.3-alpha+build.123").unwrap();
    /// assert_eq!(version.major, 1);
    /// assert_eq!(version.minor, 2);
    /// assert_eq!(version.patch, 3);
    /// assert_eq!(version.prerelease, Some("alpha".to_string()));
    /// assert_eq!(version.build_metadata, Some("build.123".to_string()));
    /// ```
    pub fn parse(version_str: &str) -> Result<Self, VersionParseError> {
        // Split on '+' to separate build metadata
        let (core_and_pre, build) = match version_str.split_once('+') {
            Some((core, build)) => (core, Some(build.to_string())),
            None => (version_str, None),
        };

        // Split on '-' to separate prerelease
        let (core, prerelease) = match core_and_pre.split_once('-') {
            Some((core, pre)) => (core, Some(pre.to_string())),
            None => (core_and_pre, None),
        };

        // Parse core version numbers
        let parts: Vec<&str> = core.split('.').collect();
        if parts.len() != 3 {
            return Err(VersionParseError::InvalidFormat);
        }

        let major = parts[0]
            .parse::<u32>()
            .map_err(|_| VersionParseError::InvalidComponent("major".to_string()))?;
        let minor = parts[1]
            .parse::<u32>()
            .map_err(|_| VersionParseError::InvalidComponent("minor".to_string()))?;
        let patch = parts[2]
            .parse::<u32>()
            .map_err(|_| VersionParseError::InvalidComponent("patch".to_string()))?;

        Ok(Self {
            major,
            minor,
            patch,
            prerelease,
            build_metadata: build,
        })
    }

    /// Check if this version is compatible with another version
    ///
    /// Versions are compatible if they have the same major version.
    /// This follows the SemVer convention that major version 0 is for initial development
    /// and anything may change at any time.
    ///
    /// # Example
    /// ```
    /// # use llm_benchmark_domain::version::SemanticVersion;
    /// let v1 = SemanticVersion::new(1, 2, 3);
    /// let v2 = SemanticVersion::new(1, 3, 0);
    /// let v3 = SemanticVersion::new(2, 0, 0);
    ///
    /// assert!(v1.is_compatible_with(&v2));
    /// assert!(!v1.is_compatible_with(&v3));
    /// ```
    pub fn is_compatible_with(&self, other: &Self) -> bool {
        self.major == other.major && self.major > 0
    }

    /// Increment the patch version
    ///
    /// Removes prerelease and build metadata.
    ///
    /// # Example
    /// ```
    /// # use llm_benchmark_domain::version::SemanticVersion;
    /// let v = SemanticVersion::new(1, 2, 3);
    /// let v_next = v.increment_patch();
    /// assert_eq!(v_next.to_string(), "1.2.4");
    /// ```
    pub fn increment_patch(&self) -> Self {
        Self {
            major: self.major,
            minor: self.minor,
            patch: self.patch + 1,
            prerelease: None,
            build_metadata: None,
        }
    }

    /// Increment the minor version
    ///
    /// Resets patch to 0 and removes prerelease and build metadata.
    ///
    /// # Example
    /// ```
    /// # use llm_benchmark_domain::version::SemanticVersion;
    /// let v = SemanticVersion::new(1, 2, 3);
    /// let v_next = v.increment_minor();
    /// assert_eq!(v_next.to_string(), "1.3.0");
    /// ```
    pub fn increment_minor(&self) -> Self {
        Self {
            major: self.major,
            minor: self.minor + 1,
            patch: 0,
            prerelease: None,
            build_metadata: None,
        }
    }

    /// Increment the major version
    ///
    /// Resets minor and patch to 0 and removes prerelease and build metadata.
    ///
    /// # Example
    /// ```
    /// # use llm_benchmark_domain::version::SemanticVersion;
    /// let v = SemanticVersion::new(1, 2, 3);
    /// let v_next = v.increment_major();
    /// assert_eq!(v_next.to_string(), "2.0.0");
    /// ```
    pub fn increment_major(&self) -> Self {
        Self {
            major: self.major + 1,
            minor: 0,
            patch: 0,
            prerelease: None,
            build_metadata: None,
        }
    }

    /// Check if this is a prerelease version
    pub fn is_prerelease(&self) -> bool {
        self.prerelease.is_some()
    }

    /// Check if this is a stable release (not prerelease)
    pub fn is_stable(&self) -> bool {
        self.prerelease.is_none()
    }
}

impl Ord for SemanticVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare major, minor, patch
        match self.major.cmp(&other.major) {
            Ordering::Equal => match self.minor.cmp(&other.minor) {
                Ordering::Equal => match self.patch.cmp(&other.patch) {
                    Ordering::Equal => {
                        // Prerelease versions have lower precedence than normal versions
                        match (&self.prerelease, &other.prerelease) {
                            (None, None) => Ordering::Equal,
                            (Some(_), None) => Ordering::Less,
                            (None, Some(_)) => Ordering::Greater,
                            (Some(a), Some(b)) => a.cmp(b),
                        }
                    }
                    other => other,
                },
                other => other,
            },
            other => other,
        }
    }
}

impl PartialOrd for SemanticVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Display for SemanticVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(ref pre) = self.prerelease {
            write!(f, "-{}", pre)?;
        }
        if let Some(ref build) = self.build_metadata {
            write!(f, "+{}", build)?;
        }
        Ok(())
    }
}

impl std::str::FromStr for SemanticVersion {
    type Err = VersionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_new() {
        let v = SemanticVersion::new(1, 2, 3);
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        assert_eq!(v.prerelease, None);
        assert_eq!(v.build_metadata, None);
    }

    #[test]
    fn test_version_parse() {
        let v = SemanticVersion::parse("1.2.3").unwrap();
        assert_eq!(v, SemanticVersion::new(1, 2, 3));

        let v = SemanticVersion::parse("1.2.3-alpha").unwrap();
        assert_eq!(v.prerelease, Some("alpha".to_string()));

        let v = SemanticVersion::parse("1.2.3-alpha.1+build.123").unwrap();
        assert_eq!(v.prerelease, Some("alpha.1".to_string()));
        assert_eq!(v.build_metadata, Some("build.123".to_string()));

        let v = SemanticVersion::parse("1.2.3+build.123").unwrap();
        assert_eq!(v.prerelease, None);
        assert_eq!(v.build_metadata, Some("build.123".to_string()));
    }

    #[test]
    fn test_version_parse_errors() {
        assert!(SemanticVersion::parse("1.2").is_err());
        assert!(SemanticVersion::parse("1.2.3.4").is_err());
        assert!(SemanticVersion::parse("a.b.c").is_err());
        assert!(SemanticVersion::parse("").is_err());
    }

    #[test]
    fn test_version_display() {
        let v = SemanticVersion::new(1, 2, 3);
        assert_eq!(v.to_string(), "1.2.3");

        let v = SemanticVersion::with_prerelease(1, 2, 3, "alpha");
        assert_eq!(v.to_string(), "1.2.3-alpha");
    }

    #[test]
    fn test_version_ordering() {
        let v1 = SemanticVersion::new(1, 0, 0);
        let v2 = SemanticVersion::new(1, 1, 0);
        let v3 = SemanticVersion::new(2, 0, 0);

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v1 < v3);

        // Prerelease versions have lower precedence
        let v_stable = SemanticVersion::new(1, 0, 0);
        let v_pre = SemanticVersion::with_prerelease(1, 0, 0, "alpha");
        assert!(v_pre < v_stable);
    }

    #[test]
    fn test_version_compatibility() {
        let v1 = SemanticVersion::new(1, 2, 3);
        let v2 = SemanticVersion::new(1, 3, 0);
        let v3 = SemanticVersion::new(2, 0, 0);

        assert!(v1.is_compatible_with(&v2));
        assert!(!v1.is_compatible_with(&v3));

        // Version 0.x.x is not compatible with anything
        let v0 = SemanticVersion::new(0, 1, 0);
        assert!(!v0.is_compatible_with(&SemanticVersion::new(0, 2, 0)));
    }

    #[test]
    fn test_version_increment() {
        let v = SemanticVersion::new(1, 2, 3);

        assert_eq!(v.increment_patch(), SemanticVersion::new(1, 2, 4));
        assert_eq!(v.increment_minor(), SemanticVersion::new(1, 3, 0));
        assert_eq!(v.increment_major(), SemanticVersion::new(2, 0, 0));
    }

    #[test]
    fn test_version_serialization() {
        let v = SemanticVersion::parse("1.2.3-alpha+build.123").unwrap();
        let json = serde_json::to_string(&v).unwrap();
        let deserialized: SemanticVersion = serde_json::from_str(&json).unwrap();
        assert_eq!(v, deserialized);
    }

    #[test]
    fn test_prerelease_checks() {
        let v_stable = SemanticVersion::new(1, 0, 0);
        let v_pre = SemanticVersion::with_prerelease(1, 0, 0, "alpha");

        assert!(!v_stable.is_prerelease());
        assert!(v_stable.is_stable());
        assert!(v_pre.is_prerelease());
        assert!(!v_pre.is_stable());
    }
}
