//! Redis-based cache implementation.

use super::CacheInterface;
use arcana_core::{ArcanaError, ArcanaResult};
use async_trait::async_trait;
use deadpool_redis::{redis::AsyncCommands, Pool};
use shaku::Component;
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;

/// Default TTL for cached items (5 minutes).
pub const DEFAULT_TTL: Duration = Duration::from_secs(300);

/// Short TTL for existence checks (1 minute).
pub const SHORT_TTL: Duration = Duration::from_secs(60);

/// Redis-based cache service.
#[derive(Component)]
#[shaku(interface = CacheInterface)]
pub struct RedisCacheService {
    /// Redis connection pool.
    pool: Option<Arc<Pool>>,
    /// Default TTL for cached items.
    #[shaku(default = DEFAULT_TTL)]
    #[allow(dead_code)]
    default_ttl: Duration,
}

impl RedisCacheService {
    /// Create a new Redis cache service.
    #[must_use]
    pub fn new(pool: Arc<Pool>) -> Self {
        Self {
            pool: Some(pool),
            default_ttl: DEFAULT_TTL,
        }
    }

    /// Create a cache service with a custom default TTL.
    #[must_use]
    pub fn with_ttl(pool: Arc<Pool>, default_ttl: Duration) -> Self {
        Self {
            pool: Some(pool),
            default_ttl,
        }
    }

    /// Create a no-op cache service (for when Redis is disabled).
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            pool: None,
            default_ttl: DEFAULT_TTL,
        }
    }

    /// Get a connection from the pool.
    async fn get_conn(&self) -> ArcanaResult<deadpool_redis::Connection> {
        match &self.pool {
            Some(pool) => pool.get().await.map_err(|e| {
                ArcanaError::Cache(format!("Failed to get Redis connection: {}", e))
            }),
            None => Err(ArcanaError::Cache("Cache is disabled".to_string())),
        }
    }
}

#[async_trait]
impl CacheInterface for RedisCacheService {
    fn is_enabled(&self) -> bool {
        self.pool.is_some()
    }

    async fn get_raw(&self, key: &str) -> ArcanaResult<Option<String>> {
        if !self.is_enabled() {
            return Ok(None);
        }

        let mut conn = self.get_conn().await?;
        let value: Option<String> = conn.get(key).await.map_err(|e| {
            ArcanaError::Cache(format!("Failed to get key '{}': {}", key, e))
        })?;

        match &value {
            Some(_) => debug!("Cache hit for key '{}'", key),
            None => debug!("Cache miss for key '{}'", key),
        }

        Ok(value)
    }

    async fn set_raw(&self, key: &str, value: &str, ttl: Duration) -> ArcanaResult<()> {
        if !self.is_enabled() {
            return Ok(());
        }

        let mut conn = self.get_conn().await?;
        let ttl_secs = ttl.as_secs().max(1) as u64;

        conn.set_ex::<_, _, ()>(key, value, ttl_secs).await.map_err(|e| {
            ArcanaError::Cache(format!("Failed to set key '{}': {}", key, e))
        })?;

        debug!("Cached key '{}' with TTL {}s", key, ttl_secs);
        Ok(())
    }

    async fn delete(&self, key: &str) -> ArcanaResult<bool> {
        if !self.is_enabled() {
            return Ok(false);
        }

        let mut conn = self.get_conn().await?;
        let deleted: i64 = conn.del(key).await.map_err(|e| {
            ArcanaError::Cache(format!("Failed to delete key '{}': {}", key, e))
        })?;

        debug!("Deleted key '{}': {}", key, deleted > 0);
        Ok(deleted > 0)
    }

    async fn exists(&self, key: &str) -> ArcanaResult<bool> {
        if !self.is_enabled() {
            return Ok(false);
        }

        let mut conn = self.get_conn().await?;
        let exists: bool = conn.exists(key).await.map_err(|e| {
            ArcanaError::Cache(format!("Failed to check key '{}': {}", key, e))
        })?;

        Ok(exists)
    }

    async fn delete_pattern(&self, pattern: &str) -> ArcanaResult<u64> {
        if !self.is_enabled() {
            return Ok(0);
        }

        let mut conn = self.get_conn().await?;

        // Use KEYS to find matching keys (SCAN would be better for production)
        let keys: Vec<String> = deadpool_redis::redis::cmd("KEYS")
            .arg(pattern)
            .query_async(&mut conn)
            .await
            .map_err(|e| ArcanaError::Cache(format!("Failed to scan keys: {}", e)))?;

        if keys.is_empty() {
            return Ok(0);
        }

        // Delete all matching keys
        let deleted: i64 = conn.del(&keys).await.map_err(|e| {
            ArcanaError::Cache(format!("Failed to delete keys: {}", e))
        })?;

        debug!("Deleted {} keys matching pattern '{}'", deleted, pattern);
        Ok(deleted as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disabled_cache() {
        let cache = RedisCacheService::disabled();
        assert!(!cache.is_enabled());
    }
}
