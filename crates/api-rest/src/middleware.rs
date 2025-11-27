//! HTTP middleware components.
//!
//! This module provides middleware for request/response processing including:
//! - Request logging and tracing
//! - Error handling
//! - Rate limiting
//! - Request ID generation

pub mod error_handler;
pub mod logging;
pub mod rate_limit;
pub mod request_id;

pub use error_handler::handle_error;
pub use logging::logging_middleware;
pub use rate_limit::RateLimitLayer;
pub use request_id::RequestIdLayer;
