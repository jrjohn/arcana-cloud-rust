//! Job status tracking and monitoring.

use crate::error::JobResult;
use crate::job::{JobInfo, JobStatus};
use crate::queue::QueueStats;
use crate::redis::RedisKeys;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use deadpool_redis::Pool;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

/// Job status tracker for monitoring and querying jobs.
pub struct JobStatusTracker {
    /// Redis connection pool.
    pool: Pool,

    /// Redis keys.
    keys: RedisKeys,
}

impl JobStatusTracker {
    /// Create a new job status tracker.
    pub fn new(pool: Pool, key_prefix: impl Into<String>) -> Self {
        Self {
            pool,
            keys: RedisKeys::new(key_prefix),
        }
    }

    /// Get job info by ID.
    pub async fn get_job(&self, job_id: &str) -> JobResult<Option<JobInfo>> {
        let mut conn = self.pool.get().await?;
        let job_key = self.keys.job(job_id);

        let data: Option<String> = conn.get(&job_key).await?;

        match data {
            Some(json) => {
                let info: JobInfo = serde_json::from_str(&json)?;
                Ok(Some(info))
            }
            None => Ok(None),
        }
    }

    /// Get multiple jobs by ID.
    pub async fn get_jobs(&self, job_ids: &[&str]) -> JobResult<Vec<Option<JobInfo>>> {
        if job_ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut conn = self.pool.get().await?;
        let keys: Vec<String> = job_ids.iter().map(|id| self.keys.job(id)).collect();

        let results: Vec<Option<String>> = conn.mget(&keys).await?;

        let jobs = results
            .into_iter()
            .map(|opt| {
                opt.and_then(|json| serde_json::from_str(&json).ok())
            })
            .collect();

        Ok(jobs)
    }

    /// Search jobs by various criteria.
    pub async fn search_jobs(&self, query: JobSearchQuery) -> JobResult<JobSearchResult> { // NOSONAR
        let mut conn = self.pool.get().await?;

        // Determine which set to search based on status
        let set_key = match query.status {
            Some(JobStatus::Pending) | Some(JobStatus::Scheduled) => {
                self.keys.priority_queue(query.queue.as_deref().unwrap_or("default"))
            }
            Some(JobStatus::Running) => self.keys.active(),
            Some(JobStatus::Completed) => self.keys.completed(),
            Some(JobStatus::Failed) | Some(JobStatus::DeadLetter) | Some(JobStatus::Cancelled) => self.keys.dlq(),
            None => {
                // Search across all queues - for simplicity, search pending queue
                self.keys.priority_queue(query.queue.as_deref().unwrap_or("default"))
            }
        };

        // Get job IDs from the set
        let job_ids: Vec<String> = conn
            .zrange(&set_key, query.offset as isize, (query.offset + query.limit - 1) as isize)
            .await?;

        // Get job data for each ID
        let mut jobs = Vec::new();
        for job_id in &job_ids {
            if let Some(info) = self.get_job(job_id).await? {
                // Apply filters
                if let Some(ref name_filter) = query.name {
                    if !info.name.contains(name_filter) {
                        continue;
                    }
                }

                if let Some(ref tag_filter) = query.tag {
                    if !info.tags.contains(tag_filter) {
                        continue;
                    }
                }

                jobs.push(info);
            }
        }

        // Get total count
        let total: u64 = conn.zcard(&set_key).await?;

        Ok(JobSearchResult {
            jobs,
            total,
            offset: query.offset,
            limit: query.limit,
        })
    }

    /// Get queue statistics.
    pub async fn get_queue_stats(&self, queue_name: &str) -> JobResult<QueueStats> {
        let mut conn = self.pool.get().await?;

        // Count jobs in various states
        let pending_key = self.keys.priority_queue(queue_name);
        let pending: u64 = conn.zcard(&pending_key).await?;

        let active_key = self.keys.active();
        let active: u64 = conn.hlen(&active_key).await?;

        let completed_key = self.keys.completed();
        let completed: u64 = conn.zcard(&completed_key).await?;

        let dlq_key = self.keys.dlq();
        let dead_letter: u64 = conn.zcard(&dlq_key).await?;

        let delayed_key = self.keys.delayed();
        let delayed: u64 = conn.zcard(&delayed_key).await?;

        // Get failed count from stats
        let stats_key = self.keys.stats(queue_name);
        let failed: u64 = conn
            .hget(&stats_key, "failed")
            .await
            .unwrap_or(0);

        Ok(QueueStats {
            queue: queue_name.to_string(),
            pending,
            active,
            completed,
            failed,
            dead_letter,
            delayed,
        })
    }

