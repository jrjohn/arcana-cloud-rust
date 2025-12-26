//! Application state for Axum handlers.

use arcana_service::{AuthService, UserService};
use std::sync::Arc;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub user_service: Arc<dyn UserService>,
    pub auth_service: Arc<dyn AuthService>,
}

impl AppState {
    /// Creates a new application state.
    pub fn new(
        user_service: Arc<dyn UserService>,
        auth_service: Arc<dyn AuthService>,
    ) -> Self {
        Self {
            user_service,
            auth_service,
        }
    }
}
