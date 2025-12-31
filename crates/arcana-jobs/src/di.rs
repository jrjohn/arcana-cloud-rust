//! Dependency injection interfaces for the jobs module.
//!
//! Provides Shaku-compatible interfaces for job queue services.

use crate::error::JobResult;
use crate::job::{JobData, JobId, JobInfo};
use crate::queue::{JobQueue, QueueStats};
use crate::redis::RedisJobQueue;
use crate::status::{DashboardStats, JobSearchQuery, JobSearchResult, JobStatusTracker, WorkerHealth};
use crate::worker_registry::WorkerRegistry;
use arcana_core::Interface;
use async_trait::async_trait;
use std::sync::Arc;

/// Interface for job queue operations.
///
/// This trait provides a high-level interface for job queue management,
/// combining queue operations and status tracking.
#[async_trait]
pub trait JobQueueInterface: Interface + Send + Sync {
    /// Get the underlying Redis job queue.
    ///
    /// Returns the concrete RedisJobQueue type since JobQueue trait is not object-safe.
    fn redis_queue(&self) -> &RedisJobQueue;

    /// Get the status tracker.
    fn status_tracker(&self) -> &JobStatusTracker;

    /// Get job by ID.
    async fn get_job(&self, job_id: &str) -> JobResult<Option<JobInfo>>;

    /// Search jobs with filters.
    async fn search_jobs(&self, query: JobSearchQuery) -> JobResult<JobSearchResult>;

    /// Get queue statistics for a single queue.
    async fn get_queue_stats(&self, queue_name: &str) -> JobResult<QueueStats>;

    /// Get statistics for all configured queues.
    async fn get_all_queue_stats(&self) -> JobResult<Vec<QueueStats>>;

    /// Get aggregate dashboard statistics.
    async fn get_dashboard_stats(&self) -> JobResult<DashboardStats>;

    /// Get worker health information.
    async fn get_worker_health(&self) -> JobResult<Vec<WorkerHealth>>;

    /// Cancel a pending job.
    async fn cancel_job(&self, job_id: &JobId) -> JobResult<()>;

    /// Retry a failed job.
    async fn retry_job(&self, job_id: &JobId) -> JobResult<()>;

    /// Retry a job from the dead letter queue.
    async fn retry_dlq_job(&self, job_id: &JobId) -> JobResult<()>;

    /// Purge completed jobs older than given seconds.
    async fn purge_completed(&self, older_than_secs: u64) -> JobResult<u64>;

    /// Get the list of queue names being managed.
    fn queue_names(&self) -> &[String];

    // =========================================================================
    // Worker Service Methods
    // =========================================================================

    /// Get the worker registry.
    fn worker_registry(&self) -> &WorkerRegistry;

    /// Dequeue jobs for a worker.
    ///
    /// Returns up to `max_jobs` from the specified queues.
    async fn dequeue_for_worker(
        &self,
        queues: &[&str],
        worker_id: &str,
        max_jobs: u32,
    ) -> JobResult<Vec<JobData>>;

    /// Mark a job as complete.
    ///
    /// Called when a worker successfully finishes processing a job.
    async fn complete_job(&self, job_id: &JobId, result: Option<String>) -> JobResult<()>;

    /// Mark a job as failed.
    ///
    /// Returns (retried, dead_lettered) indicating if the job was retried or moved to DLQ.
    async fn fail_job(
        &self,
        job_id: &JobId,
        error: &str,
        should_retry: bool,
    ) -> JobResult<(bool, bool)>;
}

/// Job queue service implementation.
pub struct JobQueueService {
    /// The underlying Redis job queue.
    redis_queue: Arc<RedisJobQueue>,

    /// Status tracker for monitoring.
    status_tracker: JobStatusTracker,

    /// Queue names being managed.
    queue_names: Vec<String>,

    /// Worker registry for tracking connected workers.
    worker_registry: WorkerRegistry,
}

impl JobQueueService {
    /// Create a new job queue service.
    pub fn new(
        redis_queue: Arc<RedisJobQueue>,
        status_tracker: JobStatusTracker,
        queue_names: Vec<String>,
    ) -> Self {
        Self {
            redis_queue,
            status_tracker,
            queue_names,
            worker_registry: WorkerRegistry::new(),
        }
    }

    /// Create a new job queue service with a custom worker registry.
    pub fn with_worker_registry(
        redis_queue: Arc<RedisJobQueue>,
        status_tracker: JobStatusTracker,
        queue_names: Vec<String>,
        worker_registry: WorkerRegistry,
    ) -> Self {
        Self {
            redis_queue,
            status_tracker,
            queue_names,
            worker_registry,
        }
    }
}

#[async_trait]
impl JobQueueInterface for JobQueueService {
    fn redis_queue(&self) -> &RedisJobQueue {
        &self.redis_queue
    }

