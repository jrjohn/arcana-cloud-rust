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

    #[test]
    fn test_email_display() {
        let email = Email::new("user@example.com").unwrap();
        assert_eq!(format!("{}", email), "user@example.com");
    }

    #[test]
    fn test_email_as_ref() {
        let email = Email::new("user@example.com").unwrap();
        let s: &str = email.as_ref();
        assert_eq!(s, "user@example.com");
    }

    #[test]
    fn test_email_into_string() {
        let email = Email::new("user@example.com").unwrap();
        let s: String = email.into();
        assert_eq!(s, "user@example.com");
    }

    #[test]
    fn test_email_try_from_string() {
        let email = Email::try_from("user@example.com".to_string()).unwrap();
        assert_eq!(email.as_str(), "user@example.com");
    }

    #[test]
    fn test_email_try_from_invalid_string() {
        assert!(Email::try_from("not-an-email".to_string()).is_err());
    }

    #[test]
    fn test_email_new_unchecked() {
        let email = Email::new_unchecked("  UPPER@DOMAIN.COM  ");
        assert_eq!(email.as_str(), "upper@domain.com");
    }

    #[test]
    fn test_email_equality() {
        let email1 = Email::new("user@example.com").unwrap();
        let email2 = Email::new("USER@EXAMPLE.COM").unwrap();
        assert_eq!(email1, email2);
    }

    #[test]
    fn test_email_clone() {
        let email = Email::new("user@example.com").unwrap();
        let cloned = email.clone();
        assert_eq!(email, cloned);
    }

    #[test]
    fn test_email_serialization() {
        let email = Email::new("user@example.com").unwrap();
        let json = serde_json::to_string(&email).unwrap();
        assert_eq!(json, "\"user@example.com\"");
        let parsed: Email = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, email);
    }

    #[test]
    fn test_email_deserialization_invalid() {
        let json = "\"not-an-email\"";
        assert!(serde_json::from_str::<Email>(json).is_err());
    }

    #[test]
    fn test_email_with_subdomain() {
        let email = Email::new("user@mail.example.com").unwrap();
        assert_eq!(email.domain(), "mail.example.com");
        assert_eq!(email.local_part(), "user");
    }

    #[test]
    fn test_email_with_plus_sign() {
        let email = Email::new("user+tag@example.com").unwrap();
        assert_eq!(email.local_part(), "user+tag");
    }

    #[test]
    fn test_email_error_display() {
        let err = Email::new("bad").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Invalid email address"));
    }
}
