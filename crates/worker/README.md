# LLM Benchmark Exchange Worker

Background job processing worker for the LLM Benchmark Exchange platform.

## Features

- **Redis-based Job Queue**: Priority-based job queue with blocking operations
- **Multiple Worker Types**: Specialized workers for different job categories
- **Job Scheduling**: Cron-like scheduling for recurring jobs
- **Retry Policies**: Exponential backoff with configurable retry limits
- **Dead Letter Queue**: Failed jobs are moved to DLQ for manual inspection
- **Metrics & Monitoring**: Comprehensive metrics including success rates, durations, and queue depths
- **Graceful Shutdown**: Clean shutdown handling with signal support

## Architecture

### Components

```
worker/
├── src/
│   ├── main.rs              # Worker entry point with CLI
│   ├── lib.rs               # Library exports
│   ├── config.rs            # Worker configuration
│   ├── metrics.rs           # Metrics collection and reporting
│   ├── queue/
│   │   ├── mod.rs           # Queue abstraction
│   │   ├── job.rs           # Job types and definitions
│   │   ├── producer.rs      # Job enqueuing
│   │   └── consumer.rs      # Job fetching and processing
│   ├── workers/
│   │   ├── mod.rs           # Worker trait and job handler
│   │   ├── verification.rs  # Submission verification
│   │   ├── leaderboard.rs   # Leaderboard computation
│   │   ├── sync.rs          # External system sync
│   │   ├── governance.rs    # Governance finalization
│   │   ├── notification.rs  # Notification sending
│   │   └── cleanup.rs       # Data cleanup
│   └── scheduler/
│       └── mod.rs           # Job scheduler
```

### Job Types

1. **VerifySubmission**: Validates and scores benchmark submissions
2. **RecomputeLeaderboard**: Recalculates leaderboard rankings
3. **SyncToRegistry**: Syncs data to LLM-Registry
4. **ExportToAnalytics**: Exports metrics to LLM-Analytics-Hub
5. **FinalizeProposal**: Finalizes governance proposals
6. **CleanupExpiredData**: Removes expired data
7. **SendNotification**: Sends notifications to users

### Job Priorities

- **Critical**: Processed immediately (system-critical operations)
- **High**: High-priority jobs (user-facing operations)
- **Normal**: Default priority (background processing)
- **Low**: Low-priority jobs (cleanup, archival)

## Usage

### Running the Worker

```bash
# Basic usage
worker --redis-url redis://localhost:6379

# With custom pool size
worker --workers 8

# With database connection
worker --database-url postgresql://localhost/benchmark

# With configuration file
worker --config /path/to/config.json

# Disable scheduler
worker --scheduler false

# Custom metrics interval
worker --metrics-interval 30
```

### Environment Variables

- `WORKER_POOL_SIZE`: Number of worker threads
- `REDIS_URL`: Redis connection URL
- `DATABASE_URL`: Database connection URL
- `WORKER_CONFIG`: Path to configuration file
- `WORKER_SCHEDULER_ENABLED`: Enable/disable scheduler
- `METRICS_INTERVAL`: Metrics reporting interval (seconds)
- `RUST_LOG`: Logging level (e.g., `info`, `debug`)

### Configuration File

```json
{
  "pool_size": 4,
  "redis_url": "redis://localhost:6379",
  "database_url": "postgresql://localhost/benchmark",
  "queue": {
    "prefix": "llm-benchmark",
    "default_queue": "jobs",
    "priority_queues": {
      "critical": "jobs:critical",
      "high": "jobs:high",
      "normal": "jobs:normal",
      "low": "jobs:low"
    },
    "blocking_timeout": 5,
    "max_retries": 3,
    "dead_letter_queue": "jobs:dlq",
    "visibility_timeout": 300
  },
  "retry": {
    "max_attempts": 3,
    "initial_backoff": 5,
    "max_backoff": 300,
    "backoff_multiplier": 2.0,
    "exponential_backoff": true
  },
  "scheduler": {
    "enabled": true,
    "tick_interval": 60,
    "max_jobs_per_tick": 100
  }
}
```

## Job Queue Operations

### Enqueuing Jobs

```rust
use llm_benchmark_worker::{JobProducer, queue::*};

// Create producer
let mut producer = JobProducer::new("redis://localhost:6379").await?;

// Enqueue a job with default priority
let job = producer.enqueue(
    JobType::VerifySubmission(VerifySubmissionJob {
        submission_id: submission_id,
        benchmark_id: benchmark_id,
    })
).await?;

// Enqueue with specific priority
let job = producer.enqueue_with_priority(
    JobType::RecomputeLeaderboard(RecomputeLeaderboardJob {
        benchmark_id: benchmark_id,
        invalidate_cache: true,
    }),
    JobPriority::High
).await?;

// Enqueue delayed job
let job = producer.enqueue_delayed(
    JobType::SendNotification(notification_job),
    JobPriority::Normal,
    chrono::Duration::minutes(5)
).await?;

// Batch enqueue
let jobs = vec![
    (JobType::CleanupExpiredData(cleanup_job), JobPriority::Low),
    // ... more jobs
];
let created_jobs = producer.enqueue_batch(jobs).await?;
```

### Queue Management

```rust
// Check queue size
let size = producer.queue_size(JobPriority::High).await?;

// Get total queue size
let total = producer.total_queue_size().await?;

// Clear queue (use with caution!)
producer.clear_queue(JobPriority::Low).await?;
```

### Dead Letter Queue

