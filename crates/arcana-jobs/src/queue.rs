//! Job queue abstraction.

use crate::error::{JobError, JobResult};
use crate::job::{Job, JobData, JobId, JobInfo};
use crate::retry::RetryPolicy;
use async_trait::async_trait;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Job priority levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(i8)]
pub enum Priority {
    /// Low priority (background tasks).
    Low = -10,
    /// Normal priority (default).
    Normal = 0,
    /// High priority (important tasks).
    High = 10,
    /// Critical priority (time-sensitive).
    Critical = 20,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

impl From<i8> for Priority {
    fn from(value: i8) -> Self {
        match value {
            v if v >= 20 => Priority::Critical,
            v if v >= 10 => Priority::High,
            v if v <= -10 => Priority::Low,
            _ => Priority::Normal,
        }
    }
}

impl From<Priority> for i8 {
    fn from(priority: Priority) -> Self {
        priority as i8
    }
}

/// Builder for enqueuing jobs with options.
pub struct QueuedJob<J: Job> {
    job: J,
    priority: Priority,
    delay: Option<Duration>,
    scheduled_at: Option<DateTime<Utc>>,
    correlation_id: Option<String>,
    tags: Vec<String>,
    retry_policy: Option<RetryPolicy>,
}

impl<J: Job> QueuedJob<J> {
    /// Create a new queued job builder.
    pub fn new(job: J) -> Self {
        Self {
            job,
            priority: Priority::Normal,
            delay: None,
            scheduled_at: None,
            correlation_id: None,
            tags: Vec::new(),
            retry_policy: None,
        }
    }

    /// Set the priority.
    pub fn priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Set a delay before execution.
    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self.scheduled_at = None;
        self
    }

    /// Schedule for a specific time.
    pub fn at(mut self, scheduled_at: DateTime<Utc>) -> Self {
        self.scheduled_at = Some(scheduled_at);
        self.delay = None;
        self
    }

    /// Set correlation ID for tracing.
    pub fn correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    /// Add a tag.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add multiple tags.
    pub fn tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags.extend(tags.into_iter().map(|t| t.into()));
        self
    }

    /// Override the retry policy.
    pub fn with_retry(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = Some(policy);
        self
    }

    /// Build the job data.
    pub fn build(self) -> JobResult<JobData> {
        let mut data = JobData::new(&self.job)?;

        data.priority = self.priority.into();
        data.correlation_id = self.correlation_id;
        data.tags = self.tags;

        if let Some(policy) = self.retry_policy {
            data.retry_policy = Some(serde_json::to_string(&policy)?);
            data.max_attempts = policy.max_retries + 1;
        }

        // Set scheduled time
        if let Some(at) = self.scheduled_at {
            data.scheduled_at = at;
        } else if let Some(delay) = self.delay {
            data.scheduled_at = Utc::now() + ChronoDuration::from_std(delay).unwrap_or_default();
        }

        Ok(data)
    }
}

/// Job queue trait for different backends.
#[async_trait]
pub trait JobQueue: Send + Sync {
    /// Enqueue a job.
    async fn enqueue<J: Job>(&self, job: J) -> JobResult<JobId> {
        self.enqueue_with(QueuedJob::new(job)).await
    }

    /// Enqueue a job with options.
    async fn enqueue_with<J: Job>(&self, queued: QueuedJob<J>) -> JobResult<JobId>;

    /// Enqueue a job for later execution.
    async fn enqueue_delayed<J: Job>(&self, job: J, delay: Duration) -> JobResult<JobId> {
        self.enqueue_with(QueuedJob::new(job).delay(delay)).await
    }

    /// Enqueue a job at a specific time.
    async fn enqueue_at<J: Job>(&self, job: J, at: DateTime<Utc>) -> JobResult<JobId> {
        self.enqueue_with(QueuedJob::new(job).at(at)).await
    }

    /// Dequeue the next job from the specified queues.
    async fn dequeue(&self, queues: &[&str], worker_id: &str) -> JobResult<Option<JobData>>;

    /// Complete a job successfully.
    async fn complete(&self, job_id: &JobId) -> JobResult<()>;

    /// Fail a job (may retry or move to DLQ).
    async fn fail(&self, job_id: &JobId, error: &JobError) -> JobResult<()>;

    /// Retry a job.
    async fn retry(&self, job_data: &JobData) -> JobResult<()>;

    /// Move a job to the dead letter queue.
    async fn dead_letter(&self, job_data: &JobData, error: &JobError) -> JobResult<()>;

    /// Get job info by ID.
    async fn get_job(&self, job_id: &JobId) -> JobResult<Option<JobInfo>>;

    /// Get queue length.
    async fn queue_length(&self, queue: &str) -> JobResult<u64>;

    /// Get jobs in queue.
    async fn list_jobs(&self, queue: &str, limit: usize, offset: usize) -> JobResult<Vec<JobInfo>>;

    /// Get dead letter queue jobs.
    async fn list_dlq(&self, limit: usize, offset: usize) -> JobResult<Vec<JobInfo>>;

    /// Retry a job from DLQ.
    async fn retry_dlq(&self, job_id: &JobId) -> JobResult<()>;

    /// Delete a job.
    async fn delete(&self, job_id: &JobId) -> JobResult<()>;

    /// Purge completed jobs older than the given duration.
    async fn purge_completed(&self, older_than: Duration) -> JobResult<u64>;

    /// Cancel a pending job.
    async fn cancel(&self, job_id: &JobId) -> JobResult<()>;

    /// Health check.
    async fn health_check(&self) -> JobResult<()>;
}

/// Queue statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    /// Queue name.
    pub queue: String,

    /// Pending jobs count.
    pub pending: u64,

    /// Active (processing) jobs count.
    pub active: u64,

    /// Completed jobs count (if tracking).
    pub completed: u64,

    /// Failed jobs count.
    pub failed: u64,

    /// Dead letter queue count.
    pub dead_letter: u64,

    /// Delayed jobs count.
    pub delayed: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Critical > Priority::High);
        assert!(Priority::High > Priority::Normal);
        assert!(Priority::Normal > Priority::Low);
    }

    #[test]
    fn test_priority_from_i8() {
        assert_eq!(Priority::from(25), Priority::Critical);
        assert_eq!(Priority::from(15), Priority::High);
        assert_eq!(Priority::from(0), Priority::Normal);
        assert_eq!(Priority::from(-15), Priority::Low);
    }
}
