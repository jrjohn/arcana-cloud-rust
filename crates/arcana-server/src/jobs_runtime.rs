//! Background job roles: `worker` and `scheduler`.
//!
//! These are the two deployment layers that serve no public API and open no
//! database pool. They exist as separate roles so a Kubernetes deployment can
//! size and scale them independently of the request-serving pods:
//!
//! - **worker** is stateless and scales on queue depth.
//! - **scheduler** elects a single leader, so extra replicas are standby only
//!   and it must never be attached to a HorizontalPodAutoscaler.
//!
//! Both expose the same small operations surface (`/health`, `/live`,
//! `/ready`, `/metrics`) on the configured REST port, which is what the
//! Kubernetes probes and the Prometheus scrape annotation point at.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use arcana_config::{AppConfig, ServerConfig};
use arcana_core::{ArcanaError, ArcanaResult};
use arcana_jobs::redis::{create_pool, RedisJobQueue};
use arcana_jobs::{JobsConfig, Scheduler, WorkerPool, WorkerPoolConfig};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use tracing::{info, warn};

/// Shared state for the operations endpoints.
#[derive(Clone)]
struct OpsState {
    /// Flipped once the role has finished its startup work.
    ready: Arc<AtomicBool>,
    metrics: PrometheusHandle,
}

/// Runs the background job worker role.
pub async fn run_worker(config: AppConfig, jobs_config: JobsConfig) -> ArcanaResult<()> {
    info!("Starting Worker role");

    let metrics = install_metrics("worker")?;
    let ready = Arc::new(AtomicBool::new(false));
    let ops = serve_ops(&config.server, ready.clone(), metrics);

    let pool = create_pool(&jobs_config.redis)
        .await
        .map_err(|e| ArcanaError::Internal(format!("Job queue Redis pool: {e}")))?;

    let worker_config = WorkerPoolConfig::from(&jobs_config.worker);
    info!(
        concurrency = worker_config.concurrency,
        queues = ?worker_config.queues,
        "Worker pool configured"
    );

    let queue = Arc::new(RedisJobQueue::new(pool, jobs_config.clone()));
    let worker_pool = Arc::new(WorkerPool::new(queue, worker_config.clone()));

    register_job_handlers(&worker_pool);

    let runner = {
        let worker_pool = worker_pool.clone();
        tokio::spawn(async move { worker_pool.start().await })
    };

    ready.store(true, Ordering::SeqCst);
    crate::shutdown::wait_for_signal().await;

    info!("Draining worker pool...");
    ready.store(false, Ordering::SeqCst);
    worker_pool.stop();
    drain(runner, worker_config.shutdown_timeout, "worker pool").await;

    ops.abort();
    info!("Worker role shutdown complete");
    Ok(())
}

/// Runs the cron scheduler role.
pub async fn run_scheduler(config: AppConfig, jobs_config: JobsConfig) -> ArcanaResult<()> {
    info!("Starting Scheduler role");

    if !jobs_config.scheduler.enabled {
        return Err(ArcanaError::Configuration(
            "deployment layer is 'scheduler' but jobs.scheduler.enabled is false".to_string(),
        ));
    }

    let metrics = install_metrics("scheduler")?;
    let ready = Arc::new(AtomicBool::new(false));
    let ops = serve_ops(&config.server, ready.clone(), metrics);

    let pool = create_pool(&jobs_config.redis)
        .await
        .map_err(|e| ArcanaError::Internal(format!("Scheduler Redis pool: {e}")))?;

    let queue = Arc::new(RedisJobQueue::new(pool.clone(), jobs_config.clone()));
    let scheduler = Arc::new(Scheduler::new(
        pool,
        queue,
        jobs_config.scheduler.clone(),
    ));

    register_scheduled_jobs(&scheduler);

    let runner = {
        let scheduler = scheduler.clone();
        tokio::spawn(async move { scheduler.start().await })
    };

    ready.store(true, Ordering::SeqCst);
    crate::shutdown::wait_for_signal().await;

    info!("Releasing scheduler leadership...");
    ready.store(false, Ordering::SeqCst);
    scheduler.stop();
    drain(runner, Duration::from_secs(15), "scheduler").await;

    ops.abort();
    info!("Scheduler role shutdown complete");
    Ok(())
}

/// Registers the job handlers this deployment knows how to execute.
///
/// The registry is deliberately explicit: an unregistered job name is failed
/// rather than silently dropped, so an empty registry is a loud condition and
/// not a quiet one.
fn register_job_handlers(pool: &WorkerPool<RedisJobQueue>) {
    // No production `Job` implementations exist in this workspace yet.
    // Register them here as they are added.
    if pool.handler_count() == 0 {
        warn!(
            "Worker started with no registered job handlers; \
             every dequeued job will be failed as unknown"
        );
    }
}

/// Registers the cron entries this deployment owns.
fn register_scheduled_jobs(scheduler: &Scheduler<RedisJobQueue>) {
    if scheduler.list_jobs().is_empty() {
        warn!("Scheduler started with no registered cron jobs; it will only hold leadership");
    }
}

/// Installs the Prometheus recorder and describes the job metrics.
///
/// Also publishes `arcana_role_up`, so a scrape returns a series from the
/// first second. Without it the endpoint answers 200 with an empty body until
/// some job activity happens, which reads exactly like a broken exporter.
fn install_metrics(role: &'static str) -> ArcanaResult<PrometheusHandle> {
    let handle = PrometheusBuilder::new()
        .install_recorder()
        .map_err(|e| ArcanaError::Internal(format!("Failed to install metrics recorder: {e}")))?;
    arcana_jobs::register_metrics();
    metrics::describe_gauge!("arcana_role_up", "1 while this deployment role is running");
    metrics::gauge!("arcana_role_up", "role" => role).set(1.0);
    Ok(handle)
}

