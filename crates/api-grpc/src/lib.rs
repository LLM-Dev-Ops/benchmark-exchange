//! gRPC API for LLM Benchmark Exchange
//!
//! This crate provides the gRPC API using Tonic framework.

pub mod conversions;
pub mod error;
pub mod interceptors;
pub mod server;
pub mod services;

pub use error::GrpcError;
pub use server::{GrpcServer, ServerConfig};

// Include generated proto code
pub mod proto {
    //! Generated protobuf code

    // Include all generated code from build.rs
    // The generated files will be placed in src/generated/ directory
    include!("generated/llm_benchmark.v1.rs");
}
