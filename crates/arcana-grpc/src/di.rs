//! Composition of the built-in gRPC service implementations.
//!
//! This is the only place in the crate that names the concrete `*Impl`
//! service types; server builders receive ready-to-mount tonic servers.

use arcana_proto::{health, jobs};
use crate::services::{HealthServiceImpl, JobQueueServiceImpl, WorkerServiceImpl};

/// Builds the gRPC health service.
#[must_use]
pub fn health_server() -> health::health_server::HealthServer<HealthServiceImpl> {
    health::health_server::HealthServer::new(HealthServiceImpl::new())
}

/// Builds the job queue gRPC service.
#[must_use]
pub fn job_queue_server(
) -> jobs::v1::job_queue_service_server::JobQueueServiceServer<JobQueueServiceImpl> {
    jobs::v1::job_queue_service_server::JobQueueServiceServer::new(JobQueueServiceImpl::new())
}

/// Builds the worker gRPC service.
#[must_use]
pub fn worker_server() -> jobs::v1::worker_service_server::WorkerServiceServer<WorkerServiceImpl> {
    jobs::v1::worker_service_server::WorkerServiceServer::new(WorkerServiceImpl::new())
}
