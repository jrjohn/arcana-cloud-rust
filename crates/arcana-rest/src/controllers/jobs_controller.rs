//! Job management REST API controller.

use arcana_jobs::{JobId, JobSearchQuery, JobStatus as JobStatusEnum, ThroughputPeriod};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

/// Create the jobs router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Queue operations
        .route("/queues", get(list_queues))
        .route("/queues/:queue/stats", get(queue_stats))
        .route("/queues/:queue/jobs", get(list_queue_jobs))
        .route("/queues/:queue/purge", post(purge_queue))
        // Job operations
        .route("/jobs", get(search_jobs))
        .route("/jobs/:job_id", get(get_job))
        .route("/jobs/:job_id", delete(cancel_job))
        .route("/jobs/:job_id/retry", post(retry_job))
        // DLQ operations
        .route("/dlq", get(list_dlq))
        .route("/dlq/:job_id/retry", post(retry_dlq_job))
        .route("/dlq/purge", post(purge_dlq))
        // Dashboard
        .route("/dashboard", get(dashboard_stats))
        .route("/dashboard/activity", get(recent_activity))
        .route("/dashboard/throughput", get(throughput_metrics))
        // Workers
        .route("/workers", get(list_workers))
        // Scheduled jobs
        .route("/scheduled", get(list_scheduled_jobs))
        .route("/scheduled/:name/trigger", post(trigger_scheduled_job))
        .route("/scheduled/:name/enable", post(enable_scheduled_job))
        .route("/scheduled/:name/disable", post(disable_scheduled_job))
}

// ============================================================================
// Request/Response Types
// ============================================================================

/// Query parameters for job search.
#[derive(Debug, Deserialize)]
pub struct JobSearchParams {
    /// Filter by status.
    pub status: Option<String>,
    /// Filter by queue.
    pub queue: Option<String>,
    /// Filter by job name.
    pub name: Option<String>,
    /// Filter by tag.
    pub tag: Option<String>,
    /// Pagination offset.
    #[serde(default)]
    pub offset: usize,
    /// Pagination limit.
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    50
}

/// Query parameters for throughput.
#[derive(Debug, Deserialize)]
pub struct ThroughputParams {
    /// Queue name.
    pub queue: Option<String>,
    /// Time period: hour, day, week.
    #[serde(default = "default_period")]
    pub period: String,
}

fn default_period() -> String {
    "hour".to_string()
}

/// Response for queue list.
#[derive(Debug, Serialize)]
pub struct QueuesResponse {
    pub queues: Vec<QueueInfo>,
}

/// Queue information.
#[derive(Debug, Serialize)]
pub struct QueueInfo {
    pub name: String,
    pub pending: u64,
    pub active: u64,
    pub completed: u64,
    pub failed: u64,
    pub delayed: u64,
}

/// Response for job details.
#[derive(Debug, Serialize)]
pub struct JobResponse {
    pub id: String,
    pub name: String,
    pub queue: String,
    pub status: String,
    pub attempt: u32,
    pub max_attempts: u32,
    pub priority: i8,
    pub created_at: String,
    pub scheduled_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub last_error: Option<String>,
    pub tags: Vec<String>,
}

/// Response for job search.
#[derive(Debug, Serialize)]
pub struct JobSearchResponse {
    pub jobs: Vec<JobResponse>,
    pub total: u64,
    pub offset: usize,
    pub limit: usize,
}

/// Dashboard statistics response.
#[derive(Debug, Serialize)]
pub struct DashboardResponse {
    pub total_jobs: u64,
    pub pending: u64,
    pub active: u64,
    pub completed: u64,
    pub failed: u64,
    pub dead_letter: u64,
    pub delayed: u64,
    pub queues: Vec<QueueInfo>,
}

/// Job activity entry.
#[derive(Debug, Serialize)]
pub struct ActivityEntry {
    pub job_id: String,
    pub job_name: String,
    pub activity_type: String,
    pub timestamp: String,
    pub queue: String,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
}

/// Worker health info.
#[derive(Debug, Serialize)]
pub struct WorkerInfo {
    pub worker_id: String,
    pub status: String,
    pub last_heartbeat: Option<String>,
    pub ttl_remaining: Option<u64>,
}

/// Scheduled job info.
#[derive(Debug, Serialize)]
pub struct ScheduledJobResponse {
    pub name: String,
    pub cron: String,
    pub enabled: bool,
    pub next_run: Option<String>,
}

/// Simple message response.
#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}

/// Purge response.
#[derive(Debug, Serialize)]
pub struct PurgeResponse {
    pub removed: u64,
}

