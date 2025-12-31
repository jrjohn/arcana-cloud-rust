//! Cache key generators for consistent key naming.

use arcana_core::UserId;

/// Prefix for all cache keys to namespace them.
const CACHE_PREFIX: &str = "arcana:cache";

/// Generate a cache key for a user by ID.
#[must_use]
pub fn user_by_id(id: UserId) -> String {
    format!("{}:user:id:{}", CACHE_PREFIX, id)
}

/// Generate a cache key for a user by username.
#[must_use]
pub fn user_by_username(username: &str) -> String {
    format!("{}:user:username:{}", CACHE_PREFIX, username.to_lowercase())
}

/// Generate a cache key for checking if a username exists.
#[must_use]
pub fn username_exists(username: &str) -> String {
    format!("{}:exists:username:{}", CACHE_PREFIX, username.to_lowercase())
}

/// Generate a cache key for checking if an email exists.
#[must_use]
pub fn email_exists(email: &str) -> String {
    format!("{}:exists:email:{}", CACHE_PREFIX, email.to_lowercase())
}

/// Pattern to invalidate all user-related cache entries for a specific user ID.
#[must_use]
pub fn user_invalidation_pattern(id: UserId) -> String {
    format!("{}:user:*:{}*", CACHE_PREFIX, id)
}

/// Pattern to invalidate all username existence checks.
#[must_use]
pub fn username_exists_pattern() -> String {
    format!("{}:exists:username:*", CACHE_PREFIX)
}

/// Pattern to invalidate all email existence checks.
#[must_use]
pub fn email_exists_pattern() -> String {
    format!("{}:exists:email:*", CACHE_PREFIX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_by_id_key() {
        let id = UserId::new();
        let key = user_by_id(id);
        assert!(key.starts_with("arcana:cache:user:id:"));
        assert!(key.contains(&id.to_string()));
    }

    #[test]
    fn test_user_by_username_key() {
        let key = user_by_username("TestUser");
        assert_eq!(key, "arcana:cache:user:username:testuser");
    }

    #[test]
    fn test_username_exists_key() {
        let key = username_exists("Admin");
        assert_eq!(key, "arcana:cache:exists:username:admin");
    }

    #[test]
    fn test_email_exists_key() {
        let key = email_exists("Test@Example.COM");
        assert_eq!(key, "arcana:cache:exists:email:test@example.com");
    }
}
