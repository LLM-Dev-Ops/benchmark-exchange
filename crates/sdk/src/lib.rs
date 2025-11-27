//! # LLM Benchmark Exchange SDK
//!
//! Official Rust SDK for the LLM Benchmark Exchange API.
//!
//! This SDK provides a type-safe, ergonomic interface for interacting with the
//! LLM Benchmark Exchange platform, allowing you to:
//!
//! - **Benchmarks**: Create, list, and manage benchmarks
//! - **Submissions**: Submit evaluation results and track verification status
//! - **Leaderboards**: View rankings and compare models
//! - **Governance**: Participate in community governance through proposals
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use llm_benchmark_sdk::{Client, ClientConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a client
//!     let client = Client::builder()
//!         .api_key("your-api-key")
//!         .build()?;
//!
//!     // List benchmarks
//!     let benchmarks = client.benchmarks().list().await?;
//!     for benchmark in benchmarks.items {
//!         println!("{}: {}", benchmark.name, benchmark.description);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Configuration
//!
//! The SDK can be configured through environment variables:
//!
//! - `LLM_BENCHMARK_API_URL`: API endpoint URL
//! - `LLM_BENCHMARK_API_KEY`: API key for authentication
//!
//! Or programmatically:
//!
//! ```rust,no_run
//! use llm_benchmark_sdk::{Client, ClientConfig};
//!
//! let client = Client::builder()
//!     .base_url("https://api.llm-benchmark.org")
//!     .api_key("your-api-key")
//!     .timeout(std::time::Duration::from_secs(30))
//!     .retry_count(3)
//!     .build()
//!     .unwrap();
//! ```
//!
//! ## Error Handling
//!
//! All operations return `Result<T, SdkError>` which provides detailed error
//! information including HTTP status codes and API error messages:
//!
//! ```rust,no_run
//! use llm_benchmark_sdk::{Client, SdkError};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # let client = Client::builder().api_key("key").build()?;
//! match client.benchmarks().get("invalid-id").await {
//!     Ok(benchmark) => println!("Found: {}", benchmark.name),
//!     Err(SdkError::NotFound { .. }) => println!("Benchmark not found"),
//!     Err(SdkError::Unauthorized { .. }) => println!("Invalid API key"),
//!     Err(e) => println!("Other error: {}", e),
//! }
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

pub mod client;
pub mod config;
pub mod error;
pub mod models;
pub mod services;

// Re-exports
pub use client::{Client, ClientBuilder};
pub use config::ClientConfig;
pub use error::{SdkError, SdkResult};
pub use models::*;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::client::{Client, ClientBuilder};
    pub use crate::config::ClientConfig;
    pub use crate::error::{SdkError, SdkResult};
    pub use crate::models::*;
    pub use crate::services::*;
}

/// SDK version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default API URL
pub const DEFAULT_API_URL: &str = "https://api.llm-benchmark.org";
