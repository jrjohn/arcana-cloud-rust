//! Prometheus metrics for job queue monitoring.
//!
//! Provides comprehensive metrics for monitoring job queue health and performance.

use metrics::{counter, describe_counter, describe_gauge, describe_histogram, gauge, histogram};
use std::time::Duration;

/// Metric names for the job queue system.
pub mod names {
    /// Total jobs enqueued.
    pub const JOBS_ENQUEUED_TOTAL: &str = "arcana_jobs_enqueued_total";
    /// Total jobs dequeued for processing.
    pub const JOBS_DEQUEUED_TOTAL: &str = "arcana_jobs_dequeued_total";
    /// Total jobs completed successfully.
    pub const JOBS_COMPLETED_TOTAL: &str = "arcana_jobs_completed_total";
    /// Total jobs failed.
    pub const JOBS_FAILED_TOTAL: &str = "arcana_jobs_failed_total";
    /// Total jobs retried.
    pub const JOBS_RETRIED_TOTAL: &str = "arcana_jobs_retried_total";
    /// Total jobs sent to dead letter queue.
    pub const JOBS_DEAD_LETTERED_TOTAL: &str = "arcana_jobs_dead_lettered_total";
    /// Total jobs cancelled.
    pub const JOBS_CANCELLED_TOTAL: &str = "arcana_jobs_cancelled_total";
    /// Total jobs timed out.
    pub const JOBS_TIMED_OUT_TOTAL: &str = "arcana_jobs_timed_out_total";

    /// Current pending jobs.
    pub const JOBS_PENDING: &str = "arcana_jobs_pending";
    /// Current active (running) jobs.
    pub const JOBS_ACTIVE: &str = "arcana_jobs_active";
    /// Current delayed jobs.
    pub const JOBS_DELAYED: &str = "arcana_jobs_delayed";
    /// Current dead letter queue size.
    pub const JOBS_DEAD_LETTER: &str = "arcana_jobs_dead_letter";

    /// Job execution duration in seconds.
    pub const JOB_DURATION_SECONDS: &str = "arcana_job_duration_seconds";
    /// Job wait time (time in queue) in seconds.
    pub const JOB_WAIT_TIME_SECONDS: &str = "arcana_job_wait_time_seconds";

    /// Active workers count.
    pub const WORKERS_ACTIVE: &str = "arcana_workers_active";
    /// Worker pool concurrency.
    pub const WORKERS_CONCURRENCY: &str = "arcana_workers_concurrency";

    /// Scheduler is leader.
    pub const SCHEDULER_IS_LEADER: &str = "arcana_scheduler_is_leader";
    /// Scheduled jobs triggered.
    pub const SCHEDULER_JOBS_TRIGGERED: &str = "arcana_scheduler_jobs_triggered_total";

    /// Redis connection pool size.
    pub const REDIS_POOL_SIZE: &str = "arcana_jobs_redis_pool_size";
    /// Redis connection pool available.
    pub const REDIS_POOL_AVAILABLE: &str = "arcana_jobs_redis_pool_available";
    /// Redis operation duration in seconds.
    pub const REDIS_OPERATION_DURATION: &str = "arcana_jobs_redis_operation_duration_seconds";
}

