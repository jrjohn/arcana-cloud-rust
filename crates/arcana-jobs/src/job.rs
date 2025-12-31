//! Job trait and definitions.

use crate::error::{JobError, JobResult};
use crate::retry::RetryPolicy;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt;
use std::time::Duration;
use uuid::Uuid;

/// Unique job identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobId(String);

impl JobId {
    /// Creates a new random job ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Creates a job ID from a string.
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the job ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for JobId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for JobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for JobId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for JobId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Job execution context.
#[derive(Debug, Clone)]
pub struct JobContext {
    /// Job ID.
    pub job_id: JobId,

    /// Current attempt number (1-based).
    pub attempt: u32,

    /// Maximum attempts allowed.
    pub max_attempts: u32,

    /// Queue name.
    pub queue: String,

    /// Job was scheduled at this time.
    pub scheduled_at: DateTime<Utc>,

    /// Job started executing at this time.
    pub started_at: DateTime<Utc>,

    /// Correlation ID for tracing.
    pub correlation_id: Option<String>,

    /// Worker ID processing this job.
    pub worker_id: String,
}

impl JobContext {
    /// Returns true if this is the last attempt.
    pub fn is_last_attempt(&self) -> bool {
        self.attempt >= self.max_attempts
    }

    /// Returns remaining attempts.
    pub fn remaining_attempts(&self) -> u32 {
        self.max_attempts.saturating_sub(self.attempt)
    }
}

/// Trait for defining jobs.
///
/// Implement this trait to create custom job types that can be
/// enqueued and processed by workers.
///
/// # Example
///
/// ```rust,ignore
/// use arcana_jobs::{Job, JobContext, JobError};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Serialize, Deserialize)]
/// struct SendEmailJob {
///     to: String,
///     subject: String,
///     body: String,
/// }
///
/// #[async_trait::async_trait]
/// impl Job for SendEmailJob {
///     const NAME: &'static str = "send_email";
///     const QUEUE: &'static str = "emails";
///
///     async fn execute(&self, ctx: JobContext) -> Result<(), JobError> {
///         println!("Sending email to: {}", self.to);
///         // Send email logic here
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait Job: Serialize + DeserializeOwned + Send + Sync + 'static {
    /// Unique name for this job type.
    const NAME: &'static str;

    /// Queue name for this job type.
    const QUEUE: &'static str = "default";

    /// Maximum number of retry attempts.
    const MAX_RETRIES: u32 = 3;

    /// Job timeout in seconds.
    const TIMEOUT_SECS: u64 = 300;

    /// Execute the job.
    async fn execute(&self, ctx: JobContext) -> Result<(), JobError>;

    /// Called before execution starts.
    fn before_execute(&self, _ctx: &JobContext) {}

    /// Called after successful execution.
    fn after_execute(&self, _ctx: &JobContext) {}

    /// Called when execution fails.
    fn on_failure(&self, _ctx: &JobContext, _error: &JobError) {}

    /// Called when all retries are exhausted.
    fn on_dead_letter(&self, _ctx: &JobContext, _error: &JobError) {}

    /// Returns the retry policy for this job.
    fn retry_policy(&self) -> RetryPolicy {
        RetryPolicy::exponential(Self::MAX_RETRIES)
    }

    /// Returns the job timeout.
    fn timeout(&self) -> Duration {
        Duration::from_secs(Self::TIMEOUT_SECS)
    }

    /// Returns true if this job is unique (prevents duplicate enqueuing).
    fn is_unique(&self) -> bool {
        false
    }

    /// Returns the unique key for deduplication.
    fn unique_key(&self) -> Option<String> {
        None
    }

    /// Returns the unique TTL in seconds.
    fn unique_ttl(&self) -> u64 {
        3600 // 1 hour
    }
}

/// Serialized job data stored in the queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobData {
    /// Job ID.
    pub id: JobId,

    /// Job type name.
    pub name: String,

    /// Queue name.
    pub queue: String,

    /// Serialized job payload.
    pub payload: String,

    /// Current attempt number.
    pub attempt: u32,

    /// Maximum attempts.
    pub max_attempts: u32,

    /// Job timeout in seconds.
    pub timeout_secs: u64,

    /// When the job was created.
    pub created_at: DateTime<Utc>,

    /// When the job should be executed (for delayed jobs).
    pub scheduled_at: DateTime<Utc>,

    /// Priority (higher = more urgent).
    pub priority: i8,

    /// Correlation ID for tracing.
    pub correlation_id: Option<String>,

    /// Tags for categorization.
    pub tags: Vec<String>,

    /// Retry policy serialized.
    pub retry_policy: Option<String>,

    /// Unique key for deduplication.
    pub unique_key: Option<String>,

    /// Error from last failed attempt.
    pub last_error: Option<String>,
}

