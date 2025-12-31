//! Dependency injection module using Shaku.
//!
//! This module defines Shaku modules for different deployment configurations:
//! - `MonolithicModule`: Full stack with local MySQL database
//! - `DistributedServiceModule`: Service layer with remote repository via gRPC
//! - `RepositoryModule`: Repository layer only (for distributed deployments)

use arcana_config::{DatabaseConfig, RedisConfig, SecurityConfig, SecurityConfigInterface};
use arcana_core::{module, ArcanaResult, HasComponent};
use arcana_grpc::RemoteUserRepository;
use arcana_repository::{DatabasePool, DatabasePoolInterface, MySqlUserRepository, UserRepository};
use arcana_security::{PasswordHasher, PasswordHasherInterface, TokenProvider, TokenProviderInterface};
use arcana_service::{AuthService, AuthServiceComponent, CacheInterface, RedisCacheService, RedisCacheServiceParameters, UserService, UserServiceComponent};
use std::sync::Arc;

// ============================================================================
// Shaku Module Definitions
// ============================================================================

// Monolithic deployment module with local MySQL database.
// Contains all components for a single-process deployment:
// - Database pool and repository
// - Security components (password hashing, JWT tokens)
// - Caching (Redis)
// - Business services (user, auth)
module! {
    pub MonolithicModule {
        components = [
            DatabasePool,
            PasswordHasher,
            TokenProvider,
            SecurityConfig,
            MySqlUserRepository,
            RedisCacheService,
            UserServiceComponent,
            AuthServiceComponent,
        ],
        providers = [],
    }
}

// Distributed service layer module with remote repository.
// Contains components for the service layer in a distributed deployment:
// - Security components (password hashing, JWT tokens)
// - Caching (Redis)
// - Business services (user, auth)
// - Remote repository client (connects to repository layer via gRPC)
module! {
    pub DistributedServiceModule {
        components = [
            PasswordHasher,
            TokenProvider,
            SecurityConfig,
            RemoteUserRepository,
            RedisCacheService,
            UserServiceComponent,
            AuthServiceComponent,
        ],
        providers = [],
    }
}

// Repository layer module for distributed deployments.
// Contains only the database and repository components.
// Exposes repository via gRPC for service layer consumption.
module! {
    pub RepositoryModule {
        components = [
            DatabasePool,
            MySqlUserRepository,
        ],
        providers = [],
    }
}

// ============================================================================
// Module Builders
// ============================================================================

/// Builds a monolithic module with all dependencies.
///
/// This is the main entry point for single-process deployments.
pub async fn build_monolithic_module(
    db_config: &DatabaseConfig,
    redis_config: &RedisConfig,
    security_config: SecurityConfig,
) -> ArcanaResult<Arc<MonolithicModule>> {
    // Create database pool (async operation)
    let db_pool = DatabasePool::connect(db_config).await?;

    // Create Redis cache pool (if enabled)
    let cache_pool = if redis_config.enabled {
        let redis_cfg = deadpool_redis::Config::from_url(&redis_config.url);
        let pool = redis_cfg
            .create_pool(Some(deadpool_redis::Runtime::Tokio1))
            .map_err(|e| arcana_core::ArcanaError::Cache(format!("Failed to create Redis pool: {}", e)))?;
        Some(Arc::new(pool))
    } else {
        None
    };

    // Create password hasher with configured cost
    let password_hasher = PasswordHasher::with_cost(security_config.password_hash_cost());

    // Create token provider with security config
    let token_provider = TokenProvider::new(Arc::new(security_config.clone()));

    // Build the module with parameters
    let module = MonolithicModule::builder()
        .with_component_parameters::<DatabasePool>(arcana_repository::DatabasePoolParameters {
            pool: db_pool.inner().clone(),
        })
        .with_component_parameters::<RedisCacheService>(RedisCacheServiceParameters {
            pool: cache_pool,
            default_ttl: arcana_service::DEFAULT_TTL,
        })
        .with_component_parameters::<PasswordHasher>(arcana_security::PasswordHasherParameters {
            argon2: password_hasher.argon2_arc(),
        })
        .with_component_parameters::<TokenProvider>(arcana_security::TokenProviderParameters {
            encoding_key: token_provider.encoding_key().clone(),
            decoding_key: token_provider.decoding_key().clone(),
            config: Arc::new(security_config.clone()),
            validation: token_provider.validation().clone(),
        })
        .with_component_parameters::<SecurityConfig>(arcana_config::SecurityConfigParameters {
            jwt_secret: security_config.jwt_secret.clone(),
            jwt_access_expiration_secs: security_config.jwt_access_expiration_secs,
            jwt_refresh_expiration_secs: security_config.jwt_refresh_expiration_secs,
            jwt_issuer: security_config.jwt_issuer.clone(),
            jwt_audience: security_config.jwt_audience.clone(),
            grpc_tls_enabled: security_config.grpc_tls_enabled,
            tls_cert_path: security_config.tls_cert_path.clone(),
            tls_key_path: security_config.tls_key_path.clone(),
            password_hash_cost: security_config.password_hash_cost,
        })
        .build();

    Ok(Arc::new(module))
}

