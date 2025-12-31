//! Caching infrastructure for the service layer.
//!
//! This module provides a cache abstraction with a Redis implementation.
//! It supports transparent caching of frequently accessed data like user lookups.

mod cache_interface;
pub mod cache_keys;
mod redis_cache;

pub use cache_interface::{CacheExt, CacheInterface};
pub use redis_cache::{RedisCacheService, RedisCacheServiceParameters, DEFAULT_TTL, SHORT_TTL};
