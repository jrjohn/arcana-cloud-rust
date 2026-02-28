//! Retry policy implementation.

use std::time::Duration;
use tracing::debug;

/// Retry policy configuration.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts.
    pub max_attempts: u32,
    /// Initial delay between retries.
    pub initial_delay: Duration,
    /// Maximum delay between retries.
    pub max_delay: Duration,
    /// Multiplier for exponential backoff.
    pub multiplier: f64,
    /// Whether to add jitter to delays.
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryPolicy {
    /// Creates a new retry policy with the specified max attempts.
    pub fn with_max_attempts(max_attempts: u32) -> Self {
        Self {
            max_attempts,
            ..Default::default()
        }
    }

    /// Calculates the delay for a given attempt number.
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::ZERO;
        }

        let base_delay = self.initial_delay.as_millis() as f64 * self.multiplier.powi(attempt as i32 - 1);
        let delay = Duration::from_millis(base_delay.min(self.max_delay.as_millis() as f64) as u64);

        if self.jitter {
            // Add up to 25% jitter
            let jitter_factor = 1.0 + (rand_simple() * 0.5 - 0.25);
            Duration::from_millis((delay.as_millis() as f64 * jitter_factor) as u64)
        } else {
            delay
        }
    }

    /// Executes a function with retry logic.
    pub async fn execute<F, Fut, T, E>(&self, mut f: F) -> Result<T, E>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        let mut last_error = None;

        for attempt in 0..self.max_attempts {
            if attempt > 0 {
                let delay = self.delay_for_attempt(attempt);
                debug!("Retry attempt {} after {:?}", attempt, delay);
                tokio::time::sleep(delay).await;
            }

            match f().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    debug!("Attempt {} failed: {}", attempt + 1, e);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.expect("at least one attempt should have been made"))
    }
}

/// Simple pseudo-random number generator for jitter.
fn rand_simple() -> f64 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    (nanos % 1000) as f64 / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_retry_success() {
        let policy = RetryPolicy::with_max_attempts(3);
        let result: Result<i32, &str> = policy.execute(|| async { Ok(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_retry_eventual_success() {
        let policy = RetryPolicy::with_max_attempts(3);
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();

        let result: Result<i32, &str> = policy
            .execute(|| {
                let attempts = attempts_clone.clone();
                async move {
                    let attempt = attempts.fetch_add(1, Ordering::SeqCst);
                    if attempt < 2 {
                        Err("not yet")
                    } else {
                        Ok(42)
                    }
                }
            })
            .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_all_failures() {
        let policy = RetryPolicy {
            max_attempts: 2,
            initial_delay: Duration::from_millis(1),
            ..Default::default()
        };

        let result: Result<i32, &str> = policy.execute(|| async { Err("always fails") }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_retry_counts_exact_attempts() {
        let policy = RetryPolicy {
            max_attempts: 3,
            initial_delay: Duration::from_millis(1),
            jitter: false,
            ..Default::default()
        };

        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();

        let result: Result<i32, &str> = policy
            .execute(|| {
                let a = attempts_clone.clone();
                async move {
                    a.fetch_add(1, Ordering::SeqCst);
                    Err("fail")
                }
            })
            .await;

        assert!(result.is_err());
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_delay_for_attempt_zero() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.delay_for_attempt(0), Duration::ZERO);
    }

    #[test]
    fn test_delay_for_attempt_increases() {
        let policy = RetryPolicy {
            initial_delay: Duration::from_millis(100),
            multiplier: 2.0,
            jitter: false,
            max_delay: Duration::from_secs(60),
            ..Default::default()
        };

        let delay1 = policy.delay_for_attempt(1);
        let delay2 = policy.delay_for_attempt(2);

        // delay2 should be larger than delay1 (exponential backoff)
        assert!(delay2 >= delay1);
    }

    #[test]
    fn test_delay_capped_at_max() {
        let policy = RetryPolicy {
            initial_delay: Duration::from_millis(100),
            multiplier: 1000.0,
            jitter: false,
            max_delay: Duration::from_millis(500),
            ..Default::default()
        };

        let delay = policy.delay_for_attempt(10);
        // Should be capped near max_delay (with possible small jitter)
        assert!(delay.as_millis() <= 750); // Max + 50% jitter ceiling
    }

    #[test]
    fn test_retry_policy_default() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 3);
        assert!(policy.jitter);
        assert!(policy.multiplier > 1.0);
    }

    #[test]
    fn test_retry_policy_with_max_attempts() {
        let policy = RetryPolicy::with_max_attempts(5);
        assert_eq!(policy.max_attempts, 5);
    }

    #[tokio::test]
    async fn test_retry_single_attempt() {
        let policy = RetryPolicy::with_max_attempts(1);
        let result: Result<i32, &str> = policy.execute(|| async { Err("fail") }).await;
        assert!(result.is_err());
    }
}
