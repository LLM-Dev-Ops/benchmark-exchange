# gRPC API Architecture

This document describes the architecture and implementation of the gRPC API crate.

## Overview

The `llm-benchmark-api-grpc` crate provides a production-ready gRPC API layer for the LLM Benchmark Exchange platform using the Tonic framework. It exposes all platform functionality through five main services.

## Architecture Layers

```
┌─────────────────────────────────────────────────────────────┐
│                     gRPC Clients                             │
│           (Rust, Go, Python, JavaScript, etc.)              │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                    Server (server.rs)                        │
│  • TLS Configuration                                        │
│  • Service Registration                                     │
│  • Health & Reflection                                      │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                 Interceptors (interceptors/)                 │
│  • AuthInterceptor: JWT validation                          │
│  • LoggingInterceptor: Request logging                      │
│  • MetricsInterceptor: Prometheus metrics                   │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│              Service Implementations (services/)             │
│  • BenchmarkServiceImpl                                     │
│  • SubmissionServiceImpl                                    │
│  • LeaderboardServiceImpl                                   │
│  • GovernanceServiceImpl                                    │
│  • UserServiceImpl                                          │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│             Type Conversions (conversions/)                  │
│  Domain Types ←→ Protobuf Types                            │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│              Application Layer Services                      │
│  (from llm-benchmark-application crate)                     │
└─────────────────────────────────────────────────────────────┘
```

## Protocol Buffers Schema

### Package Structure

All protobuf definitions use the package `llm_benchmark.v1`:

```protobuf
syntax = "proto3";
package llm_benchmark.v1;
```

### Service Definitions

#### BenchmarkService
Manages the complete lifecycle of benchmarks from creation through review and activation.

**Key Methods:**
- `CreateBenchmark`: Initialize new benchmark (Draft state)
- `SubmitForReview`: Transition to review process
- `ApproveBenchmark`: Activate benchmark (requires reviewer role)
- `RejectBenchmark`: Reject with feedback

**State Transitions:**
```
Draft → UnderReview → Active → Deprecated → Archived
         ↓
       Draft (on rejection)
```

#### SubmissionService
Handles benchmark result submissions and verification workflow.

**Key Methods:**
- `SubmitResults`: Submit evaluation results with full metadata
- `RequestVerification`: Queue for independent verification

**Verification Levels:**
```
Unverified → CommunityVerified → PlatformVerified → Audited
```

#### LeaderboardService
Provides read-only access to rankings and comparisons.

**Features:**
- Per-benchmark leaderboards with filtering
- Category-wide rankings
- Statistical model comparisons
- Confidence intervals and significance tests

#### GovernanceService
Implements community governance through proposals and voting.

**Proposal Types:**
- NewBenchmark: Community-contributed benchmarks
- UpdateBenchmark: Modifications to existing benchmarks
- DeprecateBenchmark: Lifecycle management
- PolicyChange: Platform governance changes

**Voting Process:**
```
Draft → UnderReview → Voting → [Approved/Rejected]
```

#### UserService
Authentication and profile management.

**Authentication Flow:**
1. `Register`: Create account + verify email
2. `Login`: Validate credentials + issue JWT
3. API calls: Use JWT in Authorization header
4. Token refresh: Separate endpoint (TODO)

## Implementation Details

### Build Process

The `build.rs` script uses `tonic-build` to generate Rust code from `.proto` files:

1. Read all `.proto` files from `proto/` directory
2. Generate server and client code
3. Output to `src/generated/`
4. Include via `include!` macro in `lib.rs`

**Generated Files:**
- `llm_benchmark.v1.rs`: All message and service definitions
- Request/Response types
- Service traits for implementation

### Type Conversions

The `conversions` module provides bidirectional conversions:

```rust
// Domain → Proto
impl From<BenchmarkCategory> for proto::BenchmarkCategory { ... }

// Proto → Domain
impl From<proto::BenchmarkCategory> for BenchmarkCategory { ... }
```

**Design Principles:**
- Use `From`/`Into` traits for infallible conversions
- Use `TryFrom`/`TryInto` for fallible conversions (validation)
- Preserve all semantic information
- Handle Optional fields via `google.protobuf.Wrappers`

### Error Handling

All errors flow through the `GrpcError` type and convert to `tonic::Status`:

```rust
pub enum GrpcError {
    NotFound(String),           // → Code::NotFound
    InvalidArgument(String),    // → Code::InvalidArgument
    Unauthorized(String),       // → Code::Unauthenticated
    PermissionDenied(String),   // → Code::PermissionDenied
    AlreadyExists(String),      // → Code::AlreadyExists
    Internal(String),           // → Code::Internal
    ServiceUnavailable(String), // → Code::Unavailable
    Application(ApplicationError), // → Mapped by variant
}
```

### Interceptor Chain

Interceptors execute in order for each request:

1. **AuthInterceptor**: Validate JWT, extract user context
2. **LoggingInterceptor**: Log request with metadata
3. **MetricsInterceptor**: Record request metrics

**Request Context:**

User context from auth is stored in request extensions:

```rust
struct UserContext {
    user_id: String,
    role: String,
}

// In service method:
let user = req.extensions().get::<UserContext>();
```

### Server Configuration

The `ServerConfig` struct provides comprehensive configuration:

