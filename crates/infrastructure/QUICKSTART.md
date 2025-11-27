# Infrastructure Crate - Quick Start Guide

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
infrastructure = { path = "../infrastructure" }
tokio = { version = "1", features = ["full"] }
```

## Basic Setup

### 1. Database Connection

```rust
use infrastructure::database::{create_pool, DatabaseConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Using environment variable DATABASE_URL
    let config = DatabaseConfig::default();
    let pool = create_pool(config).await?;

    Ok(())
}
```

### 2. Repository Usage

```rust
use infrastructure::database::repositories::PostgresBenchmarkRepository;

// Create repository
let repo = PostgresBenchmarkRepository::new(pool.clone());

// Get benchmark
let benchmark = repo.get_by_id(benchmark_id).await?;

// List benchmarks
let query = BenchmarkQuery {
    category: Some("Performance".to_string()),
    page: 1,
    per_page: 20,
    ..Default::default()
};
let results = repo.list(query).await?;
```

### 3. Cache Operations

```rust
use infrastructure::cache::{RedisCacheProvider, CacheProvider, CacheKey};
use std::time::Duration;

// Create cache
let config = RedisCacheConfig::default();
let cache = RedisCacheProvider::new(config).await?;

// Set value
let key = CacheKey::benchmark("123");
cache.set(&key, &benchmark, Some(Duration::from_secs(3600))).await?;

// Get value
let cached: Option<Benchmark> = cache.get(&key).await?;

// Delete pattern
cache.delete_pattern("benchmark:*").await?;
```

### 4. Storage Operations

```rust
use infrastructure::storage::{S3StorageProvider, StorageProvider, StoragePath};
use bytes::Bytes;

// Create storage
let config = S3StorageConfig::default();
let storage = S3StorageProvider::new(config).await?;

// Upload file
let data = Bytes::from("file contents");
let key = StoragePath::benchmark_dataset("bench-123", "1.0.0", "data.json");
storage.upload("my-bucket", &key, data, Some("application/json")).await?;

// Download file
let downloaded = storage.download("my-bucket", &key).await?;

// Get presigned URL
let url = storage.get_presigned_url(
    "my-bucket",
    &key,
    Duration::from_secs(3600)
).await?;
```

### 5. Event Messaging

```rust
use infrastructure::messaging::{
    RedisEventBus, EventPublisher, EventSubscriber,
    EventChannel, BenchmarkEvent
};

// Create event bus
let config = RedisEventBusConfig::default();
let bus = RedisEventBus::new(config).await?;

// Publish event
let event = BenchmarkEvent::Created {
    benchmark_id: "123".to_string(),
    name: "Test".to_string(),
    created_by: "user1".to_string(),
};
bus.publish(EventChannel::BENCHMARK, &event).await?;

// Subscribe to events
bus.subscribe(EventChannel::BENCHMARK, |msg| {
    Box::pin(async move {
        if let Ok(event) = msg.deserialize::<BenchmarkEvent>() {
            println!("Event received: {:?}", event);
        }
    })
}).await?;
```

## Environment Variables

```bash
# Database
export DATABASE_URL="postgres://user:password@localhost/llm_benchmark_exchange"

# Cache
export REDIS_URL="redis://localhost:6379"

