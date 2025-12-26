//! Dependency injection module.
//!
//! This module defines the application's dependency injection container
//! providing centralized dependency management and service resolution.

use arcana_config::SecurityConfig;
use arcana_repository::{DatabasePool, MySqlUserRepository};
use arcana_security::{PasswordHasher, TokenProvider};
use arcana_service::{AuthService, AuthServiceImpl, UserService, UserServiceImpl};
use std::sync::Arc;

// ============================================================================
// Type Aliases for Concrete Service Types
// ============================================================================

/// Concrete user service type with MySQL repository.
pub type ConcreteUserService = UserServiceImpl<MySqlUserRepository>;

/// Concrete auth service type with MySQL repository.
pub type ConcreteAuthService = AuthServiceImpl<MySqlUserRepository>;

// ============================================================================
// Application Module - Centralized Dependency Container
// ============================================================================

/// Main application DI module.
///
/// This struct holds all application dependencies and provides
/// methods to resolve services with proper dependency injection.
pub struct AppModule {
    db_pool: Arc<DatabasePool>,
    security_config: Arc<SecurityConfig>,
    password_hasher: Arc<PasswordHasher>,
    token_provider: Arc<TokenProvider>,
    user_repository: Arc<MySqlUserRepository>,
    user_service: Arc<ConcreteUserService>,
    auth_service: Arc<ConcreteAuthService>,
}

impl AppModule {
    /// Returns the database pool.
    pub fn database_pool(&self) -> Arc<DatabasePool> {
        self.db_pool.clone()
    }

    /// Returns the security configuration.
    pub fn security_config(&self) -> Arc<SecurityConfig> {
        self.security_config.clone()
    }

    /// Returns the password hasher.
    pub fn password_hasher(&self) -> Arc<PasswordHasher> {
        self.password_hasher.clone()
    }

    /// Returns the token provider.
    pub fn token_provider(&self) -> Arc<TokenProvider> {
        self.token_provider.clone()
    }

    /// Returns the user repository.
    pub fn user_repository(&self) -> Arc<MySqlUserRepository> {
        self.user_repository.clone()
    }

    /// Returns the user service.
    pub fn user_service(&self) -> Arc<dyn UserService> {
        self.user_service.clone()
    }

    /// Returns the auth service.
    pub fn auth_service(&self) -> Arc<dyn AuthService> {
        self.auth_service.clone()
    }
}

// ============================================================================
// Module Builder
// ============================================================================

/// Builder for creating the application DI module.
pub struct AppModuleBuilder {
    db_pool: Option<Arc<DatabasePool>>,
    security_config: Option<SecurityConfig>,
    password_hash_cost: u32,
}

impl AppModuleBuilder {
    /// Creates a new module builder.
    pub fn new() -> Self {
        Self {
            db_pool: None,
            security_config: None,
            password_hash_cost: 12,
        }
    }

    /// Sets the database pool.
    pub fn with_database_pool(mut self, pool: Arc<DatabasePool>) -> Self {
        self.db_pool = Some(pool);
        self
    }

    /// Sets the security configuration.
    pub fn with_security_config(mut self, config: SecurityConfig) -> Self {
        self.security_config = Some(config);
        self
    }

    /// Sets the password hash cost.
    pub fn with_password_hash_cost(mut self, cost: u32) -> Self {
        self.password_hash_cost = cost;
        self
    }

    /// Builds the application module with all dependencies wired.
    ///
    /// # Panics
    ///
    /// Panics if database pool or security config are not set.
    pub fn build(self) -> Arc<AppModule> {
        let db_pool = self.db_pool.expect("Database pool is required");
        let security_config = Arc::new(
            self.security_config
                .expect("Security config is required"),
        );

        // Build infrastructure components
        let password_hasher = Arc::new(PasswordHasher::with_cost(self.password_hash_cost));
        let token_provider = Arc::new(TokenProvider::new(security_config.clone()));

        // Build repository layer
        let user_repository = Arc::new(MySqlUserRepository::new(db_pool.clone()));

        // Build service layer with injected dependencies
        let user_service = Arc::new(UserServiceImpl::new(
            user_repository.clone(),
            password_hasher.clone(),
        ));

        let auth_service = Arc::new(AuthServiceImpl::new(
            user_repository.clone(),
            password_hasher.clone(),
            security_config.clone(),
        ));

        Arc::new(AppModule {
            db_pool,
            security_config,
            password_hasher,
            token_provider,
            user_repository,
            user_service,
            auth_service,
        })
    }
}

impl Default for AppModuleBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_builder_creation() {
        let builder = AppModuleBuilder::new();
        assert!(builder.db_pool.is_none());
        assert!(builder.security_config.is_none());
    }

    #[test]
    fn test_password_hash_cost_default() {
        let builder = AppModuleBuilder::new();
        assert_eq!(builder.password_hash_cost, 12);
    }
}
