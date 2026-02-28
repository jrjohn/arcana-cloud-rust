//! Circuit breaker implementation.

use arcana_core::ArcanaError;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Circuit breaker state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CircuitState {
    /// Circuit is closed - requests are allowed.
    Closed = 0,
    /// Circuit is open - requests are rejected.
    Open = 1,
    /// Circuit is half-open - limited requests are allowed.
    HalfOpen = 2,
}

impl From<u8> for CircuitState {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Closed,
            1 => Self::Open,
            2 => Self::HalfOpen,
            _ => Self::Closed,
        }
    }
}

/// Circuit breaker configuration.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening the circuit.
    pub failure_threshold: u64,
    /// Number of successes needed to close the circuit from half-open.
    pub success_threshold: u64,
    /// Duration to wait before transitioning from open to half-open.
    pub timeout: Duration,
    /// Number of requests allowed in half-open state.
    pub half_open_requests: u64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout: Duration::from_secs(30),
            half_open_requests: 3,
        }
    }
}

/// Circuit breaker for protecting against cascading failures.
pub struct CircuitBreaker {
    name: String,
    state: AtomicU8,
    failure_count: AtomicU64,
    success_count: AtomicU64,
    half_open_requests: AtomicU64,
    last_failure_time: RwLock<Option<Instant>>,
    config: CircuitBreakerConfig,
}

impl CircuitBreaker {
    /// Creates a new circuit breaker.
    pub fn new(name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        Self {
            name: name.into(),
            state: AtomicU8::new(CircuitState::Closed as u8),
            failure_count: AtomicU64::new(0),
            success_count: AtomicU64::new(0),
            half_open_requests: AtomicU64::new(0),
            last_failure_time: RwLock::new(None),
            config,
        }
    }

    /// Creates a new circuit breaker with default configuration.
    pub fn with_defaults(name: impl Into<String>) -> Self {
        Self::new(name, CircuitBreakerConfig::default())
    }

    /// Returns the current state of the circuit breaker.
    pub fn state(&self) -> CircuitState {
        CircuitState::from(self.state.load(Ordering::SeqCst))
    }

    /// Returns the name of the circuit breaker.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Executes a function with circuit breaker protection.
    pub async fn call<F, Fut, T, E>(&self, f: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
    {
        // Check if circuit allows the request
        if !self.allow_request().await {
            return Err(CircuitBreakerError::Open(self.name.clone()));
        }

        // Execute the function
        match f().await {
            Ok(result) => {
                self.record_success().await;
                Ok(result)
            }
            Err(e) => {
                self.record_failure().await;
                Err(CircuitBreakerError::Failure(e))
            }
        }
    }

    /// Checks if a request should be allowed.
    async fn allow_request(&self) -> bool {
        let state = self.state();

        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout has passed
                let last_failure = self.last_failure_time.read().await;
                if let Some(time) = *last_failure {
                    if time.elapsed() >= self.config.timeout {
                        // Transition to half-open
                        self.state.store(CircuitState::HalfOpen as u8, Ordering::SeqCst);
                        self.success_count.store(0, Ordering::SeqCst);
                        self.half_open_requests.store(0, Ordering::SeqCst);
                        debug!("Circuit breaker '{}' transitioning to half-open", self.name);
                        return true;
                    }
                }
                false
            }
            CircuitState::HalfOpen => {
                // Allow limited requests in half-open state
                let requests = self.half_open_requests.fetch_add(1, Ordering::SeqCst);
                requests < self.config.half_open_requests
            }
        }
    }

    /// Records a successful call.
    async fn record_success(&self) {
        let state = self.state();

        match state {
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::SeqCst);
            }
            CircuitState::HalfOpen => {
                let successes = self.success_count.fetch_add(1, Ordering::SeqCst) + 1;
                if successes >= self.config.success_threshold {
                    // Close the circuit
                    self.state.store(CircuitState::Closed as u8, Ordering::SeqCst);
                    self.failure_count.store(0, Ordering::SeqCst);
                    debug!("Circuit breaker '{}' closed after successful recovery", self.name);
                }
            }
            CircuitState::Open => {
                // Should not happen, but handle gracefully
            }
        }
    }

    /// Records a failed call.
    async fn record_failure(&self) {
        let state = self.state();

        match state {
            CircuitState::Closed => {
                let failures = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
                *self.last_failure_time.write().await = Some(Instant::now());

                if failures >= self.config.failure_threshold {
                    // Open the circuit
                    self.state.store(CircuitState::Open as u8, Ordering::SeqCst);
                    warn!(
                        "Circuit breaker '{}' opened after {} failures",
                        self.name, failures
                    );
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in half-open state opens the circuit
                self.state.store(CircuitState::Open as u8, Ordering::SeqCst);
                *self.last_failure_time.write().await = Some(Instant::now());
                warn!(
                    "Circuit breaker '{}' reopened after failure in half-open state",
                    self.name
                );
            }
            CircuitState::Open => {
                *self.last_failure_time.write().await = Some(Instant::now());
            }
        }
    }

    /// Manually resets the circuit breaker to closed state.
    pub async fn reset(&self) {
        self.state.store(CircuitState::Closed as u8, Ordering::SeqCst);
        self.failure_count.store(0, Ordering::SeqCst);
        self.success_count.store(0, Ordering::SeqCst);
        *self.last_failure_time.write().await = None;
        debug!("Circuit breaker '{}' manually reset", self.name);
    }
}