    /// Get statistics for all queues.
    pub async fn get_all_stats(&self, queue_names: &[&str]) -> JobResult<Vec<QueueStats>> {
        let mut stats = Vec::new();
        for queue_name in queue_names {
            stats.push(self.get_queue_stats(queue_name).await?);
        }
        Ok(stats)
    }

    /// Get aggregate dashboard statistics.
    pub async fn get_dashboard_stats(&self, queue_names: &[&str]) -> JobResult<DashboardStats> {
        let all_stats = self.get_all_stats(queue_names).await?;

        let mut dashboard = DashboardStats::default();

        for stats in all_stats {
            dashboard.total_pending += stats.pending;
            dashboard.total_active += stats.active;
            dashboard.total_completed += stats.completed;
            dashboard.total_failed += stats.failed;
            dashboard.total_dead_letter += stats.dead_letter;
            dashboard.total_delayed += stats.delayed;
            dashboard.queues.push(stats);
        }

        dashboard.total_jobs = dashboard.total_pending
            + dashboard.total_active
            + dashboard.total_completed
            + dashboard.total_failed
            + dashboard.total_dead_letter
            + dashboard.total_delayed;

        Ok(dashboard)
    }

    /// Get job history for a correlation ID.
    pub async fn get_job_history(&self, correlation_id: &str) -> JobResult<Vec<JobInfo>> {
        let mut conn = self.pool.get().await?;

        // Get job IDs for this correlation ID
        let history_key = format!("{}:correlation:{}", self.keys.completed(), correlation_id);
        let job_ids: Vec<String> = conn.smembers(&history_key).await?;

        let mut jobs = Vec::new();
        for job_id in job_ids {
            if let Some(info) = self.get_job(&job_id).await? {
                jobs.push(info);
            }
        }

        // Sort by created_at
        jobs.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        Ok(jobs)
    }

    /// Get recent job activity.
    pub async fn get_recent_activity(&self, limit: usize) -> JobResult<Vec<JobActivity>> {
        let mut conn = self.pool.get().await?;

        // Get recently completed jobs
        let completed_key = self.keys.completed();
        let completed_ids: Vec<String> = conn
            .zrevrange(&completed_key, 0, (limit - 1) as isize)
            .await?;

        // Get recently failed jobs
        let dlq_key = self.keys.dlq();
        let failed_ids: Vec<String> = conn
            .zrevrange(&dlq_key, 0, (limit - 1) as isize)
            .await?;

        let mut activities = Vec::new();

        // Add completed activities
        for job_id in completed_ids {
            if let Some(info) = self.get_job(&job_id).await? {
                activities.push(JobActivity {
                    job_id: info.id.to_string(),
                    job_name: info.name.clone(),
                    activity_type: ActivityType::Completed,
                    timestamp: info.completed_at.unwrap_or(info.created_at),
                    queue: info.queue.clone(),
                    duration_ms: info.completed_at.map(|c| {
                        (c - info.started_at.unwrap_or(info.created_at)).num_milliseconds() as u64
                    }),
                    error: None,
                });
            }
        }

        // Add failed activities
        for job_id in failed_ids {
            if let Some(info) = self.get_job(&job_id).await? {
                activities.push(JobActivity {
                    job_id: info.id.to_string(),
                    job_name: info.name.clone(),
                    activity_type: ActivityType::Failed,
                    timestamp: info.completed_at.unwrap_or(info.created_at),
                    queue: info.queue.clone(),
                    duration_ms: None,
                    error: info.last_error.clone(),
                });
            }
        }

        // Sort by timestamp descending
        activities.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        activities.truncate(limit);

        Ok(activities)
    }

