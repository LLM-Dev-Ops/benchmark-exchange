//! Strongly-typed identifier types for the LLM Benchmark Exchange domain.
//!
//! This module defines unique identifiers for all major domain entities, preventing
//! accidental mixing of different ID types through compile-time type safety.
//! All IDs use UUID v7 for time-ordering and distributed generation.

use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use uuid::Uuid;

macro_rules! define_id {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// Create a new ID with a time-ordered UUID v7
            #[inline]
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            /// Create an ID from an existing UUID
            #[inline]
            pub fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            /// Get a reference to the underlying UUID
            #[inline]
            pub fn as_uuid(&self) -> &Uuid {
                &self.0
            }

            /// Convert to the underlying UUID
            #[inline]
            pub fn into_uuid(self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<Uuid> for $name {
            fn from(uuid: Uuid) -> Self {
                Self(uuid)
            }
        }

        impl From<$name> for Uuid {
            fn from(id: $name) -> Self {
                id.0
            }
        }

        impl std::str::FromStr for $name {
            type Err = uuid::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(Uuid::parse_str(s)?))
            }
        }
    };
}

// Define all ID types
define_id!(
    BenchmarkId,
    "Unique identifier for benchmarks (UUID v7 for time-ordering)"
);

define_id!(
    BenchmarkVersionId,
    "Unique identifier for benchmark versions"
);

define_id!(SubmissionId, "Unique identifier for submissions");

define_id!(UserId, "Unique identifier for users");

define_id!(
    OrganizationId,
    "Unique identifier for organizations"
);

define_id!(
    ModelId,
    "Unique identifier for models being evaluated"
);

define_id!(LeaderboardId, "Unique identifier for leaderboards");

define_id!(ProposalId, "Unique identifier for governance proposals");

define_id!(
    VerificationId,
    "Unique identifier for verification runs"
);

define_id!(SubscriptionId, "Unique identifier for event subscriptions");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_creation() {
        let id = BenchmarkId::new();
        assert_ne!(id.to_string(), "");
    }

    #[test]
    fn test_id_equality() {
        let uuid = Uuid::now_v7();
        let id1 = BenchmarkId::from_uuid(uuid);
        let id2 = BenchmarkId::from_uuid(uuid);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_id_from_string() {
        let id1 = BenchmarkId::new();
        let s = id1.to_string();
        let id2: BenchmarkId = s.parse().unwrap();
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_id_serialization() {
        let id = UserId::new();
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: UserId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_different_id_types() {
        let uuid = Uuid::now_v7();
        let benchmark_id = BenchmarkId::from_uuid(uuid);
        let user_id = UserId::from_uuid(uuid);

        // This should not compile (different types):
        // assert_eq!(benchmark_id, user_id);

        // But their UUIDs are the same
        assert_eq!(benchmark_id.as_uuid(), user_id.as_uuid());
    }
}
