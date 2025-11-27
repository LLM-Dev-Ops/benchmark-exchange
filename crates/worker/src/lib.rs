//! LLM Benchmark Exchange Worker
//!
//! Background job processing for the LLM Benchmark Exchange platform.
//!
//! This crate provides:
//! - Redis-based job queue with priority handling
//! - Multiple worker types for different job categories
//! - Job scheduling with cron-like functionality
//! - Retry policies and dead letter queue
//! - Metrics and monitoring

pub mod config;
pub mod metrics;
pub mod queue;
pub mod scheduler;
pub mod workers;

pub use config::WorkerConfig;
pub use metrics::WorkerMetrics;
pub use queue::{JobConsumer, JobProducer, JobQueue};

use anyhow::Result;
use scheduler::Scheduler;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{error, info};

/// Worker pool for processing background jobs
pub struct WorkerPool {
    config: WorkerConfig,
    producer: JobProducer,
    consumer: JobConsumer,
    metrics: WorkerMetrics,
    shutdown_tx: mpsc::Sender<()>,
    shutdown_rx: mpsc::Receiver<()>,
}

impl WorkerPool {
    /// Create a new worker pool
    pub async fn new(config: WorkerConfig) -> Result<Self> {
        let producer = JobProducer::new(&config.redis_url).await?;
        let consumer = JobConsumer::new(&config.redis_url, config.pool_size).await?;
        let metrics = WorkerMetrics::new();
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        Ok(Self {
            config,
            producer,
            consumer,
            metrics,
            shutdown_tx,
            shutdown_rx,
        })
    }

    /// Start the worker pool
    pub async fn start(&mut self) -> Result<()> {
        info!(
            pool_size = self.config.pool_size,
            scheduler_enabled = self.config.scheduler.enabled,
            "Starting worker pool"
        );

        // Start worker threads
        let worker_handles = self.consumer.start(
            self.config.clone(),
            self.metrics.clone(),
        ).await?;

        // Start scheduler if enabled
        let scheduler_handle = if self.config.scheduler.enabled {
            let scheduler_producer = self.producer.clone();
            let scheduler = Scheduler::new(self.config.clone(), scheduler_producer).await?;
            Some(scheduler.start())
        } else {
            None
        };

        // Wait for shutdown signal
        self.shutdown_rx.recv().await;

        info!("Shutting down worker pool");

        // Stop scheduler if running
        if let Some(handle) = scheduler_handle {
            handle.abort();
        }

        // Wait for all workers to finish
        for handle in worker_handles {
            if let Err(e) = handle.await {
                error!("Worker thread error: {}", e);
            }
        }

        Ok(())
    }

    /// Get a handle to send shutdown signal
    pub fn shutdown_handle(&self) -> mpsc::Sender<()> {
        self.shutdown_tx.clone()
    }

    /// Get the job producer for enqueuing jobs
    pub fn producer(&self) -> &JobProducer {
        &self.producer
    }

    /// Get metrics
    pub fn metrics(&self) -> &WorkerMetrics {
        &self.metrics
    }
}
