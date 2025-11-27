//! Tests for status transitions and validation
//!
//! Tests BenchmarkStatus and ProposalStatus state machine transitions.

use llm_benchmark_domain::{
    benchmark::BenchmarkStatus,
    governance::ProposalStatus,
};

// ============================================================================
// BenchmarkStatus Tests
// ============================================================================

#[test]
fn test_benchmark_status_draft_transitions() {
    let draft = BenchmarkStatus::Draft;

    // Can transition to UnderReview
    assert!(draft.can_transition_to(BenchmarkStatus::UnderReview));

    // Cannot transition to other states directly
    assert!(!draft.can_transition_to(BenchmarkStatus::Active));
    assert!(!draft.can_transition_to(BenchmarkStatus::Deprecated));
    assert!(!draft.can_transition_to(BenchmarkStatus::Archived));
    assert!(!draft.can_transition_to(BenchmarkStatus::Draft)); // Cannot stay in draft
}

#[test]
fn test_benchmark_status_under_review_transitions() {
    let under_review = BenchmarkStatus::UnderReview;

    // Can transition to Active or back to Draft
    assert!(under_review.can_transition_to(BenchmarkStatus::Active));
    assert!(under_review.can_transition_to(BenchmarkStatus::Draft));

    // Cannot transition to other states
    assert!(!under_review.can_transition_to(BenchmarkStatus::Deprecated));
    assert!(!under_review.can_transition_to(BenchmarkStatus::Archived));
    assert!(!under_review.can_transition_to(BenchmarkStatus::UnderReview));
}

#[test]
fn test_benchmark_status_active_transitions() {
    let active = BenchmarkStatus::Active;

    // Can only transition to Deprecated
    assert!(active.can_transition_to(BenchmarkStatus::Deprecated));

    // Cannot transition to other states
    assert!(!active.can_transition_to(BenchmarkStatus::Draft));
    assert!(!active.can_transition_to(BenchmarkStatus::UnderReview));
    assert!(!active.can_transition_to(BenchmarkStatus::Archived));
    assert!(!active.can_transition_to(BenchmarkStatus::Active));
}

#[test]
fn test_benchmark_status_deprecated_transitions() {
    let deprecated = BenchmarkStatus::Deprecated;

    // Can transition to Archived or back to Active
    assert!(deprecated.can_transition_to(BenchmarkStatus::Archived));
    assert!(deprecated.can_transition_to(BenchmarkStatus::Active));

    // Cannot transition to other states
    assert!(!deprecated.can_transition_to(BenchmarkStatus::Draft));
    assert!(!deprecated.can_transition_to(BenchmarkStatus::UnderReview));
    assert!(!deprecated.can_transition_to(BenchmarkStatus::Deprecated));
}

#[test]
fn test_benchmark_status_archived_transitions() {
    let archived = BenchmarkStatus::Archived;

    // Cannot transition from Archived to any state (terminal state)
    assert!(!archived.can_transition_to(BenchmarkStatus::Draft));
    assert!(!archived.can_transition_to(BenchmarkStatus::UnderReview));
    assert!(!archived.can_transition_to(BenchmarkStatus::Active));
    assert!(!archived.can_transition_to(BenchmarkStatus::Deprecated));
    assert!(!archived.can_transition_to(BenchmarkStatus::Archived));
}

#[test]
fn test_benchmark_status_is_usable() {
    // Active benchmarks are usable
    assert!(BenchmarkStatus::Active.is_usable());

    // Deprecated benchmarks are still usable
    assert!(BenchmarkStatus::Deprecated.is_usable());

    // Other states are not usable for submissions
    assert!(!BenchmarkStatus::Draft.is_usable());
    assert!(!BenchmarkStatus::UnderReview.is_usable());
    assert!(!BenchmarkStatus::Archived.is_usable());
}

#[test]
fn test_benchmark_status_valid_workflow() {
    // Test a complete valid workflow
    let mut status = BenchmarkStatus::Draft;

    // Draft -> UnderReview
    assert!(status.can_transition_to(BenchmarkStatus::UnderReview));
    status = BenchmarkStatus::UnderReview;

    // UnderReview -> Active
    assert!(status.can_transition_to(BenchmarkStatus::Active));
    status = BenchmarkStatus::Active;
    assert!(status.is_usable());

    // Active -> Deprecated
    assert!(status.can_transition_to(BenchmarkStatus::Deprecated));
    status = BenchmarkStatus::Deprecated;
    assert!(status.is_usable());

    // Deprecated -> Archived
    assert!(status.can_transition_to(BenchmarkStatus::Archived));
    status = BenchmarkStatus::Archived;
    assert!(!status.is_usable());
}

