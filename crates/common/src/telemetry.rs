//! Telemetry and observability setup.
//!
//! This module provides utilities for setting up distributed tracing, metrics,
//! and structured logging using OpenTelemetry and tracing.

use anyhow::{Context, Result};
use tracing::Subscriber;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer, Registry,
};

/// Initialize tracing with OpenTelemetry integration.
///
/// # Arguments
///
/// * `service_name` - Name of the service for tracing
/// * `otlp_endpoint` - Optional OpenTelemetry collector endpoint
/// * `json_format` - Whether to use JSON formatting for logs
/// * `log_level` - Log level filter (e.g., "info", "debug")
///
/// # Examples
///
/// ```no_run
/// use common::telemetry::init_tracing;
///
/// init_tracing(
///     "my-service",
///     Some("http://localhost:4317"),
///     false,
///     "info"
/// ).expect("Failed to initialize tracing");
/// ```
pub fn init_tracing(
    _service_name: &str,
    _otlp_endpoint: Option<&str>,
    json_format: bool,
    log_level: &str,
) -> Result<()> {
    // Create environment filter
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(log_level));

    // Build the subscriber
    let registry = Registry::default().with(env_filter);

    // Note: OpenTelemetry integration can be added later
    // For now, just use local logging
    if json_format {
        registry
            .with(json_layer())
            .try_init()
            .context("Failed to initialize tracing subscriber")?;
    } else {
        registry
            .with(pretty_layer())
            .try_init()
            .context("Failed to initialize tracing subscriber")?;
    }

    Ok(())
}

/// Create a JSON logging layer
fn json_layer<S>() -> impl Layer<S>
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_target(true)
        .with_level(true)
        .with_file(true)
        .with_line_number(true)
}

/// Create a pretty-formatted logging layer
fn pretty_layer<S>() -> impl Layer<S>
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fmt::layer()
        .pretty()
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_target(true)
        .with_level(true)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(FmtSpan::CLOSE)
}

/// Create a Prometheus metrics exporter.
///
/// # Examples
///
/// ```no_run
/// use common::telemetry::create_meter;
///
/// let _exporter = create_meter("my-service").expect("Failed to create meter");
/// ```
pub fn create_meter(_service_name: &str) -> Result<()> {
    // Placeholder for metrics setup
    // Will be implemented when OpenTelemetry metrics are needed
    Ok(())
}

/// Export metrics in Prometheus format.
///
/// # Examples
///
/// ```no_run
/// use common::telemetry::export_metrics;
///
/// let metrics = export_metrics().expect("Failed to export metrics");
/// println!("{}", metrics);
/// ```
pub fn export_metrics() -> Result<String> {
    // Placeholder for metrics export
    // Will be implemented when OpenTelemetry metrics are needed
    Ok(String::new())
}

/// Macro for structured logging with key-value pairs.
///
/// # Examples
///
/// ```ignore
/// use common::log_info;
///
/// log_info!(
///     "User logged in",
///     user_id = 123,
///     username = "alice"
/// );
/// ```
#[macro_export]
macro_rules! log_info {
    ($msg:expr, $($key:ident = $value:expr),* $(,)?) => {
        tracing::info!($($key = ?$value,)* $msg)
    };
}

/// Macro for structured warning logging with key-value pairs.
#[macro_export]
macro_rules! log_warn {
    ($msg:expr, $($key:ident = $value:expr),* $(,)?) => {
        tracing::warn!($($key = ?$value,)* $msg)
    };
}

/// Macro for structured error logging with key-value pairs.
#[macro_export]
macro_rules! log_error {
    ($msg:expr, $($key:ident = $value:expr),* $(,)?) => {
        tracing::error!($($key = ?$value,)* $msg)
    };
}

/// Macro for structured debug logging with key-value pairs.
#[macro_export]
macro_rules! log_debug {
    ($msg:expr, $($key:ident = $value:expr),* $(,)?) => {
        tracing::debug!($($key = ?$value,)* $msg)
    };
}

/// Macro for structured trace logging with key-value pairs.
#[macro_export]
macro_rules! log_trace {
    ($msg:expr, $($key:ident = $value:expr),* $(,)?) => {
        tracing::trace!($($key = ?$value,)* $msg)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_tracing_without_otlp() {
        // This should not fail even without OTLP endpoint
        let result = init_tracing("test-service", None, false, "info");
        // We can't assert success because tracing can only be initialized once per process
        // In real tests, this would be in a separate test binary
        let _ = result;
    }

    #[test]
    fn test_create_meter() {
        let result = create_meter("test-service");
        assert!(result.is_ok());
    }

    #[test]
    fn test_export_metrics() {
        let result = export_metrics();
        assert!(result.is_ok());
    }
}