# Storage
export AWS_REGION="us-east-1"
export AWS_ACCESS_KEY_ID="your-key"
export AWS_SECRET_ACCESS_KEY="your-secret"
# Optional: for MinIO/LocalStack
export S3_ENDPOINT="http://localhost:9000"
```

## Common Patterns

### Caching with Fallback

```rust
async fn get_benchmark_cached(
    repo: &PostgresBenchmarkRepository,
    cache: &RedisCacheProvider,
    id: BenchmarkId,
) -> Result<Option<Benchmark>> {
    let key = CacheKey::benchmark(&id.to_string());

    // Try cache first
    if let Some(benchmark) = cache.get(&key).await? {
        return Ok(Some(benchmark));
    }

    // Fallback to database
    if let Some(benchmark) = repo.get_by_id(id).await? {
        // Update cache
        cache.set(&key, &benchmark, Some(Duration::from_secs(3600))).await?;
        return Ok(Some(benchmark));
    }

    Ok(None)
}
```

### Publishing Events After Database Changes

```rust
async fn create_benchmark_with_event(
    repo: &PostgresBenchmarkRepository,
    bus: &RedisEventBus,
    benchmark: &Benchmark,
) -> Result<BenchmarkId> {
    // Create in database
    let id = repo.create(benchmark).await?;

    // Publish event
    let event = BenchmarkEvent::Created {
        benchmark_id: id.to_string(),
        name: benchmark.name.clone(),
        created_by: benchmark.created_by.to_string(),
    };
    bus.publish(EventChannel::BENCHMARK, &event).await?;

    Ok(id)
}
```

### File Upload with Metadata

```rust
async fn upload_benchmark_dataset(
    storage: &S3StorageProvider,
    benchmark_id: &str,
    version: &str,
    filename: &str,
    data: Bytes,
) -> Result<String> {
    let key = StoragePath::benchmark_dataset(benchmark_id, version, filename);

    storage.upload(
        "benchmarks",
        &key,
        data,
        Some("application/json")
    ).await?;

    Ok(key)
}
```

## Error Handling

```rust
use infrastructure::Error;

match some_operation().await {
    Ok(result) => println!("Success: {:?}", result),
    Err(Error::Database(e)) => eprintln!("Database error: {}", e),
    Err(Error::Cache(e)) => eprintln!("Cache error: {}", e),
    Err(Error::Storage(e)) => eprintln!("Storage error: {}", e),
    Err(Error::Messaging(e)) => eprintln!("Messaging error: {}", e),
    Err(Error::NotFound(msg)) => eprintln!("Not found: {}", msg),
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires infrastructure services
    async fn test_benchmark_repository() {
        let config = DatabaseConfig::default();
        let pool = create_pool(config).await.unwrap();
        let repo = PostgresBenchmarkRepository::new(pool);

        // Your test here
    }
}
```

Run tests with infrastructure:
```bash
# Start services with Docker Compose
docker-compose up -d postgres redis minio

# Run ignored tests
cargo test -- --ignored

# Stop services
docker-compose down
```

## Best Practices

1. **Always use connection pools** - Don't create new connections for each operation
2. **Set appropriate TTLs** - Cache data based on update frequency
3. **Use presigned URLs** - For temporary file access instead of downloading through your service
4. **Handle errors gracefully** - Always match on specific error types
5. **Log important operations** - Use tracing for observability
6. **Use transactions** - For operations that need atomicity
7. **Pattern-based cache invalidation** - Clear related caches together
8. **Event-driven updates** - Use messaging for loose coupling

## Performance Tips

- **Database**: Use pagination, create indexes, use DISTINCT ON for latest versions
- **Cache**: Use batch operations, set reasonable TTLs, monitor hit rates
- **Storage**: Stream large files, use batch deletes, leverage CDN for public assets
- **Messaging**: Use pattern subscriptions wisely, handle messages asynchronously

## Troubleshooting

### Connection Errors
```rust
// Increase timeout
let config = DatabaseConfig {
    connect_timeout: Duration::from_secs(60),
    ..Default::default()
};
```

### Serialization Errors
```rust
// Enable debug logging
use tracing_subscriber;
tracing_subscriber::fmt::init();
```

### Cache Misses
```rust
// Check TTL and key naming
let exists = cache.exists(&key).await?;
println!("Key exists: {}", exists);
```

## Next Steps

- Read the full [README.md](README.md) for detailed API documentation
- Review [IMPLEMENTATION.md](IMPLEMENTATION.md) for architecture details
- Check repository implementations for SQL query examples
- See [tests/](tests/) for integration test examples

## Support

For issues or questions:
1. Check the documentation in this crate
2. Review the SPARC specification in `/plans`
3. Examine the implementation files for examples
