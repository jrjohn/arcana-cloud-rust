//! Database connection pool management.

use arcana_config::DatabaseConfig;
use arcana_core::{ArcanaError, ArcanaResult};
use sqlx::mysql::{MySqlPool, MySqlPoolOptions};
use std::time::Duration;
use tracing::{info, warn};

/// Database pool wrapper.
#[derive(Clone)]
pub struct DatabasePool {
    pool: MySqlPool,
}

impl DatabasePool {
    /// Creates a new database pool from configuration.
    pub async fn new(config: &DatabaseConfig) -> ArcanaResult<Self> {
        info!("Connecting to MySQL database...");

        let pool = MySqlPoolOptions::new()
            .min_connections(config.min_connections)
            .max_connections(config.max_connections)
            .acquire_timeout(Duration::from_secs(config.connect_timeout_secs))
            .idle_timeout(Some(Duration::from_secs(config.idle_timeout_secs)))
            .connect(&config.url)
            .await
            .map_err(|e| {
                warn!("Failed to connect to database: {}", e);
                ArcanaError::Database(format!("Failed to connect: {}", e))
            })?;

        info!("MySQL connection pool established");
        Ok(Self { pool })
    }

    /// Returns a reference to the underlying pool.
    #[must_use]
    pub fn inner(&self) -> &MySqlPool {
        &self.pool
    }

    /// Checks if the database connection is healthy.
    pub async fn health_check(&self) -> ArcanaResult<()> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(|e| ArcanaError::Database(format!("Health check failed: {}", e)))?;
        Ok(())
    }

    /// Runs database migrations.
    pub async fn run_migrations(&self) -> ArcanaResult<()> {
        info!("Running database migrations...");
        sqlx::migrate!("../../migrations")
            .run(&self.pool)
            .await
            .map_err(|e| ArcanaError::Database(format!("Migration failed: {}", e)))?;
        info!("Database migrations completed");
        Ok(())
    }

    /// Closes the database pool.
    pub async fn close(&self) {
        info!("Closing database connection pool...");
        self.pool.close().await;
        info!("Database connection pool closed");
    }
}

impl std::ops::Deref for DatabasePool {
    type Target = MySqlPool;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}

impl std::fmt::Debug for DatabasePool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DatabasePool")
            .field("size", &self.pool.size())
            .field("num_idle", &self.pool.num_idle())
            .finish()
    }
}

/// Creates a shared database pool.
pub async fn create_pool(config: &DatabaseConfig) -> ArcanaResult<std::sync::Arc<DatabasePool>> {
    let pool = DatabasePool::new(config).await?;
    Ok(std::sync::Arc::new(pool))
}
