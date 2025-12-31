//! Arcana Jobs - Distributed Job Queue System
//!
//! A Redis-backed distributed job queue with:
//! - Typed job definitions with serde serialization
//! - Configurable worker pools with concurrency control
//! - Retry policies with exponential backoff
//! - Dead letter queue for failed jobs
//! - Priority queues (critical, high, normal, low)
//! - Cron-based job scheduling
//! - Job status tracking and monitoring
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    Arcana Jobs Architecture                      │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                  │
//! │  Producer                                                        │
//! │     │                                                            │
//! │     ▼                                                            │
//! │  ┌─────────────────────────────────────────────────────────┐    │
//! │  │              Redis Queue Backend                         │    │
//! │  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐        │    │
//! │  │  │Critical │ │  High   │ │ Normal  │ │   Low   │        │    │
//! │  │  │ Queue   │ │  Queue  │ │  Queue  │ │  Queue  │        │    │
//! │  │  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘        │    │
//! │  │       └───────────┴──────────┴───────────┘              │    │
//! │  │                        │                                 │    │
//! │  │  ┌─────────────────────┼─────────────────────────────┐  │    │
//! │  │  │     Delayed Jobs    │      Scheduled Jobs         │  │    │
//! │  │  └─────────────────────┴─────────────────────────────┘  │    │
//! │  └─────────────────────────────────────────────────────────┘    │
//! │                           │                                      │
//! │                           ▼                                      │
//! │  ┌─────────────────────────────────────────────────────────┐    │
//! │  │                   Worker Pool                            │    │
//! │  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐    │    │
//! │  │  │ Worker 1 │ │ Worker 2 │ │ Worker 3 │ │ Worker N │    │    │
//! │  │  └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘    │    │
//! │  │       │            │            │            │          │    │
//! │  │       └────────────┴────────────┴────────────┘          │    │
//! │  │                        │                                 │    │
//! │  │              ┌─────────┴─────────┐                      │    │
//! │  │              ▼                   ▼                      │    │
//! │  │        ┌──────────┐       ┌─────────────┐               │    │
//! │  │        │ Completed│       │ Dead Letter │               │    │
//! │  │        │   Jobs   │       │    Queue    │               │    │
//! │  │        └──────────┘       └─────────────┘               │    │
//! │  └─────────────────────────────────────────────────────────┘    │
//! │                                                                  │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use arcana_jobs::{Job, JobQueue, Worker, Priority};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Serialize, Deserialize)]
//! struct SendEmailJob {
//!     to: String,
//!     subject: String,
//!     body: String,
//! }
//!
//! #[async_trait::async_trait]
//! impl Job for SendEmailJob {
//!     const NAME: &'static str = "send_email";
//!     const QUEUE: &'static str = "emails";
//!     const MAX_RETRIES: u32 = 3;
//!
//!     async fn execute(&self, ctx: JobContext) -> Result<(), JobError> {
//!         // Send email logic
//!         Ok(())
//!     }
//! }
//!
//! // Enqueue a job
//! queue.enqueue(SendEmailJob {
//!     to: "user@example.com".to_string(),
//!     subject: "Welcome!".to_string(),
//!     body: "Hello...".to_string(),
//! }).priority(Priority::High).send().await?;
//! ```

pub mod config;
pub mod di;
pub mod error;
pub mod job;
pub mod metrics;
pub mod queue;
pub mod redis;
pub mod retry;
pub mod scheduler;
pub mod status;
pub mod worker;
pub mod worker_registry;

pub use config::JobsConfig;
pub use di::{JobQueueInterface, JobQueueService};
pub use error::{JobError, JobResult};
pub use job::{Job, JobContext, JobData, JobId, JobInfo, JobStatus};
pub use metrics::{register_metrics, JobMetrics, RedisMetrics, SchedulerMetrics, WorkerMetrics};
pub use queue::{JobQueue, Priority, QueuedJob};
pub use retry::{RetryPolicy, RetryStrategy};
pub use scheduler::{cron_expressions, ScheduledJob, ScheduledJobInfo, Scheduler, SchedulerStats};
pub use status::{DashboardStats, JobSearchQuery, JobSearchResult, JobStatusTracker, ThroughputMetrics, ThroughputPeriod, WorkerHealth};
pub use worker::{Worker, WorkerPool, WorkerPoolConfig, WorkerPoolStats};
pub use worker_registry::{WorkerInfo, WorkerRegistry, DEFAULT_HEARTBEAT_TIMEOUT};

/// Re-export commonly used traits
pub mod prelude {
    pub use crate::job::{Job, JobStatus};
    pub use crate::queue::{JobQueue, Priority};
    pub use crate::retry::RetryPolicy;
    pub use crate::worker::Worker;
    pub use crate::{JobContext, JobError, JobId, JobResult};
}
