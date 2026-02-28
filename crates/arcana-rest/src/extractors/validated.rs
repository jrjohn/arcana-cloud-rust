//! Validated JSON extractor for automatic request validation.
//!
//! This module provides a `ValidatedJson<T>` extractor that deserializes JSON
//! and validates it using the `validator` crate. Validation errors are returned
//! as 422 Unprocessable Entity with field-level error details.

use arcana_core::{ErrorResponse, FieldError};
use axum::{
    async_trait,
    extract::{rejection::JsonRejection, FromRequest, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::de::DeserializeOwned;
use validator::{Validate, ValidationErrors, ValidationErrorsKind};

/// JSON extractor that automatically validates the deserialized value.
///
/// Returns 422 Unprocessable Entity with field-level errors if validation fails.
///
/// # Example
///
/// ```ignore
/// use arcana_rest::extractors::ValidatedJson;
/// use validator::Validate;
///
/// #[derive(Deserialize, Validate)]
/// struct CreateUserRequest {
///     #[validate(length(min = 3, max = 32))]
///     username: String,
///     #[validate(email)]
///     email: String,
/// }
///
/// async fn create_user(ValidatedJson(request): ValidatedJson<CreateUserRequest>) {
///     // request is guaranteed to be valid here
/// }
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedJson<T>(pub T);

impl<T> std::ops::Deref for ValidatedJson<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for ValidatedJson<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Rejection type for validated JSON extraction.
pub enum ValidatedJsonRejection {
    /// JSON parsing/deserialization error.
    JsonError(JsonRejection),
    /// Validation error with field-level details.
    ValidationError(ValidationErrors),
}

impl IntoResponse for ValidatedJsonRejection {
    fn into_response(self) -> Response {
        match self {
            Self::JsonError(rejection) => {
                let error_response = ErrorResponse {
                    code: "INVALID_JSON".to_string(),
                    message: format!("Invalid JSON: {}", rejection),
                    details: None,
                    trace_id: None,
                };
                (StatusCode::BAD_REQUEST, Json(error_response)).into_response()
            }
            Self::ValidationError(errors) => {
                let field_errors = convert_validation_errors(&errors);
                let error_response = ErrorResponse {
                    code: "VALIDATION_ERROR".to_string(),
                    message: "Request validation failed".to_string(),
                    details: Some(field_errors),
                    trace_id: None,
                };
                (StatusCode::UNPROCESSABLE_ENTITY, Json(error_response)).into_response()
            }
        }
    }
}

/// Convert validator errors to field errors.
fn convert_validation_errors(errors: &ValidationErrors) -> Vec<FieldError> { // NOSONAR
    let mut field_errors = Vec::new();

    for (field, field_errs) in errors.field_errors() {
        for err in field_errs {
            let message = err
                .message
                .as_ref()
                .map(|m| m.to_string())
                .unwrap_or_else(|| format!("Validation failed for field '{}'", field));

            let code = err.code.to_string();

            field_errors.push(FieldError {
                field: field.to_string(),
                message,
                code,
            });
        }
    }

    // Handle nested struct errors
    for (field, errors_kind) in &errors.0 {
        if let ValidationErrorsKind::Struct(nested) = errors_kind {
            for nested_err in convert_validation_errors(nested.as_ref()) {
                field_errors.push(FieldError {
                    field: format!("{}.{}", field, nested_err.field),
                    message: nested_err.message,
                    code: nested_err.code,
                });
            }
        }
        // Handle list errors (e.g., Vec<T> where T: Validate)
        if let ValidationErrorsKind::List(list_errors) = errors_kind {
            for (index, item_errors) in list_errors {
                for nested_err in convert_validation_errors(item_errors.as_ref()) {
                    field_errors.push(FieldError {
                        field: format!("{}[{}].{}", field, index, nested_err.field),
                        message: nested_err.message,
                        code: nested_err.code,
                    });
                }
            }
        }
    }

    field_errors
}

#[async_trait]
impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = ValidatedJsonRejection;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        // First, extract as regular JSON
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(ValidatedJsonRejection::JsonError)?;

        // Then validate
        value
            .validate()
            .map_err(ValidatedJsonRejection::ValidationError)?;

        Ok(ValidatedJson(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use validator::Validate;

    #[derive(Debug, Deserialize, Validate)]
    struct TestRequest {
        #[validate(length(min = 3, message = "Name must be at least 3 characters"))]
        name: String,
        #[validate(email(message = "Invalid email format"))]
        email: String,
    }

    #[derive(Debug, Deserialize, Validate)]
    struct NestedRequest {
        #[validate(length(min = 1))]
        title: String,
        #[validate(nested)]
        author: TestRequest,
    }

    #[test]
    fn test_convert_validation_errors_single_field() {
        let req = TestRequest {
            name: "ab".to_string(), // Too short
            email: "valid@example.com".to_string(),
        };

        let result = req.validate();
        assert!(result.is_err());

        let errors = result.unwrap_err();
        let field_errors = convert_validation_errors(&errors);

        assert_eq!(field_errors.len(), 1);
        assert_eq!(field_errors[0].field, "name");
        assert_eq!(field_errors[0].message, "Name must be at least 3 characters");
    }

    #[test]
    fn test_convert_validation_errors_multiple_fields() {
        let req = TestRequest {
            name: "ab".to_string(),      // Too short
            email: "invalid".to_string(), // Invalid email
        };

        let result = req.validate();
        assert!(result.is_err());

        let errors = result.unwrap_err();
        let field_errors = convert_validation_errors(&errors);

        assert_eq!(field_errors.len(), 2);

        let field_names: Vec<&str> = field_errors.iter().map(|e| e.field.as_str()).collect();
        assert!(field_names.contains(&"name"));
        assert!(field_names.contains(&"email"));
    }

    #[test]
    fn test_convert_validation_errors_nested() {
        let req = NestedRequest {
            title: "Valid Title".to_string(),
            author: TestRequest {
                name: "ab".to_string(), // Invalid
                email: "valid@example.com".to_string(),
            },
        };

        let result = req.validate();
        assert!(result.is_err());

        let errors = result.unwrap_err();
        let field_errors = convert_validation_errors(&errors);

        assert_eq!(field_errors.len(), 1);
        assert_eq!(field_errors[0].field, "author.name");
    }

    #[test]
    fn test_valid_request_passes() {
        let req = TestRequest {
            name: "Valid Name".to_string(),
            email: "valid@example.com".to_string(),
        };

        let result = req.validate();
        assert!(result.is_ok());
    }
}