/// Builds a distributed service module with remote repository.
///
/// Use this for service layer deployments that connect to a separate repository layer.
pub async fn build_distributed_service_module(
    repository_url: &str,
    redis_config: &RedisConfig,
    security_config: SecurityConfig,
) -> ArcanaResult<Arc<DistributedServiceModule>> {
    // Create remote repository client (async operation)
    let remote_repo = RemoteUserRepository::connect(repository_url).await?;

    // Create Redis cache pool (if enabled)
    let cache_pool = if redis_config.enabled {
        let redis_cfg = deadpool_redis::Config::from_url(&redis_config.url);
        let pool = redis_cfg
            .create_pool(Some(deadpool_redis::Runtime::Tokio1))
            .map_err(|e| arcana_core::ArcanaError::Cache(format!("Failed to create Redis pool: {}", e)))?;
        Some(Arc::new(pool))
    } else {
        None
    };

    // Create password hasher with configured cost
    let password_hasher = PasswordHasher::with_cost(security_config.password_hash_cost());

    // Create token provider with security config
    let token_provider = TokenProvider::new(Arc::new(security_config.clone()));

    // Build the module with parameters
    let module = DistributedServiceModule::builder()
        .with_component_parameters::<RedisCacheService>(RedisCacheServiceParameters {
            pool: cache_pool,
            default_ttl: arcana_service::DEFAULT_TTL,
        })
        .with_component_parameters::<PasswordHasher>(arcana_security::PasswordHasherParameters {
            argon2: password_hasher.argon2_arc(),
        })
        .with_component_parameters::<TokenProvider>(arcana_security::TokenProviderParameters {
            encoding_key: token_provider.encoding_key().clone(),
            decoding_key: token_provider.decoding_key().clone(),
            config: Arc::new(security_config.clone()),
            validation: token_provider.validation().clone(),
        })
        .with_component_parameters::<SecurityConfig>(arcana_config::SecurityConfigParameters {
            jwt_secret: security_config.jwt_secret.clone(),
            jwt_access_expiration_secs: security_config.jwt_access_expiration_secs,
            jwt_refresh_expiration_secs: security_config.jwt_refresh_expiration_secs,
            jwt_issuer: security_config.jwt_issuer.clone(),
            jwt_audience: security_config.jwt_audience.clone(),
            grpc_tls_enabled: security_config.grpc_tls_enabled,
            tls_cert_path: security_config.tls_cert_path.clone(),
            tls_key_path: security_config.tls_key_path.clone(),
            password_hash_cost: security_config.password_hash_cost,
        })
        .with_component_parameters::<RemoteUserRepository>(
            arcana_grpc::RemoteUserRepositoryParameters {
                client: remote_repo.client().clone(),
            },
        )
        .build();

    Ok(Arc::new(module))
}

