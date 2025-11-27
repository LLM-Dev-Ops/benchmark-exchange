# Worker Quick Start Guide

## Prerequisites

- Rust 1.75+ with Cargo
- Redis server running
- PostgreSQL database (optional)

## Build

```bash
cargo build --release -p llm-benchmark-worker
```

## Run

### Development

```bash
# With default settings (requires Redis on localhost:6379)
cargo run -p llm-benchmark-worker

# With custom Redis URL
cargo run -p llm-benchmark-worker -- --redis-url redis://redis-server:6379

# With more workers
cargo run -p llm-benchmark-worker -- --workers 8

# With database
cargo run -p llm-benchmark-worker -- \
  --redis-url redis://localhost:6379 \
  --database-url postgresql://localhost/benchmark \
  --workers 4
```

### Production

```bash
# Using config file
./target/release/worker --config /etc/llm-benchmark/worker.json

# Using environment variables
export REDIS_URL=redis://localhost:6379
export DATABASE_URL=postgresql://localhost/benchmark
export WORKER_POOL_SIZE=8
export RUST_LOG=info
./target/release/worker
```

## Testing

```bash
# Unit tests (no Redis required)
cargo test -p llm-benchmark-worker

# Integration tests (requires Redis)
docker run -d -p 6379:6379 redis:latest
cargo test -p llm-benchmark-worker -- --ignored

# All tests with logging
RUST_LOG=debug cargo test -p llm-benchmark-worker -- --nocapture
```

## Enqueue Jobs

### Using the Producer API

```rust
use llm_benchmark_worker::{JobProducer, queue::*};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create producer
    let mut producer = JobProducer::new("redis://localhost:6379").await?;

    // Enqueue verification job
    let job = producer.enqueue(
        JobType::VerifySubmission(VerifySubmissionJob {
            submission_id: uuid::Uuid::new_v4(),
            benchmark_id: uuid::Uuid::new_v4(),
        })
    ).await?;

    println!("Job enqueued: {}", job.id);

    Ok(())
}
```

### Using Redis CLI

```bash
# Add a job to the normal priority queue
redis-cli LPUSH "llm-benchmark:jobs:normal" '{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "job_type": {
    "type": "VerifySubmission",
    "data": {
      "submission_id": "550e8400-e29b-41d4-a716-446655440001",
      "benchmark_id": "550e8400-e29b-41d4-a716-446655440002"
    }
  },
  "priority": "Normal",
  "status": "Queued",
  "retry_count": 0,
  "max_retries": 3,
  "created_at": "2024-01-15T10:00:00Z",
  "scheduled_at": "2024-01-15T10:00:00Z",
  "started_at": null,
  "completed_at": null,
  "error": null
}'
```

## Monitor

### View Logs

```bash
# If running with systemd
journalctl -u llm-benchmark-worker -f

# If running directly
RUST_LOG=info cargo run -p llm-benchmark-worker
```

### Check Queue Sizes

```bash
# Check all queues
redis-cli LLEN "llm-benchmark:jobs:critical"
redis-cli LLEN "llm-benchmark:jobs:high"
redis-cli LLEN "llm-benchmark:jobs:normal"
redis-cli LLEN "llm-benchmark:jobs:low"

# Check DLQ
redis-cli LLEN "llm-benchmark:jobs:dlq"

# Check delayed jobs
redis-cli ZCARD "llm-benchmark:jobs:delayed"
```

### View Metrics

Metrics are logged every 60 seconds (configurable with `--metrics-interval`):

```json
{
  "timestamp": "2024-01-15T10:00:00Z",
  "level": "INFO",
  "fields": {
    "jobs_processed": 1234,
    "jobs_succeeded": 1200,
    "jobs_failed": 34,
    "success_rate": "97.24%",
    "avg_duration_ms": 150
  },
  "target": "llm_benchmark_worker"
}
```

## Troubleshooting

### Jobs Not Processing

1. **Check Redis connection**
   ```bash
   redis-cli -u redis://localhost:6379 PING
   ```

