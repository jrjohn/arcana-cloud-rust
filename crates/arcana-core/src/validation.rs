//! Validation utilities.

use crate::{ArcanaError, FieldError};
use validator::{Validate, ValidationErrors};

/// Extension trait for validation.
pub trait ValidateExt: Validate {
    /// Validates the struct and returns an `ArcanaError` on failure.
    fn validate_request(&self) -> Result<(), ArcanaError> {
        self.validate().map_err(|e| validation_errors_to_arcana_error(e))
    }
}

impl<T: Validate> ValidateExt for T {}

/// Converts `validator::ValidationErrors` to `ArcanaError`.
#[must_use]
pub fn validation_errors_to_arcana_error(errors: ValidationErrors) -> ArcanaError {
    let field_errors: Vec<FieldError> = errors
        .field_errors()
        .iter()
        .flat_map(|(field, errors)| {
            errors.iter().map(move |error| FieldError {
                field: (*field).to_string(),
                message: error
                    .message
                    .as_ref()
                    .map_or_else(|| error.code.to_string(), |m| m.to_string()),
                code: error.code.to_string(),
            })
        })
        .collect();

    let message = field_errors
        .iter()
        .map(|e| format!("{}: {}", e.field, e.message))
        .collect::<Vec<_>>()
        .join("; ");

    ArcanaError::Validation(message)
}

/// Common validation functions.
pub mod rules {
    use validator::ValidationError;

    /// Validates that a string is not blank (not empty after trimming).
    pub fn not_blank(value: &str) -> Result<(), ValidationError> {
        if value.trim().is_empty() {
            return Err(ValidationError::new("not_blank"));
        }
        Ok(())
    }

    /// Validates that a password meets complexity requirements.
    pub fn password_complexity(password: &str) -> Result<(), ValidationError> {
        if password.len() < 8 {
            return Err(ValidationError::new("password_too_short"));
        }

        let has_uppercase = password.chars().any(|c| c.is_uppercase());
        let has_lowercase = password.chars().any(|c| c.is_lowercase());
        let has_digit = password.chars().any(|c| c.is_ascii_digit());
        let has_special = password.chars().any(|c| !c.is_alphanumeric());

        if !has_uppercase {
            return Err(ValidationError::new("password_missing_uppercase"));
        }
        if !has_lowercase {
            return Err(ValidationError::new("password_missing_lowercase"));
        }
        if !has_digit {
            return Err(ValidationError::new("password_missing_digit"));
        }
        if !has_special {
            return Err(ValidationError::new("password_missing_special"));
        }

        Ok(())
    }

    /// Validates that a username meets requirements.
    pub fn valid_username(username: &str) -> Result<(), ValidationError> {
        if username.len() < 3 {
            return Err(ValidationError::new("username_too_short"));
        }
        if username.len() > 32 {
            return Err(ValidationError::new("username_too_long"));
        }
        if !username
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(ValidationError::new("username_invalid_characters"));
        }
        if !username.chars().next().map_or(false, |c| c.is_alphabetic()) {
            return Err(ValidationError::new("username_must_start_with_letter"));
        }
        Ok(())
    }

    /// Validates a plugin key format.
    pub fn valid_plugin_key(key: &str) -> Result<(), ValidationError> {
        if key.is_empty() {
            return Err(ValidationError::new("plugin_key_empty"));
        }
        if key.len() > 64 {
            return Err(ValidationError::new("plugin_key_too_long"));
        }
        if !key
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(ValidationError::new("plugin_key_invalid_characters"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::rules::*;

    #[test]
    fn test_not_blank() {
        assert!(not_blank("hello").is_ok());
        assert!(not_blank("   ").is_err());
        assert!(not_blank("").is_err());
    }

    #[test]
    fn test_password_complexity() {
        assert!(password_complexity("Abcd123!").is_ok());
        assert!(password_complexity("short").is_err());
        assert!(password_complexity("nouppercase1!").is_err());
        assert!(password_complexity("NOLOWERCASE1!").is_err());
        assert!(password_complexity("NoDigits!!").is_err());
        assert!(password_complexity("NoSpecial1").is_err());
    }

    #[test]
    fn test_valid_username() {
        assert!(valid_username("john_doe").is_ok());
        assert!(valid_username("john-doe").is_ok());
        assert!(valid_username("ab").is_err()); // too short
        assert!(valid_username("123abc").is_err()); // starts with number
        assert!(valid_username("john@doe").is_err()); // invalid char
    }

    #[test]
    fn test_valid_plugin_key() {
        assert!(valid_plugin_key("my-plugin").is_ok());
        assert!(valid_plugin_key("my_plugin_123").is_ok());
        assert!(valid_plugin_key("").is_err());
        assert!(valid_plugin_key("my.plugin").is_err());
    }
}