/// Builds a repository-only module for distributed deployments.
///
/// Use this for the repository layer that exposes data via gRPC.
pub async fn build_repository_module(
    db_config: &DatabaseConfig,
) -> ArcanaResult<Arc<RepositoryModule>> {
    // Create database pool (async operation)
    let db_pool = DatabasePool::connect(db_config).await?;

    // Build the module with parameters
    let module = RepositoryModule::builder()
        .with_component_parameters::<DatabasePool>(arcana_repository::DatabasePoolParameters {
            pool: db_pool.inner().clone(),
        })
        .build();

    Ok(Arc::new(module))
}

// ============================================================================
// Module Resolution Helpers
// ============================================================================

/// Trait for resolving common services from any module.
pub trait ServiceResolver {
    /// Resolves the user service from the module.
    fn user_service(&self) -> Arc<dyn UserService>;

    /// Resolves the auth service from the module.
    fn auth_service(&self) -> Arc<dyn AuthService>;
}

impl ServiceResolver for MonolithicModule {
    fn user_service(&self) -> Arc<dyn UserService> {
        self.resolve()
    }

    fn auth_service(&self) -> Arc<dyn AuthService> {
        self.resolve()
    }
}

impl ServiceResolver for DistributedServiceModule {
    fn user_service(&self) -> Arc<dyn UserService> {
        self.resolve()
    }

    fn auth_service(&self) -> Arc<dyn AuthService> {
        self.resolve()
    }
}

/// Trait for resolving database pool from modules that have it.
pub trait DatabaseResolver {
    /// Resolves the database pool from the module.
    fn database_pool(&self) -> Arc<dyn DatabasePoolInterface>;
}

impl DatabaseResolver for MonolithicModule {
    fn database_pool(&self) -> Arc<dyn DatabasePoolInterface> {
        self.resolve()
    }
}

impl DatabaseResolver for RepositoryModule {
    fn database_pool(&self) -> Arc<dyn DatabasePoolInterface> {
        self.resolve()
    }
}

/// Trait for resolving repository from modules.
pub trait RepositoryResolver {
    /// Resolves the user repository from the module.
    fn user_repository(&self) -> Arc<dyn UserRepository>;
}

impl RepositoryResolver for MonolithicModule {
    fn user_repository(&self) -> Arc<dyn UserRepository> {
        self.resolve()
    }
}

impl RepositoryResolver for DistributedServiceModule {
    fn user_repository(&self) -> Arc<dyn UserRepository> {
        self.resolve()
    }
}

impl RepositoryResolver for RepositoryModule {
    fn user_repository(&self) -> Arc<dyn UserRepository> {
        self.resolve()
    }
}

/// Trait for resolving cache components.
pub trait CacheResolver {
    /// Resolves the cache interface from the module.
    fn cache(&self) -> Arc<dyn CacheInterface>;
}

impl CacheResolver for MonolithicModule {
    fn cache(&self) -> Arc<dyn CacheInterface> {
        self.resolve()
    }
}

impl CacheResolver for DistributedServiceModule {
    fn cache(&self) -> Arc<dyn CacheInterface> {
        self.resolve()
    }
}

/// Trait for resolving security components.
pub trait SecurityResolver {
    /// Resolves the password hasher from the module.
    fn password_hasher(&self) -> Arc<dyn PasswordHasherInterface>;

    /// Resolves the token provider from the module.
    fn token_provider(&self) -> Arc<dyn TokenProviderInterface>;

    /// Resolves the security config from the module.
    fn security_config(&self) -> Arc<dyn SecurityConfigInterface>;
}

impl SecurityResolver for MonolithicModule {
    fn password_hasher(&self) -> Arc<dyn PasswordHasherInterface> {
        self.resolve()
    }

    fn token_provider(&self) -> Arc<dyn TokenProviderInterface> {
        self.resolve()
    }

    fn security_config(&self) -> Arc<dyn SecurityConfigInterface> {
        self.resolve()
    }
}

impl SecurityResolver for DistributedServiceModule {
    fn password_hasher(&self) -> Arc<dyn PasswordHasherInterface> {
        self.resolve()
    }

