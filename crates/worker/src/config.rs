//! Worker configuration

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Worker pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    /// Number of worker threads in the pool
    pub pool_size: usize,

    /// Redis connection URL
    pub redis_url: String,

    /// Database connection URL (optional)
    pub database_url: Option<String>,

    /// Queue settings
    pub queue: QueueConfig,

    /// Retry policy
    pub retry: RetryConfig,

    /// Scheduler settings
    pub scheduler: SchedulerConfig,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            pool_size: num_cpus::get(),
            redis_url: "redis://localhost:6379".to_string(),
            database_url: None,
            queue: QueueConfig::default(),
            retry: RetryConfig::default(),
            scheduler: SchedulerConfig::default(),
        }
    }
}

/// Queue configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    /// Prefix for queue keys in Redis
    pub prefix: String,

    /// Default queue name
    pub default_queue: String,

    /// Queue names for different priorities
    pub priority_queues: PriorityQueues,

    /// Blocking timeout when waiting for jobs (seconds)
    pub blocking_timeout: u64,

    /// Maximum number of retries for a job
    pub max_retries: u32,

    /// Dead letter queue name
    pub dead_letter_queue: String,

    /// Job visibility timeout (seconds)
    pub visibility_timeout: u64,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            prefix: "llm-benchmark".to_string(),
            default_queue: "jobs".to_string(),
            priority_queues: PriorityQueues::default(),
            blocking_timeout: 5,
            max_retries: 3,
            dead_letter_queue: "jobs:dlq".to_string(),
            visibility_timeout: 300, // 5 minutes
        }
    }
}

/// Priority queue names
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityQueues {
    pub critical: String,
    pub high: String,
    pub normal: String,
    pub low: String,
}

impl Default for PriorityQueues {
    fn default() -> Self {
        Self {
            critical: "jobs:critical".to_string(),
            high: "jobs:high".to_string(),
            normal: "jobs:normal".to_string(),
            low: "jobs:low".to_string(),
        }
    }
}

/// Retry policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,

    /// Initial backoff duration (seconds)
    pub initial_backoff: u64,

    /// Maximum backoff duration (seconds)
    pub max_backoff: u64,

    /// Backoff multiplier
    pub backoff_multiplier: f64,

    /// Whether to use exponential backoff
    pub exponential_backoff: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff: 5,
            max_backoff: 300, // 5 minutes
            backoff_multiplier: 2.0,
            exponential_backoff: true,
        }
    }
}

impl RetryConfig {
    /// Calculate backoff duration for a given attempt
    pub fn calculate_backoff(&self, attempt: u32) -> Duration {
        if !self.exponential_backoff {
            return Duration::from_secs(self.initial_backoff);
        }

        let backoff = if attempt == 0 {
            self.initial_backoff
        } else {
            let exponential = (self.initial_backoff as f64)
                * self.backoff_multiplier.powi(attempt as i32 - 1);
            exponential.min(self.max_backoff as f64) as u64
        };

        Duration::from_secs(backoff)
    }
}

/// Scheduler configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    /// Whether the scheduler is enabled
    pub enabled: bool,

    /// Scheduler tick interval (seconds)
    pub tick_interval: u64,

    /// Maximum number of scheduled jobs to process per tick
    pub max_jobs_per_tick: usize,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            tick_interval: 60, // 1 minute
            max_jobs_per_tick: 100,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_config_linear_backoff() {
        let config = RetryConfig {
            exponential_backoff: false,
            initial_backoff: 10,
            ..Default::default()
        };

        assert_eq!(config.calculate_backoff(0), Duration::from_secs(10));
        assert_eq!(config.calculate_backoff(1), Duration::from_secs(10));
        assert_eq!(config.calculate_backoff(5), Duration::from_secs(10));
    }

    #[test]
    fn test_retry_config_exponential_backoff() {
        let config = RetryConfig {
            exponential_backoff: true,
            initial_backoff: 5,
            max_backoff: 100,
            backoff_multiplier: 2.0,
            ..Default::default()
        };

        assert_eq!(config.calculate_backoff(0), Duration::from_secs(5));
        assert_eq!(config.calculate_backoff(1), Duration::from_secs(5));
        assert_eq!(config.calculate_backoff(2), Duration::from_secs(10));
        assert_eq!(config.calculate_backoff(3), Duration::from_secs(20));
        assert_eq!(config.calculate_backoff(4), Duration::from_secs(40));
        assert_eq!(config.calculate_backoff(5), Duration::from_secs(80));
        assert_eq!(config.calculate_backoff(6), Duration::from_secs(100)); // capped at max
    }
}
