//! Job queue configuration.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Configuration for the job queue system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobsConfig {
    /// Redis connection configuration.
    #[serde(default)]
    pub redis: RedisConfig,

    /// Worker pool configuration.
    #[serde(default)]
    pub worker: WorkerConfig,

    /// Queue configuration.
    #[serde(default)]
    pub queue: QueueConfig,

    /// Scheduler configuration.
    #[serde(default)]
    pub scheduler: SchedulerConfig,
}

impl Default for JobsConfig {
    fn default() -> Self {
        Self {
            redis: RedisConfig::default(),
            worker: WorkerConfig::default(),
            queue: QueueConfig::default(),
            scheduler: SchedulerConfig::default(),
        }
    }
}

/// Redis connection configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis URL.
    #[serde(default = "default_redis_url")]
    pub url: String,

    /// Connection pool size.
    #[serde(default = "default_pool_size")]
    pub pool_size: usize,

    /// Connection timeout in seconds.
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout_secs: u64,

    /// Key prefix for all job-related keys.
    #[serde(default = "default_key_prefix")]
    pub key_prefix: String,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: default_redis_url(),
            pool_size: default_pool_size(),
            connect_timeout_secs: default_connect_timeout(),
            key_prefix: default_key_prefix(),
        }
    }
}

fn default_redis_url() -> String {
    "redis://localhost:6379".to_string()
}

fn default_pool_size() -> usize {
    10
}

fn default_connect_timeout() -> u64 {
    5
}

fn default_key_prefix() -> String {
    "arcana:jobs".to_string()
}

/// Worker pool configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    /// Number of worker threads.
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,

    /// Job execution timeout in seconds.
    #[serde(default = "default_job_timeout")]
    pub job_timeout_secs: u64,

    /// Polling interval in milliseconds.
    #[serde(default = "default_poll_interval")]
    pub poll_interval_ms: u64,

    /// Shutdown timeout in seconds.
    #[serde(default = "default_shutdown_timeout")]
    pub shutdown_timeout_secs: u64,

    /// Heartbeat interval in seconds.
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval_secs: u64,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            concurrency: default_concurrency(),
            job_timeout_secs: default_job_timeout(),
            poll_interval_ms: default_poll_interval(),
            shutdown_timeout_secs: default_shutdown_timeout(),
            heartbeat_interval_secs: default_heartbeat_interval(),
        }
    }
}

fn default_concurrency() -> usize {
    // Use available parallelism or fallback to 4
    std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(4)
        .max(4)
}

fn default_job_timeout() -> u64 {
    300 // 5 minutes
}

fn default_poll_interval() -> u64 {
    100 // 100ms
}

fn default_shutdown_timeout() -> u64 {
    30
}

fn default_heartbeat_interval() -> u64 {
    30
}

/// Queue configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    /// Maximum queue size (0 = unlimited).
    #[serde(default)]
    pub max_size: usize,

    /// Default retry policy.
    #[serde(default)]
    pub default_retry: RetryConfig,

    /// Dead letter queue configuration.
    #[serde(default)]
    pub dlq: DlqConfig,

    /// Job retention period in seconds (for completed jobs).
    #[serde(default = "default_retention")]
    pub retention_secs: u64,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            max_size: 0,
            default_retry: RetryConfig::default(),
            dlq: DlqConfig::default(),
            retention_secs: default_retention(),
        }
    }
}

fn default_retention() -> u64 {
    86400 * 7 // 7 days
}

/// Retry configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retries.
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Initial delay in milliseconds.
    #[serde(default = "default_initial_delay")]
    pub initial_delay_ms: u64,

    /// Maximum delay in milliseconds.
    #[serde(default = "default_max_delay")]
    pub max_delay_ms: u64,

    /// Backoff multiplier.
    #[serde(default = "default_multiplier")]
    pub multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: default_max_retries(),
            initial_delay_ms: default_initial_delay(),
            max_delay_ms: default_max_delay(),
            multiplier: default_multiplier(),
        }
    }
}

fn default_max_retries() -> u32 {
    3
}

fn default_initial_delay() -> u64 {
    1000 // 1 second
}

fn default_max_delay() -> u64 {
    3600000 // 1 hour
}

fn default_multiplier() -> f64 {
    2.0
}

/// Dead letter queue configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlqConfig {
    /// Enable dead letter queue.
    #[serde(default = "default_dlq_enabled")]
    pub enabled: bool,

    /// Maximum DLQ size.
    #[serde(default = "default_dlq_max_size")]
    pub max_size: usize,

    /// DLQ retention in seconds.
    #[serde(default = "default_dlq_retention")]
    pub retention_secs: u64,
}

impl Default for DlqConfig {
    fn default() -> Self {
        Self {
            enabled: default_dlq_enabled(),
            max_size: default_dlq_max_size(),
            retention_secs: default_dlq_retention(),
        }
    }
}

fn default_dlq_enabled() -> bool {
    true
}

fn default_dlq_max_size() -> usize {
    10000
}

fn default_dlq_retention() -> u64 {
    86400 * 30 // 30 days
}

/// Scheduler configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    /// Enable the scheduler.
    #[serde(default = "default_scheduler_enabled")]
    pub enabled: bool,

    /// Check interval in seconds.
    #[serde(default = "default_check_interval")]
    pub check_interval_secs: u64,

    /// Poll interval in seconds (for checking scheduled jobs).
    #[serde(default = "default_poll_interval_secs")]
    pub poll_interval_secs: u64,

    /// Leader check interval in seconds.
    #[serde(default = "default_leader_check_interval")]
    pub leader_check_interval_secs: u64,

    /// Leader election TTL in seconds.
    #[serde(default = "default_leader_ttl")]
    pub leader_ttl_secs: u64,

    /// Key prefix for scheduler keys.
    #[serde(default = "default_scheduler_key_prefix")]
    pub key_prefix: String,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: default_scheduler_enabled(),
            check_interval_secs: default_check_interval(),
            poll_interval_secs: default_poll_interval_secs(),
            leader_check_interval_secs: default_leader_check_interval(),
            leader_ttl_secs: default_leader_ttl(),
            key_prefix: default_scheduler_key_prefix(),
        }
    }
}

fn default_scheduler_enabled() -> bool {
    true
}

fn default_check_interval() -> u64 {
    60
}

fn default_poll_interval_secs() -> u64 {
    10
}

fn default_leader_check_interval() -> u64 {
    15
}

fn default_leader_ttl() -> u64 {
    30
}

fn default_scheduler_key_prefix() -> String {
    "arcana:jobs".to_string()
}

impl WorkerConfig {
    /// Returns job timeout as Duration.
    pub fn job_timeout(&self) -> Duration {
        Duration::from_secs(self.job_timeout_secs)
    }

    /// Returns poll interval as Duration.
    pub fn poll_interval(&self) -> Duration {
        Duration::from_millis(self.poll_interval_ms)
    }

    /// Returns shutdown timeout as Duration.
    pub fn shutdown_timeout(&self) -> Duration {
        Duration::from_secs(self.shutdown_timeout_secs)
    }
}

