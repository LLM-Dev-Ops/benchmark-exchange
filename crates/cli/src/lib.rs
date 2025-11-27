//! LLM Benchmark Exchange CLI Library
//!
//! This library provides the core functionality for the LLM Benchmark Exchange
//! command-line interface, including API client, configuration management,
//! and output formatting.

pub mod client;
pub mod commands;
pub mod config;
pub mod interactive;
pub mod output;

pub use client::ApiClient;
pub use config::Config;
pub use output::{JsonFormatter, OutputFormat, PlainFormatter, TableFormatter};

/// Re-export common types
pub use anyhow::{Context, Result};
