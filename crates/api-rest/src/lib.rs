//! LLM Benchmark Exchange REST API
//!
//! This crate provides a complete Axum-based REST API for the LLM Benchmark Exchange platform.
//! It includes OpenAPI documentation, authentication, authorization, rate limiting, and
//! comprehensive error handling.
//!
//! ## Architecture
//!
//! The API is organized into the following modules:
//!
//! - **app**: Application builder and state management
//! - **routes**: HTTP route handlers organized by domain
//! - **middleware**: Request/response middleware (auth, logging, rate limiting)
//! - **extractors**: Custom Axum extractors for common patterns
//! - **responses**: Standardized response types
//! - **error**: HTTP error handling and conversion
//!
//! ## Usage
//!
//! ```rust,no_run
//! use llm_benchmark_api_rest::app::create_app;
//! use llm_benchmark_api_rest::config::ApiConfig;
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = ApiConfig::from_env().expect("Failed to load config");
//!     let app = create_app(config).await.expect("Failed to create app");
//!
//!     let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
//!         .await
//!         .expect("Failed to bind");
//!
//!     axum::serve(listener, app)
//!         .await
//!         .expect("Server error");
//! }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![allow(clippy::too_many_arguments)]

pub mod app;
pub mod config;
pub mod error;
pub mod extractors;
pub mod middleware;
pub mod responses;
pub mod routes;
pub mod state;

// Re-export commonly used types
pub use app::create_app;
pub use config::ApiConfig;
pub use error::{ApiError, ApiResult};
pub use state::AppState;
