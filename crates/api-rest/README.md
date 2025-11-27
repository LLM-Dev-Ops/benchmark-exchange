# LLM Benchmark Exchange REST API

A complete Axum-based REST API implementation for the LLM Benchmark Exchange platform.

## Features

- **Modern Async Framework**: Built with Axum for high performance and type safety
- **OpenAPI Documentation**: Automatic API documentation with Swagger UI
- **Authentication & Authorization**: JWT-based authentication with role-based access control
- **Comprehensive Error Handling**: Structured error responses with proper HTTP status codes
- **Rate Limiting**: Built-in rate limiting to prevent abuse
- **Request Tracing**: Request ID tracking for observability
- **Pagination**: Standardized pagination for list endpoints
- **Validation**: Automatic request validation using the `validator` crate
- **Middleware Stack**: Logging, compression, CORS, timeout handling

## Architecture

### Module Structure

```
src/
├── lib.rs                      # Library entry point
├── app.rs                      # Application builder
├── config.rs                   # Configuration management
├── state.rs                    # Application state
├── error.rs                    # Error handling
│
├── middleware/                 # HTTP middleware
│   ├── mod.rs
│   ├── auth.rs                 # Authentication
│   ├── logging.rs              # Request logging
│   ├── error_handler.rs        # Error response formatting
│   ├── rate_limit.rs           # Rate limiting
│   └── request_id.rs           # Request ID generation
│
├── extractors/                 # Custom Axum extractors
│   ├── mod.rs
│   ├── auth.rs                 # AuthenticatedUser extractor
│   ├── pagination.rs           # Pagination extractor
│   └── validated_json.rs       # JSON validation extractor
│
├── responses/                  # Response types
│   └── mod.rs                  # ApiResponse, PaginatedResponse, etc.
│
└── routes/                     # Route handlers
    ├── mod.rs
    ├── health.rs               # Health check endpoints
    └── v1/                     # API v1 routes
        ├── mod.rs
        ├── benchmarks.rs       # Benchmark management
        ├── submissions.rs      # Result submissions
        ├── leaderboards.rs     # Leaderboard queries
        ├── governance.rs       # Governance & proposals
        └── users.rs            # User & authentication
```

## API Endpoints

### Health Checks

- `GET /health` - Basic health check
- `GET /ready` - Readiness check with dependency status

### Authentication (`/api/v1`)

- `POST /auth/register` - Register new user
- `POST /auth/login` - Login and receive JWT token
- `GET /users/me` - Get current user profile
- `PUT /users/me` - Update current user profile
- `GET /users/:id` - Get user by ID
- `PATCH /users/:id/role` - Update user role (admin only)

### Benchmarks (`/api/v1`)

- `GET /benchmarks` - List benchmarks (paginated)
- `POST /benchmarks` - Create benchmark (contributor+)
- `GET /benchmarks/:id` - Get benchmark by ID
- `GET /benchmarks/slug/:slug` - Get benchmark by slug
- `PUT /benchmarks/:id` - Update benchmark
- `POST /benchmarks/:id/submit-for-review` - Submit for review
- `POST /benchmarks/:id/approve` - Approve benchmark (reviewer+)
- `POST /benchmarks/:id/reject` - Reject benchmark (reviewer+)
- `POST /benchmarks/:id/deprecate` - Deprecate benchmark (reviewer+)

### Submissions (`/api/v1`)

- `POST /benchmarks/:id/submissions` - Submit results
- `GET /submissions/:id` - Get submission details
- `GET /benchmarks/:id/submissions` - List benchmark submissions
- `POST /submissions/:id/request-verification` - Request verification
- `PATCH /submissions/:id/visibility` - Update visibility

### Leaderboards (`/api/v1`)

- `GET /benchmarks/:id/leaderboard` - Get benchmark leaderboard
- `GET /categories/:category/leaderboard` - Get category leaderboard
- `GET /models/compare` - Compare multiple models
- `GET /models/:id/history` - Get model submission history

### Governance (`/api/v1`)

- `GET /proposals` - List proposals (paginated)
- `POST /proposals` - Create proposal (contributor+)
- `GET /proposals/:id` - Get proposal details
- `POST /proposals/:id/vote` - Vote on proposal
- `POST /proposals/:id/comments` - Add comment to proposal

## Configuration

The API can be configured via environment variables:

```bash
# Server configuration
API_HOST=0.0.0.0
API_PORT=8080

# Security
JWT_SECRET=your-secret-key-here
JWT_EXPIRATION_SECONDS=86400

# CORS
CORS_ALLOWED_ORIGINS=*

# Limits
MAX_BODY_SIZE=10485760
REQUEST_TIMEOUT_SECONDS=30
RATE_LIMIT_PER_MINUTE=60

# Database
DB_POOL_SIZE=10

# Features
ENABLE_SWAGGER=true

# Logging
LOG_LEVEL=info
```

## Usage Example

