//! Redis job queue implementation.

use super::RedisKeys;
use crate::config::JobsConfig;
use crate::error::{JobError, JobResult};
use crate::job::{Job, JobData, JobId, JobInfo};
use crate::queue::{JobQueue, Priority, QueuedJob};
use crate::retry::RetryPolicy;
use async_trait::async_trait;
use chrono::{Duration as ChronoDuration, Utc};
use deadpool_redis::Pool;
use redis::AsyncCommands;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Redis-backed job queue.
pub struct RedisJobQueue {
    pool: Pool,
    keys: RedisKeys,
    config: JobsConfig,
}

impl RedisJobQueue {
    /// Create a new Redis job queue.
    pub fn new(pool: Pool, config: JobsConfig) -> Self {
        let keys = RedisKeys::new(&config.redis.key_prefix);
        Self { pool, keys, config }
    }

    /// Get a connection from the pool.
    async fn conn(&self) -> JobResult<deadpool_redis::Connection> {
        Ok(self.pool.get().await?)
    }

    /// Calculate priority score for sorted set.
    /// Higher priority = lower score (processed first).
    /// Score = -priority * 1e12 + timestamp_ms
    fn priority_score(priority: i8, scheduled_at: i64) -> f64 {
        let priority_component = -(priority as f64) * 1_000_000_000_000.0;
        let time_component = scheduled_at as f64;
        priority_component + time_component
    }

    /// Move delayed jobs to their queues.
    pub async fn process_delayed(&self) -> JobResult<u64> {
        let mut conn = self.conn().await?;
        let now = Utc::now().timestamp_millis();

        // Get jobs ready to be processed
        let jobs: Vec<String> = conn
            .zrangebyscore(&self.keys.delayed(), 0i64, now)
            .await?;

        let mut moved = 0u64;

        for job_json in jobs {
            if let Ok(job_data) = JobData::from_json(&job_json) {
                // Move to appropriate queue
                let queue_key = self.keys.priority_queue(&job_data.queue);
                let score = Self::priority_score(job_data.priority, job_data.scheduled_at.timestamp_millis());

                let _: () = redis::pipe()
                    .zrem(&self.keys.delayed(), &job_json)
                    .zadd(&queue_key, &job_json, score)
                    .query_async(&mut *conn)
                    .await?;

                moved += 1;
                debug!(job_id = %job_data.id, queue = %job_data.queue, "Moved delayed job to queue");
            }
        }

        if moved > 0 {
            debug!(count = moved, "Processed delayed jobs");
        }

        Ok(moved)
    }

    /// Check for stale active jobs and requeue them.
    pub async fn recover_stale_jobs(&self, stale_threshold: Duration) -> JobResult<u64> {
        let mut conn = self.conn().await?;
        let _threshold = Utc::now() - ChronoDuration::from_std(stale_threshold).unwrap_or_default();

        // Get all active jobs
        let active_jobs: std::collections::HashMap<String, String> =
            conn.hgetall(&self.keys.active()).await?;

        let mut recovered = 0u64;

        for (job_id, worker_id) in active_jobs {
            // Check if worker is still alive
            let worker_key = self.keys.worker(&worker_id);
            let worker_alive: bool = conn.exists(&worker_key).await?;

            if !worker_alive {
                // Worker is dead, requeue the job
                let job_key = self.keys.job(&job_id);
                let job_json: Option<String> = conn.get(&job_key).await?;

                if let Some(json) = job_json {
                    if let Ok(mut job_data) = JobData::from_json(&json) {
                        job_data.increment_attempt();
                        job_data.set_error(&JobError::Worker("Worker died".to_string()));

                        // Requeue
                        self.retry(&job_data).await?;

                        // Remove from active
                        let _: () = conn.hdel(&self.keys.active(), &job_id).await?;

                        recovered += 1;
                        warn!(job_id = %job_id, worker_id = %worker_id, "Recovered stale job from dead worker");
                    }
                }
            }
        }

        if recovered > 0 {
            info!(count = recovered, "Recovered stale jobs");
        }

        Ok(recovered)
    }
}

