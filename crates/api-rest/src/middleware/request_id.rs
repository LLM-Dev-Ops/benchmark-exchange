//! Request ID middleware.

use axum::{
    body::Body,
    http::{Request, Response},
    middleware::Next,
};
use tower::{Layer, Service};
use uuid::Uuid;

const REQUEST_ID_HEADER: &str = "x-request-id";

/// Layer that adds a request ID to each request
#[derive(Clone)]
pub struct RequestIdLayer;

impl<S> Layer<S> for RequestIdLayer {
    type Service = RequestIdService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequestIdService { inner }
    }
}

/// Service that adds request IDs
#[derive(Clone)]
pub struct RequestIdService<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for RequestIdService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        // Generate or use existing request ID
        let request_id = req
            .headers()
            .get(REQUEST_ID_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        // Store request ID in extensions for access in handlers
        req.extensions_mut().insert(request_id.clone());

        self.inner.call(req)
    }
}

/// Middleware function to add request ID
pub async fn request_id_middleware(
    mut req: Request<Body>,
    next: Next,
) -> Response<Body> {
    // Generate or use existing request ID
    let request_id = req
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Store in extensions
    req.extensions_mut().insert(request_id.clone());

    // Process request
    let mut response = next.run(req).await;

    // Add request ID to response headers
    response.headers_mut().insert(
        REQUEST_ID_HEADER,
        request_id.parse().unwrap(),
    );

    response
}
