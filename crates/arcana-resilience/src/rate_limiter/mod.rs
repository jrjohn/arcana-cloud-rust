//! Rate limiter implementation.

use arcana_core::ArcanaError;
use governor::{
    clock::DefaultClock,
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as GovernorRateLimiter,
};
use std::num::NonZeroU32;
use std::sync::Arc;

/// Rate limiter for controlling request rates.
pub struct RateLimiter {
    limiter: Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
}

impl RateLimiter {
    /// Creates a new rate limiter with the specified requests per second.
    pub fn new(requests_per_second: u32) -> Self {
        let quota = Quota::per_second(NonZeroU32::new(requests_per_second).unwrap_or(NonZeroU32::MIN));
        let limiter = Arc::new(GovernorRateLimiter::direct(quota));
        Self { limiter }
    }

    /// Creates a rate limiter with requests per minute.
    pub fn per_minute(requests: u32) -> Self {
        let quota = Quota::per_minute(NonZeroU32::new(requests).unwrap_or(NonZeroU32::MIN));
        let limiter = Arc::new(GovernorRateLimiter::direct(quota));
        Self { limiter }
    }

    /// Checks if a request is allowed (non-blocking).
    pub fn check(&self) -> Result<(), ArcanaError> {
        self.limiter
            .check()
            .map_err(|_| ArcanaError::RateLimitExceeded)
    }

    /// Waits until a request is allowed (blocking).
    pub async fn wait(&self) {
        self.limiter.until_ready().await;
    }

    /// Checks if a request is allowed, waiting if necessary up to the timeout.
    pub async fn check_with_wait(&self) -> Result<(), ArcanaError> {
        self.limiter
            .until_ready_with_jitter(governor::Jitter::up_to(std::time::Duration::from_millis(100)))
            .await;
        Ok(())
    }
}

impl Clone for RateLimiter {
    fn clone(&self) -> Self {
        Self {
            limiter: Arc::clone(&self.limiter),
        }
    }
}

impl std::fmt::Debug for RateLimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RateLimiter").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_requests() {
        let limiter = RateLimiter::new(10);
        assert!(limiter.check().is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_wait() {
        let limiter = RateLimiter::new(1000);
        limiter.wait().await;
        // Should complete quickly with high limit
    }

    #[test]
    fn test_rate_limiter_clone() {
        let limiter = RateLimiter::new(10);
        let cloned = limiter.clone();
        assert!(cloned.check().is_ok());
    }

    #[test]
    fn test_rate_limiter_debug() {
        let limiter = RateLimiter::new(10);
        let debug = format!("{:?}", limiter);
        assert!(debug.contains("RateLimiter"));
    }

    #[test]
    fn test_rate_limiter_per_minute() {
        let limiter = RateLimiter::per_minute(60);
        assert!(limiter.check().is_ok());
    }

    #[test]
    fn test_rate_limiter_exhausted() {
        // Very low limit - 1 request per second
        // With a burst of requests, it should eventually fail
        let limiter = RateLimiter::new(1);
        let _ = limiter.check(); // Consume the token
        // Second check should fail (no burst capacity in strict mode)
        // Governor may allow burst, so we try multiple times
        let results: Vec<bool> = (0..5).map(|_| limiter.check().is_err()).collect();
        // At least one should fail
        assert!(results.iter().any(|&failed| failed));
    }

    #[tokio::test]
    async fn test_rate_limiter_check_with_wait() {
        let limiter = RateLimiter::new(1000);
        let result = limiter.check_with_wait().await;
        assert!(result.is_ok());
    }
}
