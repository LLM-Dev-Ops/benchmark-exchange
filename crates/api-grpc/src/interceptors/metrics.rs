//! Metrics interceptor for gRPC requests

use tonic::{Request, Status};
use tracing::debug;

/// Metrics interceptor
#[derive(Clone)]
pub struct MetricsInterceptor {
    // TODO: Add Prometheus metrics
}

impl MetricsInterceptor {
    /// Create a new metrics interceptor
    pub fn new() -> Self {
        Self {}
    }

    /// Intercept and record metrics
    pub fn intercept<T>(&self, req: Request<T>) -> Result<Request<T>, Status> {
        let method = req
            .metadata()
            .get("x-grpc-method")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown");

        // TODO: Increment request counter
        // GRPC_REQUESTS_TOTAL.with_label_values(&[method]).inc();

        debug!(method = %method, "Recording gRPC request metrics");

        // TODO: Start request duration timer
        // Store start time in request extensions for response-time metrics

        Ok(req)
    }
}

impl Default for MetricsInterceptor {
    fn default() -> Self {
        Self::new()
    }
}
