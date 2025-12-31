//! Retry policies for failed jobs.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Retry strategy enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetryStrategy {
    /// No retry.
    None,
    /// Fixed delay between retries.
    Fixed,
    /// Exponential backoff with optional jitter.
    Exponential,
    /// Linear backoff.
    Linear,
}

/// Retry policy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Retry strategy.
    pub strategy: RetryStrategy,

    /// Maximum number of retries.
    pub max_retries: u32,

    /// Initial delay in milliseconds.
    pub initial_delay_ms: u64,

    /// Maximum delay in milliseconds.
    pub max_delay_ms: u64,

    /// Backoff multiplier (for exponential/linear).
    pub multiplier: f64,

    /// Add random jitter to delays.
    pub jitter: bool,

    /// Jitter factor (0.0 to 1.0).
    pub jitter_factor: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::exponential(3)
    }
}

impl RetryPolicy {
    /// Creates a policy with no retries.
    pub fn none() -> Self {
        Self {
            strategy: RetryStrategy::None,
            max_retries: 0,
            initial_delay_ms: 0,
            max_delay_ms: 0,
            multiplier: 1.0,
            jitter: false,
            jitter_factor: 0.0,
        }
    }

    /// Creates a fixed delay retry policy.
    pub fn fixed(max_retries: u32, delay_ms: u64) -> Self {
        Self {
            strategy: RetryStrategy::Fixed,
            max_retries,
            initial_delay_ms: delay_ms,
            max_delay_ms: delay_ms,
            multiplier: 1.0,
            jitter: false,
            jitter_factor: 0.0,
        }
    }

    /// Creates an exponential backoff retry policy.
    pub fn exponential(max_retries: u32) -> Self {
        Self {
            strategy: RetryStrategy::Exponential,
            max_retries,
            initial_delay_ms: 1000, // 1 second
            max_delay_ms: 3600000,  // 1 hour
            multiplier: 2.0,
            jitter: true,
            jitter_factor: 0.1,
        }
    }

    /// Creates a linear backoff retry policy.
    pub fn linear(max_retries: u32, increment_ms: u64) -> Self {
        Self {
            strategy: RetryStrategy::Linear,
            max_retries,
            initial_delay_ms: increment_ms,
            max_delay_ms: increment_ms * max_retries as u64,
            multiplier: 1.0,
            jitter: false,
            jitter_factor: 0.0,
        }
    }

    /// Sets the initial delay.
    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay_ms = delay.as_millis() as u64;
        self
    }

    /// Sets the maximum delay.
    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay_ms = delay.as_millis() as u64;
        self
    }

    /// Sets the backoff multiplier.
    pub fn with_multiplier(mut self, multiplier: f64) -> Self {
        self.multiplier = multiplier;
        self
    }

    /// Enables jitter.
    pub fn with_jitter(mut self, factor: f64) -> Self {
        self.jitter = true;
        self.jitter_factor = factor.clamp(0.0, 1.0);
        self
    }

    /// Disables jitter.
    pub fn without_jitter(mut self) -> Self {
        self.jitter = false;
        self.jitter_factor = 0.0;
        self
    }

    /// Returns true if retries are allowed.
    pub fn should_retry(&self, attempt: u32) -> bool {
        self.strategy != RetryStrategy::None && attempt <= self.max_retries
    }

    /// Calculate delay for the given attempt number.
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 || self.strategy == RetryStrategy::None {
            return Duration::ZERO;
        }

        let base_delay = match self.strategy {
            RetryStrategy::None => 0,
            RetryStrategy::Fixed => self.initial_delay_ms,
            RetryStrategy::Exponential => {
                let exp = (attempt - 1) as u32;
                let delay = self.initial_delay_ms as f64 * self.multiplier.powi(exp as i32);
                delay as u64
            }
            RetryStrategy::Linear => {
                self.initial_delay_ms * attempt as u64
            }
        };

        // Cap at max delay
        let capped_delay = base_delay.min(self.max_delay_ms);

        // Apply jitter if enabled
        let final_delay = if self.jitter && self.jitter_factor > 0.0 {
            let jitter_range = (capped_delay as f64 * self.jitter_factor) as u64;
            let jitter = rand_jitter(jitter_range);
            capped_delay.saturating_add(jitter).saturating_sub(jitter_range / 2)
        } else {
            capped_delay
        };

        Duration::from_millis(final_delay)
    }
}

/// Generate random jitter using a simple LCG.
fn rand_jitter(range: u64) -> u64 {
    use std::time::SystemTime;

    if range == 0 {
        return 0;
    }

    // Simple pseudo-random based on time
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;

    // LCG parameters
    let a: u64 = 6364136223846793005;
    let c: u64 = 1442695040888963407;

    let random = seed.wrapping_mul(a).wrapping_add(c);
    random % range
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_retry() {
        let policy = RetryPolicy::none();
        assert!(!policy.should_retry(1));
        assert_eq!(policy.delay_for_attempt(1), Duration::ZERO);
    }

    #[test]
    fn test_fixed_retry() {
        let policy = RetryPolicy::fixed(3, 5000);

        assert!(policy.should_retry(1));
        assert!(policy.should_retry(3));
        assert!(!policy.should_retry(4));

        assert_eq!(policy.delay_for_attempt(1), Duration::from_millis(5000));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_millis(5000));
    }

    #[test]
    fn test_exponential_backoff() {
        let policy = RetryPolicy::exponential(3).without_jitter();

        // 1st retry: 1000ms
        // 2nd retry: 2000ms
        // 3rd retry: 4000ms
        assert_eq!(policy.delay_for_attempt(1), Duration::from_millis(1000));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_millis(2000));
        assert_eq!(policy.delay_for_attempt(3), Duration::from_millis(4000));
    }

    #[test]
    fn test_linear_backoff() {
        let policy = RetryPolicy::linear(3, 1000);

        assert_eq!(policy.delay_for_attempt(1), Duration::from_millis(1000));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_millis(2000));
        assert_eq!(policy.delay_for_attempt(3), Duration::from_millis(3000));
    }

    #[test]
    fn test_max_delay_cap() {
        let policy = RetryPolicy::exponential(10)
            .with_max_delay(Duration::from_secs(10))
            .without_jitter();

        // Should be capped at 10 seconds
        assert!(policy.delay_for_attempt(10) <= Duration::from_secs(10));
    }
}
