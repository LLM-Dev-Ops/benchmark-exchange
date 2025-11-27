# Infrastructure Crate

This crate provides infrastructure implementations for the LLM Benchmark Exchange platform, including database access, caching, object storage, and event messaging.

## Features

- **Database**: PostgreSQL access with type-safe repository implementations using sqlx
- **Cache**: Redis-based caching with key-value operations
- **Storage**: S3-compatible object storage for files and artifacts
- **Messaging**: Redis pub/sub for event-driven architecture

## Modules

### Database (`database`)

PostgreSQL database connectivity and repository implementations.

#### Usage

```rust
use infrastructure::database::{create_pool, DatabaseConfig};
use infrastructure::database::repositories::PostgresBenchmarkRepository;

// Create a database connection pool
let config = DatabaseConfig::default();
let pool = create_pool(config).await?;

// Create repository instances
let benchmark_repo = PostgresBenchmarkRepository::new(pool.clone());
```

#### Repositories

- **PostgresBenchmarkRepository**: Benchmark CRUD operations and querying
- **PostgresSubmissionRepository**: Submission management and leaderboard queries
- **PostgresUserRepository**: User account management
- **PostgresOrganizationRepository**: Organization and membership management

### Cache (`cache`)

Redis-based caching with support for TTL, atomic operations, and pattern-based deletion.

#### Usage

```rust
use infrastructure::cache::{RedisCacheProvider, CacheProvider, CacheKey};
use std::time::Duration;

// Create a Redis cache provider
let config = RedisCacheConfig::default();
let cache = RedisCacheProvider::new(config).await?;

// Set a value with TTL
let key = CacheKey::benchmark("bench-123");
cache.set(&key, &benchmark_data, Some(Duration::from_secs(3600))).await?;

// Get a value
let cached: Option<BenchmarkData> = cache.get(&key).await?;

// Delete by pattern
cache.delete_pattern("benchmark:*").await?;

// Atomic increment
let count = cache.increment("counter:submissions", 1).await?;

// Set if not exists (atomic)
let was_set = cache.set_nx(&key, &data, Some(Duration::from_secs(60))).await?;
```

#### Cache Key Utilities

The `CacheKey` builder provides consistent key naming:

```rust
CacheKey::benchmark("123")           // "benchmark:123"
CacheKey::leaderboard("bench-456")   // "leaderboard:bench-456"
CacheKey::user("user-789")           // "user:user-789"
CacheKey::session("sess-abc")        // "session:sess-abc"
CacheKey::rate_limit("user1", "api") // "ratelimit:user1:api"
```

### Storage (`storage`)

S3-compatible object storage for files, datasets, and artifacts.

#### Usage

```rust
use infrastructure::storage::{S3StorageProvider, StorageProvider, StoragePath};
use bytes::Bytes;
use std::time::Duration;

// Create an S3 storage provider
let config = S3StorageConfig::default();
let storage = S3StorageProvider::new(config).await?;

// Upload a file
let data = Bytes::from("file contents");
let key = StoragePath::benchmark_dataset("bench-123", "1.0.0", "data.json");
storage.upload("my-bucket", &key, data, Some("application/json")).await?;

// Download a file
let downloaded = storage.download("my-bucket", &key).await?;

// Generate presigned URL for temporary access
let url = storage.get_presigned_url(
    "my-bucket",
    &key,
    Duration::from_secs(3600)
).await?;

// Check if object exists
let exists = storage.exists("my-bucket", &key).await?;

// List objects with prefix
let keys = storage.list_objects(
    "my-bucket",
    Some("benchmarks/"),
    Some(100)
).await?;

// Get object metadata
let metadata = storage.get_metadata("my-bucket", &key).await?;
println!("Size: {} bytes", metadata.size);

// Batch delete (S3StorageProvider specific)
storage.delete_batch("my-bucket", vec![key1, key2, key3]).await?;

// Copy object (S3StorageProvider specific)
storage.copy("src-bucket", "src-key", "dst-bucket", "dst-key").await?;
```

#### Storage Path Utilities

The `StoragePath` builder provides consistent path naming:

```rust
StoragePath::benchmark_dataset("bench1", "1.0.0", "data.json")
// "benchmarks/bench1/1.0.0/data.json"

StoragePath::submission_results("sub123", "results.json")
// "submissions/sub123/results.json"

StoragePath::user_avatar("user456", "avatar.png")
// "avatars/user456/avatar.png"

StoragePath::verification_artifacts("ver789", "log.txt")
// "verifications/ver789/log.txt"
```

### Messaging (`messaging`)