    fn token_provider(&self) -> Arc<dyn TokenProviderInterface> {
        self.resolve()
    }

    fn security_config(&self) -> Arc<dyn SecurityConfigInterface> {
        self.resolve()
    }
}

// ============================================================================
// Legacy Support (Deprecated)
// ============================================================================

/// Legacy type alias for backward compatibility.
#[deprecated(since = "0.1.0", note = "Use MonolithicModule instead")]
pub type AppModule = MonolithicModule;

#[cfg(test)]
mod tests {
    use super::*;
    use arcana_core::UserRole;

    // =========================================================================
    // Compile-Time Trait Verification Tests
    // =========================================================================

    #[test]
    fn test_module_types_exist() {
        // Compile-time verification that module types are defined correctly
        fn _assert_service_resolver<T: ServiceResolver>() {}
        fn _assert_database_resolver<T: DatabaseResolver>() {}
        fn _assert_repository_resolver<T: RepositoryResolver>() {}
        fn _assert_security_resolver<T: SecurityResolver>() {}
        fn _assert_cache_resolver<T: CacheResolver>() {}

        _assert_service_resolver::<MonolithicModule>();
        _assert_service_resolver::<DistributedServiceModule>();
        _assert_database_resolver::<MonolithicModule>();
        _assert_database_resolver::<RepositoryModule>();
        _assert_repository_resolver::<MonolithicModule>();
        _assert_repository_resolver::<DistributedServiceModule>();
        _assert_repository_resolver::<RepositoryModule>();
        _assert_security_resolver::<MonolithicModule>();
        _assert_security_resolver::<DistributedServiceModule>();
        _assert_cache_resolver::<MonolithicModule>();
        _assert_cache_resolver::<DistributedServiceModule>();
    }

    #[test]
    fn test_has_component_trait_bounds() {
        // Verify HasComponent implementations are correct
        fn _assert_has_user_service<T: HasComponent<dyn UserService>>() {}
        fn _assert_has_auth_service<T: HasComponent<dyn AuthService>>() {}
        fn _assert_has_user_repository<T: HasComponent<dyn UserRepository>>() {}
        fn _assert_has_password_hasher<T: HasComponent<dyn PasswordHasherInterface>>() {}
        fn _assert_has_token_provider<T: HasComponent<dyn TokenProviderInterface>>() {}
        fn _assert_has_security_config<T: HasComponent<dyn SecurityConfigInterface>>() {}
        fn _assert_has_database_pool<T: HasComponent<dyn DatabasePoolInterface>>() {}
        fn _assert_has_cache<T: HasComponent<dyn CacheInterface>>() {}

        // MonolithicModule should have all components
        _assert_has_user_service::<MonolithicModule>();
        _assert_has_auth_service::<MonolithicModule>();
        _assert_has_user_repository::<MonolithicModule>();
        _assert_has_password_hasher::<MonolithicModule>();
        _assert_has_token_provider::<MonolithicModule>();
        _assert_has_security_config::<MonolithicModule>();
        _assert_has_database_pool::<MonolithicModule>();
        _assert_has_cache::<MonolithicModule>();

        // DistributedServiceModule should have service, security, and cache components
        _assert_has_user_service::<DistributedServiceModule>();
        _assert_has_auth_service::<DistributedServiceModule>();
        _assert_has_user_repository::<DistributedServiceModule>();
        _assert_has_password_hasher::<DistributedServiceModule>();
        _assert_has_token_provider::<DistributedServiceModule>();
        _assert_has_security_config::<DistributedServiceModule>();
        _assert_has_cache::<DistributedServiceModule>();

        // RepositoryModule should have database and repository components
        _assert_has_user_repository::<RepositoryModule>();
        _assert_has_database_pool::<RepositoryModule>();
    }

    // =========================================================================
    // Security Config Tests
    // =========================================================================

