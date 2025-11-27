//! Worker metrics and monitoring

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// Worker metrics
#[derive(Clone)]
pub struct WorkerMetrics {
    inner: Arc<RwLock<MetricsInner>>,
}

struct MetricsInner {
    /// Total number of jobs processed
    jobs_processed: u64,
    /// Number of successfully completed jobs
    jobs_succeeded: u64,
    /// Number of failed jobs
    jobs_failed: u64,
    /// Number of retried jobs
    jobs_retried: u64,
    /// Job durations (for calculating percentiles)
    durations: Vec<Duration>,
    /// Queue depths by priority
    queue_depths: HashMap<String, usize>,
    /// Job processing rates (jobs per second)
    processing_rate: f64,
    /// Last update timestamp
    last_update: std::time::Instant,
}

impl Default for MetricsInner {
    fn default() -> Self {
        Self {
            jobs_processed: 0,
            jobs_succeeded: 0,
            jobs_failed: 0,
            jobs_retried: 0,
            durations: Vec::new(),
            queue_depths: HashMap::new(),
            processing_rate: 0.0,
            last_update: std::time::Instant::now(),
        }
    }
}

impl WorkerMetrics {
    /// Create new metrics
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(MetricsInner {
                last_update: std::time::Instant::now(),
                ..Default::default()
            })),
        }
    }

    /// Increment jobs processed counter
    pub fn increment_jobs_processed(&self) {
        let mut inner = self.inner.write();
        inner.jobs_processed += 1;
        self.update_processing_rate(&mut inner);
    }

    /// Increment jobs succeeded counter
    pub fn increment_jobs_succeeded(&self) {
        let mut inner = self.inner.write();
        inner.jobs_succeeded += 1;
    }

    /// Increment jobs failed counter
    pub fn increment_jobs_failed(&self) {
        let mut inner = self.inner.write();
        inner.jobs_failed += 1;
    }

    /// Increment jobs retried counter
    pub fn increment_jobs_retried(&self) {
        let mut inner = self.inner.write();
        inner.jobs_retried += 1;
    }

    /// Record job duration
    pub fn record_job_duration(&self, duration: Duration) {
        let mut inner = self.inner.write();
        inner.durations.push(duration);

        // Keep only last 1000 durations to prevent unbounded growth
        if inner.durations.len() > 1000 {
            inner.durations.drain(0..500);
        }
    }

    /// Update queue depth for a specific queue
    pub fn update_queue_depth(&self, queue_name: String, depth: usize) {
        let mut inner = self.inner.write();
        inner.queue_depths.insert(queue_name, depth);
    }

    /// Get total jobs processed
    pub fn jobs_processed(&self) -> u64 {
        self.inner.read().jobs_processed
    }

    /// Get jobs succeeded
    pub fn jobs_succeeded(&self) -> u64 {
        self.inner.read().jobs_succeeded
    }

    /// Get jobs failed
    pub fn jobs_failed(&self) -> u64 {
        self.inner.read().jobs_failed
    }

    /// Get jobs retried
    pub fn jobs_retried(&self) -> u64 {
        self.inner.read().jobs_retried
    }

    /// Get success rate (0.0 - 1.0)
    pub fn success_rate(&self) -> f64 {
        let inner = self.inner.read();
        if inner.jobs_processed == 0 {
            0.0
        } else {
            inner.jobs_succeeded as f64 / inner.jobs_processed as f64
        }
    }

    /// Get failure rate (0.0 - 1.0)
    pub fn failure_rate(&self) -> f64 {
        let inner = self.inner.read();
        if inner.jobs_processed == 0 {
            0.0
        } else {
            inner.jobs_failed as f64 / inner.jobs_processed as f64
        }
    }

    /// Get average job duration
    pub fn average_duration(&self) -> Option<Duration> {
        let inner = self.inner.read();
        if inner.durations.is_empty() {
            return None;
        }

        let total: Duration = inner.durations.iter().sum();
        Some(total / inner.durations.len() as u32)
    }

    /// Get median job duration
    pub fn median_duration(&self) -> Option<Duration> {
        let inner = self.inner.read();
        if inner.durations.is_empty() {
            return None;
        }

        let mut sorted = inner.durations.clone();
        sorted.sort();
        Some(sorted[sorted.len() / 2])
    }

    /// Get p95 job duration
    pub fn p95_duration(&self) -> Option<Duration> {
        let inner = self.inner.read();
        if inner.durations.is_empty() {
            return None;
        }

        let mut sorted = inner.durations.clone();
        sorted.sort();
        let index = (sorted.len() as f64 * 0.95) as usize;
        Some(sorted[index.min(sorted.len() - 1)])
    }

    /// Get p99 job duration
    pub fn p99_duration(&self) -> Option<Duration> {
        let inner = self.inner.read();
        if inner.durations.is_empty() {
            return None;
        }

        let mut sorted = inner.durations.clone();
        sorted.sort();
        let index = (sorted.len() as f64 * 0.99) as usize;
        Some(sorted[index.min(sorted.len() - 1)])
    }

    /// Get queue depth for a specific queue
    pub fn queue_depth(&self, queue_name: &str) -> Option<usize> {
        let inner = self.inner.read();
        inner.queue_depths.get(queue_name).copied()
    }

    /// Get total queue depth across all queues
    pub fn total_queue_depth(&self) -> usize {
        let inner = self.inner.read();
        inner.queue_depths.values().sum()
    }

    /// Get processing rate (jobs per second)
    pub fn processing_rate(&self) -> f64 {
        self.inner.read().processing_rate
    }

    /// Get metrics snapshot
    pub fn snapshot(&self) -> MetricsSnapshot {
        let inner = self.inner.read();
        MetricsSnapshot {
            jobs_processed: inner.jobs_processed,
            jobs_succeeded: inner.jobs_succeeded,
            jobs_failed: inner.jobs_failed,
            jobs_retried: inner.jobs_retried,
            success_rate: self.success_rate(),
            failure_rate: self.failure_rate(),
            average_duration: self.average_duration(),
            median_duration: self.median_duration(),
            p95_duration: self.p95_duration(),
            p99_duration: self.p99_duration(),
            total_queue_depth: self.total_queue_depth(),
            processing_rate: inner.processing_rate,
        }
    }

    /// Reset all metrics
    pub fn reset(&self) {
        let mut inner = self.inner.write();
        *inner = MetricsInner {
            last_update: std::time::Instant::now(),
            ..Default::default()
        };
    }

    /// Update processing rate
    fn update_processing_rate(&self, inner: &mut MetricsInner) {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(inner.last_update).as_secs_f64();

        if elapsed > 0.0 {
            inner.processing_rate = 1.0 / elapsed;
            inner.last_update = now;
        }
    }
}