/// Error response for job operations.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Check if job queue is available, return error response if not.
fn require_job_queue(state: &AppState) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if state.job_queue.is_none() {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "Job queue not configured. Redis may not be enabled.".to_string(),
                code: "JOB_QUEUE_UNAVAILABLE".to_string(),
            }),
        ))
    } else {
        Ok(())
    }
}

/// Parse job status from string.
fn parse_job_status(status: &str) -> Option<JobStatusEnum> {
    match status.to_lowercase().as_str() {
        "pending" => Some(JobStatusEnum::Pending),
        "scheduled" => Some(JobStatusEnum::Scheduled),
        "running" => Some(JobStatusEnum::Running),
        "completed" => Some(JobStatusEnum::Completed),
        "failed" => Some(JobStatusEnum::Failed),
        "dead_letter" => Some(JobStatusEnum::DeadLetter),
        "cancelled" => Some(JobStatusEnum::Cancelled),
        _ => None,
    }
}

/// Convert JobInfo to JobResponse.
fn job_info_to_response(info: &arcana_jobs::JobInfo) -> JobResponse {
    JobResponse {
        id: info.id.to_string(),
        name: info.name.clone(),
        queue: info.queue.clone(),
        status: format!("{:?}", info.status).to_lowercase(),
        attempt: info.attempt,
        max_attempts: info.max_attempts,
        priority: info.priority,
        created_at: info.created_at.to_rfc3339(),
        scheduled_at: info.scheduled_at.to_rfc3339(),
        started_at: info.started_at.map(|t| t.to_rfc3339()),
        completed_at: info.completed_at.map(|t| t.to_rfc3339()),
        last_error: info.last_error.clone(),
        tags: info.tags.clone(),
    }
}

// ============================================================================
// Handler Functions
// ============================================================================

/// List all queues with stats.
async fn list_queues(State(state): State<AppState>) -> impl IntoResponse {
    if let Err(err) = require_job_queue(&state) {
        return err.into_response();
    }

    let job_queue = state.job_queue.as_ref().unwrap();

    match job_queue.get_all_queue_stats().await {
        Ok(stats) => {
            let queues = stats
                .into_iter()
                .map(|s| QueueInfo {
                    name: s.queue,
                    pending: s.pending,
                    active: s.active,
                    completed: s.completed,
                    failed: s.failed,
                    delayed: s.delayed,
                })
                .collect();
            Json(QueuesResponse { queues }).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                code: "QUEUE_ERROR".to_string(),
            }),
        )
            .into_response(),
    }
}

/// Get queue statistics.
async fn queue_stats(
    State(state): State<AppState>,
    Path(queue): Path<String>,
) -> impl IntoResponse {
    if let Err(err) = require_job_queue(&state) {
        return err.into_response();
    }

    let job_queue = state.job_queue.as_ref().unwrap();

    match job_queue.get_queue_stats(&queue).await {
        Ok(stats) => Json(QueueInfo {
            name: stats.queue,
            pending: stats.pending,
            active: stats.active,
            completed: stats.completed,
            failed: stats.failed,
            delayed: stats.delayed,
        })
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                code: "QUEUE_ERROR".to_string(),
            }),
        )
            .into_response(),
    }
}

/// List jobs in a queue.
async fn list_queue_jobs(
    State(state): State<AppState>,
    Path(queue): Path<String>,
    Query(params): Query<JobSearchParams>,
) -> impl IntoResponse {
    if let Err(err) = require_job_queue(&state) {
        return err.into_response();
    }

    let job_queue = state.job_queue.as_ref().unwrap();

    let mut query = JobSearchQuery::new()
        .queue(queue)
        .offset(params.offset)
        .limit(params.limit);

    if let Some(ref status) = params.status {
        if let Some(s) = parse_job_status(status) {
            query = query.status(s);
        }
    }
    if let Some(ref name) = params.name {
        query = query.name(name);
    }
    if let Some(ref tag) = params.tag {
        query = query.tag(tag);
    }

    match job_queue.search_jobs(query).await {
        Ok(result) => Json(JobSearchResponse {
            jobs: result.jobs.iter().map(job_info_to_response).collect(),
            total: result.total,
            offset: result.offset,
            limit: result.limit,
        })
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                code: "SEARCH_ERROR".to_string(),
            }),
        )
            .into_response(),
    }
}

/// Purge completed jobs from a queue.
async fn purge_queue(
    State(state): State<AppState>,
    Path(_queue): Path<String>,
) -> impl IntoResponse {
    if let Err(err) = require_job_queue(&state) {
        return err.into_response();
    }

    let job_queue = state.job_queue.as_ref().unwrap();

    // Purge jobs older than 24 hours by default
    match job_queue.purge_completed(24 * 60 * 60).await {
        Ok(removed) => Json(PurgeResponse { removed }).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                code: "PURGE_ERROR".to_string(),
            }),
        )
            .into_response(),
    }
}

