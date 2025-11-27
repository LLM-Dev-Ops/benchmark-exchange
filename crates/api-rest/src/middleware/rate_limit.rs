//! Rate limiting middleware.
//!
//! This is a simple in-memory rate limiter. In production, you would
//! want to use Redis or a similar distributed cache.

use axum::{
    body::Body,
    http::{Request, Response, StatusCode},
    response::IntoResponse,
};
use parking_lot::RwLock;
use std::{
    collections::HashMap,
    net::IpAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use tower::{Layer, Service};

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per window
    pub max_requests: u32,

    /// Time window duration
    pub window: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 60,
            window: Duration::from_secs(60),
        }
    }
}

/// Rate limiter state
#[derive(Debug)]
struct RateLimiter {
    requests: HashMap<IpAddr, Vec<Instant>>,
    config: RateLimitConfig,
}

impl RateLimiter {
    fn new(config: RateLimitConfig) -> Self {
        Self {
            requests: HashMap::new(),
            config,
        }
    }

    fn check_rate_limit(&mut self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let window_start = now - self.config.window;

        // Get or create request history for this IP
        let requests = self.requests.entry(ip).or_insert_with(Vec::new);

        // Remove old requests outside the window
        requests.retain(|&timestamp| timestamp > window_start);

        // Check if limit exceeded
        if requests.len() >= self.config.max_requests as usize {
            return false;
        }

        // Record this request
        requests.push(now);
        true
    }

    fn cleanup(&mut self) {
        let now = Instant::now();
        let window_start = now - self.config.window;

        // Remove IPs with no recent requests
        self.requests.retain(|_, requests| {
            requests.retain(|&timestamp| timestamp > window_start);
            !requests.is_empty()
        });
    }
}

/// Layer for rate limiting
#[derive(Clone)]
pub struct RateLimitLayer {
    limiter: Arc<RwLock<RateLimiter>>,
}

impl RateLimitLayer {
    /// Create a new rate limit layer with default config
    pub fn new() -> Self {
        Self::with_config(RateLimitConfig::default())
    }

    /// Create a new rate limit layer with custom config
    pub fn with_config(config: RateLimitConfig) -> Self {
        Self {
            limiter: Arc::new(RwLock::new(RateLimiter::new(config))),
        }
    }
}

impl Default for RateLimitLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitService {
            inner,
            limiter: self.limiter.clone(),
        }
    }
}

/// Service that performs rate limiting
#[derive(Clone)]
pub struct RateLimitService<S> {
    inner: S,
    limiter: Arc<RwLock<RateLimiter>>,
}

impl<S> Service<Request<Body>> for RateLimitService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = futures::future::BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let limiter = self.limiter.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // Extract client IP (in production, consider X-Forwarded-For)
            let ip = req
                .extensions()
                .get::<std::net::SocketAddr>()
                .map(|addr| addr.ip())
                .unwrap_or_else(|| IpAddr::from([127, 0, 0, 1]));

            // Check rate limit
            let allowed = {
                let mut limiter = limiter.write();
                limiter.check_rate_limit(ip)
            };

            if !allowed {
                // Rate limit exceeded
                let response = (
                    StatusCode::TOO_MANY_REQUESTS,
                    "Rate limit exceeded",
                )
                    .into_response();
                return Ok(response);
            }

            // Periodically cleanup old entries
            if rand::random::<f32>() < 0.01 {
                limiter.write().cleanup();
            }

            inner.call(req).await
        })
    }
}
