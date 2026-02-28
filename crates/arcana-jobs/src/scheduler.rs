//! Cron scheduler for recurring jobs with distributed leader election.

use crate::config::SchedulerConfig;
use crate::error::{JobError, JobResult};
use crate::job::{Job, JobData};
use crate::queue::JobQueue;
use crate::redis::RedisKeys;
use chrono::{DateTime, Utc};
use cron::Schedule;
use deadpool_redis::Pool;
use parking_lot::RwLock;
use redis::AsyncCommands;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::interval;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Scheduled job definition.
#[derive(Clone)]
pub struct ScheduledJob {
    /// Unique name for this scheduled job.
    pub name: String,

    /// Cron expression.
    pub cron: String,

    /// Parsed cron schedule.
    schedule: Schedule,

    /// Job factory function.
    factory: Arc<dyn Fn() -> JobResult<JobData> + Send + Sync>,

    /// Next scheduled execution time.
    next_run: Option<DateTime<Utc>>,

    /// Is job enabled.
    pub enabled: bool,

    /// Timezone offset (optional).
    pub timezone_offset_hours: i32,
}

impl std::fmt::Debug for ScheduledJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScheduledJob")
            .field("name", &self.name)
            .field("cron", &self.cron)
            .field("enabled", &self.enabled)
            .field("timezone_offset_hours", &self.timezone_offset_hours)
            .field("next_run", &self.next_run)
            .finish()
    }
}

impl ScheduledJob {
    /// Create a new scheduled job.
    pub fn new<J: Job>(
        name: impl Into<String>,
        cron_expr: &str,
        job_factory: impl Fn() -> J + Send + Sync + 'static,
    ) -> JobResult<Self> {
        let schedule = Schedule::from_str(cron_expr)
            .map_err(|e| JobError::Configuration(format!("Invalid cron expression: {}", e)))?;

        let factory: Arc<dyn Fn() -> JobResult<JobData> + Send + Sync> =
            Arc::new(move || {
                let job = job_factory();
                JobData::new(&job)
            });

        Ok(Self {
            name: name.into(),
            cron: cron_expr.to_string(),
            schedule,
            factory,
            next_run: None,
            enabled: true,
            timezone_offset_hours: 0,
        })
    }

    /// Set whether the job is enabled.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set timezone offset in hours.
    pub fn timezone_offset(mut self, hours: i32) -> Self {
        self.timezone_offset_hours = hours;
        self
    }

    /// Calculate the next run time from now.
    pub fn next_run_from(&self, from: DateTime<Utc>) -> Option<DateTime<Utc>> {
        self.schedule.after(&from).next()
    }

    /// Create job data for execution.
    pub fn create_job_data(&self) -> JobResult<JobData> {
        (self.factory)()
    }
}

/// Scheduler statistics.
#[derive(Debug, Clone, Default)]
pub struct SchedulerStats {
    /// Scheduler ID.
    pub id: String,

    /// Is this instance the leader.
    pub is_leader: bool,

    /// Number of scheduled jobs.
    pub scheduled_jobs: usize,

    /// Jobs executed count.
    pub jobs_executed: u64,

    /// Last leader election time.
    pub last_election: Option<DateTime<Utc>>,
}

/// Distributed cron scheduler with leader election.
pub struct Scheduler<Q: JobQueue> {
    /// Unique scheduler ID.
    id: String,

    /// Redis connection pool.
    pool: Pool,

    /// Job queue (will be used for enqueuing scheduled jobs).
    #[allow(dead_code)]
    queue: Arc<Q>,

    /// Scheduler configuration.
    config: SchedulerConfig,

    /// Redis keys.
    keys: RedisKeys,

    /// Registered scheduled jobs.
    jobs: Arc<RwLock<HashMap<String, ScheduledJob>>>,

    /// Shutdown signal sender.
    shutdown_tx: broadcast::Sender<()>,

    /// Running flag.
    running: Arc<AtomicBool>,

    /// Is this instance the leader.
    is_leader: Arc<AtomicBool>,

    /// Jobs executed counter.
    jobs_executed: Arc<std::sync::atomic::AtomicU64>,
}