impl Default for WorkerMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of metrics at a point in time
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub jobs_processed: u64,
    pub jobs_succeeded: u64,
    pub jobs_failed: u64,
    pub jobs_retried: u64,
    pub success_rate: f64,
    pub failure_rate: f64,
    pub average_duration: Option<Duration>,
    pub median_duration: Option<Duration>,
    pub p95_duration: Option<Duration>,
    pub p99_duration: Option<Duration>,
    pub total_queue_depth: usize,
    pub processing_rate: f64,
}

impl MetricsSnapshot {
    /// Format metrics for display
    pub fn format(&self) -> String {
        format!(
            r#"Worker Metrics:
  Jobs Processed: {}
  Jobs Succeeded: {}
  Jobs Failed: {}
  Jobs Retried: {}
  Success Rate: {:.2}%
  Failure Rate: {:.2}%
  Average Duration: {}
  Median Duration: {}
  P95 Duration: {}
  P99 Duration: {}
  Total Queue Depth: {}
  Processing Rate: {:.2} jobs/sec"#,
            self.jobs_processed,
            self.jobs_succeeded,
            self.jobs_failed,
            self.jobs_retried,
            self.success_rate * 100.0,
            self.failure_rate * 100.0,
            format_duration(self.average_duration),
            format_duration(self.median_duration),
            format_duration(self.p95_duration),
            format_duration(self.p99_duration),
            self.total_queue_depth,
            self.processing_rate,
        )
    }
}

fn format_duration(duration: Option<Duration>) -> String {
    match duration {
        Some(d) => format!("{:.2}ms", d.as_secs_f64() * 1000.0),
        None => "N/A".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_counters() {
        let metrics = WorkerMetrics::new();

        assert_eq!(metrics.jobs_processed(), 0);
        assert_eq!(metrics.jobs_succeeded(), 0);

        metrics.increment_jobs_processed();
        metrics.increment_jobs_succeeded();

        assert_eq!(metrics.jobs_processed(), 1);
        assert_eq!(metrics.jobs_succeeded(), 1);
    }

    #[test]
    fn test_success_rate() {
        let metrics = WorkerMetrics::new();

        metrics.increment_jobs_processed();
        metrics.increment_jobs_succeeded();
        metrics.increment_jobs_processed();
        metrics.increment_jobs_failed();

        assert_eq!(metrics.success_rate(), 0.5);
        assert_eq!(metrics.failure_rate(), 0.5);
    }

    #[test]
    fn test_duration_metrics() {
        let metrics = WorkerMetrics::new();

        metrics.record_job_duration(Duration::from_millis(100));
        metrics.record_job_duration(Duration::from_millis(200));
        metrics.record_job_duration(Duration::from_millis(300));

        assert!(metrics.average_duration().is_some());
        assert!(metrics.median_duration().is_some());
    }

    #[test]
    fn test_queue_depth() {
        let metrics = WorkerMetrics::new();

        metrics.update_queue_depth("high".to_string(), 10);
        metrics.update_queue_depth("normal".to_string(), 5);

        assert_eq!(metrics.queue_depth("high"), Some(10));
        assert_eq!(metrics.total_queue_depth(), 15);
    }
}