    /// Get throughput metrics.
    pub async fn get_throughput(&self, queue_name: &str, period: ThroughputPeriod) -> JobResult<ThroughputMetrics> {
        let mut conn = self.pool.get().await?;

        let now = Utc::now();
        let (start_time, _bucket_count, _bucket_duration) = match period {
            ThroughputPeriod::LastHour => {
                (now - ChronoDuration::hours(1), 12, ChronoDuration::minutes(5))
            }
            ThroughputPeriod::Last24Hours => {
                (now - ChronoDuration::hours(24), 24, ChronoDuration::hours(1))
            }
            ThroughputPeriod::Last7Days => {
                (now - ChronoDuration::days(7), 7, ChronoDuration::days(1))
            }
        };

        let completed_key = self.keys.completed();
        let start_score = start_time.timestamp_millis() as f64;
        let end_score = now.timestamp_millis() as f64;

        // Count completed jobs in the time range
        let completed_count: u64 = conn
            .zcount(&completed_key, start_score, end_score)
            .await?;

        let dlq_key = self.keys.dlq();
        let failed_count: u64 = conn
            .zcount(&dlq_key, start_score, end_score)
            .await?;

        let total_processed = completed_count + failed_count;
        let duration_secs = (now - start_time).num_seconds() as f64;
        let avg_per_second = if duration_secs > 0.0 {
            total_processed as f64 / duration_secs
        } else {
            0.0
        };

        let success_rate = if total_processed > 0 {
            (completed_count as f64 / total_processed as f64) * 100.0
        } else {
            100.0
        };

        Ok(ThroughputMetrics {
            queue: queue_name.to_string(),
            period,
            total_processed,
            completed: completed_count,
            failed: failed_count,
            avg_per_second,
            success_rate,
            buckets: Vec::new(), // TODO: Implement bucket breakdown
        })
    }

    /// Get worker health information.
    pub async fn get_worker_health(&self) -> JobResult<Vec<WorkerHealth>> {
        let mut conn = self.pool.get().await?;

        // Scan for worker keys
        let pattern = format!("{}:worker:*", self.keys.worker("").trim_end_matches(':'));
        let mut cursor = 0;
        let mut workers = Vec::new();

        loop {
            let (new_cursor, keys): (u64, Vec<String>) =
                redis::cmd("SCAN")
                    .arg(cursor)
                    .arg("MATCH")
                    .arg(&pattern)
                    .arg("COUNT")
                    .arg(100)
                    .query_async(&mut *conn)
                    .await?;

            for key in keys {
                let worker_id = key.rsplit(':').next().unwrap_or_default();
                let ttl: i64 = conn.ttl(&key).await?;

                let last_heartbeat: Option<String> = conn.get(&key).await?;
                let last_heartbeat_time = last_heartbeat
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc));

                workers.push(WorkerHealth {
                    worker_id: worker_id.to_string(),
                    status: if ttl > 0 { WorkerStatus::Active } else { WorkerStatus::Stale },
                    last_heartbeat: last_heartbeat_time,
                    ttl_remaining: if ttl > 0 { Some(ttl as u64) } else { None },
                });
            }

            cursor = new_cursor;
            if cursor == 0 {
                break;
            }
        }

        Ok(workers)
    }
}

/// Job search query.
#[derive(Debug, Clone, Default)]
pub struct JobSearchQuery {
    /// Filter by status.
    pub status: Option<JobStatus>,

    /// Filter by queue name.
    pub queue: Option<String>,

    /// Filter by job name (partial match).
    pub name: Option<String>,

    /// Filter by tag.
    pub tag: Option<String>,

    /// Pagination offset.
    pub offset: usize,

    /// Pagination limit.
    pub limit: usize,
}

impl JobSearchQuery {
    /// Create a new search query with defaults.
    pub fn new() -> Self {
        Self {
            offset: 0,
            limit: 50,
            ..Default::default()
        }
    }

