//! Worker pool for processing jobs.

use crate::config::WorkerConfig;
use crate::error::{JobError, JobResult};
use crate::job::{Job, JobContext, JobData};
use crate::queue::JobQueue;
use async_trait::async_trait;
use futures::future::BoxFuture;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Semaphore};
use tokio::time::timeout;
use tracing::{debug, error, info, warn, Instrument};
use uuid::Uuid;

/// Worker pool configuration.
#[derive(Debug, Clone)]
pub struct WorkerPoolConfig {
    /// Number of concurrent workers.
    pub concurrency: usize,

    /// Queues to process (in priority order).
    pub queues: Vec<String>,

    /// Job execution timeout.
    pub job_timeout: Duration,

    /// Polling interval.
    pub poll_interval: Duration,

    /// Shutdown timeout.
    pub shutdown_timeout: Duration,

    /// Heartbeat interval.
    pub heartbeat_interval: Duration,
}

impl Default for WorkerPoolConfig {
    fn default() -> Self {
        Self {
            concurrency: 4,
            queues: vec!["default".to_string()],
            job_timeout: Duration::from_secs(300),
            poll_interval: Duration::from_millis(100),
            shutdown_timeout: Duration::from_secs(30),
            heartbeat_interval: Duration::from_secs(30),
        }
    }
}

impl From<&WorkerConfig> for WorkerPoolConfig {
    fn from(config: &WorkerConfig) -> Self {
        Self {
            concurrency: config.concurrency,
            queues: vec!["default".to_string()],
            job_timeout: config.job_timeout(),
            poll_interval: config.poll_interval(),
            shutdown_timeout: config.shutdown_timeout(),
            heartbeat_interval: Duration::from_secs(config.heartbeat_interval_secs),
        }
    }
}

/// Job handler function type.
pub type JobHandler = Box<
    dyn Fn(JobData, JobContext) -> BoxFuture<'static, Result<(), JobError>> + Send + Sync,
>;

/// Worker trait for processing jobs.
#[async_trait]
pub trait Worker: Send + Sync {
    /// Process a job.
    async fn process(&self, job_data: &JobData, ctx: JobContext) -> Result<(), JobError>;

    /// Check if this worker can handle the job type.
    fn can_handle(&self, job_name: &str) -> bool;
}

/// Worker pool for concurrent job processing.
pub struct WorkerPool<Q: JobQueue> {
    /// Unique pool ID.
    id: String,

    /// Job queue.
    queue: Arc<Q>,

    /// Pool configuration.
    config: WorkerPoolConfig,

    /// Registered job handlers.
    handlers: Arc<RwLock<HashMap<String, JobHandler>>>,

    /// Shutdown signal sender.
    shutdown_tx: broadcast::Sender<()>,

    /// Running flag.
    running: Arc<AtomicBool>,

    /// Jobs processed counter.
    jobs_processed: Arc<AtomicU64>,

    /// Jobs failed counter.
    jobs_failed: Arc<AtomicU64>,
}

