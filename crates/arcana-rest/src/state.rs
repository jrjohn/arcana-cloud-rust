//! Application state for Axum handlers.
//!
//! This module provides the application state used by Axum handlers.
//! Services are resolved from a Shaku module and stored in the state.

use arcana_jobs::JobQueueInterface;
use arcana_service::{AuthService, UserService};
use shaku::{HasComponent, Module};
use std::sync::Arc;

/// Shared application state holding resolved services.
///
/// This state is created by resolving services from a Shaku module,
/// then passed to Axum handlers via the State extractor.
#[derive(Clone)]
pub struct AppState {
    /// User management service.
    pub user_service: Arc<dyn UserService>,
    /// Authentication service.
    pub auth_service: Arc<dyn AuthService>,
    /// Job queue interface (optional, only available when Redis is configured).
    pub job_queue: Option<Arc<dyn JobQueueInterface>>,
}

impl AppState {
    /// Creates a new application state with the given services.
    pub fn new(
        user_service: Arc<dyn UserService>,
        auth_service: Arc<dyn AuthService>,
    ) -> Self {
        Self {
            user_service,
            auth_service,
            job_queue: None,
        }
    }

    /// Creates a new application state with job queue support.
    pub fn with_jobs(
        user_service: Arc<dyn UserService>,
        auth_service: Arc<dyn AuthService>,
        job_queue: Arc<dyn JobQueueInterface>,
    ) -> Self {
        Self {
            user_service,
            auth_service,
            job_queue: Some(job_queue),
        }
    }

    /// Creates application state by resolving services from a Shaku module.
    ///
    /// This is the preferred way to create AppState, as it ensures
    /// services are properly wired through dependency injection.
    pub fn from_module<M>(module: &M) -> Self
    where
        M: Module + HasComponent<dyn UserService> + HasComponent<dyn AuthService>,
    {
        Self {
            user_service: module.resolve(),
            auth_service: module.resolve(),
            job_queue: None,
        }
    }

    /// Creates application state with job queue from a Shaku module.
    pub fn from_module_with_jobs<M>(module: &M, job_queue: Arc<dyn JobQueueInterface>) -> Self
    where
        M: Module + HasComponent<dyn UserService> + HasComponent<dyn AuthService>,
    {
        Self {
            user_service: module.resolve(),
            auth_service: module.resolve(),
            job_queue: Some(job_queue),
        }
    }
}
