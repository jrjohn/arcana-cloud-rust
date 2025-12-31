//! gRPC Job Queue Service implementation.

use crate::proto::jobs::v1::{
    job_queue_service_server::JobQueueService,
    worker_service_server::WorkerService,
    CancelJobRequest, CancelJobResponse, CompleteRequest, CompleteResponse,
    DequeueRequest, DequeueResponse, EnqueueBatchRequest, EnqueueBatchResponse,
    EnqueueRequest, EnqueueResponse, EnqueueResult, FailRequest, FailResponse,
    GetJobRequest, GetJobResponse, GetQueueStatsRequest, GetQueueStatsResponse,
    HeartbeatRequest, HeartbeatResponse, Job as ProtoJob, JobEvent, JobInfo as ProtoJobInfo,
    JobStatus as ProtoJobStatus, Priority as ProtoPriority, QueueStats, RegisterWorkerRequest,
    RegisterWorkerResponse, RetryJobRequest, RetryJobResponse, WatchJobsRequest,
};
use arcana_jobs::{JobId, JobQueueInterface};
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::Stream;
use tonic::{Request, Response, Status};
use tracing::{debug, info};

/// Job Queue gRPC Service.
pub struct JobQueueServiceImpl {
    /// Job queue interface (optional).
    job_queue: Option<Arc<dyn JobQueueInterface>>,
}

impl JobQueueServiceImpl {
    /// Create a new job queue service without a backend.
    pub fn new() -> Self {
        Self { job_queue: None }
    }

    /// Create a new job queue service with a backend.
    pub fn with_queue(job_queue: Arc<dyn JobQueueInterface>) -> Self {
        Self {
            job_queue: Some(job_queue),
        }
    }

    /// Get the job queue, returning an error if not configured.
    fn require_queue(&self) -> Result<&Arc<dyn JobQueueInterface>, Status> {
        self.job_queue
            .as_ref()
            .ok_or_else(|| Status::unavailable("Job queue not configured"))
    }
}

impl Default for JobQueueServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[tonic::async_trait]
impl JobQueueService for JobQueueServiceImpl {
    async fn enqueue(
        &self,
        request: Request<EnqueueRequest>,
    ) -> Result<Response<EnqueueResponse>, Status> {
        let req = request.into_inner();
        info!(job_name = %req.name, queue = %req.queue, "Enqueuing job via gRPC");

        // For now, return stub - full implementation would require Job trait impl
        let job_id = uuid::Uuid::new_v4().to_string();
        let scheduled_at = chrono::Utc::now().to_rfc3339();

        Ok(Response::new(EnqueueResponse {
            job_id,
            scheduled_at,
        }))
    }

    async fn enqueue_batch(
        &self,
        request: Request<EnqueueBatchRequest>,
    ) -> Result<Response<EnqueueBatchResponse>, Status> {
        let req = request.into_inner();
        info!(count = req.jobs.len(), "Enqueuing batch of jobs via gRPC");

        let results: Vec<EnqueueResult> = req
            .jobs
            .into_iter()
            .map(|_job| {
                let job_id = uuid::Uuid::new_v4().to_string();
                EnqueueResult {
                    success: true,
                    job_id: Some(job_id),
                    error: None,
                }
            })
            .collect();

        Ok(Response::new(EnqueueBatchResponse { results }))
    }