```rust
use llm_benchmark_worker::JobConsumer;

let consumer = JobConsumer::new("redis://localhost:6379", 4).await?;

// Get DLQ jobs
let dlq_jobs = consumer.get_dlq_jobs(10).await?;

// Retry a job from DLQ
consumer.retry_dlq_job(&job_id).await?;
```

## Scheduler

The scheduler runs recurring jobs based on cron-like schedules.

### Default Scheduled Jobs

- **Cleanup Expired Sessions**: Daily at 2 AM
- **Cleanup Temp Files**: Daily at 3 AM
- **Archive Old Submissions**: Weekly on Sunday at 4 AM

### Custom Schedules

```rust
use llm_benchmark_worker::scheduler::{Schedule, ScheduledJob};

// Every minute
let schedule = Schedule::every_minute();

// Hourly at minute 30
let schedule = Schedule::hourly(30);

// Daily at 14:30
let schedule = Schedule::daily(14, 30);

// Weekly on Monday at 09:00
let schedule = Schedule::weekly(1, 9, 0);

// Cron expression (minute hour day month day_of_week)
let schedule = Schedule::parse("30 14 * * *")?;

// Create scheduled job
let job = ScheduledJob::new(
    "daily_cleanup",
    schedule,
    JobType::CleanupExpiredData(cleanup_job),
    JobPriority::Low
);
```

## Metrics

The worker collects comprehensive metrics:

- **Jobs Processed**: Total number of jobs processed
- **Jobs Succeeded**: Number of successful jobs
- **Jobs Failed**: Number of failed jobs
- **Jobs Retried**: Number of retried jobs
- **Success Rate**: Percentage of successful jobs
- **Failure Rate**: Percentage of failed jobs
- **Average Duration**: Average job processing time
- **Median Duration**: Median job processing time
- **P95/P99 Duration**: 95th/99th percentile durations
- **Queue Depth**: Number of jobs waiting in each queue
- **Processing Rate**: Jobs processed per second

### Accessing Metrics

```rust
use llm_benchmark_worker::WorkerMetrics;

let metrics = WorkerMetrics::new();

// Increment counters
metrics.increment_jobs_processed();
metrics.increment_jobs_succeeded();

// Record duration
metrics.record_job_duration(duration);

// Get metrics
let success_rate = metrics.success_rate();
let avg_duration = metrics.average_duration();

// Get snapshot
let snapshot = metrics.snapshot();
println!("{}", snapshot.format());
```

## Error Handling

### Retry Policy

Jobs that fail are automatically retried according to the retry policy:

1. **Initial Backoff**: 5 seconds (configurable)
2. **Exponential Backoff**: Each retry doubles the backoff time
3. **Max Backoff**: Capped at 5 minutes (configurable)
4. **Max Retries**: 3 attempts by default (configurable)

### Dead Letter Queue

Jobs that exceed the maximum retry attempts are moved to the dead letter queue (DLQ). These jobs can be:

- Inspected manually
- Retried after fixing underlying issues
- Archived for debugging

## Development

### Running Tests

```bash
# Run all tests
cargo test -p llm-benchmark-worker

# Run integration tests (requires Redis)
cargo test -p llm-benchmark-worker -- --ignored

# Run with logging
RUST_LOG=debug cargo test -p llm-benchmark-worker
```

### Adding a New Worker

1. Create a new file in `src/workers/` (e.g., `my_worker.rs`)
2. Implement the `Worker` trait
3. Add to `src/workers/mod.rs`
4. Add job type to `src/queue/job.rs`
5. Update `JobHandler` in `src/workers/mod.rs`

Example:

```rust
// src/workers/my_worker.rs
use super::Worker;
use crate::config::WorkerConfig;
use crate::queue::job::{Job, JobType, MyJob};
use async_trait::async_trait;

pub struct MyWorker {
    config: WorkerConfig,
}

impl MyWorker {
    pub fn new(config: WorkerConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Worker for MyWorker {
    async fn process(&self, job: &Job) -> Result<()> {
        match &job.job_type {
            JobType::MyJob(job_data) => {
                // Process job
                Ok(())
            }
            _ => Err(anyhow::anyhow!("Invalid job type")),
        }
    }

    fn name(&self) -> &str {
        "MyWorker"
    }
}
```

## Deployment

### Docker

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release -p llm-benchmark-worker

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/worker /usr/local/bin/worker
ENTRYPOINT ["worker"]
```

### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: benchmark-worker
spec:
  replicas: 3
  selector:
    matchLabels:
      app: benchmark-worker
  template:
    metadata:
      labels:
        app: benchmark-worker
    spec:
      containers:
      - name: worker
        image: llm-benchmark-worker:latest
        env:
        - name: REDIS_URL
          value: "redis://redis-service:6379"
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: db-secret
              key: url
        - name: WORKER_POOL_SIZE
          value: "4"
        - name: RUST_LOG
          value: "info"
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
```

## Monitoring

### Health Checks

The worker exposes metrics through logging. For production deployments:

1. Use structured logging (JSON format)
2. Ship logs to centralized logging system
3. Set up alerts for:
   - High failure rates
   - Long job durations
   - Growing queue depths
   - Dead letter queue size

### Recommended Alerts

- **Job Failure Rate > 10%**: Investigate failing jobs
- **Average Duration > 5s**: Performance degradation
- **Queue Depth > 1000**: Backlog building up
- **DLQ Size > 10**: Jobs consistently failing

## License

MIT License - see LICENSE file for details