/// Register all metric descriptions.
pub fn register_metrics() {
    // Job counters
    describe_counter!(
        names::JOBS_ENQUEUED_TOTAL,
        "Total number of jobs enqueued"
    );
    describe_counter!(
        names::JOBS_DEQUEUED_TOTAL,
        "Total number of jobs dequeued for processing"
    );
    describe_counter!(
        names::JOBS_COMPLETED_TOTAL,
        "Total number of jobs completed successfully"
    );
    describe_counter!(
        names::JOBS_FAILED_TOTAL,
        "Total number of jobs that failed"
    );
    describe_counter!(
        names::JOBS_RETRIED_TOTAL,
        "Total number of job retries"
    );
    describe_counter!(
        names::JOBS_DEAD_LETTERED_TOTAL,
        "Total number of jobs sent to dead letter queue"
    );
    describe_counter!(
        names::JOBS_CANCELLED_TOTAL,
        "Total number of jobs cancelled"
    );
    describe_counter!(
        names::JOBS_TIMED_OUT_TOTAL,
        "Total number of jobs that timed out"
    );

    // Job gauges
    describe_gauge!(
        names::JOBS_PENDING,
        "Current number of pending jobs"
    );
    describe_gauge!(
        names::JOBS_ACTIVE,
        "Current number of active (running) jobs"
    );
    describe_gauge!(
        names::JOBS_DELAYED,
        "Current number of delayed jobs"
    );
    describe_gauge!(
        names::JOBS_DEAD_LETTER,
        "Current size of dead letter queue"
    );

    // Duration histograms
    describe_histogram!(
        names::JOB_DURATION_SECONDS,
        "Job execution duration in seconds"
    );
    describe_histogram!(
        names::JOB_WAIT_TIME_SECONDS,
        "Job wait time (time in queue) in seconds"
    );

    // Worker metrics
    describe_gauge!(
        names::WORKERS_ACTIVE,
        "Number of active workers"
    );
    describe_gauge!(
        names::WORKERS_CONCURRENCY,
        "Worker pool concurrency setting"
    );

    // Scheduler metrics
    describe_gauge!(
        names::SCHEDULER_IS_LEADER,
        "Whether this instance is the scheduler leader (1) or not (0)"
    );
    describe_counter!(
        names::SCHEDULER_JOBS_TRIGGERED,
        "Total number of scheduled jobs triggered"
    );

    // Redis metrics
    describe_gauge!(
        names::REDIS_POOL_SIZE,
        "Redis connection pool size"
    );
    describe_gauge!(
        names::REDIS_POOL_AVAILABLE,
        "Available connections in Redis pool"
    );
    describe_histogram!(
        names::REDIS_OPERATION_DURATION,
        "Redis operation duration in seconds"
    );
}

/// Job metrics recorder.
#[derive(Clone)]
pub struct JobMetrics;

impl JobMetrics {
    /// Record a job enqueued.
    pub fn job_enqueued(queue: &str, job_name: &str, priority: &str) {
        counter!(
            names::JOBS_ENQUEUED_TOTAL,
            "queue" => queue.to_string(),
            "job_name" => job_name.to_string(),
            "priority" => priority.to_string()
        )
        .increment(1);
    }

    /// Record a job dequeued.
    pub fn job_dequeued(queue: &str, job_name: &str) {
        counter!(
            names::JOBS_DEQUEUED_TOTAL,
            "queue" => queue.to_string(),
            "job_name" => job_name.to_string()
        )
        .increment(1);
    }

    /// Record a job completed.
    pub fn job_completed(queue: &str, job_name: &str, duration: Duration) {
        counter!(
            names::JOBS_COMPLETED_TOTAL,
            "queue" => queue.to_string(),
            "job_name" => job_name.to_string()
        )
        .increment(1);

        histogram!(
            names::JOB_DURATION_SECONDS,
            "queue" => queue.to_string(),
            "job_name" => job_name.to_string(),
            "status" => "completed"
        )
        .record(duration.as_secs_f64());
    }

    /// Record a job failed.
    pub fn job_failed(queue: &str, job_name: &str, error_type: &str, duration: Duration) {
        counter!(
            names::JOBS_FAILED_TOTAL,
            "queue" => queue.to_string(),
            "job_name" => job_name.to_string(),
            "error_type" => error_type.to_string()
        )
        .increment(1);

        histogram!(
            names::JOB_DURATION_SECONDS,
            "queue" => queue.to_string(),
            "job_name" => job_name.to_string(),
            "status" => "failed"
        )
        .record(duration.as_secs_f64());
    }

    /// Record a job retried.
    pub fn job_retried(queue: &str, job_name: &str, attempt: u32) {
        counter!(
            names::JOBS_RETRIED_TOTAL,
            "queue" => queue.to_string(),
            "job_name" => job_name.to_string(),
            "attempt" => attempt.to_string()
        )
        .increment(1);
    }

    /// Record a job sent to DLQ.
    pub fn job_dead_lettered(queue: &str, job_name: &str, reason: &str) {
        counter!(
            names::JOBS_DEAD_LETTERED_TOTAL,
            "queue" => queue.to_string(),
            "job_name" => job_name.to_string(),
            "reason" => reason.to_string()
        )
        .increment(1);
    }