    async fn get_job(
        &self,
        request: Request<GetJobRequest>,
    ) -> Result<Response<GetJobResponse>, Status> {
        let req = request.into_inner();
        debug!(job_id = %req.job_id, "Getting job via gRPC");

        let queue = self.require_queue()?;

        match queue.get_job(&req.job_id).await {
            Ok(Some(info)) => {
                let job = ProtoJob {
                    id: info.id.to_string(),
                    name: info.name.clone(),
                    queue: info.queue.clone(),
                    payload: String::new(), // Payload not stored in JobInfo
                    priority: priority_to_proto(info.priority),
                    attempt: info.attempt,
                    max_attempts: info.max_attempts,
                    timeout_secs: 300, // Default timeout
                    created_at: info.created_at.to_rfc3339(),
                    scheduled_at: info.scheduled_at.to_rfc3339(),
                    correlation_id: None,
                    tags: info.tags.clone(),
                };
                let job_info = ProtoJobInfo {
                    job: Some(job),
                    status: job_status_to_proto(&info.status.to_lowercase()),
                    started_at: info.started_at.map(|t| t.to_rfc3339()),
                    completed_at: info.completed_at.map(|t| t.to_rfc3339()),
                    last_error: info.last_error.clone(),
                    worker_id: info.worker_id.clone(),
                };
                Ok(Response::new(GetJobResponse { job: Some(job_info) }))
            }
            Ok(None) => Err(Status::not_found(format!("Job {} not found", req.job_id))),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn cancel_job(
        &self,
        request: Request<CancelJobRequest>,
    ) -> Result<Response<CancelJobResponse>, Status> {
        let req = request.into_inner();
        info!(job_id = %req.job_id, "Cancelling job via gRPC");

        let queue = self.require_queue()?;
        let job_id = JobId::from_string(&req.job_id);

        match queue.cancel_job(&job_id).await {
            Ok(()) => Ok(Response::new(CancelJobResponse { success: true })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn retry_job(
        &self,
        request: Request<RetryJobRequest>,
    ) -> Result<Response<RetryJobResponse>, Status> {
        let req = request.into_inner();
        info!(job_id = %req.job_id, "Retrying job via gRPC");

        let queue = self.require_queue()?;
        let job_id = JobId::from_string(&req.job_id);

        match queue.retry_job(&job_id).await {
            Ok(()) => Ok(Response::new(RetryJobResponse {
                job_id: job_id.to_string(),
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn get_queue_stats(
        &self,
        request: Request<GetQueueStatsRequest>,
    ) -> Result<Response<GetQueueStatsResponse>, Status> {
        let req = request.into_inner();
        debug!(queues = ?req.queues, "Getting queue stats via gRPC");

        let queue = self.require_queue()?;

        let stats_list = if req.queues.is_empty() {
            // Get all queues
            queue
                .get_all_queue_stats()
                .await
                .map_err(|e| Status::internal(e.to_string()))?
        } else {
            // Get specific queues
            let mut list = Vec::new();
            for queue_name in &req.queues {
                let stats = queue
                    .get_queue_stats(queue_name)
                    .await
                    .map_err(|e| Status::internal(e.to_string()))?;
                list.push(stats);
            }
            list
        };

        let proto_stats: Vec<QueueStats> = stats_list
            .iter()
            .map(|s| QueueStats {
                queue: s.queue.clone(),
                pending: s.pending,
                active: s.active,
                completed: s.completed,
                failed: s.failed,
                dead_letter: s.dead_letter,
                delayed: s.delayed,
            })
            .collect();

        // Calculate totals
        let total = QueueStats {
            queue: "total".to_string(),
            pending: stats_list.iter().map(|s| s.pending).sum(),
            active: stats_list.iter().map(|s| s.active).sum(),
            completed: stats_list.iter().map(|s| s.completed).sum(),
            failed: stats_list.iter().map(|s| s.failed).sum(),
            dead_letter: stats_list.iter().map(|s| s.dead_letter).sum(),
            delayed: stats_list.iter().map(|s| s.delayed).sum(),
        };

        Ok(Response::new(GetQueueStatsResponse {
            queues: proto_stats,
            total: Some(total),
        }))
    }

    type WatchJobsStream = Pin<Box<dyn Stream<Item = Result<JobEvent, Status>> + Send>>;

    async fn watch_jobs(
        &self,
        request: Request<WatchJobsRequest>,
    ) -> Result<Response<Self::WatchJobsStream>, Status> {
        let req = request.into_inner();
        info!(queues = ?req.queues, "Starting job watch stream via gRPC");

        // TODO: Implement actual job event streaming
        let stream = tokio_stream::empty();
        Ok(Response::new(Box::pin(stream)))
    }
}

/// Worker gRPC Service for job processing coordination.
pub struct WorkerServiceImpl {
    /// Job queue interface (optional).
    job_queue: Option<Arc<dyn JobQueueInterface>>,
}

impl WorkerServiceImpl {
    /// Create a new worker service without a backend.
    pub fn new() -> Self {
        Self { job_queue: None }
    }

    /// Create a new worker service with a backend.
    pub fn with_queue(job_queue: Arc<dyn JobQueueInterface>) -> Self {
        Self {
            job_queue: Some(job_queue),
        }
    }

    /// Get the job queue, returning an error if not configured.
    fn require_queue(&self) -> Result<&Arc<dyn JobQueueInterface>, Status> {
        self.job_queue
            .as_ref()
            .ok_or_else(|| Status::unavailable("Job queue not configured"))
    }
}

impl Default for WorkerServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[tonic::async_trait]
impl WorkerService for WorkerServiceImpl {
    async fn register_worker(
        &self,
        request: Request<RegisterWorkerRequest>,
    ) -> Result<Response<RegisterWorkerResponse>, Status> {
        let req = request.into_inner();
        info!(
            worker_id = %req.worker_id,
            queues = ?req.queues,
            concurrency = req.concurrency,
            "Worker registering via gRPC"
        );

        let queue = self.require_queue()?;
        let _seq = queue
            .worker_registry()
            .register(&req.worker_id, req.queues, req.concurrency);

        Ok(Response::new(RegisterWorkerResponse {
            success: true,
            heartbeat_interval_secs: 30,
        }))
    }

    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        let req = request.into_inner();
        debug!(
            worker_id = %req.worker_id,
            active_jobs = req.active_jobs,
            "Worker heartbeat via gRPC"
        );

        let queue = self.require_queue()?;
        let is_alive = queue
            .worker_registry()
            .heartbeat(&req.worker_id, req.active_jobs);

        Ok(Response::new(HeartbeatResponse {
            continue_processing: is_alive,
        }))
    }

    async fn dequeue(
        &self,
        request: Request<DequeueRequest>,
    ) -> Result<Response<DequeueResponse>, Status> {
        let req = request.into_inner();
        debug!(
            worker_id = %req.worker_id,
            queues = ?req.queues,
            max_jobs = req.max_jobs,
            "Worker dequeuing jobs via gRPC"
        );

        let queue = self.require_queue()?;
        let queues: Vec<&str> = req.queues.iter().map(|s| s.as_str()).collect();

        let job_data_list = queue
            .dequeue_for_worker(&queues, &req.worker_id, req.max_jobs)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let proto_jobs: Vec<ProtoJob> = job_data_list
            .into_iter()
            .map(|data| job_data_to_proto(&data))
            .collect();

        Ok(Response::new(DequeueResponse { jobs: proto_jobs }))
    }

    async fn complete(
        &self,
        request: Request<CompleteRequest>,
    ) -> Result<Response<CompleteResponse>, Status> {
        let req = request.into_inner();
        info!(
            worker_id = %req.worker_id,
            job_id = %req.job_id,
            "Job completed via gRPC"
        );

        let queue = self.require_queue()?;
        let job_id = JobId::from_string(&req.job_id);

        queue
            .complete_job(&job_id, req.result)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Record the job as processed
        queue.worker_registry().record_job_processed(&req.worker_id);

        Ok(Response::new(CompleteResponse { success: true }))
    }

    async fn fail(
        &self,
        request: Request<FailRequest>,
    ) -> Result<Response<FailResponse>, Status> {
        let req = request.into_inner();
        info!(
            worker_id = %req.worker_id,
            job_id = %req.job_id,
            error = %req.error,
            should_retry = req.should_retry,
            "Job failed via gRPC"
        );

        let queue = self.require_queue()?;
        let job_id = JobId::from_string(&req.job_id);

        let (retried, dead_lettered) = queue
            .fail_job(&job_id, &req.error, req.should_retry)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Record the job as failed
        queue.worker_registry().record_job_failed(&req.worker_id);

        Ok(Response::new(FailResponse {
            retried,
            dead_lettered,
        }))
    }
}

/// Convert JobData to proto Job.
fn job_data_to_proto(data: &arcana_jobs::JobData) -> ProtoJob {
    ProtoJob {
        id: data.id.to_string(),
        name: data.name.clone(),
        queue: data.queue.clone(),
        payload: data.payload.clone(),
        priority: priority_to_proto(data.priority),
        attempt: data.attempt,
        max_attempts: data.max_attempts,
        timeout_secs: data.timeout_secs,
        created_at: data.created_at.to_rfc3339(),
        scheduled_at: data.scheduled_at.to_rfc3339(),
        correlation_id: data.correlation_id.clone(),
        tags: data.tags.clone(),
    }
}

/// Convert Priority enum to proto.
pub fn priority_to_proto(priority: i8) -> i32 {
    match priority {
        p if p >= 20 => ProtoPriority::Critical as i32,
        p if p >= 10 => ProtoPriority::High as i32,
        p if p <= -10 => ProtoPriority::Low as i32,
        _ => ProtoPriority::Normal as i32,
    }
}

/// Convert proto Priority to i8.
pub fn proto_to_priority(proto: i32) -> i8 {
    match ProtoPriority::try_from(proto) {
        Ok(ProtoPriority::Critical) => 20,
        Ok(ProtoPriority::High) => 10,
        Ok(ProtoPriority::Low) => -10,
        _ => 0,
    }
}

/// Convert JobStatus enum to proto.
pub fn job_status_to_proto(status: &str) -> i32 {
    match status {
        "pending" => ProtoJobStatus::Pending as i32,
        "scheduled" => ProtoJobStatus::Scheduled as i32,
        "running" => ProtoJobStatus::Running as i32,
        "completed" => ProtoJobStatus::Completed as i32,
        "failed" => ProtoJobStatus::Failed as i32,
        "dead_letter" => ProtoJobStatus::DeadLetter as i32,
        "cancelled" => ProtoJobStatus::Cancelled as i32,
        _ => ProtoJobStatus::Unspecified as i32,
    }
}
