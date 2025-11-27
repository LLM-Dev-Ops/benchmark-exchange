//! Comprehensive tests for SemanticVersion
//!
//! Tests parsing, comparison, compatibility, and incrementing operations.

use llm_benchmark_domain::version::{SemanticVersion, VersionParseError};
use proptest::prelude::*;

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
fn test_version_with_prerelease() {
    let v = SemanticVersion::with_prerelease(1, 0, 0, "alpha.1");
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 0);
    assert_eq!(v.patch, 0);
    assert_eq!(v.prerelease, Some("alpha.1".to_string()));
    assert!(v.is_prerelease());
    assert!(!v.is_stable());
}

#[test]
fn test_version_parse_simple() {
    let v = SemanticVersion::parse("1.2.3").unwrap();
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 2);
    assert_eq!(v.patch, 3);
    assert_eq!(v.to_string(), "1.2.3");
}

#[test]
fn test_version_parse_with_prerelease() {
    let v = SemanticVersion::parse("1.2.3-alpha").unwrap();
    assert_eq!(v.prerelease, Some("alpha".to_string()));
    assert_eq!(v.to_string(), "1.2.3-alpha");

    let v2 = SemanticVersion::parse("1.2.3-beta.1").unwrap();
    assert_eq!(v2.prerelease, Some("beta.1".to_string()));
}

#[test]
fn test_version_parse_with_build_metadata() {
    let v = SemanticVersion::parse("1.2.3+build.123").unwrap();
    assert_eq!(v.build_metadata, Some("build.123".to_string()));
    assert_eq!(v.to_string(), "1.2.3+build.123");
}

#[test]
fn test_version_parse_full() {
    let v = SemanticVersion::parse("1.2.3-alpha.1+build.456").unwrap();
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 2);
    assert_eq!(v.patch, 3);
    assert_eq!(v.prerelease, Some("alpha.1".to_string()));
    assert_eq!(v.build_metadata, Some("build.456".to_string()));
    assert_eq!(v.to_string(), "1.2.3-alpha.1+build.456");
}

#[test]
fn test_version_parse_errors() {
    assert!(matches!(
        SemanticVersion::parse("1.2"),
        Err(VersionParseError::InvalidFormat)
    ));
    assert!(matches!(
        SemanticVersion::parse("1.2.3.4"),
        Err(VersionParseError::InvalidFormat)
    ));
    assert!(SemanticVersion::parse("a.b.c").is_err());
    assert!(SemanticVersion::parse("").is_err());
    assert!(SemanticVersion::parse("1").is_err());
    assert!(SemanticVersion::parse("v1.2.3").is_err());
}

#[test]
fn test_version_ordering_major() {
    let v1 = SemanticVersion::new(1, 0, 0);
    let v2 = SemanticVersion::new(2, 0, 0);
    assert!(v1 < v2);
    assert!(v2 > v1);
    assert_eq!(v1.cmp(&v2), std::cmp::Ordering::Less);
}

#[test]
fn test_version_ordering_minor() {
    let v1 = SemanticVersion::new(1, 0, 0);
    let v2 = SemanticVersion::new(1, 1, 0);
    assert!(v1 < v2);
    assert!(v2 > v1);
}

#[test]
fn test_version_ordering_patch() {
    let v1 = SemanticVersion::new(1, 0, 0);
    let v2 = SemanticVersion::new(1, 0, 1);
    assert!(v1 < v2);
    assert!(v2 > v1);
}

#[test]
fn test_version_ordering_prerelease() {
    // Prerelease versions have lower precedence than normal versions
    let v_stable = SemanticVersion::new(1, 0, 0);
    let v_pre = SemanticVersion::with_prerelease(1, 0, 0, "alpha");
    assert!(v_pre < v_stable);
    assert!(v_stable > v_pre);

    // Alphabetical ordering of prerelease identifiers
    let v_alpha = SemanticVersion::with_prerelease(1, 0, 0, "alpha");
    let v_beta = SemanticVersion::with_prerelease(1, 0, 0, "beta");
    assert!(v_alpha < v_beta);
}

#[test]
fn test_version_equality() {
    let v1 = SemanticVersion::new(1, 2, 3);
    let v2 = SemanticVersion::new(1, 2, 3);
    assert_eq!(v1, v2);

    let v3 = SemanticVersion::with_prerelease(1, 2, 3, "alpha");
    let v4 = SemanticVersion::with_prerelease(1, 2, 3, "alpha");
    assert_eq!(v3, v4);
    assert_ne!(v1, v3);
}