```rust
pub struct ServerConfig {
    pub addr: SocketAddr,                    // Bind address
    pub enable_tls: bool,                    // TLS on/off
    pub tls_cert_path: Option<String>,       // Certificate path
    pub tls_key_path: Option<String>,        // Private key path
    pub enable_reflection: bool,             // gRPC reflection
    pub enable_health: bool,                 // Health service
    pub max_concurrent_streams: Option<u32>, // HTTP/2 streams
    pub tcp_keepalive: Option<Duration>,     // TCP keepalive
    pub timeout: Option<Duration>,           // Request timeout
}
```

**Production Settings:**
- Enable TLS with valid certificates
- Disable reflection (security)
- Enable health checks
- Set reasonable timeouts (30-60s)
- Configure keepalive (60-120s)
- Limit concurrent streams (1000-5000)

## Service Implementation Pattern

Each service follows this pattern:

```rust
#[derive(Debug, Clone)]
pub struct BenchmarkServiceImpl {
    // Application service dependencies
    // benchmark_service: Arc<BenchmarkService>,
    // repository: Arc<dyn BenchmarkRepository>,
}

#[tonic::async_trait]
impl BenchmarkService for BenchmarkServiceImpl {
    async fn create_benchmark(
        &self,
        request: Request<CreateBenchmarkRequest>,
    ) -> Result<Response<CreateBenchmarkResponse>, Status> {
        // 1. Extract request
        let req = request.into_inner();

        // 2. Get user context
        // let user = request.extensions().get::<UserContext>();

        // 3. Validate input
        // validate_request(&req)?;

        // 4. Convert proto → domain
        // let domain_request = convert_request(req);

        // 5. Call application service
        // let result = self.benchmark_service.create(domain_request).await?;

        // 6. Convert domain → proto
        // let proto_response = convert_response(result);

        // 7. Return response
        Ok(Response::new(proto_response))
    }
}
```

## Testing Strategy

### Unit Tests
- Test type conversions (domain ↔ proto)
- Test error mapping
- Test validation logic

### Integration Tests
- Start test server on random port
- Make gRPC calls via client
- Verify responses
- Test error scenarios

### Example:
```rust
#[tokio::test]
async fn test_create_benchmark() {
    let addr = "127.0.0.1:50052".parse().unwrap();
    let server = start_test_server(addr).await;

    let mut client = BenchmarkServiceClient::connect(
        format!("http://{}", addr)
    ).await.unwrap();

    let request = CreateBenchmarkRequest { /* ... */ };
    let response = client.create_benchmark(request).await.unwrap();

    assert!(response.into_inner().benchmark.is_some());
}
```

## Performance Considerations

### Message Size Limits
```rust
BenchmarkServiceServer::new(service)
    .max_decoding_message_size(64 * 1024 * 1024)  // 64MB
    .max_encoding_message_size(64 * 1024 * 1024)
```

Large submissions may exceed default limits (4MB). Configure per-service.

### Streaming

For large result sets, consider server streaming:

```protobuf
rpc StreamSubmissions(StreamSubmissionsRequest)
    returns (stream Submission);
```

### Connection Pooling

Clients should reuse connections:

```rust
// Create once, reuse
let client = BenchmarkServiceClient::connect("...").await?;

// Make multiple calls
for req in requests {
    client.create_benchmark(req).await?;
}
```

## Security

### TLS Configuration

**Production Requirements:**
- Use valid TLS certificates (not self-signed)
- Enable TLS 1.3 minimum
- Configure certificate rotation
- Use strong cipher suites

### Authentication

**JWT Tokens:**
- Include in `Authorization: Bearer <token>` header
- Validate signature and expiration
- Extract claims (user_id, role, permissions)
- Store in request context

### Authorization

**Role-Based Access Control:**
- Anonymous: Read-only public data
- Registered: Submit results, view profile
- Contributor: Create proposals, vote
- Reviewer: Approve benchmarks, verify results
- Admin: System configuration, user management

### Rate Limiting

**TODO:** Implement rate limiting per:
- IP address
- User ID
- API key

## Monitoring

### Health Checks

The health service reports:
- Overall server health
- Per-service health
- Dependencies (database, cache)

**Kubernetes Integration:**
```yaml
livenessProbe:
  grpc:
    port: 50051
    service: grpc.health.v1.Health
readinessProbe:
  grpc:
    port: 50051
    service: grpc.health.v1.Health
```

### Metrics

**Prometheus Metrics (TODO):**
- `grpc_requests_total{method, status}`
- `grpc_request_duration_seconds{method}`
- `grpc_request_size_bytes{method}`
- `grpc_response_size_bytes{method}`

### Logging

Structured logging with tracing:
```rust
info!(
    request_id = %request_id,
    method = %method,
    user_id = %user_id,
    "Processing request"
);
```

## Future Enhancements

1. **Bidirectional Streaming**: For real-time benchmark execution
2. **gRPC-Web**: Browser client support
3. **API Versioning**: Multiple proto versions (v1, v2)
4. **Circuit Breaker**: Resilience patterns
5. **Request Tracing**: OpenTelemetry integration
6. **API Gateway**: Kong/Envoy integration
7. **Load Balancing**: Client-side load balancing
8. **Service Mesh**: Istio/Linkerd integration

## References

- [Tonic Documentation](https://github.com/hyperium/tonic)
- [Protocol Buffers Guide](https://developers.google.com/protocol-buffers)
- [gRPC Best Practices](https://grpc.io/docs/guides/performance/)
- [Service Mesh Patterns](https://servicemesh.io/)