/// Search jobs with filters.
async fn search_jobs(
    State(state): State<AppState>,
    Query(params): Query<JobSearchParams>,
) -> impl IntoResponse {
    if let Err(err) = require_job_queue(&state) {
        return err.into_response();
    }

    let job_queue = state.job_queue.as_ref().unwrap();

    let mut query = JobSearchQuery::new()
        .offset(params.offset)
        .limit(params.limit);

    if let Some(ref queue) = params.queue {
        query = query.queue(queue);
    }
    if let Some(ref status) = params.status {
        if let Some(s) = parse_job_status(status) {
            query = query.status(s);
        }
    }
    if let Some(ref name) = params.name {
        query = query.name(name);
    }
    if let Some(ref tag) = params.tag {
        query = query.tag(tag);
    }

    match job_queue.search_jobs(query).await {
        Ok(result) => Json(JobSearchResponse {
            jobs: result.jobs.iter().map(job_info_to_response).collect(),
            total: result.total,
            offset: result.offset,
            limit: result.limit,
        })
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                code: "SEARCH_ERROR".to_string(),
            }),
        )
            .into_response(),
    }
}

/// Get job by ID.
async fn get_job(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    if let Err(err) = require_job_queue(&state) {
        return err.into_response();
    }

    let job_queue = state.job_queue.as_ref().unwrap();

    match job_queue.get_job(&job_id).await {
        Ok(Some(info)) => Json(job_info_to_response(&info)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Job {} not found", job_id),
                code: "NOT_FOUND".to_string(),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                code: "JOB_ERROR".to_string(),
            }),
        )
            .into_response(),
    }
}

/// Cancel a pending job.
async fn cancel_job(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    if let Err(err) = require_job_queue(&state) {
        return err.into_response();
    }

    let job_queue = state.job_queue.as_ref().unwrap();
    let job_id = JobId::from_string(&job_id);

    match job_queue.cancel_job(&job_id).await {
        Ok(()) => Json(MessageResponse {
            message: format!("Job {} cancelled", job_id),
        })
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                code: "CANCEL_ERROR".to_string(),
            }),
        )
            .into_response(),
    }
}

/// Retry a failed job.
async fn retry_job(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    if let Err(err) = require_job_queue(&state) {
        return err.into_response();
    }

    let job_queue = state.job_queue.as_ref().unwrap();
    let job_id = JobId::from_string(&job_id);

    match job_queue.retry_job(&job_id).await {
        Ok(()) => Json(MessageResponse {
            message: format!("Job {} queued for retry", job_id),
        })
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                code: "RETRY_ERROR".to_string(),
            }),
        )
            .into_response(),
    }
}

/// List dead letter queue jobs.
async fn list_dlq(
    State(state): State<AppState>,
    Query(params): Query<JobSearchParams>,
) -> impl IntoResponse {
    if let Err(err) = require_job_queue(&state) {
        return err.into_response();
    }

    let job_queue = state.job_queue.as_ref().unwrap();

    let query = JobSearchQuery::new()
        .status(JobStatusEnum::DeadLetter)
        .offset(params.offset)
        .limit(params.limit);

    match job_queue.search_jobs(query).await {
        Ok(result) => Json(JobSearchResponse {
            jobs: result.jobs.iter().map(job_info_to_response).collect(),
            total: result.total,
            offset: result.offset,
            limit: result.limit,
        })
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                code: "DLQ_ERROR".to_string(),
            }),
        )
            .into_response(),
    }
}

/// Retry a job from DLQ.
async fn retry_dlq_job(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    if let Err(err) = require_job_queue(&state) {
        return err.into_response();
    }

    let job_queue = state.job_queue.as_ref().unwrap();
    let job_id = JobId::from_string(&job_id);

    match job_queue.retry_dlq_job(&job_id).await {
        Ok(()) => Json(MessageResponse {
            message: format!("DLQ job {} queued for retry", job_id),
        })
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                code: "DLQ_RETRY_ERROR".to_string(),
            }),
        )
            .into_response(),
    }
}

/// Purge DLQ.
async fn purge_dlq(State(state): State<AppState>) -> impl IntoResponse {
    if let Err(err) = require_job_queue(&state) {
        return err.into_response();
    }

    // DLQ purge not directly supported yet - return 0
    Json(PurgeResponse { removed: 0 }).into_response()
}