impl<Q: JobQueue + 'static> Scheduler<Q> {
    /// Create a new scheduler.
    pub fn new(pool: Pool, queue: Arc<Q>, config: SchedulerConfig) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        let keys = RedisKeys::new(&config.key_prefix);

        Self {
            id: format!("scheduler-{}", Uuid::new_v4()),
            pool,
            queue,
            config,
            keys,
            jobs: Arc::new(RwLock::new(HashMap::new())),
            shutdown_tx,
            running: Arc::new(AtomicBool::new(false)),
            is_leader: Arc::new(AtomicBool::new(false)),
            jobs_executed: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Register a scheduled job.
    pub fn register(&self, job: ScheduledJob) {
        let name = job.name.clone();
        self.jobs.write().insert(name.clone(), job);
        info!(job_name = %name, "Registered scheduled job");
    }

    /// Register a job with cron expression.
    pub fn schedule<J: Job>(
        &self,
        name: impl Into<String>,
        cron_expr: &str,
        job_factory: impl Fn() -> J + Send + Sync + 'static,
    ) -> JobResult<()> {
        let scheduled_job = ScheduledJob::new(name, cron_expr, job_factory)?;
        self.register(scheduled_job);
        Ok(())
    }

    /// Unregister a scheduled job.
    pub fn unregister(&self, name: &str) -> Option<ScheduledJob> {
        self.jobs.write().remove(name)
    }

    /// Check if this instance is the leader.
    pub fn is_leader(&self) -> bool {
        self.is_leader.load(Ordering::SeqCst)
    }

    /// Get scheduler ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Try to acquire leadership.
    async fn try_acquire_leadership(&self) -> JobResult<bool> {
        let mut conn = self.pool.get().await?;
        let lock_key = self.keys.scheduler_lock();
        let ttl_secs = self.config.leader_ttl_secs as i64;

        // Try to set lock with NX (only if not exists)
        let result: Option<String> = redis::cmd("SET")
            .arg(&lock_key)
            .arg(&self.id)
            .arg("NX")
            .arg("EX")
            .arg(ttl_secs)
            .query_async(&mut *conn)
            .await?;

        if result.is_some() {
            // We acquired the lock
            self.is_leader.store(true, Ordering::SeqCst);
            info!(scheduler_id = %self.id, "Acquired scheduler leadership");
            return Ok(true);
        }

        // Check if we already own the lock
        let current_leader: Option<String> = conn.get(&lock_key).await?;
        if current_leader.as_ref() == Some(&self.id) {
            // Refresh our lock
            let _: () = conn.expire(&lock_key, ttl_secs).await?;
            return Ok(true);
        }

        self.is_leader.store(false, Ordering::SeqCst);
        Ok(false)
    }

    /// Release leadership.
    async fn release_leadership(&self) -> JobResult<()> {
        if !self.is_leader.load(Ordering::SeqCst) {
            return Ok(());
        }

        let mut conn = self.pool.get().await?;
        let lock_key = self.keys.scheduler_lock();

        // Only delete if we own the lock
        let lua_script = r#"
            if redis.call("get", KEYS[1]) == ARGV[1] then
                return redis.call("del", KEYS[1])
            else
                return 0
            end
        "#;

        let _: i32 = redis::Script::new(lua_script)
            .key(&lock_key)
            .arg(&self.id)
            .invoke_async(&mut *conn)
            .await?;

        self.is_leader.store(false, Ordering::SeqCst);
        info!(scheduler_id = %self.id, "Released scheduler leadership");

        Ok(())
    }

    /// Start the scheduler.
    pub async fn start(&self) -> JobResult<()> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Err(JobError::Configuration("Scheduler already running".to_string()));
        }

        info!(
            scheduler_id = %self.id,
            poll_interval_secs = self.config.poll_interval_secs,
            "Starting scheduler"
        );

        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let mut poll_interval = interval(Duration::from_secs(self.config.poll_interval_secs));
        let mut leader_check_interval = interval(Duration::from_secs(
            self.config.leader_check_interval_secs,
        ));

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    info!(scheduler_id = %self.id, "Received shutdown signal");
                    break;
                }

                _ = leader_check_interval.tick() => {
                    // Try to acquire or maintain leadership
                    if let Err(e) = self.try_acquire_leadership().await {
                        error!(error = %e, "Failed to check leadership");
                    }
                }

                _ = poll_interval.tick() => {
                    // Only run jobs if we are the leader
                    if self.is_leader.load(Ordering::SeqCst) {
                        if let Err(e) = self.check_and_enqueue_jobs().await {
                            error!(error = %e, "Failed to check scheduled jobs");
                        }
                    }
                }
            }
        }

        // Release leadership on shutdown
        if let Err(e) = self.release_leadership().await {
            warn!(error = %e, "Failed to release leadership on shutdown");
        }

        self.running.store(false, Ordering::SeqCst);
        info!(scheduler_id = %self.id, "Scheduler stopped");

        Ok(())
    }

    /// Stop the scheduler.
    pub fn stop(&self) {
        info!(scheduler_id = %self.id, "Stopping scheduler...");
        let _ = self.shutdown_tx.send(());
    }

    /// Check scheduled jobs and enqueue those due for execution.
    async fn check_and_enqueue_jobs(&self) -> JobResult<()> { // NOSONAR - complex scheduling logic
        let now = Utc::now();
        let mut conn = self.pool.get().await?;

        let jobs_to_run: Vec<(String, JobData)> = {
            let jobs = self.jobs.read();
            let mut to_run = Vec::new();

            for (name, scheduled_job) in jobs.iter() {
                if !scheduled_job.enabled {
                    continue;
                }

                // Check last run time from Redis
                let last_run_key = format!("{}:last_run:{}", self.keys.scheduled(), name);
                let last_run: Option<String> = conn.get(&last_run_key).await?;

                let should_run = match last_run {
                    Some(last_run_str) => {
                        if let Ok(last_run_time) = DateTime::parse_from_rfc3339(&last_run_str) {
                            // Check if next scheduled time after last run has passed
                            if let Some(next_run) = scheduled_job.next_run_from(last_run_time.into()) {
                                next_run <= now
                            } else {
                                false
                            }
                        } else {
                            true // Invalid last run time, run anyway
                        }
                    }
                    None => {
                        // Never run before, check if we should run now
                        true
                    }
                };

                if should_run {
                    match scheduled_job.create_job_data() {
                        Ok(job_data) => {
                            to_run.push((name.clone(), job_data));
                        }
                        Err(e) => {
                            error!(
                                job_name = %name,
                                error = %e,
                                "Failed to create job data for scheduled job"
                            );
                        }
                    }
                }
            }

            to_run
        };

        // Enqueue jobs outside of the lock
        for (name, job_data) in jobs_to_run {
            // Update last run time first (to prevent duplicate runs)
            let last_run_key = format!("{}:last_run:{}", self.keys.scheduled(), name);
            let _: () = conn.set(&last_run_key, now.to_rfc3339()).await?;

            // Enqueue the job
            match self.enqueue_job_data(job_data.clone()).await {
                Ok(job_id) => {
                    debug!(
                        job_name = %name,
                        job_id = %job_id,
                        "Enqueued scheduled job"
                    );
                    self.jobs_executed.fetch_add(1, Ordering::Relaxed);
                }
                Err(e) => {
                    error!(
                        job_name = %name,
                        error = %e,
                        "Failed to enqueue scheduled job"
                    );
                    // Reset last run time on failure
                    let _: () = conn.del(&last_run_key).await?;
                }
            }
        }

        Ok(())
    }

    /// Enqueue job data directly.
    async fn enqueue_job_data(&self, job_data: JobData) -> JobResult<String> {
        let mut conn = self.pool.get().await?;

        let job_id_str = job_data.id.as_str();

        // Store job data
        let job_key = format!("{}:{}", self.keys.job(job_id_str), "data");
        let serialized = serde_json::to_string(&job_data)?;
        let _: () = conn.set(&job_key, &serialized).await?;

        // Add to queue
        let queue_key = self.keys.priority_queue(&job_data.queue);
        let score = Self::calculate_priority_score(&job_data);
        let _: () = conn.zadd(&queue_key, job_id_str, score).await?;

        Ok(job_id_str.to_string())
    }

    /// Calculate priority score for sorted set ordering.
    fn calculate_priority_score(job_data: &JobData) -> f64 {
        let priority_weight = -(job_data.priority as f64) * 1_000_000_000_000.0;
        let time_weight = job_data.scheduled_at.timestamp_millis() as f64;
        priority_weight + time_weight
    }

    /// Get scheduler statistics.
    pub fn stats(&self) -> SchedulerStats {
        SchedulerStats {
            id: self.id.clone(),
            is_leader: self.is_leader.load(Ordering::SeqCst),
            scheduled_jobs: self.jobs.read().len(),
            jobs_executed: self.jobs_executed.load(Ordering::Relaxed),
            last_election: None, // TODO: Track this
        }
    }

    /// List all registered scheduled jobs.
    pub fn list_jobs(&self) -> Vec<ScheduledJobInfo> {
        let now = Utc::now();
        self.jobs
            .read()
            .values()
            .map(|job| ScheduledJobInfo {
                name: job.name.clone(),
                cron: job.cron.clone(),
                enabled: job.enabled,
                next_run: job.next_run_from(now),
            })
            .collect()
    }

    /// Enable a scheduled job.
    pub fn enable_job(&self, name: &str) -> bool {
        if let Some(job) = self.jobs.write().get_mut(name) {
            job.enabled = true;
            true
        } else {
            false
        }
    }

    /// Disable a scheduled job.
    pub fn disable_job(&self, name: &str) -> bool {
        if let Some(job) = self.jobs.write().get_mut(name) {
            job.enabled = false;
            true
        } else {
            false
        }
    }

    /// Trigger a scheduled job immediately.
    pub async fn trigger_job(&self, name: &str) -> JobResult<String> {
        let job_data = {
            let jobs = self.jobs.read();
            let scheduled_job = jobs
                .get(name)
                .ok_or_else(|| JobError::NotFound(format!("Scheduled job not found: {}", name)))?;

            scheduled_job.create_job_data()?
        };

        self.enqueue_job_data(job_data).await
    }
}

