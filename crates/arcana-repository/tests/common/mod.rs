//! Common test infrastructure for database integration tests.

use arcana_config::DatabaseConfig;
use arcana_repository::DatabasePool;
use std::sync::Arc;
use testcontainers::{
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
    ContainerAsync, GenericImage, ImageExt,
};

/// MySQL image used for integration tests.
///
/// Built from `GenericImage` instead of `testcontainers_modules::mysql::Mysql`:
/// testcontainers-modules 0.15 (latest) requires testcontainers ^0.27 and cannot
/// be resolved together with testcontainers 0.28. Name, tag and ready conditions
/// match the module's definition.
fn mysql_image() -> GenericImage {
    GenericImage::new("mysql", "8.1")
        .with_exposed_port(3306.tcp())
        .with_wait_for(WaitFor::message_on_stderr(
            "X Plugin ready for connections. Bind-address",
        ))
        .with_wait_for(WaitFor::message_on_stderr(
            "/usr/sbin/mysqld: ready for connections.",
        ))
}

/// Test database container wrapper.
///
/// Manages a MySQL testcontainer lifecycle and provides a database pool.
pub struct TestDatabase {
    _container: ContainerAsync<GenericImage>,
    pool: Arc<DatabasePool>,
}

impl TestDatabase {
    /// Creates a new test database with a fresh MySQL container.
    ///
    /// Runs migrations automatically after container startup.
    pub async fn new() -> Self {
        // Start MySQL container
        let container = mysql_image()
            .with_env_var("MYSQL_ROOT_PASSWORD", "testpass")
            .with_env_var("MYSQL_DATABASE", "arcana_test")
            .with_env_var("MYSQL_USER", "arcana")
            .with_env_var("MYSQL_PASSWORD", "arcana")
            .start()
            .await
            .expect("Failed to start MySQL container");

        // Get the mapped port
        let port = container
            .get_host_port_ipv4(3306)
            .await
            .expect("Failed to get MySQL port");

        // Build database URL
        let database_url = format!(
            "mysql://arcana:arcana@127.0.0.1:{}/arcana_test",
            port
        );

        // Create database config
        let config = DatabaseConfig {
            url: database_url,
            min_connections: 1,
            max_connections: 5,
            connect_timeout_secs: 30,
            idle_timeout_secs: 600,
            log_queries: true,
        };

        // Wait for MySQL to be ready and connect
        let pool = Self::connect_with_retry(&config, 30).await;

        // Run migrations
        pool.run_migrations()
            .await
            .expect("Failed to run migrations");

        Self {
            _container: container,
            pool: Arc::new(pool),
        }
    }

    /// Returns a reference to the database pool.
    pub fn pool(&self) -> Arc<DatabasePool> {
        Arc::clone(&self.pool)
    }

    /// Connects to the database with retry logic.
    async fn connect_with_retry(config: &DatabaseConfig, max_attempts: u32) -> DatabasePool {
        let mut attempts = 0;
        loop {
            attempts += 1;
            match DatabasePool::new(config).await {
                Ok(pool) => return pool,
                Err(e) => {
                    if attempts >= max_attempts {
                        panic!("Failed to connect to database after {} attempts: {}", max_attempts, e);
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
    }
}