#[async_trait]
impl JobQueue for RedisJobQueue {
    async fn enqueue_with<J: Job>(&self, queued: QueuedJob<J>) -> JobResult<JobId> {
        let job_data = queued.build()?;
        let job_id = job_data.id.clone();
        let job_json = job_data.to_json()?;

        let mut conn = self.conn().await?;

        // Check uniqueness if enabled
        if let Some(unique_key) = &job_data.unique_key {
            let unique_redis_key = self.keys.unique(unique_key);
            let exists: bool = conn.exists(&unique_redis_key).await?;

            if exists {
                return Err(JobError::QueueFull(format!(
                    "Duplicate job with unique key: {}",
                    unique_key
                )));
            }

            // Set unique key with TTL
            let _: () = conn
                .set_ex(&unique_redis_key, job_id.as_str(), 3600)
                .await?;
        }

        // Store job data
        let job_key = self.keys.job(job_id.as_str());
        let _: () = conn.set(&job_key, &job_json).await?;

        let now = Utc::now().timestamp_millis();

        if job_data.scheduled_at.timestamp_millis() > now {
            // Delayed job - add to delayed queue
            let score = job_data.scheduled_at.timestamp_millis() as f64;
            let _: () = conn
                .zadd(&self.keys.delayed(), &job_json, score)
                .await?;

            debug!(
                job_id = %job_id,
                queue = %job_data.queue,
                scheduled_at = %job_data.scheduled_at,
                "Enqueued delayed job"
            );
        } else {
            // Immediate job - add to priority queue
            let queue_key = self.keys.priority_queue(&job_data.queue);
            let score = Self::priority_score(job_data.priority, now);
            let _: () = conn.zadd(&queue_key, &job_json, score).await?;

            debug!(
                job_id = %job_id,
                queue = %job_data.queue,
                priority = ?Priority::from(job_data.priority),
                "Enqueued job"
            );
        }

        Ok(job_id)
    }

    async fn dequeue(&self, queues: &[&str], worker_id: &str) -> JobResult<Option<JobData>> {
        let mut conn = self.conn().await?;

        // First, process any delayed jobs
        let _ = self.process_delayed().await;

        // Try to dequeue from each queue in order
        for queue_name in queues {
            let queue_key = self.keys.priority_queue(queue_name);

            // Use ZPOPMIN to atomically get and remove the job with lowest score (highest priority)
            let result: Vec<(String, f64)> = conn.zpopmin(&queue_key, 1).await?;

            if let Some((job_json, _score)) = result.into_iter().next() {
                match JobData::from_json(&job_json) {
                    Ok(mut job_data) => {
                        job_data.increment_attempt();

                        // Store updated job data
                        let job_key = self.keys.job(job_data.id.as_str());
                        let _: () = conn.set(&job_key, job_data.to_json()?).await?;

                        // Mark as active
                        let _: () = conn
                            .hset(&self.keys.active(), job_data.id.as_str(), worker_id)
                            .await?;

                        debug!(
                            job_id = %job_data.id,
                            queue = %job_data.queue,
                            attempt = job_data.attempt,
                            worker_id = %worker_id,
                            "Dequeued job"
                        );

                        return Ok(Some(job_data));
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to deserialize job data");
                        continue;
                    }
                }
            }
        }

        Ok(None)
    }

    async fn complete(&self, job_id: &JobId) -> JobResult<()> {
        let mut conn = self.conn().await?;

        // Remove from active
        let _: () = conn.hdel(&self.keys.active(), job_id.as_str()).await?;

        // Get job data for stats
        let job_key = self.keys.job(job_id.as_str());
        let job_json: Option<String> = conn.get(&job_key).await?;

        if let Some(json) = job_json {
            if let Ok(job_data) = JobData::from_json(&json) {
                // Add to completed set
                let now = Utc::now().timestamp_millis();
                let _: () = conn
                    .zadd(&self.keys.completed(), &json, now as f64)
                    .await?;

                // Update stats
                let stats_key = self.keys.stats(&job_data.queue);
                let _: () = conn.hincr(&stats_key, "completed", 1i64).await?;

                // Clear unique key if set
                if let Some(unique_key) = &job_data.unique_key {
                    let _: () = conn.del(&self.keys.unique(unique_key)).await?;
                }
            }
        }

        // Delete job data
        let _: () = conn.del(&job_key).await?;

        debug!(job_id = %job_id, "Completed job");

        Ok(())
    }

    async fn fail(&self, job_id: &JobId, error: &JobError) -> JobResult<()> {
        let mut conn = self.conn().await?;

        // Get job data
        let job_key = self.keys.job(job_id.as_str());
        let job_json: Option<String> = conn.get(&job_key).await?;

        if let Some(json) = job_json {
            let mut job_data = JobData::from_json(&json)?;
            job_data.set_error(error);

            // Check if should retry
            let should_retry = if let Some(policy_json) = &job_data.retry_policy {
                if let Ok(policy) = serde_json::from_str::<RetryPolicy>(policy_json) {
                    policy.should_retry(job_data.attempt) && error.is_retryable()
                } else {
                    job_data.attempt < job_data.max_attempts && error.is_retryable()
                }
            } else {
                job_data.attempt < job_data.max_attempts && error.is_retryable()
            };

            if should_retry {
                self.retry(&job_data).await?;
            } else {
                self.dead_letter(&job_data, error).await?;
            }

            // Remove from active
            let _: () = conn.hdel(&self.keys.active(), job_id.as_str()).await?;

            // Update stats
            let stats_key = self.keys.stats(&job_data.queue);
            let _: () = conn.hincr(&stats_key, "failed", 1i64).await?;
        }

        Ok(())
    }

