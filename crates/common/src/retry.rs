//! Retry utilities.
//!
//! This module provides utilities for retrying operations with exponential backoff.

use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;

/// Configuration for retry behavior.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts (0 means no retries)
    pub max_attempts: u32,

    /// Initial delay between retries
    pub initial_delay: Duration,

    /// Maximum delay between retries
    pub max_delay: Duration,

    /// Backoff multiplier (e.g., 2.0 for doubling)
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    /// Create a new retry configuration.
    pub fn new(max_attempts: u32, initial_delay: Duration) -> Self {
        Self {
            max_attempts,
            initial_delay,
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        }
    }

    /// Set the maximum delay between retries.
    pub fn with_max_delay(mut self, max_delay: Duration) -> Self {
        self.max_delay = max_delay;
        self
    }

    /// Set the backoff multiplier.
    pub fn with_backoff_multiplier(mut self, multiplier: f64) -> Self {
        self.backoff_multiplier = multiplier;
        self
    }

    /// Create a configuration with no retries.
    pub fn no_retry() -> Self {
        Self {
            max_attempts: 0,
            initial_delay: Duration::from_millis(0),
            max_delay: Duration::from_millis(0),
            backoff_multiplier: 1.0,
        }
    }

    /// Create a configuration with exponential backoff.
    pub fn exponential(max_attempts: u32) -> Self {
        Self {
            max_attempts,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        }
    }

    /// Create a configuration with linear backoff.
    pub fn linear(max_attempts: u32, delay: Duration) -> Self {
        Self {
            max_attempts,
            initial_delay: delay,
            max_delay: delay,
            backoff_multiplier: 1.0,
        }
    }
}

/// Exponential backoff calculator.
#[derive(Debug, Clone)]
pub struct ExponentialBackoff {
    config: RetryConfig,
    current_attempt: u32,
}

impl ExponentialBackoff {
    /// Create a new exponential backoff calculator.
    pub fn new(config: RetryConfig) -> Self {
        Self {
            config,
            current_attempt: 0,
        }
    }

    /// Calculate the delay for the current attempt.
    pub fn delay(&self) -> Duration {
        if self.current_attempt == 0 {
            return Duration::from_millis(0);
        }

        let delay_ms = self.config.initial_delay.as_millis() as f64
            * self.config.backoff_multiplier.powi((self.current_attempt - 1) as i32);

        let delay = Duration::from_millis(delay_ms as u64);
        delay.min(self.config.max_delay)
    }

    /// Move to the next attempt.
    pub fn next_attempt(&mut self) {
        self.current_attempt += 1;
    }

    /// Check if there are more attempts remaining.
    pub fn has_attempts_remaining(&self) -> bool {
        self.current_attempt <= self.config.max_attempts
    }

    /// Reset the backoff state.
    pub fn reset(&mut self) {
        self.current_attempt = 0;
    }
}

/// Retry an async operation with exponential backoff.
///
/// # Arguments
///
/// * `config` - Retry configuration
/// * `operation` - The async operation to retry
///
/// # Examples
///
/// ```no_run
/// use common::retry::{retry_with_backoff, RetryConfig};
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() {
///     let config = RetryConfig::exponential(3);
///
///     let result = retry_with_backoff(config, || async {
///         // Your async operation here
///         Ok::<_, std::io::Error>(())
///     }).await;
/// }
/// ```
pub async fn retry_with_backoff<F, Fut, T, E>(
    config: RetryConfig,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let mut backoff = ExponentialBackoff::new(config);

    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(error) => {
                backoff.next_attempt();

                if !backoff.has_attempts_remaining() {
                    return Err(error);
                }

                let delay = backoff.delay();
                tracing::debug!(
                    attempt = backoff.current_attempt,
                    delay_ms = delay.as_millis(),
                    "Retrying operation after error"
                );
                sleep(delay).await;
            }
        }
    }
}

