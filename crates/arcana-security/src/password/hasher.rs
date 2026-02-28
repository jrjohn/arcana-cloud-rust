//! Password hashing using Argon2.

use arcana_core::{ArcanaError, ArcanaResult, Interface};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher as _, PasswordVerifier, SaltString},
    Argon2, Params,
};
use shaku::Component;
use std::sync::Arc;
use tracing::debug;

/// Interface for password hashing operations.
///
/// This trait abstracts password hashing functionality for dependency injection.
pub trait PasswordHasherInterface: Interface + Send + Sync {
    /// Hashes a password.
    fn hash(&self, password: &str) -> ArcanaResult<String>;

    /// Verifies a password against a hash.
    fn verify(&self, password: &str, hash: &str) -> ArcanaResult<bool>;

    /// Checks if a hash needs to be rehashed.
    fn needs_rehash(&self, hash: &str) -> bool;
}

/// Password hasher service using Argon2.
#[derive(Component, Clone)]
#[shaku(interface = PasswordHasherInterface)]
pub struct PasswordHasher {
    argon2: Arc<Argon2<'static>>,
}

impl PasswordHasher {
    /// Creates a new password hasher with default parameters.
    #[must_use]
    pub fn new() -> Self {
        Self::with_params(Params::DEFAULT)
    }

    /// Creates a new password hasher with custom parameters.
    #[must_use]
    pub fn with_params(params: Params) -> Self {
        let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
        Self {
            argon2: Arc::new(argon2),
        }
    }

    /// Creates a password hasher from a cost parameter (memory cost in KB).
    #[must_use]
    pub fn with_cost(cost: u32) -> Self {
        let params = Params::new(
            cost * 1024, // Memory cost in KB
            3,           // Time cost (iterations)
            1,           // Parallelism
            None,        // Output length (default)
        )
        .unwrap_or(Params::DEFAULT);

        Self::with_params(params)
    }

    /// Returns the internal Argon2 instance wrapped in Arc.
    ///
    /// This is used for Shaku component parameter extraction.
    #[must_use]
    pub fn argon2_arc(&self) -> Arc<Argon2<'static>> {
        self.argon2.clone()
    }

    /// Hashes a password.
    pub fn hash(&self, password: &str) -> ArcanaResult<String> {
        let salt = SaltString::generate(&mut OsRng);

        let hash = self
            .argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| ArcanaError::Internal(format!("Failed to hash password: {}", e)))?;

        debug!("Password hashed successfully");
        Ok(hash.to_string())
    }

    /// Verifies a password against a hash.
    pub fn verify(&self, password: &str, hash: &str) -> ArcanaResult<bool> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| ArcanaError::Internal(format!("Invalid password hash format: {}", e)))?;

        match self.argon2.verify_password(password.as_bytes(), &parsed_hash) {
            Ok(()) => {
                debug!("Password verified successfully");
                Ok(true)
            }
            Err(argon2::password_hash::Error::Password) => {
                debug!("Password verification failed: incorrect password");
                Ok(false)
            }
            Err(e) => Err(ArcanaError::Internal(format!(
                "Password verification error: {}",
                e
            ))),
        }
    }

    /// Checks if a hash needs to be rehashed (e.g., due to parameter changes).
    pub fn needs_rehash(&self, hash: &str) -> bool {
        // Parse the hash to check its parameters
        if let Ok(parsed) = PasswordHash::new(hash) {
            // Check if the algorithm matches
            if parsed.algorithm != argon2::Algorithm::Argon2id.ident() {
                return true;
            }
            // Could add more sophisticated checks here for parameter changes
            false
        } else {
            true
        }
    }
}

impl Default for PasswordHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl PasswordHasherInterface for PasswordHasher {
    fn hash(&self, password: &str) -> ArcanaResult<String> {
        let salt = SaltString::generate(&mut OsRng);

        let hash = self
            .argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| ArcanaError::Internal(format!("Failed to hash password: {}", e)))?;

        debug!("Password hashed successfully");
        Ok(hash.to_string())
    }

    fn verify(&self, password: &str, hash: &str) -> ArcanaResult<bool> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| ArcanaError::Internal(format!("Invalid password hash format: {}", e)))?;

        match self.argon2.verify_password(password.as_bytes(), &parsed_hash) {
            Ok(()) => {
                debug!("Password verified successfully");
                Ok(true)
            }
            Err(argon2::password_hash::Error::Password) => {
                debug!("Password verification failed: incorrect password");
                Ok(false)
            }
            Err(e) => Err(ArcanaError::Internal(format!(
                "Password verification error: {}",
                e
            ))),
        }
    }

    fn needs_rehash(&self, hash: &str) -> bool {
        if let Ok(parsed) = PasswordHash::new(hash) {
            if parsed.algorithm != argon2::Algorithm::Argon2id.ident() {
                return true;
            }
            false
        } else {
            true
        }
    }
}

impl std::fmt::Debug for PasswordHasher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PasswordHasher").finish_non_exhaustive()
    }
}