/// Information about a scheduled job.
#[derive(Debug, Clone)]
pub struct ScheduledJobInfo {
    /// Job name.
    pub name: String,

    /// Cron expression.
    pub cron: String,

    /// Is enabled.
    pub enabled: bool,

    /// Next scheduled run time.
    pub next_run: Option<DateTime<Utc>>,
}

/// Common cron expressions.
pub mod cron_expressions {
    /// Every minute.
    pub const EVERY_MINUTE: &str = "0 * * * * *";

    /// Every 5 minutes.
    pub const EVERY_5_MINUTES: &str = "0 */5 * * * *";

    /// Every 15 minutes.
    pub const EVERY_15_MINUTES: &str = "0 */15 * * * *";

    /// Every 30 minutes.
    pub const EVERY_30_MINUTES: &str = "0 */30 * * * *";

    /// Every hour.
    pub const EVERY_HOUR: &str = "0 0 * * * *";

    /// Every day at midnight.
    pub const DAILY_MIDNIGHT: &str = "0 0 0 * * *";

    /// Every day at 6 AM.
    pub const DAILY_6AM: &str = "0 0 6 * * *";

    /// Every Monday at midnight.
    pub const WEEKLY_MONDAY: &str = "0 0 0 * * MON";

    /// First day of every month at midnight.
    pub const MONTHLY: &str = "0 0 0 1 * *";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cron_expression_parsing() {
        let schedule = Schedule::from_str(cron_expressions::EVERY_MINUTE).unwrap();
        let next = schedule.after(&Utc::now()).next();
        assert!(next.is_some());
    }

    #[test]
    fn test_scheduled_job_next_run() {
        use async_trait::async_trait;
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Serialize, Deserialize)]
        struct TestJob;

        #[async_trait]
        impl Job for TestJob {
            const NAME: &'static str = "test_job";

            async fn execute(&self, _ctx: crate::job::JobContext) -> Result<(), JobError> {
                Ok(())
            }
        }

        let scheduled = ScheduledJob::new("test", cron_expressions::EVERY_MINUTE, || TestJob)
            .unwrap();

        let now = Utc::now();
        let next = scheduled.next_run_from(now);
        assert!(next.is_some());
        assert!(next.unwrap() > now);
    }
}
