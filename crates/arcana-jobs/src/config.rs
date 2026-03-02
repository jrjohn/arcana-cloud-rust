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

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // JobsConfig tests
    // =========================================================================

    #[test]
    fn test_jobs_config_default() {
        let cfg = JobsConfig::default();
        // Smoke check: subfields are constructed
        assert!(!cfg.redis.url.is_empty());
        assert!(cfg.worker.concurrency >= 4);
        assert_eq!(cfg.queue.max_size, 0);
        assert!(cfg.scheduler.enabled);
    }

    #[test]
    fn test_jobs_config_clone() {
        let cfg = JobsConfig::default();
        let cloned = cfg.clone();
        assert_eq!(cfg.redis.url, cloned.redis.url);
        assert_eq!(cfg.worker.concurrency, cloned.worker.concurrency);
    }

    #[test]
    fn test_jobs_config_debug() {
        let cfg = JobsConfig::default();
        let s = format!("{:?}", cfg);
        assert!(s.contains("JobsConfig"));
    }

    #[test]
    fn test_jobs_config_serde_roundtrip() {
        let cfg = JobsConfig::default();
        let json = serde_json::to_string(&cfg).expect("serialize");
        let restored: JobsConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(cfg.redis.url, restored.redis.url);
        assert_eq!(cfg.worker.job_timeout_secs, restored.worker.job_timeout_secs);
    }

    #[test]
    fn test_jobs_config_partial_deserialize() {
        // Only override redis url; rest should use defaults
        let json = r#"{"redis": {"url": "redis://myhost:6380"}}"#;
        let cfg: JobsConfig = serde_json::from_str(json).expect("deserialize partial");
        assert_eq!(cfg.redis.url, "redis://myhost:6380");
        assert_eq!(cfg.worker.job_timeout_secs, 300);
    }

    // =========================================================================
    // RedisConfig tests
    // =========================================================================

    #[test]
    fn test_redis_config_default_values() {
        let cfg = RedisConfig::default();
        assert_eq!(cfg.url, "redis://localhost:6379");
        assert_eq!(cfg.pool_size, 10);
        assert_eq!(cfg.connect_timeout_secs, 5);
        assert_eq!(cfg.key_prefix, "arcana:jobs");
    }

    #[test]
    fn test_redis_config_clone() {
        let cfg = RedisConfig::default();
        let c2 = cfg.clone();
        assert_eq!(cfg.url, c2.url);
        assert_eq!(cfg.pool_size, c2.pool_size);
    }

    #[test]
    fn test_redis_config_debug() {
        let cfg = RedisConfig::default();
        let s = format!("{:?}", cfg);
        assert!(s.contains("RedisConfig"));
    }

    #[test]
    fn test_redis_config_serde_roundtrip() {
        let cfg = RedisConfig::default();
        let json = serde_json::to_string(&cfg).expect("serialize");
        let r: RedisConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(cfg.url, r.url);
        assert_eq!(cfg.key_prefix, r.key_prefix);
    }

    #[test]
    fn test_redis_config_custom_values() {
        let json = r#"{
            "url": "redis://prod:6379",
            "pool_size": 20,
            "connect_timeout_secs": 10,
            "key_prefix": "myapp:jobs"
        }"#;
        let cfg: RedisConfig = serde_json::from_str(json).expect("deserialize");
        assert_eq!(cfg.url, "redis://prod:6379");
        assert_eq!(cfg.pool_size, 20);
        assert_eq!(cfg.connect_timeout_secs, 10);
        assert_eq!(cfg.key_prefix, "myapp:jobs");
    }

    // =========================================================================
    // WorkerConfig tests
    // =========================================================================

    #[test]
    fn test_worker_config_default_values() {
        let cfg = WorkerConfig::default();
        assert!(cfg.concurrency >= 4);
        assert_eq!(cfg.job_timeout_secs, 300);
        assert_eq!(cfg.poll_interval_ms, 100);
        assert_eq!(cfg.shutdown_timeout_secs, 30);
        assert_eq!(cfg.heartbeat_interval_secs, 30);
    }

    #[test]
    fn test_worker_config_job_timeout_duration() {
        let cfg = WorkerConfig::default();
        assert_eq!(cfg.job_timeout(), Duration::from_secs(300));
    }

    #[test]
    fn test_worker_config_poll_interval_duration() {
        let cfg = WorkerConfig::default();
        assert_eq!(cfg.poll_interval(), Duration::from_millis(100));
    }

    #[test]
    fn test_worker_config_shutdown_timeout_duration() {
        let cfg = WorkerConfig::default();
        assert_eq!(cfg.shutdown_timeout(), Duration::from_secs(30));
    }

    #[test]
    fn test_worker_config_custom_durations() {
        let cfg = WorkerConfig {
            concurrency: 8,
            job_timeout_secs: 600,
            poll_interval_ms: 250,
            shutdown_timeout_secs: 60,
            heartbeat_interval_secs: 15,
        };
        assert_eq!(cfg.job_timeout(), Duration::from_secs(600));
        assert_eq!(cfg.poll_interval(), Duration::from_millis(250));
        assert_eq!(cfg.shutdown_timeout(), Duration::from_secs(60));
    }

    #[test]
    fn test_worker_config_clone() {
        let cfg = WorkerConfig::default();
        let c2 = cfg.clone();
        assert_eq!(cfg.concurrency, c2.concurrency);
        assert_eq!(cfg.job_timeout_secs, c2.job_timeout_secs);
    }

    #[test]
    fn test_worker_config_debug() {
        let cfg = WorkerConfig::default();
        let s = format!("{:?}", cfg);
        assert!(s.contains("WorkerConfig"));
    }

    #[test]
    fn test_worker_config_serde_roundtrip() {
        let cfg = WorkerConfig::default();
        let json = serde_json::to_string(&cfg).expect("serialize");
        let r: WorkerConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(cfg.job_timeout_secs, r.job_timeout_secs);
        assert_eq!(cfg.poll_interval_ms, r.poll_interval_ms);
    }

    // =========================================================================
    // QueueConfig tests
    // =========================================================================

    #[test]
    fn test_queue_config_default_values() {
        let cfg = QueueConfig::default();
        assert_eq!(cfg.max_size, 0);
        assert_eq!(cfg.retention_secs, 86400 * 7);
    }

    #[test]
    fn test_queue_config_clone() {
        let cfg = QueueConfig::default();
        let c2 = cfg.clone();
        assert_eq!(cfg.max_size, c2.max_size);
        assert_eq!(cfg.retention_secs, c2.retention_secs);
    }

    #[test]
    fn test_queue_config_debug() {
        let cfg = QueueConfig::default();
        let s = format!("{:?}", cfg);
        assert!(s.contains("QueueConfig"));
    }

    #[test]
    fn test_queue_config_serde_roundtrip() {
        let cfg = QueueConfig::default();
        let json = serde_json::to_string(&cfg).expect("serialize");
        let r: QueueConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(cfg.max_size, r.max_size);
        assert_eq!(cfg.retention_secs, r.retention_secs);
    }

    // =========================================================================
    // RetryConfig tests
    // =========================================================================

    #[test]
    fn test_retry_config_default_values() {
        let cfg = RetryConfig::default();
        assert_eq!(cfg.max_retries, 3);
        assert_eq!(cfg.initial_delay_ms, 1000);
        assert_eq!(cfg.max_delay_ms, 3_600_000);
        assert!((cfg.multiplier - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_retry_config_clone() {
        let cfg = RetryConfig::default();
        let c2 = cfg.clone();
        assert_eq!(cfg.max_retries, c2.max_retries);
        assert!((cfg.multiplier - c2.multiplier).abs() < f64::EPSILON);
    }

    #[test]
    fn test_retry_config_debug() {
        let cfg = RetryConfig::default();
        let s = format!("{:?}", cfg);
        assert!(s.contains("RetryConfig"));
    }

    #[test]
    fn test_retry_config_serde_roundtrip() {
        let cfg = RetryConfig::default();
        let json = serde_json::to_string(&cfg).expect("serialize");
        let r: RetryConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(cfg.max_retries, r.max_retries);
        assert_eq!(cfg.initial_delay_ms, r.initial_delay_ms);
    }

    #[test]
    fn test_retry_config_custom() {
        let json = r#"{
            "max_retries": 5,
            "initial_delay_ms": 500,
            "max_delay_ms": 60000,
            "multiplier": 1.5
        }"#;
        let cfg: RetryConfig = serde_json::from_str(json).expect("deserialize");
        assert_eq!(cfg.max_retries, 5);
        assert_eq!(cfg.initial_delay_ms, 500);
        assert_eq!(cfg.max_delay_ms, 60_000);
        assert!((cfg.multiplier - 1.5).abs() < f64::EPSILON);
    }

    // =========================================================================
    // DlqConfig tests
    // =========================================================================

    #[test]
    fn test_dlq_config_default_values() {
        let cfg = DlqConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.max_size, 10_000);
        assert_eq!(cfg.retention_secs, 86400 * 30);
    }

    #[test]
    fn test_dlq_config_clone() {
        let cfg = DlqConfig::default();
        let c2 = cfg.clone();
        assert_eq!(cfg.enabled, c2.enabled);
        assert_eq!(cfg.max_size, c2.max_size);
    }

    #[test]
    fn test_dlq_config_debug() {
        let cfg = DlqConfig::default();
        let s = format!("{:?}", cfg);
        assert!(s.contains("DlqConfig"));
    }

    #[test]
    fn test_dlq_config_serde_roundtrip() {
        let cfg = DlqConfig::default();
        let json = serde_json::to_string(&cfg).expect("serialize");
        let r: DlqConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(cfg.enabled, r.enabled);
        assert_eq!(cfg.max_size, r.max_size);
    }

    #[test]
    fn test_dlq_config_disabled() {
        let json = r#"{"enabled": false, "max_size": 500, "retention_secs": 3600}"#;
        let cfg: DlqConfig = serde_json::from_str(json).expect("deserialize");
        assert!(!cfg.enabled);
        assert_eq!(cfg.max_size, 500);
        assert_eq!(cfg.retention_secs, 3600);
    }

    // =========================================================================
    // SchedulerConfig tests
    // =========================================================================

    #[test]
    fn test_scheduler_config_default_values() {
        let cfg = SchedulerConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.check_interval_secs, 60);
        assert_eq!(cfg.poll_interval_secs, 10);
        assert_eq!(cfg.leader_check_interval_secs, 15);
        assert_eq!(cfg.leader_ttl_secs, 30);
        assert_eq!(cfg.key_prefix, "arcana:jobs");
    }

    #[test]
    fn test_scheduler_config_clone() {
        let cfg = SchedulerConfig::default();
        let c2 = cfg.clone();
        assert_eq!(cfg.enabled, c2.enabled);
        assert_eq!(cfg.check_interval_secs, c2.check_interval_secs);
        assert_eq!(cfg.key_prefix, c2.key_prefix);
    }

    #[test]
    fn test_scheduler_config_debug() {
        let cfg = SchedulerConfig::default();
        let s = format!("{:?}", cfg);
        assert!(s.contains("SchedulerConfig"));
    }

    #[test]
    fn test_scheduler_config_serde_roundtrip() {
        let cfg = SchedulerConfig::default();
        let json = serde_json::to_string(&cfg).expect("serialize");
        let r: SchedulerConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(cfg.enabled, r.enabled);
        assert_eq!(cfg.poll_interval_secs, r.poll_interval_secs);
        assert_eq!(cfg.leader_ttl_secs, r.leader_ttl_secs);
    }

    #[test]
    fn test_scheduler_config_custom() {
        let json = r#"{
            "enabled": false,
            "check_interval_secs": 30,
            "poll_interval_secs": 5,
            "leader_check_interval_secs": 10,
            "leader_ttl_secs": 20,
            "key_prefix": "custom:sched"
        }"#;
        let cfg: SchedulerConfig = serde_json::from_str(json).expect("deserialize");
        assert!(!cfg.enabled);
        assert_eq!(cfg.check_interval_secs, 30);
        assert_eq!(cfg.poll_interval_secs, 5);
        assert_eq!(cfg.leader_check_interval_secs, 10);
        assert_eq!(cfg.leader_ttl_secs, 20);
        assert_eq!(cfg.key_prefix, "custom:sched");
    }

    // =========================================================================
    // Full config override test
    // =========================================================================

    #[test]
    fn test_full_config_override() {
        let json = r#"{
            "redis": {"url": "redis://redis:6379", "pool_size": 5},
            "worker": {"concurrency": 2, "job_timeout_secs": 60},
            "queue": {"max_size": 1000, "retention_secs": 3600},
            "scheduler": {"enabled": false}
        }"#;
        let cfg: JobsConfig = serde_json::from_str(json).expect("deserialize full config");
        assert_eq!(cfg.redis.url, "redis://redis:6379");
        assert_eq!(cfg.redis.pool_size, 5);
        assert_eq!(cfg.worker.concurrency, 2);
        assert_eq!(cfg.worker.job_timeout_secs, 60);
        assert_eq!(cfg.queue.max_size, 1000);
        assert!(!cfg.scheduler.enabled);
    }
}