/// Starts the operations HTTP server on the configured REST port.
///
/// Returned handle is aborted on shutdown; the server is intentionally not
/// part of the readiness path, so probes keep answering while the role drains.
fn serve_ops(
    server_config: &ServerConfig,
    ready: Arc<AtomicBool>,
    metrics: PrometheusHandle,
) -> tokio::task::JoinHandle<()> {
    let addr = server_config.rest_addr();
    let state = OpsState { ready, metrics };

    let router = Router::new()
        .route("/health", get(health))
        .route("/live", get(health))
        .route("/ready", get(readiness))
        .route("/metrics", get(render_metrics))
        .with_state(state);

    tokio::spawn(async move {
        match tokio::net::TcpListener::bind(&addr).await {
            Ok(listener) => {
                info!("Operations endpoints listening on http://{addr}");
                if let Err(e) = axum::serve(listener, router).await {
                    warn!("Operations server stopped: {e}");
                }
            }
            Err(e) => warn!("Failed to bind operations server on {addr}: {e}"),
        }
    })
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

async fn readiness(State(state): State<OpsState>) -> impl IntoResponse {
    if state.ready.load(Ordering::SeqCst) {
        (StatusCode::OK, "ready")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "not ready")
    }
}

async fn render_metrics(State(state): State<OpsState>) -> impl IntoResponse {
    (StatusCode::OK, state.metrics.render())
}

/// Waits for a spawned role to finish, bounded by `timeout`.
async fn drain<E: std::fmt::Display>(
    runner: tokio::task::JoinHandle<Result<(), E>>,
    timeout: Duration,
    what: &str,
) {
    match tokio::time::timeout(timeout, runner).await {
        Ok(Ok(Ok(()))) => info!("{what} stopped cleanly"),
        Ok(Ok(Err(e))) => warn!("{what} stopped with error: {e}"),
        Ok(Err(e)) => warn!("{what} task panicked: {e}"),
        Err(_) => warn!("{what} did not stop within {timeout:?}; exiting anyway"),
    }
}

// =============================================================================
// Embedded job runtime (monolithic / single-container deployments)
// =============================================================================

/// Worker and scheduler running inside another process.
///
/// Monolithic and `layer = all` deployments are meant to be one container that
/// does everything, so they host the same job runtime in-process instead of
/// requiring the separate pods a layered deployment uses. The behaviour of a
/// job is identical either way -- only where it runs differs.
pub struct EmbeddedJobs {
    worker_pool: Arc<WorkerPool<RedisJobQueue>>,
    scheduler: Option<Arc<Scheduler<RedisJobQueue>>>,
    worker_task: tokio::task::JoinHandle<arcana_jobs::JobResult<()>>,
    scheduler_task: Option<tokio::task::JoinHandle<arcana_jobs::JobResult<()>>>,
    shutdown_timeout: Duration,
}

impl EmbeddedJobs {
    /// Stops both components and waits for them to drain.
    pub async fn shutdown(self) {
        info!("Stopping embedded job runtime...");
        self.worker_pool.stop();
        if let Some(scheduler) = &self.scheduler {
            scheduler.stop();
        }
        drain(self.worker_task, self.shutdown_timeout, "embedded worker pool").await;
        if let Some(task) = self.scheduler_task {
            drain(task, Duration::from_secs(15), "embedded scheduler").await;
        }
    }
}

/// Starts the embedded job runtime when `jobs.enabled` is set.
///
/// Returns `Ok(None)` when the subsystem is switched off, which is the default:
/// a deployment without Redis must still start.
pub async fn spawn_embedded(jobs_config: &JobsConfig) -> ArcanaResult<Option<EmbeddedJobs>> {
    if !jobs_config.enabled {
        info!("Embedded job runtime disabled (jobs.enabled = false)");
        return Ok(None);
    }

    info!("Starting embedded job runtime");

    let pool = create_pool(&jobs_config.redis)
        .await
        .map_err(|e| ArcanaError::Internal(format!("Job queue Redis pool: {e}")))?;

    let worker_config = WorkerPoolConfig::from(&jobs_config.worker);
    let shutdown_timeout = worker_config.shutdown_timeout;
    let queue = Arc::new(RedisJobQueue::new(pool.clone(), jobs_config.clone()));

    let worker_pool = Arc::new(WorkerPool::new(queue.clone(), worker_config));
    register_job_handlers(&worker_pool);
    let worker_task = {
        let worker_pool = worker_pool.clone();
        tokio::spawn(async move { worker_pool.start().await })
    };

    let (scheduler, scheduler_task) = if jobs_config.scheduler.enabled {
        let scheduler = Arc::new(Scheduler::new(pool, queue, jobs_config.scheduler.clone()));
        register_scheduled_jobs(&scheduler);
        let task = {
            let scheduler = scheduler.clone();
            tokio::spawn(async move { scheduler.start().await })
        };
        (Some(scheduler), Some(task))
    } else {
        info!("Embedded scheduler disabled (jobs.scheduler.enabled = false)");
        (None, None)
    };

    Ok(Some(EmbeddedJobs {
        worker_pool,
        scheduler,
        worker_task,
        scheduler_task,
        shutdown_timeout,
    }))
}