impl<Q: JobQueue + 'static> WorkerPool<Q> {
    /// Create a new worker pool.
    pub fn new(queue: Arc<Q>, config: WorkerPoolConfig) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);

        Self {
            id: format!("worker-pool-{}", Uuid::new_v4()),
            queue,
            config,
            handlers: Arc::new(RwLock::new(HashMap::new())),
            shutdown_tx,
            running: Arc::new(AtomicBool::new(false)),
            jobs_processed: Arc::new(AtomicU64::new(0)),
            jobs_failed: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Register a job handler.
    pub fn register<J: Job>(&self, handler: impl Fn(J, JobContext) -> BoxFuture<'static, Result<(), JobError>> + Send + Sync + 'static) {
        let handler_fn: JobHandler = Box::new(move |job_data, ctx| {
            match job_data.deserialize::<J>() {
                Ok(job) => handler(job, ctx),
                Err(e) => Box::pin(async move { Err(e) }),
            }
        });

        self.handlers.write().insert(J::NAME.to_string(), handler_fn);
        info!(job_type = J::NAME, "Registered job handler");
    }

    /// Register a job type with default execution.
    pub fn register_job<J: Job>(&self) {
        let handler_fn: JobHandler = Box::new(move |job_data, ctx| {
            Box::pin(async move {
                let job: J = job_data.deserialize()?;
                job.before_execute(&ctx);
                let result = job.execute(ctx.clone()).await;
                match &result {
                    Ok(()) => job.after_execute(&ctx),
                    Err(e) => job.on_failure(&ctx, e),
                }
                result
            })
        });

        self.handlers.write().insert(J::NAME.to_string(), handler_fn);
        info!(job_type = J::NAME, "Registered job type");
    }

    /// Start the worker pool.
    pub async fn start(&self) -> JobResult<()> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Err(JobError::Worker("Worker pool already running".to_string()));
        }

        info!(
            pool_id = %self.id,
            concurrency = self.config.concurrency,
            queues = ?self.config.queues,
            "Starting worker pool"
        );

        let semaphore = Arc::new(Semaphore::new(self.config.concurrency));
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        // Spawn worker tasks
        let _queues: Vec<&str> = self.config.queues.iter().map(|s| s.as_str()).collect();

        loop {
            tokio::select! {
                // Check for shutdown signal
                _ = shutdown_rx.recv() => {
                    info!(pool_id = %self.id, "Received shutdown signal");
                    break;
                }

                // Try to acquire a worker slot
                permit = semaphore.clone().acquire_owned() => {
                    if let Ok(permit) = permit {
                        let queue = self.queue.clone();
                        let handlers = self.handlers.clone();
                        let worker_id = format!("{}-{}", self.id, Uuid::new_v4());
                        let queues_owned: Vec<String> = self.config.queues.clone();
                        let job_timeout = self.config.job_timeout;
                        let jobs_processed = self.jobs_processed.clone();
                        let jobs_failed = self.jobs_failed.clone();

                        tokio::spawn(async move {
                            let queues_ref: Vec<&str> = queues_owned.iter().map(|s| s.as_str()).collect();

                            // Try to dequeue a job
                            match queue.dequeue(&queues_ref, &worker_id).await {
                                Ok(Some(job_data)) => {
                                    let job_id = job_data.id.clone();
                                    let job_name = job_data.name.clone();
                                    let ctx = job_data.to_context(&worker_id);

                                    debug!(
                                        job_id = %job_id,
                                        job_name = %job_name,
                                        worker_id = %worker_id,
                                        "Processing job"
                                    );

                                    // Find handler
                                    let handler = handlers.read().get(&job_name).map(|_h| {
                                        // We need to call the handler - this is tricky with the borrow
                                        // For now, we'll just check if it exists
                                        true
                                    });

                                    if handler.is_none() {
                                        error!(job_name = %job_name, "No handler registered for job type");
                                        let _ = queue.fail(&job_id, &JobError::Configuration(
                                            format!("No handler for job type: {}", job_name)
                                        )).await;
                                        jobs_failed.fetch_add(1, Ordering::Relaxed);
                                        drop(permit);
                                        return;
                                    }

                                    // Execute with timeout
                                    let handler_future = {
                                        let handlers_guard = handlers.read();
                                        handlers_guard.get(&job_name).map(|handler| {
                                            handler(job_data.clone(), ctx.clone())
                                        })
                                    };

                                    let result = match handler_future {
                                        Some(future) => {
                                            Some(timeout(job_timeout, future).await)
                                        }
                                        None => None
                                    };

                                    // If no handler was found (shouldn't happen as we checked above)
                                    let result = match result {
                                        Some(r) => r,
                                        None => {
                                            error!(job_name = %job_name, "Handler not found during execution");
                                            let _ = queue.fail(&job_id, &JobError::Configuration(
                                                format!("Handler disappeared for job type: {}", job_name)
                                            )).await;
                                            jobs_failed.fetch_add(1, Ordering::Relaxed);
                                            drop(permit);
                                            return;
                                        }
                                    };

                                    match result {
                                        Ok(Ok(())) => {
                                            debug!(job_id = %job_id, "Job completed successfully");
                                            if let Err(e) = queue.complete(&job_id).await {
                                                error!(job_id = %job_id, error = %e, "Failed to mark job as complete");
                                            }
                                            jobs_processed.fetch_add(1, Ordering::Relaxed);
                                        }
                                        Ok(Err(e)) => {
                                            warn!(job_id = %job_id, error = %e, "Job execution failed");
                                            if let Err(e) = queue.fail(&job_id, &e).await {
                                                error!(job_id = %job_id, error = %e, "Failed to mark job as failed");
                                            }
                                            jobs_failed.fetch_add(1, Ordering::Relaxed);
                                        }
                                        Err(_) => {
                                            warn!(job_id = %job_id, timeout_secs = ?job_timeout, "Job timed out");
                                            let error = JobError::Timeout(job_timeout.as_secs());
                                            if let Err(e) = queue.fail(&job_id, &error).await {
                                                error!(job_id = %job_id, error = %e, "Failed to mark job as timed out");
                                            }
                                            jobs_failed.fetch_add(1, Ordering::Relaxed);
                                        }
                                    }
                                }
                                Ok(None) => {
                                    // No job available, wait before polling again
                                }
                                Err(e) => {
                                    error!(error = %e, "Failed to dequeue job");
                                }
                            }

                            drop(permit);
                        }.instrument(tracing::info_span!("worker")));
                    }
                }
            }

            // Small delay to prevent busy-waiting
            tokio::time::sleep(self.config.poll_interval).await;
        }

        // Wait for all workers to finish
        info!(pool_id = %self.id, "Waiting for workers to finish...");
        let _ = timeout(
            self.config.shutdown_timeout,
            async {
                while semaphore.available_permits() < self.config.concurrency {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        ).await;

        self.running.store(false, Ordering::SeqCst);

        info!(
            pool_id = %self.id,
            processed = self.jobs_processed.load(Ordering::Relaxed),
            failed = self.jobs_failed.load(Ordering::Relaxed),
            "Worker pool stopped"
        );

        Ok(())
    }

    /// Stop the worker pool.
    pub fn stop(&self) {
        info!(pool_id = %self.id, "Stopping worker pool...");
        let _ = self.shutdown_tx.send(());
    }

    /// Check if the pool is running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Get the number of jobs processed.
    pub fn jobs_processed(&self) -> u64 {
        self.jobs_processed.load(Ordering::Relaxed)
    }

    /// Get the number of jobs failed.
    pub fn jobs_failed(&self) -> u64 {
        self.jobs_failed.load(Ordering::Relaxed)
    }

    /// Get the pool ID.
    pub fn id(&self) -> &str {
        &self.id
    }
}

/// Worker pool statistics.
#[derive(Debug, Clone)]
pub struct WorkerPoolStats {
    /// Pool ID.
    pub id: String,

    /// Is running.
    pub running: bool,

    /// Configured concurrency.
    pub concurrency: usize,

    /// Jobs processed.
    pub jobs_processed: u64,

    /// Jobs failed.
    pub jobs_failed: u64,

    /// Queues being processed.
    pub queues: Vec<String>,
}

impl<Q: JobQueue + 'static> WorkerPool<Q> {
    /// Get pool statistics.
    pub fn stats(&self) -> WorkerPoolStats {
        WorkerPoolStats {
            id: self.id.clone(),
            running: self.is_running(),
            concurrency: self.config.concurrency,
            jobs_processed: self.jobs_processed(),
            jobs_failed: self.jobs_failed(),
            queues: self.config.queues.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_pool_config_default() {
        let config = WorkerPoolConfig::default();
        assert_eq!(config.concurrency, 4);
        assert_eq!(config.queues, vec!["default".to_string()]);
    }
}