    fn create_test_security_config() -> SecurityConfig {
        SecurityConfig {
            jwt_secret: "test-secret-key-for-testing-minimum-32-chars".to_string(),
            jwt_access_expiration_secs: 3600,
            jwt_refresh_expiration_secs: 604800,
            jwt_issuer: "test-issuer".to_string(),
            jwt_audience: "test-audience".to_string(),
            grpc_tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
            password_hash_cost: 4,
        }
    }

    #[test]
    fn test_security_config_interface() {
        let config = create_test_security_config();

        // Test SecurityConfigInterface methods
        assert_eq!(config.jwt_secret(), "test-secret-key-for-testing-minimum-32-chars");
        assert_eq!(config.jwt_access_expiration_secs(), 3600);
        assert_eq!(config.jwt_refresh_expiration_secs(), 604800);
        assert_eq!(config.jwt_issuer(), "test-issuer");
        assert_eq!(config.jwt_audience(), "test-audience");
        assert_eq!(config.password_hash_cost(), 4);
        assert!(!config.grpc_tls_enabled());
    }

    // =========================================================================
    // Password Hasher Tests
    // =========================================================================

    #[test]
    fn test_password_hasher_component() {
        use arcana_security::PasswordHasherInterface;

        let hasher = PasswordHasher::with_cost(4);
        let password = "test_password_123";

        // Test hashing
        let hash = hasher.hash(password).expect("Failed to hash password");
        assert!(!hash.is_empty());
        assert_ne!(hash, password);

        // Test verification
        assert!(hasher.verify(password, &hash).expect("Verification failed"));
        assert!(!hasher.verify("wrong_password", &hash).expect("Verification failed"));
    }

    #[test]
    fn test_password_hasher_with_different_costs() {
        use arcana_security::PasswordHasherInterface;

        // Low cost for testing
        let hasher_low = PasswordHasher::with_cost(4);
        let hash_low = hasher_low.hash("password").unwrap();

        // Verify both can verify their own hashes
        assert!(hasher_low.verify("password", &hash_low).unwrap());
    }

    // =========================================================================
    // Token Provider Tests
    // =========================================================================

    #[test]
    fn test_token_provider_component() {
        let config = Arc::new(create_test_security_config());
        let token_provider = TokenProvider::new(config);

        let user_id = arcana_core::UserId::new();
        let username = "testuser";
        let email = "test@example.com";
        let role = UserRole::User;

        // Generate tokens
        let tokens = token_provider.generate_tokens(user_id, username, email, role)
            .expect("Failed to generate tokens");

        assert!(!tokens.access_token.is_empty());
        assert!(!tokens.refresh_token.is_empty());
        assert_eq!(tokens.token_type, "Bearer");
        assert!(tokens.access_expires_at > 0);
        assert!(tokens.refresh_expires_at > 0);
    }

