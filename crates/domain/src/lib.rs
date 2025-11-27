//! LLM Benchmark Exchange Domain Types
//!
//! This crate provides the core domain model for the LLM Benchmark Exchange platform.
//! It defines all domain entities, value objects, errors, and events using strongly-typed
//! Rust structures with comprehensive validation and serialization support.
//!
//! ## Architecture
//!
//! The domain layer is organized into the following modules:
//!
//! - **identifiers**: Strongly-typed UUID-based identifiers for all entities
//! - **version**: Semantic versioning implementation
//! - **benchmark**: Benchmark definitions, categories, and metadata
//! - **test_case**: Test case specifications and evaluation methods
//! - **evaluation**: Evaluation criteria, metrics, and execution configuration
//! - **submission**: Benchmark result submissions and verification
//! - **user**: User accounts, roles, and organizations
//! - **governance**: Community governance proposals and voting
//! - **events**: Domain events for event-driven architecture
//! - **errors**: Comprehensive error types with HTTP status codes
//! - **validation**: Validation result types
//!
//! ## Usage
//!
//! ```rust
//! use llm_benchmark_domain::{
//!     identifiers::BenchmarkId,
//!     version::SemanticVersion,
//!     benchmark::{BenchmarkCategory, BenchmarkStatus},
//! };
//!
//! // Create a new benchmark ID
//! let id = BenchmarkId::new();
//!
//! // Parse a semantic version
//! let version = SemanticVersion::parse("1.2.3").unwrap();
//!
//! // Work with categories
//! let category = BenchmarkCategory::Performance;
//! assert_eq!(category.display_name(), "Performance");
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![allow(clippy::too_many_arguments)]

// Core domain modules
pub mod identifiers;
pub mod version;
pub mod benchmark;
pub mod test_case;
pub mod evaluation;
pub mod submission;
pub mod user;
pub mod governance;
pub mod events;
pub mod errors;
pub mod validation;

// Re-export commonly used types
pub use identifiers::*;
pub use version::{SemanticVersion, VersionParseError};
pub use errors::{AppError, AppResult};
pub use validation::{ValidationResult, ValidationIssue, IssueSeverity};

// Re-export key domain types
pub use benchmark::{BenchmarkCategory, BenchmarkStatus, BenchmarkMetadata, LicenseType, Citation};
pub use user::{UserRole, OrganizationType, OrganizationRole};
pub use submission::{VerificationLevel, SubmissionVisibility};
pub use governance::{ProposalType, ProposalStatus, ProposalOutcome, Vote};
