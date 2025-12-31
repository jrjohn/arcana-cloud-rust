//! Email value object.

use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;
use validator::ValidateEmail;

/// Error type for email validation.
#[derive(Debug, Error)]
#[error("Invalid email address: {0}")]
pub struct EmailError(String);

/// Email value object with validation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Email(String);

impl Email {
    /// Creates a new Email after validating the format.
    pub fn new(email: impl Into<String>) -> Result<Self, EmailError> {
        let email = email.into().trim().to_lowercase();

        if !email.validate_email() {
            return Err(EmailError(email));
        }

        Ok(Self(email))
    }

    /// Creates a new Email without validation (for trusted sources).
    ///
    /// # Safety
    ///
    /// This should only be used for data coming from trusted sources
    /// like the database where the email was already validated.
    #[must_use]
    pub fn new_unchecked(email: impl Into<String>) -> Self {
        Self(email.into().trim().to_lowercase())
    }

    /// Returns the email as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the local part of the email (before @).
    #[must_use]
    pub fn local_part(&self) -> &str {
        self.0.split('@').next().unwrap_or("")
    }

    /// Returns the domain part of the email (after @).
    #[must_use]
    pub fn domain(&self) -> &str {
        self.0.split('@').nth(1).unwrap_or("")
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for Email {
    type Error = EmailError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Email> for String {
    fn from(email: Email) -> Self {
        email.0
    }
}

impl AsRef<str> for Email {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_email() {
        let email = Email::new("test@example.com").unwrap();
        assert_eq!(email.as_str(), "test@example.com");
    }

    #[test]
    fn test_email_normalization() {
        let email = Email::new("  TEST@EXAMPLE.COM  ").unwrap();
        assert_eq!(email.as_str(), "test@example.com");
    }

    #[test]
    fn test_invalid_email() {
        assert!(Email::new("invalid").is_err());
        assert!(Email::new("@example.com").is_err());
        assert!(Email::new("test@").is_err());
    }

    #[test]
    fn test_email_parts() {
        let email = Email::new("user@example.com").unwrap();
        assert_eq!(email.local_part(), "user");
        assert_eq!(email.domain(), "example.com");
    }
}