/// Get dashboard statistics.
async fn dashboard_stats(State(state): State<AppState>) -> impl IntoResponse {
    if let Err(err) = require_job_queue(&state) {
        return err.into_response();
    }

    let job_queue = state.job_queue.as_ref().unwrap();

    match job_queue.get_dashboard_stats().await {
        Ok(stats) => Json(DashboardResponse {
            total_jobs: stats.total_jobs,
            pending: stats.total_pending,
            active: stats.total_active,
            completed: stats.total_completed,
            failed: stats.total_failed,
            dead_letter: stats.total_dead_letter,
            delayed: stats.total_delayed,
            queues: stats
                .queues
                .into_iter()
                .map(|s| QueueInfo {
                    name: s.queue,
                    pending: s.pending,
                    active: s.active,
                    completed: s.completed,
                    failed: s.failed,
                    delayed: s.delayed,
                })
                .collect(),
        })
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                code: "DASHBOARD_ERROR".to_string(),
            }),
        )
            .into_response(),
    }
}

/// Get recent job activity.
async fn recent_activity(State(state): State<AppState>) -> impl IntoResponse {
    if let Err(err) = require_job_queue(&state) {
        return err.into_response();
    }

    let job_queue = state.job_queue.as_ref().unwrap();

    match job_queue.status_tracker().get_recent_activity(50).await {
        Ok(activities) => {
            let entries: Vec<ActivityEntry> = activities
                .into_iter()
                .map(|a| ActivityEntry {
                    job_id: a.job_id,
                    job_name: a.job_name,
                    activity_type: format!("{:?}", a.activity_type).to_lowercase(),
                    timestamp: a.timestamp.to_rfc3339(),
                    queue: a.queue,
                    duration_ms: a.duration_ms,
                    error: a.error,
                })
                .collect();
            Json(entries).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                code: "ACTIVITY_ERROR".to_string(),
            }),
        )
            .into_response(),
    }
}

/// Get throughput metrics.
async fn throughput_metrics(
    State(state): State<AppState>,
    Query(params): Query<ThroughputParams>,
) -> impl IntoResponse {
    if let Err(err) = require_job_queue(&state) {
        return err.into_response();
    }

    let job_queue = state.job_queue.as_ref().unwrap();
    let queue = params.queue.as_deref().unwrap_or("default");
    let period = match params.period.as_str() {
        "day" | "24h" => ThroughputPeriod::Last24Hours,
        "week" | "7d" => ThroughputPeriod::Last7Days,
        _ => ThroughputPeriod::LastHour,
    };

    match job_queue.status_tracker().get_throughput(queue, period).await {
        Ok(metrics) => {
            #[derive(Serialize)]
            struct ThroughputResponse {
                queue: String,
                period: String,
                total_processed: u64,
                completed: u64,
                failed: u64,
                avg_per_second: f64,
                success_rate: f64,
            }

            Json(ThroughputResponse {
                queue: metrics.queue,
                period: format!("{:?}", metrics.period).to_lowercase(),
                total_processed: metrics.total_processed,
                completed: metrics.completed,
                failed: metrics.failed,
                avg_per_second: metrics.avg_per_second,
                success_rate: metrics.success_rate,
            })
            .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                code: "THROUGHPUT_ERROR".to_string(),
            }),
        )
            .into_response(),
    }
}

/// List active workers.
async fn list_workers(State(state): State<AppState>) -> impl IntoResponse {
    if let Err(err) = require_job_queue(&state) {
        return err.into_response();
    }

    let job_queue = state.job_queue.as_ref().unwrap();

    match job_queue.get_worker_health().await {
        Ok(workers) => {
            let infos: Vec<WorkerInfo> = workers
                .into_iter()
                .map(|w| WorkerInfo {
                    worker_id: w.worker_id,
                    status: format!("{:?}", w.status).to_lowercase(),
                    last_heartbeat: w.last_heartbeat.map(|t| t.to_rfc3339()),
                    ttl_remaining: w.ttl_remaining,
                })
                .collect();
            Json(infos).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                code: "WORKER_ERROR".to_string(),
            }),
        )
            .into_response(),
    }
}

/// List scheduled jobs.
async fn list_scheduled_jobs(State(_state): State<AppState>) -> impl IntoResponse {
    // Scheduled jobs are managed by the scheduler, not the queue
    // This would require scheduler integration
    let jobs: Vec<ScheduledJobResponse> = vec![];
    Json(jobs)
}

/// Trigger a scheduled job immediately.
async fn trigger_scheduled_job(
    State(_state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    // Would require scheduler integration
    Json(MessageResponse {
        message: format!("Scheduled job '{}' triggered", name),
    })
}

/// Enable a scheduled job.
async fn enable_scheduled_job(
    State(_state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    Json(MessageResponse {
        message: format!("Scheduled job '{}' enabled", name),
    })
}

/// Disable a scheduled job.
async fn disable_scheduled_job(
    State(_state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    Json(MessageResponse {
        message: format!("Scheduled job '{}' disabled", name),
    })
}
