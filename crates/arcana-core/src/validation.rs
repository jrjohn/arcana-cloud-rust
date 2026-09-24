//! Validation utilities.

use crate::{ArcanaError, FieldError};
use validator::{Validate, ValidationErrors};

/// Extension trait for validation.
pub trait ValidateExt: Validate {
    /// Validates the struct and returns an `ArcanaError` on failure.
    ///
    /// # Errors
    ///
    /// Returns [`ArcanaError::Validation`] listing every failing field as
    /// `field: message` if `validate()` reports any errors.
    fn validate_request(&self) -> Result<(), ArcanaError> {
        self.validate().map_err(validation_errors_to_arcana_error)
    }
}

impl<T: Validate> ValidateExt for T {}

/// Converts `validator::ValidationErrors` to `ArcanaError`.
#[must_use]
#[allow(
    clippy::needless_pass_by_value,
    reason = "public API used as a `map_err` function pointer on `validate()` results; taking a reference would change its signature"
)]
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
                    .map_or_else(|| error.code.to_string(), std::string::ToString::to_string),
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
    ///
    /// # Errors
    ///
    /// Returns `not_blank` if `value` is empty or only whitespace.
    pub fn not_blank(value: &str) -> Result<(), ValidationError> {
        if value.trim().is_empty() {
            return Err(ValidationError::new("not_blank"));
        }
        Ok(())
    }

    /// Validates that a password meets complexity requirements.
    ///
    /// # Errors
    ///
    /// Returns the first failing rule, checked in this order:
    /// `password_too_short` (fewer than 8 bytes), `password_missing_uppercase`,
    /// `password_missing_lowercase`, `password_missing_digit` (ASCII digit),
    /// `password_missing_special` (any non-alphanumeric character).
    pub fn password_complexity(password: &str) -> Result<(), ValidationError> {
        if password.len() < 8 {
            return Err(ValidationError::new("password_too_short"));
        }

        let has_uppercase = password.chars().any(char::is_uppercase);
        let has_lowercase = password.chars().any(char::is_lowercase);
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
    ///
    /// # Errors
    ///
    /// Returns the first failing rule, checked in this order:
    /// `username_too_short` (fewer than 3 bytes), `username_too_long` (more than
    /// 32 bytes), `username_invalid_characters` (anything other than alphanumerics,
    /// `_` or `-`), `username_must_start_with_letter`.
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
        if !username.chars().next().is_some_and(char::is_alphabetic) {
            return Err(ValidationError::new("username_must_start_with_letter"));
        }
        Ok(())
    }

    /// Validates a plugin key format.
    ///
    /// # Errors
    ///
    /// Returns `plugin_key_empty` for an empty key, `plugin_key_too_long` for
    /// more than 64 bytes, or `plugin_key_invalid_characters` if it contains
    /// anything other than alphanumerics, `-` or `_`.
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