```rust
use llm_benchmark_api_rest::{create_app, ApiConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration
    let config = ApiConfig::from_env()?;

    // Create application
    let app = create_app(config.clone()).await?;

    // Start server
    let listener = tokio::net::TcpListener::bind(config.server_address())
        .await?;

    println!("Server listening on {}", config.server_address());

    axum::serve(listener, app).await?;

    Ok(())
}
```

## Authentication

The API uses JWT (JSON Web Tokens) for authentication. To access protected endpoints:

1. Register or login to obtain a token
2. Include the token in the `Authorization` header:

```
Authorization: Bearer <your-token-here>
```

### Token Claims

```json
{
  "sub": "user-id",
  "role": "registered",
  "exp": 1234567890,
  "iat": 1234567890
}
```

## Authorization

The API implements role-based access control (RBAC) with the following roles:

- **Anonymous** (0) - Public access only
- **Registered** (1) - Can submit results
- **Contributor** (2) - Can propose benchmarks and vote
- **Reviewer** (3) - Can review and verify submissions
- **Admin** (4) - Full system access

## Middleware Stack

The middleware stack is applied in the following order (outer to inner):

1. **TraceLayer** - OpenTelemetry tracing
2. **CompressionLayer** - Response compression (gzip)
3. **CorsLayer** - CORS headers
4. **TimeoutLayer** - Request timeout
5. **RateLimitLayer** - Rate limiting
6. **RequestIdMiddleware** - Request ID generation
7. **LoggingMiddleware** - Request/response logging

## Error Handling

All errors are converted to standardized JSON responses:

```json
{
  "error": "ERROR_CODE",
  "message": "Human-readable error message",
  "details": { ... },
  "request_id": "abc-123"
}
```

HTTP status codes are automatically determined based on the error type:

- `400` - Bad Request (validation errors)
- `401` - Unauthorized (authentication required)
- `403` - Forbidden (insufficient permissions)
- `404` - Not Found
- `429` - Too Many Requests (rate limit)
- `500` - Internal Server Error
- `503` - Service Unavailable (database issues)

## OpenAPI Documentation

When `ENABLE_SWAGGER=true`, the API exposes:

- **Swagger UI**: `http://localhost:8080/swagger-ui`
- **OpenAPI JSON**: `http://localhost:8080/api-docs/openapi.json`

## Custom Extractors

### AuthenticatedUser

Extracts and validates JWT token, providing user information:

```rust
async fn handler(user: AuthenticatedUser) -> Result<Json<Response>> {
    if !user.can_review() {
        return Err(ApiError::Forbidden);
    }
    // ...
}
```

### Pagination

Extracts pagination parameters from query string:

```rust
async fn handler(pagination: Pagination) -> Result<Json<PaginatedResponse<Item>>> {
    let offset = pagination.offset();
    let limit = pagination.limit();
    // ...
}
```

### ValidatedJson

Automatically validates JSON payloads:

```rust
async fn handler(
    ValidatedJson(req): ValidatedJson<CreateRequest>
) -> Result<Json<Response>> {
    // req is already validated
}
```

## Response Types

### ApiResponse

Standard wrapper for successful responses:

```json
{
  "success": true,
  "data": { ... },
  "message": "Optional message"
}
```

### PaginatedResponse

Paginated list responses:

```json
{
  "items": [...],
  "pagination": {
    "page": 1,
    "per_page": 20,
    "total": 100,
    "total_pages": 5,
    "has_next": true,
    "has_prev": false
  }
}
```

## Testing

```bash
# Run tests
cargo test -p llm-benchmark-api-rest

# Run with coverage
cargo tarpaulin -p llm-benchmark-api-rest
```

## Development

### Adding New Endpoints

1. Create request/response types in the appropriate route module
2. Implement the handler function with `#[utoipa::path]` annotation
3. Register the route in the module's `routes()` function
4. Update OpenAPI documentation in `app.rs` if needed

### Adding Middleware

1. Create middleware module in `src/middleware/`
2. Implement as either a `Layer` or middleware function
3. Register in `app.rs` middleware stack

## Production Considerations

### Database Integration

The current implementation uses placeholder responses. To integrate with a real database:

1. Add database connection pool to `AppState`
2. Inject service layer instances (e.g., `BenchmarkService`)
3. Implement actual database queries in handlers

Example:

```rust
pub struct AppState {
    pub config: Arc<ApiConfig>,
    pub db_pool: Arc<sqlx::PgPool>,
    pub benchmark_service: Arc<dyn BenchmarkService>,
}
```

### Caching

Add Redis client to `AppState` for caching:

```rust
pub struct AppState {
    // ...
    pub redis: Arc<redis::Client>,
}
```

### Monitoring

- Configure OpenTelemetry exporters
- Add metrics collection
- Implement health checks for all dependencies

### Security

- Use strong JWT secrets in production
- Enable HTTPS/TLS
- Implement proper CORS configuration
- Add rate limiting per user/IP
- Validate all inputs
- Sanitize error messages (don't leak internal details)

## License

MIT