    async fn retry(&self, job_data: &JobData) -> JobResult<()> {
        let mut conn = self.conn().await?;

        // Calculate retry delay
        let delay = if let Some(policy_json) = &job_data.retry_policy {
            if let Ok(policy) = serde_json::from_str::<RetryPolicy>(policy_json) {
                policy.delay_for_attempt(job_data.attempt)
            } else {
                Duration::from_secs(1)
            }
        } else {
            Duration::from_secs(1)
        };

        let scheduled_at = Utc::now() + ChronoDuration::from_std(delay).unwrap_or_default();
        let mut updated_data = job_data.clone();
        updated_data.scheduled_at = scheduled_at;

        let job_json = updated_data.to_json()?;

        // Update job data
        let job_key = self.keys.job(job_data.id.as_str());
        let _: () = conn.set(&job_key, &job_json).await?;

        // Add to delayed queue
        let score = scheduled_at.timestamp_millis() as f64;
        let _: () = conn.zadd(&self.keys.delayed(), &job_json, score).await?;

        debug!(
            job_id = %job_data.id,
            attempt = job_data.attempt,
            retry_at = %scheduled_at,
            "Scheduled job retry"
        );

        Ok(())
    }

    async fn dead_letter(&self, job_data: &JobData, error: &JobError) -> JobResult<()> {
        if !self.config.queue.dlq.enabled {
            // Just delete the job
            let mut conn = self.conn().await?;
            let job_key = self.keys.job(job_data.id.as_str());
            let _: () = conn.del(&job_key).await?;
            return Ok(());
        }

        let mut conn = self.conn().await?;

        let mut dlq_data = job_data.clone();
        dlq_data.set_error(error);

        let job_json = dlq_data.to_json()?;
        let now = Utc::now().timestamp_millis();

        // Add to DLQ
        let _: () = conn.zadd(&self.keys.dlq(), &job_json, now as f64).await?;

        // Update job data
        let job_key = self.keys.job(job_data.id.as_str());
        let _: () = conn.set(&job_key, &job_json).await?;

        // Update stats
        let stats_key = self.keys.stats(&job_data.queue);
        let _: () = conn.hincr(&stats_key, "dead_letter", 1i64).await?;

        warn!(
            job_id = %job_data.id,
            error = %error,
            attempts = job_data.attempt,
            "Moved job to dead letter queue"
        );

        Ok(())
    }

    async fn get_job(&self, job_id: &JobId) -> JobResult<Option<JobInfo>> {
        let mut conn = self.conn().await?;

        let job_key = self.keys.job(job_id.as_str());
        let job_json: Option<String> = conn.get(&job_key).await?;

        if let Some(json) = job_json {
            let job_data = JobData::from_json(&json)?;
            let mut info = JobInfo::from(job_data);

            // Check if active
            let worker_id: Option<String> = conn
                .hget(&self.keys.active(), job_id.as_str())
                .await?;

            if worker_id.is_some() {
                info.status = "active".to_string();
                info.worker_id = worker_id;
            }

            Ok(Some(info))
        } else {
            Ok(None)
        }
    }

    async fn queue_length(&self, queue: &str) -> JobResult<u64> {
        let mut conn = self.conn().await?;
        let queue_key = self.keys.priority_queue(queue);
        let count: u64 = conn.zcard(&queue_key).await?;
        Ok(count)
    }

    async fn list_jobs(&self, queue: &str, limit: usize, offset: usize) -> JobResult<Vec<JobInfo>> {
        let mut conn = self.conn().await?;
        let queue_key = self.keys.priority_queue(queue);

        let jobs: Vec<String> = conn
            .zrange(&queue_key, offset as isize, (offset + limit - 1) as isize)
            .await?;

        let mut infos = Vec::with_capacity(jobs.len());
        for job_json in jobs {
            if let Ok(job_data) = JobData::from_json(&job_json) {
                infos.push(JobInfo::from(job_data));
            }
        }

        Ok(infos)
    }

    async fn list_dlq(&self, limit: usize, offset: usize) -> JobResult<Vec<JobInfo>> {
        let mut conn = self.conn().await?;

        let jobs: Vec<String> = conn
            .zrevrange(&self.keys.dlq(), offset as isize, (offset + limit - 1) as isize)
            .await?;

        let mut infos = Vec::with_capacity(jobs.len());
        for job_json in jobs {
            if let Ok(job_data) = JobData::from_json(&job_json) {
                let mut info = JobInfo::from(job_data);
                info.status = "dead_letter".to_string();
                infos.push(info);
            }
        }

        Ok(infos)
    }