Redis-based pub/sub for event-driven architecture.

#### Usage

```rust
use infrastructure::messaging::{
    RedisEventBus, EventPublisher, EventSubscriber,
    EventChannel, BenchmarkEvent
};

// Create an event bus
let config = RedisEventBusConfig::default();
let event_bus = RedisEventBus::new(config).await?;

// Publish an event
let event = BenchmarkEvent::Created {
    benchmark_id: "bench-123".to_string(),
    name: "Test Benchmark".to_string(),
    created_by: "user-456".to_string(),
};
event_bus.publish(EventChannel::BENCHMARK, &event).await?;

// Subscribe to events
event_bus.subscribe(EventChannel::BENCHMARK, |msg| {
    Box::pin(async move {
        match msg.deserialize::<BenchmarkEvent>() {
            Ok(event) => {
                println!("Received event: {:?}", event);
                // Handle the event
            }
            Err(e) => {
                eprintln!("Failed to deserialize event: {}", e);
            }
        }
    })
}).await?;

// Subscribe to multiple channels
event_bus.subscribe_many(
    &[EventChannel::BENCHMARK, EventChannel::SUBMISSION],
    |msg| {
        Box::pin(async move {
            println!("Received on {}: {}", msg.channel, msg.payload);
        })
    }
).await?;

// Pattern-based subscription (specific to RedisEventBus)
event_bus.psubscribe("events:benchmark:*", |msg| {
    Box::pin(async move {
        println!("Benchmark event: {}", msg.channel);
    })
}).await?;
```

#### Event Channels

Predefined event channels for different event types:

```rust
EventChannel::BENCHMARK      // "events:benchmark"
EventChannel::SUBMISSION     // "events:submission"
EventChannel::VERIFICATION   // "events:verification"
EventChannel::LEADERBOARD    // "events:leaderboard"
EventChannel::USER           // "events:user"
EventChannel::ORGANIZATION   // "events:organization"
EventChannel::GOVERNANCE     // "events:governance"
EventChannel::SYSTEM         // "events:system"

// Specific channels
EventChannel::benchmark_specific("123")  // "events:benchmark:123"
EventChannel::submission_specific("456") // "events:submission:456"
```

#### Event Types

Common event types are provided:

- `BenchmarkEvent`: Created, Updated, StatusChanged, Deprecated
- `SubmissionEvent`: Created, VerificationStarted, VerificationCompleted, LeaderboardPositionChanged
- `UserEvent`: Created, RoleChanged, ProfileUpdated
- `SystemEvent`: MaintenanceScheduled, HealthCheckFailed, CacheCleared

## Configuration

All infrastructure components can be configured via environment variables:

### Database
```bash
DATABASE_URL=postgres://user:password@localhost/llm_benchmark_exchange
```

### Cache
```bash
REDIS_URL=redis://localhost:6379
```

### Storage
```bash
AWS_REGION=us-east-1
S3_ENDPOINT=http://localhost:9000  # Optional, for MinIO/LocalStack
AWS_ACCESS_KEY_ID=your-access-key
AWS_SECRET_ACCESS_KEY=your-secret-key
```

## Error Handling

All operations return `Result<T, infrastructure::Error>` with the following error types:

- `Error::Database`: Database operation failures
- `Error::Cache`: Cache operation failures
- `Error::Storage`: Storage operation failures
- `Error::Messaging`: Event messaging failures
- `Error::Serialization`: JSON serialization/deserialization failures
- `Error::NotFound`: Resource not found
- `Error::Configuration`: Configuration errors

## Testing

The crate includes test utilities. Most tests are marked with `#[ignore]` as they require running infrastructure services:

```bash
# Run tests with infrastructure services running
cargo test -- --ignored

# Or use Docker Compose to start test infrastructure
docker-compose -f docker-compose.test.yml up -d
cargo test -- --ignored
docker-compose -f docker-compose.test.yml down
```

## Dependencies

- **sqlx**: Type-safe SQL queries for PostgreSQL
- **redis**: Redis client with async support
- **aws-sdk-s3**: S3-compatible object storage
- **tokio**: Async runtime
- **serde/serde_json**: Serialization
- **tracing**: Structured logging

## License

Apache 2.0

## Integration with Domain Crate

This infrastructure crate is designed to implement traits defined in the `domain` crate. Once the domain crate is created, update the imports to use the actual domain types instead of the placeholder types currently defined in each repository file.

## Future Enhancements

- Connection pooling optimizations
- Circuit breakers for external services
- Metrics and observability
- Distributed tracing
- Multi-region storage support
- Message queue alternatives (RabbitMQ, NATS)