/// Retry an async operation with a custom predicate to determine if a retry should occur.
///
/// # Arguments
///
/// * `config` - Retry configuration
/// * `operation` - The async operation to retry
/// * `should_retry` - Predicate to determine if the error is retryable
///
/// # Examples
///
/// ```no_run
/// use common::retry::{retry_with_predicate, RetryConfig};
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() {
///     let config = RetryConfig::exponential(3);
///
///     let result = retry_with_predicate(
///         config,
///         || async {
///             // Your async operation here
///             Err::<(), _>(std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout"))
///         },
///         |err| err.kind() == std::io::ErrorKind::TimedOut
///     ).await;
/// }
/// ```
pub async fn retry_with_predicate<F, Fut, T, E, P>(
    config: RetryConfig,
    mut operation: F,
    should_retry: P,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    P: Fn(&E) -> bool,
{
    let mut backoff = ExponentialBackoff::new(config);

    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(error) => {
                if !should_retry(&error) {
                    return Err(error);
                }

                backoff.next_attempt();

                if !backoff.has_attempts_remaining() {
                    return Err(error);
                }

                let delay = backoff.delay();
                tracing::debug!(
                    attempt = backoff.current_attempt,
                    delay_ms = delay.as_millis(),
                    "Retrying operation after retryable error"
                );
                sleep(delay).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.initial_delay, Duration::from_millis(100));
    }

    #[test]
    fn test_retry_config_no_retry() {
        let config = RetryConfig::no_retry();
        assert_eq!(config.max_attempts, 0);
    }

    #[test]
    fn test_retry_config_exponential() {
        let config = RetryConfig::exponential(5);
        assert_eq!(config.max_attempts, 5);
        assert_eq!(config.backoff_multiplier, 2.0);
    }

    #[test]
    fn test_retry_config_linear() {
        let config = RetryConfig::linear(3, Duration::from_secs(1));
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.initial_delay, Duration::from_secs(1));
        assert_eq!(config.backoff_multiplier, 1.0);
    }

    #[test]
    fn test_exponential_backoff() {
        let config = RetryConfig::exponential(3);
        let mut backoff = ExponentialBackoff::new(config);

        // First attempt has no delay
        assert_eq!(backoff.delay(), Duration::from_millis(0));
        assert!(backoff.has_attempts_remaining());

        // Second attempt
        backoff.next_attempt();
        assert_eq!(backoff.delay(), Duration::from_millis(100));
        assert!(backoff.has_attempts_remaining());

        // Third attempt
        backoff.next_attempt();
        assert_eq!(backoff.delay(), Duration::from_millis(200));
        assert!(backoff.has_attempts_remaining());

        // Fourth attempt
        backoff.next_attempt();
        assert_eq!(backoff.delay(), Duration::from_millis(400));
        assert!(backoff.has_attempts_remaining());

        // No more attempts
        backoff.next_attempt();
        assert!(!backoff.has_attempts_remaining());
    }

    #[test]
    fn test_exponential_backoff_max_delay() {
        let config = RetryConfig::exponential(10).with_max_delay(Duration::from_millis(500));
        let mut backoff = ExponentialBackoff::new(config);

        // Keep incrementing until we hit max delay
        for _ in 0..10 {
            backoff.next_attempt();
        }

        // Delay should be capped at max_delay
        assert!(backoff.delay() <= Duration::from_millis(500));
    }

    #[tokio::test]
    async fn test_retry_with_backoff_success() {
        let config = RetryConfig::exponential(3);
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = retry_with_backoff(config, || {
            let counter = counter_clone.clone();
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok::<_, std::io::Error>(42)
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_with_backoff_eventual_success() {
        let config = RetryConfig::exponential(3);
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = retry_with_backoff(config, || {
            let counter = counter_clone.clone();
            async move {
                let count = counter.fetch_add(1, Ordering::SeqCst);
                if count < 2 {
                    Err(std::io::Error::new(std::io::ErrorKind::Other, "error"))
                } else {
                    Ok(42)
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_with_backoff_failure() {
        let config = RetryConfig::exponential(2);
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = retry_with_backoff(config, || {
            let counter = counter_clone.clone();
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Err::<i32, _>(std::io::Error::new(std::io::ErrorKind::Other, "error"))
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 3); // Initial + 2 retries
    }

    #[tokio::test]
    async fn test_retry_with_predicate() {
        let config = RetryConfig::exponential(3);
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = retry_with_predicate(
            config,
            || {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Err::<i32, _>(std::io::Error::new(
                        std::io::ErrorKind::ConnectionRefused,
                        "error",
                    ))
                }
            },
            |err| err.kind() == std::io::ErrorKind::TimedOut,
        )
        .await;

        // Should not retry because predicate returns false
        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}