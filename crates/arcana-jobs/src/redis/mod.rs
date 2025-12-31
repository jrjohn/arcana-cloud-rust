//! Redis-backed job queue implementation.

mod queue;

pub use queue::RedisJobQueue;

use crate::config::RedisConfig;
use crate::error::{JobError, JobResult};
use deadpool_redis::{Config, Pool, Runtime};
use tracing::info;

/// Create a Redis connection pool.
pub async fn create_pool(config: &RedisConfig) -> JobResult<Pool> {
    info!("Creating Redis connection pool for job queue...");

    let cfg = Config::from_url(&config.url);

    let pool = cfg
        .builder()
        .map_err(|e| JobError::Configuration(format!("Invalid Redis config: {}", e)))?
        .max_size(config.pool_size)
        .runtime(Runtime::Tokio1)
        .build()
        .map_err(|e| JobError::Configuration(format!("Failed to create pool: {}", e)))?;

    // Test connection
    let mut conn = pool.get().await?;
    redis::cmd("PING")
        .query_async::<String>(&mut *conn)
        .await?;

    info!("Redis connection pool created successfully");

    Ok(pool)
}

/// Redis key builder for job queue.
pub struct RedisKeys {
    prefix: String,
}

impl RedisKeys {
    /// Create a new key builder with the given prefix.
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }

    /// Queue key for pending jobs (sorted set by scheduled time).
    pub fn queue(&self, queue_name: &str) -> String {
        format!("{}:queue:{}", self.prefix, queue_name)
    }

    /// Priority queue key (sorted set by priority + time).
    pub fn priority_queue(&self, queue_name: &str) -> String {
        format!("{}:pqueue:{}", self.prefix, queue_name)
    }

    /// Delayed jobs key (sorted set by execution time).
    pub fn delayed(&self) -> String {
        format!("{}:delayed", self.prefix)
    }

    /// Active jobs key (hash: job_id -> worker_id).
    pub fn active(&self) -> String {
        format!("{}:active", self.prefix)
    }

    /// Job data key (hash: job_id -> job data).
    pub fn job(&self, job_id: &str) -> String {
        format!("{}:job:{}", self.prefix, job_id)
    }

    /// Dead letter queue key (sorted set).
    pub fn dlq(&self) -> String {
        format!("{}:dlq", self.prefix)
    }

    /// Completed jobs key (sorted set by completion time).
    pub fn completed(&self) -> String {
        format!("{}:completed", self.prefix)
    }

    /// Unique job key for deduplication.
    pub fn unique(&self, key: &str) -> String {
        format!("{}:unique:{}", self.prefix, key)
    }

    /// Worker heartbeat key.
    pub fn worker(&self, worker_id: &str) -> String {
        format!("{}:worker:{}", self.prefix, worker_id)
    }

    /// Scheduler lock key.
    pub fn scheduler_lock(&self) -> String {
        format!("{}:scheduler:lock", self.prefix)
    }

    /// Scheduled jobs key (hash: job_name -> cron expression).
    pub fn scheduled(&self) -> String {
        format!("{}:scheduled", self.prefix)
    }

    /// Stats key.
    pub fn stats(&self, queue_name: &str) -> String {
        format!("{}:stats:{}", self.prefix, queue_name)
    }
}

impl Default for RedisKeys {
    fn default() -> Self {
        Self::new("arcana:jobs")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redis_keys() {
        let keys = RedisKeys::new("test");

        assert_eq!(keys.queue("default"), "test:queue:default");
        assert_eq!(keys.job("123"), "test:job:123");
        assert_eq!(keys.dlq(), "test:dlq");
        assert_eq!(keys.worker("w1"), "test:worker:w1");
    }
}