#[test]
fn test_benchmark_status_invalid_transitions() {
    // Test some invalid transition paths

    // Cannot skip UnderReview
    assert!(!BenchmarkStatus::Draft.can_transition_to(BenchmarkStatus::Active));

    // Cannot go directly to Archived
    assert!(!BenchmarkStatus::Active.can_transition_to(BenchmarkStatus::Archived));
    assert!(!BenchmarkStatus::Draft.can_transition_to(BenchmarkStatus::Archived));

    // Cannot resurrect from Archived
    assert!(!BenchmarkStatus::Archived.can_transition_to(BenchmarkStatus::Active));
}

#[test]
fn test_benchmark_status_serialization() {
    use serde_json;

    let status = BenchmarkStatus::Active;
    let json = serde_json::to_string(&status).unwrap();
    assert_eq!(json, "\"active\"");

    let deserialized: BenchmarkStatus = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, BenchmarkStatus::Active);
}

// ============================================================================
// ProposalStatus Tests
// ============================================================================

#[test]
fn test_proposal_status_values() {
    // Ensure all expected states exist
    let _draft = ProposalStatus::Draft;
    let _under_review = ProposalStatus::UnderReview;
    let _voting = ProposalStatus::Voting;
    let _approved = ProposalStatus::Approved;
    let _rejected = ProposalStatus::Rejected;
    let _withdrawn = ProposalStatus::Withdrawn;
}

#[test]
fn test_proposal_status_equality() {
    assert_eq!(ProposalStatus::Draft, ProposalStatus::Draft);
    assert_ne!(ProposalStatus::Draft, ProposalStatus::UnderReview);
    assert_ne!(ProposalStatus::Approved, ProposalStatus::Rejected);
}

#[test]
fn test_proposal_status_serialization() {
    use serde_json;

    let status = ProposalStatus::Voting;
    let json = serde_json::to_string(&status).unwrap();
    assert_eq!(json, "\"voting\"");

    let deserialized: ProposalStatus = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, ProposalStatus::Voting);
}

#[test]
fn test_proposal_status_all_variants() {
    // Test serialization of all variants
    use serde_json;

    let statuses = vec![
        (ProposalStatus::Draft, "\"draft\""),
        (ProposalStatus::UnderReview, "\"under_review\""),
        (ProposalStatus::Voting, "\"voting\""),
        (ProposalStatus::Approved, "\"approved\""),
        (ProposalStatus::Rejected, "\"rejected\""),
        (ProposalStatus::Withdrawn, "\"withdrawn\""),
    ];

    for (status, expected_json) in statuses {
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, expected_json);

        let deserialized: ProposalStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, status);
    }
}

// ============================================================================
// Cross-status Integration Tests
// ============================================================================

#[test]
fn test_all_benchmark_statuses_are_unique() {
    use std::collections::HashSet;

    let statuses = vec![
        BenchmarkStatus::Draft,
        BenchmarkStatus::UnderReview,
        BenchmarkStatus::Active,
        BenchmarkStatus::Deprecated,
        BenchmarkStatus::Archived,
    ];

    // Check uniqueness by comparing each pair
    for (i, s1) in statuses.iter().enumerate() {
        for (j, s2) in statuses.iter().enumerate() {
            if i != j {
                assert_ne!(s1, s2, "Status {:?} and {:?} should be different", s1, s2);
            }
        }
    }
}

#[test]
fn test_all_proposal_statuses_are_unique() {
    let statuses = vec![
        ProposalStatus::Draft,
        ProposalStatus::UnderReview,
        ProposalStatus::Voting,
        ProposalStatus::Approved,
        ProposalStatus::Rejected,
        ProposalStatus::Withdrawn,
    ];

    // Check uniqueness by comparing each pair
    for (i, s1) in statuses.iter().enumerate() {
        for (j, s2) in statuses.iter().enumerate() {
            if i != j {
                assert_ne!(s1, s2, "Status {:?} and {:?} should be different", s1, s2);
            }
        }
    }
}