/// Validates password strength.
pub fn validate_password_strength(password: &str) -> Result<(), Vec<&'static str>> {
    let mut errors = Vec::new();

    if password.len() < 8 {
        errors.push("Password must be at least 8 characters long");
    }

    if password.len() > 128 {
        errors.push("Password must be at most 128 characters long");
    }

    if !password.chars().any(|c| c.is_uppercase()) {
        errors.push("Password must contain at least one uppercase letter");
    }

    if !password.chars().any(|c| c.is_lowercase()) {
        errors.push("Password must contain at least one lowercase letter");
    }

    if !password.chars().any(|c| c.is_ascii_digit()) {
        errors.push("Password must contain at least one digit");
    }

    if !password.chars().any(|c| !c.is_alphanumeric()) {
        errors.push("Password must contain at least one special character");
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify() {
        let hasher = PasswordHasher::new();
        let password = "MySecurePassword123!";

        let hash = hasher.hash(password).unwrap();
        assert!(hasher.verify(password, &hash).unwrap());
        assert!(!hasher.verify("wrong-password", &hash).unwrap());
    }

    #[test]
    fn test_different_hashes() {
        let hasher = PasswordHasher::new();
        let password = "TestPassword123!";

        let hash1 = hasher.hash(password).unwrap();
        let hash2 = hasher.hash(password).unwrap();

        // Same password should produce different hashes (different salts)
        assert_ne!(hash1, hash2);

        // But both should verify correctly
        assert!(hasher.verify(password, &hash1).unwrap());
        assert!(hasher.verify(password, &hash2).unwrap());
    }

    #[test]
    fn test_password_strength_validation() {
        assert!(validate_password_strength("StrongP@ss1").is_ok());
        assert!(validate_password_strength("weak").is_err());
        assert!(validate_password_strength("NoDigits!").is_err());
        assert!(validate_password_strength("nospecial123").is_err());
    }

    #[test]
    fn test_invalid_hash_format_returns_error() {
        let hasher = PasswordHasher::new();
        let result = hasher.verify("password", "not-a-valid-hash");
        assert!(result.is_err());
    }

    #[test]
    fn test_needs_rehash_valid_argon2id_hash() {
        let hasher = PasswordHasher::new();
        let hash = hasher.hash("password").unwrap();
        assert!(!hasher.needs_rehash(&hash));
    }

    #[test]
    fn test_needs_rehash_invalid_hash() {
        let hasher = PasswordHasher::new();
        assert!(hasher.needs_rehash("invalid-hash"));
    }

    #[test]
    fn test_hasher_default() {
        let hasher = PasswordHasher::default();
        let hash = hasher.hash("test_password").unwrap();
        assert!(hasher.verify("test_password", &hash).unwrap());
    }

    #[test]
    fn test_hasher_with_cost() {
        let hasher = PasswordHasher::with_cost(1); // Minimal cost for testing speed
        let hash = hasher.hash("test_password").unwrap();
        assert!(hasher.verify("test_password", &hash).unwrap());
    }

    #[test]
    fn test_password_strength_no_uppercase() {
        let result = validate_password_strength("nouppercaseletter1!");
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("uppercase")));
    }

    #[test]
    fn test_password_strength_no_lowercase() {
        let result = validate_password_strength("NOLOWERCASE1!");
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("lowercase")));
    }

    #[test]
    fn test_password_strength_no_digit() {
        let result = validate_password_strength("NoDigitsHere!");
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("digit")));
    }

    #[test]
    fn test_password_strength_no_special() {
        let result = validate_password_strength("NoSpecial1234");
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("special")));
    }

    #[test]
    fn test_password_strength_too_short() {
        let result = validate_password_strength("Ab1!");
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("8 characters")));
    }

    #[test]
    fn test_password_strength_strong() {
        assert!(validate_password_strength("Strong@Pass1").is_ok());
        assert!(validate_password_strength("ValidP@55word").is_ok());
        assert!(validate_password_strength("Sup3r$ecure!").is_ok());
    }

    #[test]
    fn test_interface_hash_and_verify() {
        let hasher = PasswordHasher::new();
        let hash = PasswordHasherInterface::hash(&hasher, "TestPass!1").unwrap();
        assert!(PasswordHasherInterface::verify(&hasher, "TestPass!1", &hash).unwrap());
        assert!(!PasswordHasherInterface::verify(&hasher, "WrongPass!1", &hash).unwrap());
    }

    #[test]
    fn test_interface_needs_rehash() {
        let hasher = PasswordHasher::new();
        let hash = hasher.hash("TestPass!1").unwrap();
        assert!(!PasswordHasherInterface::needs_rehash(&hasher, &hash));
        assert!(PasswordHasherInterface::needs_rehash(&hasher, "garbage-hash"));
    }

    #[test]
    fn test_hasher_debug_does_not_leak_secrets() {
        let hasher = PasswordHasher::new();
        let debug_str = format!("{:?}", hasher);
        assert!(debug_str.contains("PasswordHasher"));
    }
}
