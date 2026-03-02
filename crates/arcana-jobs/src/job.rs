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

    // =========================================================================
    // JobId tests
    // =========================================================================

    #[test]
    fn test_job_id_generation() {
        let id1 = JobId::new();
        let id2 = JobId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_job_id_from_string() {
        let id = JobId::from_string("my-job-123");
        assert_eq!(id.as_str(), "my-job-123");
    }

    #[test]
    fn test_job_id_as_str() {
        let id = JobId::from_string("abc");
        assert_eq!(id.as_str(), "abc");
    }

    #[test]
    fn test_job_id_display() {
        let id = JobId::from_string("display-test");
        assert_eq!(id.to_string(), "display-test");
    }

    #[test]
    fn test_job_id_from_owned_string() {
        let s = "owned-string".to_string();
        let id: JobId = JobId::from(s);
        assert_eq!(id.as_str(), "owned-string");
    }

    #[test]
    fn test_job_id_from_str_ref() {
        let id: JobId = JobId::from("str-ref");
        assert_eq!(id.as_str(), "str-ref");
    }

    #[test]
    fn test_job_id_default() {
        let id1 = JobId::default();
        let id2 = JobId::default();
        // Both should be non-empty UUIDs
        assert!(!id1.as_str().is_empty());
        assert!(!id2.as_str().is_empty());
        // And different
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_job_id_equality() {
        let id1 = JobId::from_string("same");
        let id2 = JobId::from_string("same");
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_job_id_clone() {
        let id = JobId::from_string("clone-me");
        let id2 = id.clone();
        assert_eq!(id, id2);
    }

    // =========================================================================
    // JobData tests
    // =========================================================================

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
    fn test_job_data_increment_attempt() {
        let job = TestJob { message: "inc".to_string() };
        let mut data = JobData::new(&job).unwrap();
        assert_eq!(data.attempt, 0);
        data.increment_attempt();
        assert_eq!(data.attempt, 1);
        data.increment_attempt();
        assert_eq!(data.attempt, 2);
    }

    #[test]
    fn test_job_data_is_exhausted_false() {
        let job = TestJob { message: "not done".to_string() };
        let data = JobData::new(&job).unwrap();
        // attempt=0, max_attempts=4
        assert!(!data.is_exhausted());
    }

    #[test]
    fn test_job_data_is_exhausted_true() {
        let job = TestJob { message: "done".to_string() };
        let mut data = JobData::new(&job).unwrap();
        // max_attempts = MAX_RETRIES + 1 = 4
        data.attempt = data.max_attempts;
        assert!(data.is_exhausted());
    }

    #[test]
    fn test_job_data_set_error() {
        let job = TestJob { message: "err".to_string() };
        let mut data = JobData::new(&job).unwrap();
        assert!(data.last_error.is_none());
        let err = JobError::ExecutionFailed("something went wrong".to_string());
        data.set_error(&err);
        assert!(data.last_error.is_some());
        assert!(data.last_error.unwrap().contains("something went wrong"));
    }

    #[test]
    fn test_job_data_deserialize_payload() {
        let job = TestJob { message: "deserialize me".to_string() };
        let data = JobData::new(&job).unwrap();
        let restored: TestJob = data.deserialize().unwrap();
        assert_eq!(restored.message, "deserialize me");
    }

    #[test]
    fn test_job_data_from_json_error() {
        let result = JobData::from_json("not valid json at all!!!");
        assert!(result.is_err());
    }

    #[test]
    fn test_job_data_clone() {
        let job = TestJob { message: "clone".to_string() };
        let data = JobData::new(&job).unwrap();
        let c = data.clone();
        assert_eq!(data.id, c.id);
    }

    // =========================================================================
    // JobContext tests
    // =========================================================================

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

    #[test]
    fn test_job_context_is_last_attempt_true() {
        let job = TestJob { message: "last".to_string() };
        let mut data = JobData::new(&job).unwrap();
        data.attempt = data.max_attempts; // at/beyond max
        let ctx = data.to_context("worker-1");
        assert!(ctx.is_last_attempt());
    }

    #[test]
    fn test_job_context_remaining_attempts() {
        let job = TestJob { message: "remaining".to_string() };
        let data = JobData::new(&job).unwrap();
        let ctx = data.to_context("worker-1");
        // attempt=0, max_attempts=4 → remaining=4
        assert_eq!(ctx.remaining_attempts(), 4);
    }

    #[test]
    fn test_job_context_remaining_attempts_partial() {
        let job = TestJob { message: "partial".to_string() };
        let mut data = JobData::new(&job).unwrap();
        data.attempt = 2;
        let ctx = data.to_context("worker-x");
        assert_eq!(ctx.remaining_attempts(), 2);
    }

    #[test]
    fn test_job_context_remaining_attempts_saturating() {
        let job = TestJob { message: "over".to_string() };
        let mut data = JobData::new(&job).unwrap();
        data.attempt = data.max_attempts + 5; // over max
        let ctx = data.to_context("worker-y");
        // saturating_sub should return 0, not panic
        assert_eq!(ctx.remaining_attempts(), 0);
    }

    #[test]
    fn test_job_context_worker_id() {
        let job = TestJob { message: "worker".to_string() };
        let data = JobData::new(&job).unwrap();
        let ctx = data.to_context("worker-42");
        assert_eq!(ctx.worker_id, "worker-42");
    }

    // =========================================================================
    // JobInfo / From<JobData> tests
    // =========================================================================

    #[test]
    fn test_job_info_from_job_data() {
        let job = TestJob { message: "info".to_string() };
        let data = JobData::new(&job).unwrap();
        let data_id = data.id.clone();
        let info: JobInfo = JobInfo::from(data);
        assert_eq!(info.id, data_id);
        assert_eq!(info.name, "test_job");
        assert_eq!(info.queue, "test");
        assert_eq!(info.status, "pending");
        assert!(info.started_at.is_none());
        assert!(info.completed_at.is_none());
        assert!(info.worker_id.is_none());
    }

    // =========================================================================
    // JobStatus tests
    // =========================================================================

    #[test]
    fn test_job_status_default() {
        let s = JobStatus::default();
        assert_eq!(s, JobStatus::Pending);
    }

    #[test]
    fn test_job_status_display() {
        assert_eq!(JobStatus::Pending.to_string(), "pending");
        assert_eq!(JobStatus::Scheduled.to_string(), "scheduled");
        assert_eq!(JobStatus::Running.to_string(), "running");
        assert_eq!(JobStatus::Completed.to_string(), "completed");
        assert_eq!(JobStatus::Failed.to_string(), "failed");
        assert_eq!(JobStatus::DeadLetter.to_string(), "dead_letter");
        assert_eq!(JobStatus::Cancelled.to_string(), "cancelled");
    }

    #[test]
    fn test_job_status_equality() {
        assert_eq!(JobStatus::Pending, JobStatus::Pending);
        assert_ne!(JobStatus::Pending, JobStatus::Failed);
    }

    #[test]
    fn test_job_status_clone_copy() {
        let s = JobStatus::Running;
        let s2 = s; // Copy
        assert_eq!(s, s2);
        let s3 = s.clone();
        assert_eq!(s, s3);
    }

    #[test]
    fn test_job_status_serde() {
        let s = JobStatus::DeadLetter;
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("dead_letter"));
        let back: JobStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, JobStatus::DeadLetter);
    }
}
