//! Logging interceptor for gRPC requests

use tonic::{Request, Status};
use tracing::{debug, info, Span};

/// Logging interceptor
#[derive(Clone)]
pub struct LoggingInterceptor {
    // Configuration options
}

impl LoggingInterceptor {
    /// Create a new logging interceptor
    pub fn new() -> Self {
        Self {}
    }

    /// Intercept and log request
    pub fn intercept<T>(&self, req: Request<T>) -> Result<Request<T>, Status> {
        // Extract metadata
        let metadata = req.metadata();
        let request_id = metadata
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown");

        let method = metadata
            .get("x-grpc-method")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown");

        // Log the incoming request
        info!(
            request_id = %request_id,
            method = %method,
            "Incoming gRPC request"
        );

        // Add request info to current span
        let span = Span::current();
        span.record("grpc.method", method);
        span.record("request_id", request_id);

        debug!(
            request_id = %request_id,
            method = %method,
            "Request details"
        );

        Ok(req)
    }
}

impl Default for LoggingInterceptor {
    fn default() -> Self {
        Self::new()
    }
}
