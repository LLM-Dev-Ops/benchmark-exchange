# LLM Benchmark Exchange - gRPC API

This crate provides a complete Tonic-based gRPC API for the LLM Benchmark Exchange platform.

## Features

- **Complete gRPC Services**: BenchmarkService, SubmissionService, LeaderboardService, GovernanceService, UserService
- **Protocol Buffers**: Well-defined protobuf schemas for all operations
- **Interceptors**: Authentication, logging, and metrics collection
- **Type Conversions**: Seamless conversion between domain and protobuf types
- **TLS Support**: Optional TLS configuration for production deployments
- **Health Checks**: Built-in health service
- **Reflection**: Optional gRPC reflection for development

## Prerequisites

To build this crate, you need:

1. **Rust toolchain** (installed via rustup)
2. **Protocol Buffers compiler** (`protoc`):
   - Debian/Ubuntu: `apt-get install protobuf-compiler`
   - macOS: `brew install protobuf`
   - Download from: https://github.com/protocolbuffers/protobuf/releases

## Structure

```
api-grpc/
├── proto/                    # Protobuf definitions
│   ├── benchmark.proto       # Benchmark service
│   ├── submission.proto      # Submission service
│   ├── leaderboard.proto     # Leaderboard service
│   ├── governance.proto      # Governance service
│   └── user.proto           # User service
├── src/
│   ├── conversions/         # Domain <-> Proto type conversions
│   ├── interceptors/        # gRPC interceptors
│   │   ├── auth.rs         # Authentication
│   │   ├── logging.rs      # Request logging
│   │   └── metrics.rs      # Prometheus metrics
│   ├── services/           # Service implementations
│   │   ├── benchmark.rs    # Benchmark operations
│   │   ├── submission.rs   # Result submissions
│   │   ├── leaderboard.rs  # Rankings and comparisons
│   │   ├── governance.rs   # Proposals and voting
│   │   └── user.rs         # User management
│   ├── generated/          # Auto-generated from .proto files
│   ├── error.rs            # Error types and Status conversion
│   ├── server.rs           # Server setup and configuration
│   └── lib.rs              # Module exports
└── build.rs                # Proto compilation

## Usage

### Starting the Server

```rust
use llm_benchmark_api_grpc::{GrpcServer, ServerConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ServerConfig {
        addr: "0.0.0.0:50051".parse()?,
        enable_tls: false,
        enable_reflection: true,
        enable_health: true,
        ..Default::default()
    };

    let server = GrpcServer::new(config);
    server.serve().await?;

    Ok(())
}
```

### Client Example

```rust
use llm_benchmark_api_grpc::proto::{
    benchmark_service_client::BenchmarkServiceClient,
    ListBenchmarksRequest,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = BenchmarkServiceClient::connect("http://localhost:50051").await?;

    let request = tonic::Request::new(ListBenchmarksRequest {
        page: 1,
        page_size: 10,
        ..Default::default()
    });

    let response = client.list_benchmarks(request).await?;
    println!("Benchmarks: {:?}", response.into_inner());

    Ok(())
}
```

## Services

### BenchmarkService

Manages benchmark lifecycle:
- `CreateBenchmark`: Create new benchmark definitions
- `GetBenchmark`: Retrieve benchmark by ID
- `ListBenchmarks`: Query benchmarks with filters
- `UpdateBenchmark`: Modify existing benchmarks
- `SubmitForReview`: Submit for community review
- `ApproveBenchmark`: Approve benchmark (requires reviewer role)
- `RejectBenchmark`: Reject benchmark with reason

### SubmissionService

Handles result submissions:
- `SubmitResults`: Submit benchmark evaluation results
- `GetSubmission`: Retrieve submission details
- `ListSubmissions`: Query submissions with filters
- `RequestVerification`: Request independent verification

### LeaderboardService

Provides ranking data:
- `GetLeaderboard`: Get rankings for a benchmark
- `GetCategoryLeaderboard`: Get category-wide rankings
- `CompareModels`: Compare multiple model submissions

### GovernanceService

Community governance:
- `CreateProposal`: Create governance proposals
- `GetProposal`: Retrieve proposal details
- `ListProposals`: Query proposals
- `CastVote`: Vote on proposals

### UserService

User management:
- `Register`: Create new user account
- `Login`: Authenticate and get tokens
- `GetProfile`: Retrieve user profile
- `UpdateProfile`: Modify user information

## Configuration

### TLS Configuration

```rust
let config = ServerConfig {
    addr: "0.0.0.0:50051".parse()?,
    enable_tls: true,
    tls_cert_path: Some("/path/to/cert.pem".to_string()),
    tls_key_path: Some("/path/to/key.pem".to_string()),
    ..Default::default()
};
```

### Reflection (Development)

Enable with the `reflection` feature:

```toml
[dependencies]
llm-benchmark-api-grpc = { path = "crates/api-grpc", features = ["reflection"] }
```

## Interceptors

### Authentication

The `AuthInterceptor` validates JWT tokens from the `authorization` header:
- Public endpoints (Register, Login, GetLeaderboard, ListBenchmarks) are accessible without auth
- Protected endpoints require valid Bearer token
- User context is extracted and available to services

### Logging

The `LoggingInterceptor` logs all incoming requests with:
- Request ID (from `x-request-id` header)
- Method path
- Request metadata

### Metrics

The `MetricsInterceptor` records:
- Request counts by method
- Request duration (TODO: implement)
- Error rates (TODO: implement)

## Error Handling

All errors are converted to `tonic::Status` with appropriate gRPC status codes:

- `NOT_FOUND`: Resource not found
- `INVALID_ARGUMENT`: Invalid input
- `UNAUTHENTICATED`: Authentication required
- `PERMISSION_DENIED`: Insufficient permissions
- `ALREADY_EXISTS`: Conflict
- `INTERNAL`: Server error
- `UNAVAILABLE`: Service unavailable

## Development

### Building

```bash
cargo build -p llm-benchmark-api-grpc
```

### Testing

```bash
cargo test -p llm-benchmark-api-grpc
```

### Using grpcurl

```bash
# List services (requires reflection)
grpcurl -plaintext localhost:50051 list

# Call a method
grpcurl -plaintext -d '{"page": 1, "page_size": 10}' \
  localhost:50051 llm_benchmark.v1.BenchmarkService/ListBenchmarks
```

## TODO

- [ ] Connect service implementations to application layer
- [ ] Implement JWT token validation in AuthInterceptor
- [ ] Add Prometheus metrics in MetricsInterceptor
- [ ] Add request/response validation
- [ ] Add rate limiting
- [ ] Generate API documentation from proto files
- [ ] Add integration tests
- [ ] Add gRPC-web support for browser clients

## License

MIT
