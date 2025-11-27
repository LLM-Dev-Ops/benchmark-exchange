//! Testing utilities for LLM Benchmark Exchange
//!
//! This crate provides comprehensive testing utilities including:
//! - Test fixtures for all domain types
//! - Builder patterns for complex test data construction
//! - Mock implementations of repositories and services
//! - Test database setup with testcontainers
//! - Property-based testing utilities
//!
//! # Examples
//!
//! ```
//! use llm_benchmark_testing::{fixtures::*, builders::*};
//!
//! // Create a test user
//! let user = create_test_user();
//!
//! // Build a custom benchmark
//! let benchmark = BenchmarkBuilder::new()
//!     .with_name("My Benchmark")
//!     .with_category(BenchmarkCategory::Performance)
//!     .build();
//! ```

pub mod builders;
pub mod database;
pub mod fixtures;
pub mod mocks;
pub mod testcontainers_ext;

// Re-export commonly used types
pub use builders::*;
pub use fixtures::*;
pub use mocks::*;

// Re-export testing dependencies for convenience
pub use fake;
pub use proptest;
pub use testcontainers;
pub use wiremock;