    /// Filter by status.
    pub fn status(mut self, status: JobStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Filter by queue.
    pub fn queue(mut self, queue: impl Into<String>) -> Self {
        self.queue = Some(queue.into());
        self
    }

    /// Filter by name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Filter by tag.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    /// Set pagination offset.
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    /// Set pagination limit.
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
}

/// Job search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSearchResult {
    /// Found jobs.
    pub jobs: Vec<JobInfo>,

    /// Total count (for pagination).
    pub total: u64,

    /// Current offset.
    pub offset: usize,

    /// Limit used.
    pub limit: usize,
}

/// Dashboard statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DashboardStats {
    /// Total jobs across all states.
    pub total_jobs: u64,

    /// Total pending jobs.
    pub total_pending: u64,

    /// Total active (running) jobs.
    pub total_active: u64,

    /// Total completed jobs.
    pub total_completed: u64,

    /// Total failed jobs.
    pub total_failed: u64,

    /// Total dead letter jobs.
    pub total_dead_letter: u64,

    /// Total delayed jobs.
    pub total_delayed: u64,

    /// Per-queue statistics.
    pub queues: Vec<QueueStats>,
}

/// Job activity entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobActivity {
    /// Job ID.
    pub job_id: String,

    /// Job name.
    pub job_name: String,

    /// Activity type.
    pub activity_type: ActivityType,

    /// When the activity occurred.
    pub timestamp: DateTime<Utc>,

    /// Queue name.
    pub queue: String,

    /// Duration in milliseconds (for completed jobs).
    pub duration_ms: Option<u64>,

    /// Error message (for failed jobs).
    pub error: Option<String>,
}

/// Activity type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityType {
    /// Job was enqueued.
    Enqueued,
    /// Job started processing.
    Started,
    /// Job completed successfully.
    Completed,
    /// Job failed.
    Failed,
    /// Job was retried.
    Retried,
    /// Job was moved to DLQ.
    DeadLettered,
}

/// Throughput period.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThroughputPeriod {
    /// Last hour (5 minute buckets).
    LastHour,
    /// Last 24 hours (1 hour buckets).
    Last24Hours,
    /// Last 7 days (1 day buckets).
    Last7Days,
}

/// Throughput metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputMetrics {
    /// Queue name.
    pub queue: String,

    /// Time period.
    pub period: ThroughputPeriod,

    /// Total processed.
    pub total_processed: u64,

    /// Completed count.
    pub completed: u64,

    /// Failed count.
    pub failed: u64,

    /// Average jobs per second.
    pub avg_per_second: f64,

    /// Success rate percentage.
    pub success_rate: f64,

    /// Time series buckets.
    pub buckets: Vec<ThroughputBucket>,
}

/// Throughput bucket for time series.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputBucket {
    /// Bucket start time.
    pub start: DateTime<Utc>,

    /// Bucket end time.
    pub end: DateTime<Utc>,

    /// Completed in this bucket.
    pub completed: u64,

    /// Failed in this bucket.
    pub failed: u64,
}

/// Worker health information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerHealth {
    /// Worker ID.
    pub worker_id: String,

    /// Worker status.
    pub status: WorkerStatus,

    /// Last heartbeat time.
    pub last_heartbeat: Option<DateTime<Utc>>,

    /// TTL remaining in seconds.
    pub ttl_remaining: Option<u64>,
}

/// Worker status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerStatus {
    /// Worker is active and healthy.
    Active,
    /// Worker hasn't sent heartbeat recently.
    Stale,
    /// Worker is shutting down.
    ShuttingDown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_query_builder() {
        let query = JobSearchQuery::new()
            .status(JobStatus::Pending)
            .queue("high-priority")
            .limit(100);

        assert_eq!(query.status, Some(JobStatus::Pending));
        assert_eq!(query.queue.as_deref(), Some("high-priority"));
        assert_eq!(query.limit, 100);
    }

    #[test]
    fn test_dashboard_stats_default() {
        let stats = DashboardStats::default();
        assert_eq!(stats.total_jobs, 0);
        assert_eq!(stats.queues.len(), 0);
    }
}
