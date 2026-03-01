//! Job error types.

use thiserror::Error;

/// Result type for job operations.
pub type JobResult<T> = Result<T, JobError>;

/// Job-related errors.
#[derive(Debug, Error)]
pub enum JobError {
    /// Job execution failed.
    #[error("Job execution failed: {0}")]
    ExecutionFailed(String),

    /// Job was cancelled.
    #[error("Job was cancelled")]
    Cancelled,

    /// Job timed out.
    #[error("Job timed out after {0} seconds")]
    Timeout(u64),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Redis error.
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    /// Redis pool error.
    #[error("Redis pool error: {0}")]
    Pool(#[from] deadpool_redis::PoolError),

    /// Job not found.
    #[error("Job not found: {0}")]
    NotFound(String),

    /// Invalid job state.
    #[error("Invalid job state: expected {expected}, got {actual}")]
    InvalidState { expected: String, actual: String },

    /// Queue is full.
    #[error("Queue is full: {0}")]
    QueueFull(String),

    /// Worker error.
    #[error("Worker error: {0}")]
    Worker(String),

    /// Scheduler error.
    #[error("Scheduler error: {0}")]
    Scheduler(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Max retries exceeded.
    #[error("Max retries exceeded for job {job_id}: {attempts} attempts")]
    MaxRetriesExceeded { job_id: String, attempts: u32 },

    /// Internal error.
    #[error("Internal error: {0}")]
    Internal(String),
}

impl JobError {
    /// Returns true if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            JobError::ExecutionFailed(_)
                | JobError::Timeout(_)
                | JobError::Redis(_)
                | JobError::Pool(_)
                | JobError::Worker(_)
        )
    }

    /// Returns true if the job should be moved to dead letter queue.
    pub fn should_dlq(&self) -> bool {
        matches!(
            self,
            JobError::MaxRetriesExceeded { .. }
                | JobError::Serialization(_)
                | JobError::Configuration(_)
        )
    }
}

impl From<arcana_core::ArcanaError> for JobError {
    fn from(err: arcana_core::ArcanaError) -> Self {
        JobError::Internal(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_retryable_execution_failed() {
        let err = JobError::ExecutionFailed("oops".into());
        assert!(err.is_retryable());
    }

    #[test]
    fn test_is_retryable_timeout() {
        let err = JobError::Timeout(30);
        assert!(err.is_retryable());
    }

    #[test]
    fn test_is_retryable_worker_error() {
        let err = JobError::Worker("crash".into());
        assert!(err.is_retryable());
    }

    #[test]
    fn test_is_not_retryable_cancelled() {
        let err = JobError::Cancelled;
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_is_not_retryable_not_found() {
        let err = JobError::NotFound("job-123".into());
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_is_not_retryable_invalid_state() {
        let err = JobError::InvalidState {
            expected: "pending".into(),
            actual: "running".into(),
        };
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_should_dlq_max_retries_exceeded() {
        let err = JobError::MaxRetriesExceeded {
            job_id: "abc".into(),
            attempts: 5,
        };
        assert!(err.should_dlq());
    }

    #[test]
    fn test_should_dlq_configuration() {
        let err = JobError::Configuration("missing key".into());
        assert!(err.should_dlq());
    }

    #[test]
    fn test_should_not_dlq_execution_failed() {
        let err = JobError::ExecutionFailed("transient".into());
        assert!(!err.should_dlq());
    }

    #[test]
    fn test_from_arcana_error() {
        let arcana_err = arcana_core::ArcanaError::Internal("database down".into());
        let job_err = JobError::from(arcana_err);
        match job_err {
            JobError::Internal(msg) => assert!(msg.contains("database down")),
            _ => panic!("Expected Internal error"),
        }
    }

    #[test]
    fn test_error_display_timeout() {
        let err = JobError::Timeout(60);
        assert!(err.to_string().contains("60"));
    }

    #[test]
    fn test_error_display_invalid_state() {
        let err = JobError::InvalidState {
            expected: "ready".into(),
            actual: "done".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("ready") && msg.contains("done"));
    }

    #[test]
    fn test_error_display_max_retries() {
        let err = JobError::MaxRetriesExceeded {
            job_id: "job-xyz".into(),
            attempts: 3,
        };
        let msg = err.to_string();
        assert!(msg.contains("job-xyz") && msg.contains("3"));
    }

    #[test]
    fn test_queue_full_error() {
        let err = JobError::QueueFull("main-queue".into());
        assert!(!err.is_retryable());
        assert!(!err.should_dlq());
    }

    #[test]
    fn test_scheduler_error() {
        let err = JobError::Scheduler("cron parse failed".into());
        assert!(!err.is_retryable());
        assert!(!err.should_dlq());
    }
}