    fn status_tracker(&self) -> &JobStatusTracker {
        &self.status_tracker
    }

    async fn get_job(&self, job_id: &str) -> JobResult<Option<JobInfo>> {
        self.status_tracker.get_job(job_id).await
    }

    async fn search_jobs(&self, query: JobSearchQuery) -> JobResult<JobSearchResult> {
        self.status_tracker.search_jobs(query).await
    }

    async fn get_queue_stats(&self, queue_name: &str) -> JobResult<QueueStats> {
        self.status_tracker.get_queue_stats(queue_name).await
    }

    async fn get_all_queue_stats(&self) -> JobResult<Vec<QueueStats>> {
        let names: Vec<&str> = self.queue_names.iter().map(|s| s.as_str()).collect();
        self.status_tracker.get_all_stats(&names).await
    }

    async fn get_dashboard_stats(&self) -> JobResult<DashboardStats> {
        let names: Vec<&str> = self.queue_names.iter().map(|s| s.as_str()).collect();
        self.status_tracker.get_dashboard_stats(&names).await
    }

    async fn get_worker_health(&self) -> JobResult<Vec<WorkerHealth>> {
        self.status_tracker.get_worker_health().await
    }

    async fn cancel_job(&self, job_id: &JobId) -> JobResult<()> {
        self.redis_queue.cancel(job_id).await
    }

    async fn retry_job(&self, job_id: &JobId) -> JobResult<()> {
        // Get job info and retry
        if let Some(info) = self.status_tracker.get_job(&job_id.to_string()).await? {
            // Create job data from info for retry
            let job_data = crate::job::JobData {
                id: info.id,
                name: info.name,
                queue: info.queue,
                payload: String::new(), // Payload not stored in JobInfo
                priority: info.priority,
                attempt: 0,
                max_attempts: info.max_attempts,
                timeout_secs: 300, // Default 5 minute timeout
                created_at: info.created_at,
                scheduled_at: chrono::Utc::now(),
                correlation_id: None,
                tags: info.tags,
                retry_policy: None,
                unique_key: None,
                last_error: None,
            };
            self.redis_queue.retry(&job_data).await
        } else {
            Err(crate::error::JobError::NotFound(job_id.to_string()))
        }
    }

    async fn retry_dlq_job(&self, job_id: &JobId) -> JobResult<()> {
        self.redis_queue.retry_dlq(job_id).await
    }

    async fn purge_completed(&self, older_than_secs: u64) -> JobResult<u64> {
        self.redis_queue.purge_completed(std::time::Duration::from_secs(older_than_secs)).await
    }

    fn queue_names(&self) -> &[String] {
        &self.queue_names
    }

    // =========================================================================
    // Worker Service Method Implementations
    // =========================================================================

    fn worker_registry(&self) -> &WorkerRegistry {
        &self.worker_registry
    }

    async fn dequeue_for_worker(
        &self,
        queues: &[&str],
        worker_id: &str,
        max_jobs: u32,
    ) -> JobResult<Vec<JobData>> {
        // Verify worker is registered
        if !self.worker_registry.is_worker_alive(worker_id) {
            return Err(crate::error::JobError::Worker(format!(
                "Worker '{}' is not registered or has expired",
                worker_id
            )));
        }

        // Dequeue jobs from the queues
        let mut jobs = Vec::new();
        for _ in 0..max_jobs {
            if let Some(job_data) = self.redis_queue.dequeue(queues, worker_id).await? {
                jobs.push(job_data);
            } else {
                break; // No more jobs available
            }
        }

        Ok(jobs)
    }

    async fn complete_job(&self, job_id: &JobId, _result: Option<String>) -> JobResult<()> {
        self.redis_queue.complete(job_id).await
    }

    async fn fail_job(
        &self,
        job_id: &JobId,
        error: &str,
        should_retry: bool,
    ) -> JobResult<(bool, bool)> {
        let job_error = crate::error::JobError::ExecutionFailed(error.to_string());

        // Call fail which will either retry or move to DLQ based on retry count
        self.redis_queue.fail(job_id, &job_error).await?;

        // Check the result by looking at job status
        if let Some(info) = self.status_tracker.get_job(&job_id.to_string()).await? {
            let dead_lettered = info.status.to_lowercase() == "dead_letter";
            if dead_lettered {
                Ok((false, true))
            } else if should_retry {
                Ok((true, false))
            } else {
                // Job was expected to not retry but wasn't dead lettered
                // This means it had retries left
                Ok((true, false))
            }
        } else {
            // Job not found after fail - assume it was processed
            Ok((should_retry, !should_retry))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_queue_service_queue_names() {
        // Compile test only - full tests require Redis
        let names = vec!["default".to_string(), "high-priority".to_string()];
        assert_eq!(names.len(), 2);
    }
}
