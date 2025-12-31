//! gRPC health service implementation.

use crate::proto::health::{
    health_check_response::ServingStatus,
    health_server::Health,
    HealthCheckRequest, HealthCheckResponse,
};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use tracing::debug;

/// Health service implementation.
#[derive(Debug, Default)]
pub struct HealthServiceImpl {
    // In a real implementation, you would track the status of various components
}

impl HealthServiceImpl {
    /// Creates a new health service.
    pub fn new() -> Self {
        Self {}
    }

    /// Gets the health status for a service.
    fn get_status(&self, service: &str) -> ServingStatus {
        debug!("Health check for service: {}", service);

        // In a real implementation, check actual service health
        match service {
            "" => ServingStatus::Serving, // Overall health
            "arcana.user.UserService" => ServingStatus::Serving,
            "arcana.auth.AuthService" => ServingStatus::Serving,
            _ => ServingStatus::ServiceUnknown,
        }
    }
}

#[tonic::async_trait]
impl Health for HealthServiceImpl {
    async fn check(
        &self,
        request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        let req = request.into_inner();
        let status = self.get_status(&req.service);

        Ok(Response::new(HealthCheckResponse {
            status: status.into(),
        }))
    }

    type WatchStream = ReceiverStream<Result<HealthCheckResponse, Status>>;

    async fn watch(
        &self,
        request: Request<HealthCheckRequest>,
    ) -> Result<Response<Self::WatchStream>, Status> {
        let req = request.into_inner();
        let status = self.get_status(&req.service);

        let (tx, rx) = tokio::sync::mpsc::channel(1);

        // Send initial status
        let _ = tx
            .send(Ok(HealthCheckResponse {
                status: status.into(),
            }))
            .await;

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