    /// Record a job cancelled.
    pub fn job_cancelled(queue: &str, job_name: &str) {
        counter!(
            names::JOBS_CANCELLED_TOTAL,
            "queue" => queue.to_string(),
            "job_name" => job_name.to_string()
        )
        .increment(1);
    }

    /// Record a job timeout.
    pub fn job_timed_out(queue: &str, job_name: &str) {
        counter!(
            names::JOBS_TIMED_OUT_TOTAL,
            "queue" => queue.to_string(),
            "job_name" => job_name.to_string()
        )
        .increment(1);
    }

    /// Record job wait time.
    pub fn job_wait_time(queue: &str, job_name: &str, wait_time: Duration) {
        histogram!(
            names::JOB_WAIT_TIME_SECONDS,
            "queue" => queue.to_string(),
            "job_name" => job_name.to_string()
        )
        .record(wait_time.as_secs_f64());
    }

    /// Update queue size gauges.
    pub fn update_queue_sizes(queue: &str, pending: u64, active: u64, delayed: u64, dlq: u64) {
        gauge!(
            names::JOBS_PENDING,
            "queue" => queue.to_string()
        )
        .set(pending as f64);

        gauge!(
            names::JOBS_ACTIVE,
            "queue" => queue.to_string()
        )
        .set(active as f64);

        gauge!(
            names::JOBS_DELAYED,
            "queue" => queue.to_string()
        )
        .set(delayed as f64);

        gauge!(
            names::JOBS_DEAD_LETTER,
            "queue" => queue.to_string()
        )
        .set(dlq as f64);
    }
}

/// Worker metrics recorder.
#[derive(Clone)]
pub struct WorkerMetrics;

impl WorkerMetrics {
    /// Update worker count.
    pub fn update_workers(pool_id: &str, active: u64, concurrency: usize) {
        gauge!(
            names::WORKERS_ACTIVE,
            "pool_id" => pool_id.to_string()
        )
        .set(active as f64);

        gauge!(
            names::WORKERS_CONCURRENCY,
            "pool_id" => pool_id.to_string()
        )
        .set(concurrency as f64);
    }
}

/// Scheduler metrics recorder.
#[derive(Clone)]
pub struct SchedulerMetrics;

impl SchedulerMetrics {
    /// Update leader status.
    pub fn update_leader_status(scheduler_id: &str, is_leader: bool) {
        gauge!(
            names::SCHEDULER_IS_LEADER,
            "scheduler_id" => scheduler_id.to_string()
        )
        .set(if is_leader { 1.0 } else { 0.0 });
    }

    /// Record a scheduled job triggered.
    pub fn job_triggered(scheduler_id: &str, job_name: &str) {
        counter!(
            names::SCHEDULER_JOBS_TRIGGERED,
            "scheduler_id" => scheduler_id.to_string(),
            "job_name" => job_name.to_string()
        )
        .increment(1);
    }
}

/// Redis metrics recorder.
#[derive(Clone)]
pub struct RedisMetrics;

impl RedisMetrics {
    /// Update pool status.
    pub fn update_pool_status(pool_size: usize, available: usize) {
        gauge!(names::REDIS_POOL_SIZE).set(pool_size as f64);
        gauge!(names::REDIS_POOL_AVAILABLE).set(available as f64);
    }

    /// Record operation duration.
    pub fn operation_duration(operation: &str, duration: Duration) {
        histogram!(
            names::REDIS_OPERATION_DURATION,
            "operation" => operation.to_string()
        )
        .record(duration.as_secs_f64());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_metrics() {
        // Just verify registration doesn't panic
        register_metrics();
    }

    #[test]
    fn test_job_metrics() {
        JobMetrics::job_enqueued("default", "test_job", "normal");
        JobMetrics::job_dequeued("default", "test_job");
        JobMetrics::job_completed("default", "test_job", Duration::from_secs(1));
        JobMetrics::job_failed("default", "test_job", "timeout", Duration::from_secs(5));
    }
}