#[test]
fn test_version_compatibility() {
    let v1_0_0 = SemanticVersion::new(1, 0, 0);
    let v1_1_0 = SemanticVersion::new(1, 1, 0);
    let v1_2_3 = SemanticVersion::new(1, 2, 3);
    let v2_0_0 = SemanticVersion::new(2, 0, 0);

    // Same major version is compatible (for major >= 1)
    assert!(v1_0_0.is_compatible_with(&v1_1_0));
    assert!(v1_1_0.is_compatible_with(&v1_2_3));
    assert!(v1_2_3.is_compatible_with(&v1_0_0));

    // Different major version is not compatible
    assert!(!v1_0_0.is_compatible_with(&v2_0_0));
    assert!(!v2_0_0.is_compatible_with(&v1_0_0));
}

#[test]
fn test_version_compatibility_zero_major() {
    // Version 0.x.x is not compatible with anything (initial development)
    let v0_1_0 = SemanticVersion::new(0, 1, 0);
    let v0_2_0 = SemanticVersion::new(0, 2, 0);
    let v0_1_1 = SemanticVersion::new(0, 1, 1);

    assert!(!v0_1_0.is_compatible_with(&v0_2_0));
    assert!(!v0_1_0.is_compatible_with(&v0_1_1));
    assert!(!v0_1_0.is_compatible_with(&v0_1_0)); // Not even with itself!
}

#[test]
fn test_version_increment_patch() {
    let v = SemanticVersion::new(1, 2, 3);
    let next = v.increment_patch();
    assert_eq!(next, SemanticVersion::new(1, 2, 4));

    // Prerelease and build metadata are removed
    let v_pre = SemanticVersion::parse("1.2.3-alpha+build").unwrap();
    let next_pre = v_pre.increment_patch();
    assert_eq!(next_pre, SemanticVersion::new(1, 2, 4));
    assert!(next_pre.is_stable());
}

#[test]
fn test_version_increment_minor() {
    let v = SemanticVersion::new(1, 2, 3);
    let next = v.increment_minor();
    assert_eq!(next, SemanticVersion::new(1, 3, 0));
    assert_eq!(next.patch, 0); // Patch is reset
}

#[test]
fn test_version_increment_major() {
    let v = SemanticVersion::new(1, 2, 3);
    let next = v.increment_major();
    assert_eq!(next, SemanticVersion::new(2, 0, 0));
    assert_eq!(next.minor, 0); // Minor is reset
    assert_eq!(next.patch, 0); // Patch is reset
}

#[test]
fn test_version_display() {
    assert_eq!(SemanticVersion::new(1, 2, 3).to_string(), "1.2.3");
    assert_eq!(
        SemanticVersion::with_prerelease(1, 2, 3, "alpha").to_string(),
        "1.2.3-alpha"
    );

    let mut v = SemanticVersion::new(1, 2, 3);
    v.build_metadata = Some("build.123".to_string());
    assert_eq!(v.to_string(), "1.2.3+build.123");
}

#[test]
fn test_version_from_str() {
    use std::str::FromStr;

    let v = SemanticVersion::from_str("1.2.3").unwrap();
    assert_eq!(v, SemanticVersion::new(1, 2, 3));

    let v_err = SemanticVersion::from_str("invalid");
    assert!(v_err.is_err());
}

#[test]
fn test_version_serialization() {
    let v = SemanticVersion::parse("1.2.3-alpha+build").unwrap();
    let json = serde_json::to_string(&v).unwrap();
    let deserialized: SemanticVersion = serde_json::from_str(&json).unwrap();
    assert_eq!(v, deserialized);
}

// Property-based tests using proptest
proptest! {
    #[test]
    fn test_version_parse_roundtrip(major in 0u32..100, minor in 0u32..100, patch in 0u32..100) {
        let v = SemanticVersion::new(major, minor, patch);
        let s = v.to_string();
        let parsed = SemanticVersion::parse(&s).unwrap();
        prop_assert_eq!(v, parsed);
    }

    #[test]
    fn test_version_ordering_transitive(
        a_major in 0u32..10,
        a_minor in 0u32..10,
        a_patch in 0u32..10,
        b_major in 0u32..10,
        b_minor in 0u32..10,
        b_patch in 0u32..10,
        c_major in 0u32..10,
        c_minor in 0u32..10,
        c_patch in 0u32..10,
    ) {
        let a = SemanticVersion::new(a_major, a_minor, a_patch);
        let b = SemanticVersion::new(b_major, b_minor, b_patch);
        let c = SemanticVersion::new(c_major, c_minor, c_patch);

        // If a <= b and b <= c, then a <= c (transitivity)
        if a <= b && b <= c {
            prop_assert!(a <= c);
        }
    }

    #[test]
    fn test_version_increment_increases(major in 1u32..100, minor in 0u32..100, patch in 0u32..100) {
        let v = SemanticVersion::new(major, minor, patch);

        let next_patch = v.increment_patch();
        prop_assert!(next_patch > v);

        let next_minor = v.increment_minor();
        prop_assert!(next_minor > v);

        let next_major = v.increment_major();
        prop_assert!(next_major > v);
    }
}
