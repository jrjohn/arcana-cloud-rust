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
