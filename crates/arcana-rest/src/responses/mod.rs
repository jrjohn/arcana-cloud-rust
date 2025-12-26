//! API response types.

use arcana_core::{ArcanaError, ErrorResponse};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

/// Standard API response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorResponse>,
}

impl<T> ApiResponse<T> {
    /// Creates a successful response.
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    /// Creates an error response.
    pub fn error(error: ErrorResponse) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

/// Application error type for Axum.
#[derive(Debug)]
pub struct AppError(pub ArcanaError);

impl From<ArcanaError> for AppError {
    fn from(err: ArcanaError) -> Self {
        Self(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.0.status_code())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        let error_response = ErrorResponse::from_error(&self.0);
        let body = Json(ApiResponse::<()>::error(error_response));

        (status, body).into_response()
    }
}

/// Result type for Axum handlers.
pub type ApiResult<T> = Result<Json<ApiResponse<T>>, AppError>;

/// Helper to create a success response.
pub fn ok<T: Serialize>(data: T) -> ApiResult<T> {
    Ok(Json(ApiResponse::success(data)))
}

/// Helper to create a created (201) response.
pub fn created<T: Serialize>(data: T) -> (StatusCode, Json<ApiResponse<T>>) {
    (StatusCode::CREATED, Json(ApiResponse::success(data)))
}

/// Helper to create a no content (204) response.
pub fn no_content() -> StatusCode {
    StatusCode::NO_CONTENT
}