2. **Check queue sizes**
   ```bash
   redis-cli LLEN "llm-benchmark:jobs:normal"
   ```

3. **Check worker logs**
   ```bash
   RUST_LOG=debug cargo run -p llm-benchmark-worker
   ```

### High Failure Rate

1. **Check DLQ**
   ```bash
   redis-cli LRANGE "llm-benchmark:jobs:dlq" 0 10
   ```

2. **Review error logs**
   Look for patterns in error messages

3. **Adjust retry policy**
   Increase `max_retries` or `max_backoff` in config

### Memory Issues

1. **Reduce pool size**
   ```bash
   worker --workers 2
   ```

2. **Check for memory leaks**
   Monitor process memory over time

3. **Increase limits**
   Update systemd service file `LimitNOFILE` and `LimitNPROC`

## Common Operations

### Retry Failed Jobs

```rust
use llm_benchmark_worker::JobConsumer;

let consumer = JobConsumer::new("redis://localhost:6379", 4).await?;

// Get DLQ jobs
let dlq_jobs = consumer.get_dlq_jobs(100).await?;

// Retry specific job
for job in dlq_jobs {
    if should_retry(&job) {
        consumer.retry_dlq_job(&job.id).await?;
    }
}
```

### Clear All Queues

```bash
# Clear all job queues (CAUTION!)
redis-cli DEL "llm-benchmark:jobs:critical"
redis-cli DEL "llm-benchmark:jobs:high"
redis-cli DEL "llm-benchmark:jobs:normal"
redis-cli DEL "llm-benchmark:jobs:low"
redis-cli DEL "llm-benchmark:jobs:dlq"
redis-cli DEL "llm-benchmark:jobs:delayed"
```

### Graceful Shutdown

1. Send SIGTERM or SIGINT (Ctrl+C)
2. Worker completes current jobs
3. Exits cleanly

```bash
# Send signal
kill -SIGTERM <worker-pid>

# Or use systemd
systemctl stop llm-benchmark-worker
```

## Performance Tuning

### Optimal Pool Size

```bash
# Rule of thumb: 1-2x CPU cores for I/O bound jobs
worker --workers $(nproc)

# For CPU-bound jobs, use CPU core count
worker --workers 4
```

### Redis Optimization

```ini
# redis.conf
maxmemory 2gb
maxmemory-policy allkeys-lru
timeout 300
tcp-keepalive 60
```

### Database Connection Pool

For workers that use database:
- Pool size: 2-4 connections per worker
- Connection timeout: 5-10 seconds
- Statement timeout: 30 seconds

## Integration Examples

### With REST API

```rust
// In your API handler
use llm_benchmark_worker::{JobProducer, queue::*};

async fn submit_benchmark(
    producer: Arc<Mutex<JobProducer>>,
    submission: Submission,
) -> Result<Json<Response>> {
    // Enqueue verification job
    let job = producer.lock().await.enqueue(
        JobType::VerifySubmission(VerifySubmissionJob {
            submission_id: submission.id,
            benchmark_id: submission.benchmark_id,
        })
    ).await?;

    Ok(Json(Response {
        job_id: job.id,
        status: "queued",
    }))
}
```

### With gRPC

```rust
use llm_benchmark_worker::{JobProducer, queue::*};

impl BenchmarkService for MyService {
    async fn submit_benchmark(
        &self,
        request: Request<SubmitRequest>,
    ) -> Result<Response<SubmitResponse>, Status> {
        let req = request.into_inner();

        let job = self.producer.lock().await
            .enqueue_with_priority(
                JobType::VerifySubmission(VerifySubmissionJob {
                    submission_id: Uuid::parse_str(&req.submission_id)?,
                    benchmark_id: Uuid::parse_str(&req.benchmark_id)?,
                }),
                JobPriority::High,
            )
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(SubmitResponse {
            job_id: job.id.to_string(),
        }))
    }
}
```

## Next Steps

- Read the [full README](README.md) for detailed documentation
- Review [configuration options](config.example.json)
- Check out the [systemd service](llm-benchmark-worker.service) for deployment
- Explore the source code in `src/` directory