impl JobData {
    /// Creates new job data from a Job instance.
    pub fn new<J: Job>(job: &J) -> JobResult<Self> {
        let payload = serde_json::to_string(job)?;

        Ok(Self {
            id: JobId::new(),
            name: J::NAME.to_string(),
            queue: J::QUEUE.to_string(),
            payload,
            attempt: 0,
            max_attempts: J::MAX_RETRIES + 1, // +1 for initial attempt
            timeout_secs: J::TIMEOUT_SECS,
            created_at: Utc::now(),
            scheduled_at: Utc::now(),
            priority: 0,
            correlation_id: None,
            tags: Vec::new(),
            retry_policy: Some(serde_json::to_string(&job.retry_policy())?),
            unique_key: job.unique_key(),
            last_error: None,
        })
    }

    /// Deserialize the job payload.
    pub fn deserialize<J: Job>(&self) -> JobResult<J> {
        Ok(serde_json::from_str(&self.payload)?)
    }

    /// Increment attempt counter.
    pub fn increment_attempt(&mut self) {
        self.attempt += 1;
    }

    /// Check if max attempts reached.
    pub fn is_exhausted(&self) -> bool {
        self.attempt >= self.max_attempts
    }

    /// Set the last error.
    pub fn set_error(&mut self, error: &JobError) {
        self.last_error = Some(error.to_string());
    }

    /// Create job context for execution.
    pub fn to_context(&self, worker_id: &str) -> JobContext {
        JobContext {
            job_id: self.id.clone(),
            attempt: self.attempt,
            max_attempts: self.max_attempts,
            queue: self.queue.clone(),
            scheduled_at: self.scheduled_at,
            started_at: Utc::now(),
            correlation_id: self.correlation_id.clone(),
            worker_id: worker_id.to_string(),
        }
    }

    /// Serialize to JSON.
    pub fn to_json(&self) -> JobResult<String> {
        Ok(serde_json::to_string(self)?)
    }

    /// Deserialize from JSON.
    pub fn from_json(json: &str) -> JobResult<Self> {
        Ok(serde_json::from_str(json)?)
    }
}

/// Job information for status queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobInfo {
    /// Job ID.
    pub id: JobId,

    /// Job type name.
    pub name: String,

    /// Queue name.
    pub queue: String,

    /// Current status.
    pub status: String,

    /// Current attempt.
    pub attempt: u32,

    /// Max attempts.
    pub max_attempts: u32,

    /// Created timestamp.
    pub created_at: DateTime<Utc>,

    /// Scheduled timestamp.
    pub scheduled_at: DateTime<Utc>,

    /// Started timestamp.
    pub started_at: Option<DateTime<Utc>>,

    /// Completed timestamp.
    pub completed_at: Option<DateTime<Utc>>,

    /// Priority.
    pub priority: i8,

    /// Last error.
    pub last_error: Option<String>,

    /// Tags.
    pub tags: Vec<String>,

    /// Worker ID (if being processed).
    pub worker_id: Option<String>,
}

impl From<JobData> for JobInfo {
    fn from(data: JobData) -> Self {
        Self {
            id: data.id,
            name: data.name,
            queue: data.queue,
            status: "pending".to_string(),
            attempt: data.attempt,
            max_attempts: data.max_attempts,
            created_at: data.created_at,
            scheduled_at: data.scheduled_at,
            started_at: None,
            completed_at: None,
            priority: data.priority,
            last_error: data.last_error,
            tags: data.tags,
            worker_id: None,
        }
    }
}

/// Job status enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    /// Job is pending execution.
    Pending,
    /// Job is scheduled for later execution.
    Scheduled,
    /// Job is currently running.
    Running,
    /// Job completed successfully.
    Completed,
    /// Job failed.
    Failed,
    /// Job is in the dead letter queue.
    DeadLetter,
    /// Job was cancelled.
    Cancelled,
}

impl Default for JobStatus {
    fn default() -> Self {
        JobStatus::Pending
    }
}

impl fmt::Display for JobStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobStatus::Pending => write!(f, "pending"),
            JobStatus::Scheduled => write!(f, "scheduled"),
            JobStatus::Running => write!(f, "running"),
            JobStatus::Completed => write!(f, "completed"),
            JobStatus::Failed => write!(f, "failed"),
            JobStatus::DeadLetter => write!(f, "dead_letter"),
            JobStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Serialize, Deserialize)]
    struct TestJob {
        message: String,
    }

    #[async_trait]
    impl Job for TestJob {
        const NAME: &'static str = "test_job";
        const QUEUE: &'static str = "test";

        async fn execute(&self, _ctx: JobContext) -> Result<(), JobError> {
            Ok(())
        }
    }

    #[test]
    fn test_job_id_generation() {
        let id1 = JobId::new();
        let id2 = JobId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_job_data_serialization() {
        let job = TestJob {
            message: "Hello".to_string(),
        };

        let data = JobData::new(&job).unwrap();
        assert_eq!(data.name, "test_job");
        assert_eq!(data.queue, "test");

        let json = data.to_json().unwrap();
        let restored = JobData::from_json(&json).unwrap();
        assert_eq!(data.id, restored.id);
    }

    #[test]
    fn test_job_context() {
        let job = TestJob {
            message: "Test".to_string(),
        };
        let data = JobData::new(&job).unwrap();
        let ctx = data.to_context("worker-1");

        assert_eq!(ctx.attempt, 0);
        assert_eq!(ctx.max_attempts, 4); // 3 retries + 1 initial
        assert!(!ctx.is_last_attempt());
    }
}
