//! Cache interface trait for abstracted caching operations.

use arcana_core::ArcanaResult;
use async_trait::async_trait;
use shaku::Interface;
use std::time::Duration;

/// Cache interface for storing and retrieving cached data.
///
/// This trait provides an abstraction over caching implementations,
/// allowing for easy swapping between Redis, in-memory, or other cache backends.
///
/// Uses JSON strings for type-erased storage to maintain dyn-compatibility.
#[async_trait]
pub trait CacheInterface: Interface + Send + Sync {
    /// Get a raw JSON value from the cache.
    ///
    /// Returns `None` if the key doesn't exist or has expired.
    async fn get_raw(&self, key: &str) -> ArcanaResult<Option<String>>;

    /// Set a raw JSON value in the cache with a TTL.
    async fn set_raw(&self, key: &str, value: &str, ttl: Duration) -> ArcanaResult<()>;

    /// Delete a value from the cache.
    ///
    /// Returns `true` if the key existed and was deleted.
    async fn delete(&self, key: &str) -> ArcanaResult<bool>;

    /// Check if a key exists in the cache.
    async fn exists(&self, key: &str) -> ArcanaResult<bool>;

    /// Delete multiple keys matching a pattern.
    ///
    /// Returns the number of keys deleted.
    async fn delete_pattern(&self, pattern: &str) -> ArcanaResult<u64>;

    /// Check if caching is enabled.
    fn is_enabled(&self) -> bool;
}

/// Extension trait with typed methods for convenience.
///
/// This trait provides generic get/set methods that work with any serializable type.
#[async_trait]
pub trait CacheExt: CacheInterface {
    /// Get a typed value from the cache.
    async fn get<T: serde::de::DeserializeOwned + Send>(&self, key: &str) -> ArcanaResult<Option<T>> {
        match self.get_raw(key).await? {
            Some(json) => {
                let value: T = serde_json::from_str(&json)?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// Set a typed value in the cache.
    async fn set<T: serde::Serialize + Send + Sync>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> ArcanaResult<()> {
        let json = serde_json::to_string(value)?;
        self.set_raw(key, &json, ttl).await
    }

    /// Get a value or compute and cache it if not present.
    async fn get_or_set<T, F, Fut>(
        &self,
        key: &str,
        ttl: Duration,
        factory: F,
    ) -> ArcanaResult<T>
    where
        T: serde::Serialize + serde::de::DeserializeOwned + Send + Sync,
        F: FnOnce() -> Fut + Send,
        Fut: std::future::Future<Output = ArcanaResult<T>> + Send,
    {
        // Try to get from cache first
        if let Some(cached) = self.get::<T>(key).await? {
            return Ok(cached);
        }

        // Compute the value
        let value = factory().await?;

        // Cache it (ignore errors as the value is still valid)
        let _ = self.set(key, &value, ttl).await;

        Ok(value)
    }
}

// Blanket implementation for all CacheInterface implementations
impl<T: CacheInterface + ?Sized> CacheExt for T {}
