//! LLM Benchmark Exchange Worker
//!
//! Background worker for processing benchmark submissions and computations.

use anyhow::Result;
use clap::Parser;
use llm_benchmark_worker::{WorkerConfig, WorkerPool};
use std::time::Duration;
use tokio::signal;
use tracing::{error, info};

#[derive(Parser, Debug)]
#[command(name = "worker")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Worker pool size
    #[arg(short, long, env = "WORKER_POOL_SIZE")]
    workers: Option<usize>,

    /// Redis connection URL
    #[arg(long, env = "REDIS_URL", default_value = "redis://localhost:6379")]
    redis_url: String,

    /// Database connection URL
    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,

    /// Configuration file path
    #[arg(short, long, env = "WORKER_CONFIG")]
    config: Option<String>,

    /// Enable scheduler
    #[arg(long, env = "WORKER_SCHEDULER_ENABLED")]
    scheduler: Option<bool>,

    /// Print metrics interval (seconds)
    #[arg(long, env = "METRICS_INTERVAL", default_value = "60")]
    metrics_interval: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .json()
        .init();

    let args = Args::parse();

    info!(
        redis_url = %args.redis_url,
        "Starting LLM Benchmark Worker"
    );

    // Load configuration
    let mut config = if let Some(config_path) = args.config {
        info!(config_path = %config_path, "Loading configuration from file");
        load_config_from_file(&config_path)?
    } else {
        WorkerConfig::default()
    };

    // Override with CLI arguments
    if let Some(workers) = args.workers {
        config.pool_size = workers;
    }
    config.redis_url = args.redis_url;
    config.database_url = args.database_url;

    if let Some(scheduler_enabled) = args.scheduler {
        config.scheduler.enabled = scheduler_enabled;
    }

    info!(
        pool_size = config.pool_size,
        scheduler_enabled = config.scheduler.enabled,
        "Worker configuration loaded"
    );

    // Create worker pool
    let mut pool = WorkerPool::new(config).await?;

    // Get shutdown handle
    let shutdown_handle = pool.shutdown_handle();
    let metrics = pool.metrics().clone();

    // Setup graceful shutdown
    tokio::spawn(async move {
        if let Err(e) = signal::ctrl_c().await {
            error!(error = %e, "Failed to listen for shutdown signal");
            return;
        }
        info!("Received shutdown signal");
        let _ = shutdown_handle.send(()).await;
    });

    // Start metrics reporting
    let metrics_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(args.metrics_interval));
        loop {
            interval.tick().await;
            let snapshot = metrics.snapshot();
            info!(
                jobs_processed = snapshot.jobs_processed,
                jobs_succeeded = snapshot.jobs_succeeded,
                jobs_failed = snapshot.jobs_failed,
                success_rate = format!("{:.2}%", snapshot.success_rate * 100.0),
                avg_duration_ms = snapshot.average_duration
                    .map(|d| d.as_millis())
                    .unwrap_or(0),
                "Worker metrics"
            );
        }
    });

    // Start the worker pool
    info!("Worker pool started");
    if let Err(e) = pool.start().await {
        error!(error = %e, "Worker pool error");
    }

    // Cancel metrics reporting
    metrics_handle.abort();

    info!("Worker shutting down gracefully");

    Ok(())
}

/// Load configuration from file
fn load_config_from_file(path: &str) -> Result<WorkerConfig> {
    let config_str = std::fs::read_to_string(path)?;
    let config: WorkerConfig = serde_json::from_str(&config_str)?;
    Ok(config)
}
