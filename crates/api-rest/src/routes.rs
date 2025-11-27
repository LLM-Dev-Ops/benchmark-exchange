//! HTTP route handlers.
//!
//! This module organizes all API endpoints by domain and version.

pub mod health;
pub mod v1;

// Re-export for convenience
pub use health::routes as health_routes;
pub use v1::routes as v1_routes;
