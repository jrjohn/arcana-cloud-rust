//! Timeout wrapper for async operations.

use arcana_core::ArcanaError;
use std::time::Duration;

/// Wraps an async operation with a timeout.
pub async fn with_timeout<F, Fut, T>(duration: Duration, f: F) -> Result<T, ArcanaError>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T, ArcanaError>>,
{
    tokio::time::timeout(duration, f())
        .await
        .map_err(|_| ArcanaError::Timeout(format!("Operation timed out after {:?}", duration)))?
}

/// Timeout configuration.
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    /// Default timeout duration.
    pub default_timeout: Duration,
    /// Timeout for database operations.
    pub database_timeout: Duration,
    /// Timeout for external service calls.
    pub external_service_timeout: Duration,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            default_timeout: Duration::from_secs(30),
            database_timeout: Duration::from_secs(10),
            external_service_timeout: Duration::from_secs(60),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_timeout_success() {
        let result = with_timeout(Duration::from_secs(1), || async { Ok::<_, ArcanaError>(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_timeout_exceeded() {
        let result = with_timeout(Duration::from_millis(10), || async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok::<_, ArcanaError>(42)
        })
        .await;

        assert!(matches!(result, Err(ArcanaError::Timeout(_))));
    }
}