/// Error type for circuit breaker operations.
#[derive(Debug)]
pub enum CircuitBreakerError<E> {
    /// Circuit is open, request was rejected.
    Open(String),
    /// The underlying operation failed.
    Failure(E),
}

impl<E: std::fmt::Display> std::fmt::Display for CircuitBreakerError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open(name) => write!(f, "Circuit breaker '{}' is open", name),
            Self::Failure(e) => write!(f, "Operation failed: {}", e),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for CircuitBreakerError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Open(_) => None,
            Self::Failure(e) => Some(e),
        }
    }
}

impl<E> From<CircuitBreakerError<E>> for ArcanaError
where
    E: std::fmt::Display,
{
    fn from(err: CircuitBreakerError<E>) -> Self {
        match err {
            CircuitBreakerError::Open(name) => ArcanaError::CircuitBreakerOpen(name),
            CircuitBreakerError::Failure(e) => ArcanaError::Internal(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_closed() {
        let cb = CircuitBreaker::with_defaults("test");

        let result = cb.call(|| async { Ok::<i32, &str>(42) }).await;
        assert!(result.is_ok());
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_initial_state() {
        let cb = CircuitBreaker::with_defaults("test");
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.name(), "test");
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_on_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            ..Default::default()
        };
        let cb = CircuitBreaker::new("test", config);

        // First failure
        let _ = cb.call(|| async { Err::<i32, &str>("error") }).await;
        assert_eq!(cb.state(), CircuitState::Closed);

        // Second failure - should open
        let _ = cb.call(|| async { Err::<i32, &str>("error") }).await;
        assert_eq!(cb.state(), CircuitState::Open);

        // Third call should be rejected
        let result = cb.call(|| async { Ok::<i32, &str>(42) }).await;
        assert!(matches!(result, Err(CircuitBreakerError::Open(_))));
    }

    #[tokio::test]
    async fn test_circuit_breaker_successful_call_returns_value() {
        let cb = CircuitBreaker::with_defaults("test");
        let result = cb.call(|| async { Ok::<i32, &str>(99) }).await;
        assert_eq!(result.unwrap(), 99);
    }

    #[tokio::test]
    async fn test_circuit_breaker_failure_returns_error() {
        let cb = CircuitBreaker::with_defaults("test-failure");
        let result = cb.call(|| async { Err::<i32, &str>("some error") }).await;
        assert!(result.is_err());
        match result {
            Err(CircuitBreakerError::Failure(_)) => {},
            other => panic!("Expected Failure error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_circuit_state_from_u8() {
        assert_eq!(CircuitState::from(0), CircuitState::Closed);
        assert_eq!(CircuitState::from(1), CircuitState::Open);
        assert_eq!(CircuitState::from(2), CircuitState::HalfOpen);
        assert_eq!(CircuitState::from(255), CircuitState::Closed); // unknown -> Closed
    }

    #[tokio::test]
    async fn test_circuit_breaker_single_failure_threshold_one() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            ..Default::default()
        };
        let cb = CircuitBreaker::new("single-fail", config);

        // Single failure should open
        let _ = cb.call(|| async { Err::<i32, &str>("error") }).await;
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_name() {
        let cb = CircuitBreaker::with_defaults("my-service");
        assert_eq!(cb.name(), "my-service");
    }

    #[tokio::test]
    async fn test_circuit_breaker_config_default() {
        let config = CircuitBreakerConfig::default();
        assert_eq!(config.failure_threshold, 5);
        assert_eq!(config.success_threshold, 3);
        assert_eq!(config.half_open_requests, 3);
        assert!(config.timeout.as_secs() > 0);
    }
}