    async fn retry_dlq(&self, job_id: &JobId) -> JobResult<()> {
        let mut conn = self.conn().await?;

        // Get job data
        let job_key = self.keys.job(job_id.as_str());
        let job_json: Option<String> = conn.get(&job_key).await?;

        if let Some(json) = job_json {
            let mut job_data = JobData::from_json(&json)?;

            // Reset attempt count
            job_data.attempt = 0;
            job_data.last_error = None;
            job_data.scheduled_at = Utc::now();

            let updated_json = job_data.to_json()?;

            // Remove from DLQ
            let _: () = conn.zrem(&self.keys.dlq(), &json).await?;

            // Add to queue
            let queue_key = self.keys.priority_queue(&job_data.queue);
            let score = Self::priority_score(job_data.priority, job_data.scheduled_at.timestamp_millis());
            let _: () = conn.zadd(&queue_key, &updated_json, score).await?;

            // Update job data
            let _: () = conn.set(&job_key, &updated_json).await?;

            info!(job_id = %job_id, "Retried job from DLQ");
        } else {
            return Err(JobError::NotFound(job_id.to_string()));
        }

        Ok(())
    }

    async fn delete(&self, job_id: &JobId) -> JobResult<()> {
        let mut conn = self.conn().await?;

        // Get job data first
        let job_key = self.keys.job(job_id.as_str());
        let job_json: Option<String> = conn.get(&job_key).await?;

        if let Some(json) = job_json {
            if let Ok(job_data) = JobData::from_json(&json) {
                // Remove from all possible locations
                let queue_key = self.keys.priority_queue(&job_data.queue);

                let _: () = redis::pipe()
                    .del(&job_key)
                    .zrem(&queue_key, &json)
                    .zrem(&self.keys.delayed(), &json)
                    .zrem(&self.keys.dlq(), &json)
                    .zrem(&self.keys.completed(), &json)
                    .hdel(&self.keys.active(), job_id.as_str())
                    .query_async(&mut *conn)
                    .await?;

                // Clear unique key if set
                if let Some(unique_key) = &job_data.unique_key {
                    let _: () = conn.del(&self.keys.unique(unique_key)).await?;
                }
            }
        }

        debug!(job_id = %job_id, "Deleted job");

        Ok(())
    }

    async fn purge_completed(&self, older_than: Duration) -> JobResult<u64> {
        let mut conn = self.conn().await?;

        let threshold = Utc::now() - ChronoDuration::from_std(older_than).unwrap_or_default();
        let threshold_ms = threshold.timestamp_millis();

        // Remove old completed jobs using raw command
        let removed: u64 = redis::cmd("ZREMRANGEBYSCORE")
            .arg(&self.keys.completed())
            .arg(0i64)
            .arg(threshold_ms)
            .query_async(&mut *conn)
            .await?;

        if removed > 0 {
            info!(count = removed, "Purged completed jobs");
        }

        Ok(removed)
    }

    async fn cancel(&self, job_id: &JobId) -> JobResult<()> {
        let mut conn = self.conn().await?;

        // Get job data
        let job_key = self.keys.job(job_id.as_str());
        let job_json: Option<String> = conn.get(&job_key).await?;

        if let Some(json) = job_json {
            let job_data = JobData::from_json(&json)?;

            // Check if active - can't cancel active jobs
            let is_active: bool = conn
                .hexists(&self.keys.active(), job_id.as_str())
                .await?;

            if is_active {
                return Err(JobError::InvalidState {
                    expected: "pending".to_string(),
                    actual: "active".to_string(),
                });
            }

            // Remove from queues
            let queue_key = self.keys.priority_queue(&job_data.queue);

            let _: () = redis::pipe()
                .del(&job_key)
                .zrem(&queue_key, &json)
                .zrem(&self.keys.delayed(), &json)
                .query_async(&mut *conn)
                .await?;

            // Clear unique key if set
            if let Some(unique_key) = &job_data.unique_key {
                let _: () = conn.del(&self.keys.unique(unique_key)).await?;
            }

            info!(job_id = %job_id, "Cancelled job");
        } else {
            return Err(JobError::NotFound(job_id.to_string()));
        }

        Ok(())
    }

    async fn health_check(&self) -> JobResult<()> {
        let mut conn = self.conn().await?;
        let _: String = redis::cmd("PING").query_async(&mut *conn).await?;
        Ok(())
    }
}