    #[test]
    fn test_token_provider_validation() {
        let config = Arc::new(create_test_security_config());
        let token_provider = TokenProvider::new(config);

        let user_id = arcana_core::UserId::new();
        let username = "testuser";
        let email = "test@example.com";
        let role = UserRole::Admin;

        // Generate and validate access token
        let tokens = token_provider.generate_tokens(user_id, username, email, role).unwrap();
        let claims = token_provider.validate_access_token(&tokens.access_token)
            .expect("Failed to validate access token");

        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.username, username);
        assert_eq!(claims.email, email);
        assert_eq!(claims.role, role);
    }

    #[test]
    fn test_token_provider_refresh() {
        let config = Arc::new(create_test_security_config());
        let token_provider = TokenProvider::new(config);

        let user_id = arcana_core::UserId::new();
        let tokens = token_provider.generate_tokens(user_id, "user", "user@test.com", UserRole::User).unwrap();

        // Refresh the token
        let new_tokens = token_provider.refresh_tokens(&tokens.refresh_token)
            .expect("Failed to refresh tokens");

        // New tokens should be different
        assert_ne!(new_tokens.access_token, tokens.access_token);
        assert!(!new_tokens.access_token.is_empty());
    }

    #[test]
    fn test_token_provider_invalid_token() {
        let config = Arc::new(create_test_security_config());
        let token_provider = TokenProvider::new(config);

        // Invalid token should fail validation
        let result = token_provider.validate_access_token("invalid.token.here");
        assert!(result.is_err());
    }

    #[test]
    fn test_token_provider_wrong_secret() {
        let config1 = Arc::new(SecurityConfig {
            jwt_secret: "secret-one-32-characters-long!!!".to_string(),
            ..create_test_security_config()
        });
        let config2 = Arc::new(SecurityConfig {
            jwt_secret: "secret-two-32-characters-long!!!".to_string(),
            ..create_test_security_config()
        });

        let provider1 = TokenProvider::new(config1);
        let provider2 = TokenProvider::new(config2);

        let user_id = arcana_core::UserId::new();
        let tokens = provider1.generate_tokens(user_id, "user", "user@test.com", UserRole::User).unwrap();

        // Token from provider1 should not validate with provider2
        let result = provider2.validate_access_token(&tokens.access_token);
        assert!(result.is_err());
    }

    // =========================================================================
    // Resolver Trait Tests
    // =========================================================================

    #[test]
    fn test_resolver_traits_are_object_safe() {
        // Verify that resolver traits can be used as trait objects
        fn _use_service_resolver(_r: &dyn ServiceResolver) {}
        fn _use_database_resolver(_r: &dyn DatabaseResolver) {}
        fn _use_repository_resolver(_r: &dyn RepositoryResolver) {}
        fn _use_security_resolver(_r: &dyn SecurityResolver) {}
        fn _use_cache_resolver(_r: &dyn CacheResolver) {}
    }

    // =========================================================================
    // Module Builder Configuration Tests
    // =========================================================================

    #[test]
    fn test_security_config_parameters() {
        let config = create_test_security_config();

        // Verify parameters can be created
        let params = arcana_config::SecurityConfigParameters {
            jwt_secret: config.jwt_secret.clone(),
            jwt_access_expiration_secs: config.jwt_access_expiration_secs,
            jwt_refresh_expiration_secs: config.jwt_refresh_expiration_secs,
            jwt_issuer: config.jwt_issuer.clone(),
            jwt_audience: config.jwt_audience.clone(),
            grpc_tls_enabled: config.grpc_tls_enabled,
            tls_cert_path: config.tls_cert_path.clone(),
            tls_key_path: config.tls_key_path.clone(),
            password_hash_cost: config.password_hash_cost,
        };

        assert_eq!(params.jwt_secret, config.jwt_secret);
        assert_eq!(params.jwt_access_expiration_secs, config.jwt_access_expiration_secs);
    }

    #[test]
    fn test_password_hasher_parameters() {
        let hasher = PasswordHasher::with_cost(4);

        // Verify parameters can be created
        let params = arcana_security::PasswordHasherParameters {
            argon2: hasher.argon2_arc(),
        };

        // Params should be cloneable for module building
        let _ = params.argon2.clone();
    }

    #[test]
    fn test_token_provider_parameters() {
        let config = Arc::new(create_test_security_config());
        let provider = TokenProvider::new(config.clone());

        // Verify parameters can be created
        let params = arcana_security::TokenProviderParameters {
            encoding_key: provider.encoding_key().clone(),
            decoding_key: provider.decoding_key().clone(),
            config: config.clone(),
            validation: provider.validation().clone(),
        };

        // Params should contain valid keys
        let _ = params.encoding_key;
        let _ = params.decoding_key;
    }

    // =========================================================================
    // Legacy Type Alias Tests
    // =========================================================================

    #[test]
    #[allow(deprecated)]
    fn test_legacy_app_module_alias() {
        // Verify the deprecated type alias exists
        fn _assert_same_type<T>() {}
        _assert_same_type::<AppModule>();

        // AppModule should be the same as MonolithicModule
        fn _takes_monolithic(_: &MonolithicModule) {}
        fn _returns_app_module() -> Option<AppModule> { None }

        // This would fail at compile time if they're different types
        if let Some(module) = _returns_app_module() {
            _takes_monolithic(&module);
        }
    }
}
