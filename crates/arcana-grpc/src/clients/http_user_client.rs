//! HTTP-based remote user service client for benchmark comparison.
//!
//! This client uses HTTP/JSON for inter-service communication,
//! providing a baseline for comparison with gRPC performance.

use arcana_core::{ArcanaError, ArcanaResult, PageRequest, UserId};
use arcana_service::dto::{
    ChangePasswordRequest, CreateUserRequest, UpdateUserRequest, UpdateUserRoleRequest,
    UpdateUserStatusRequest, UserListResponse, UserResponse,
};
use arcana_service::UserService;
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;

/// HTTP-based remote user service client.
///
/// Uses HTTP/1.1 with JSON serialization for inter-service communication.
/// This provides a baseline for comparing with gRPC performance.
pub struct HttpUserServiceClient {
    client: Client,
    base_url: String,
}

impl HttpUserServiceClient {
    /// Creates a new HTTP user service client.
    pub fn new(base_url: &str) -> ArcanaResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(100)
            .pool_idle_timeout(Duration::from_secs(90))
            .build()
            .map_err(|e| ArcanaError::Internal(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        })
    }

    /// Creates a new HTTP user service client with custom configuration.
    pub fn with_client(client: Client, base_url: &str) -> Self {
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct HttpCreateUserRequest {
    username: String,
    email: String,
    password: String,
    first_name: Option<String>,
    last_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct HttpUpdateUserRequest {
    first_name: Option<String>,
    last_name: Option<String>,
    avatar_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct HttpUpdateRoleRequest {
    role: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct HttpUpdateStatusRequest {
    status: String,
    reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct HttpPageResponse<T> {
    data: Vec<T>,
    page: usize,
    size: usize,
    total_elements: u64,
    total_pages: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct HttpExistsResponse {
    exists: bool,
}

#[async_trait]
impl UserService for HttpUserServiceClient {
    async fn create_user(&self, request: CreateUserRequest) -> ArcanaResult<UserResponse> {
        debug!("HTTP CreateUser: {}", request.username);

        let http_request = HttpCreateUserRequest {
            username: request.username,
            email: request.email,
            password: request.password,
            first_name: request.first_name,
            last_name: request.last_name,
        };

        let response = self
            .client
            .post(self.url("/api/v1/users"))
            .json(&http_request)
            .send()
            .await
            .map_err(|e| ArcanaError::Internal(format!("HTTP error: {}", e)))?;

        handle_response(response).await
    }

    async fn get_user(&self, id: UserId) -> ArcanaResult<UserResponse> {
        debug!("HTTP GetUser: {}", id);

        let response = self
            .client
            .get(self.url(&format!("/api/v1/users/{}", id)))
            .send()
            .await
            .map_err(|e| ArcanaError::Internal(format!("HTTP error: {}", e)))?;

        handle_response(response).await
    }

    async fn get_user_by_username(&self, username: &str) -> ArcanaResult<UserResponse> {
        debug!("HTTP GetUserByUsername: {}", username);

        let response = self
            .client
            .get(self.url(&format!("/api/v1/users/username/{}", username)))
            .send()
            .await
            .map_err(|e| ArcanaError::Internal(format!("HTTP error: {}", e)))?;

        handle_response(response).await
    }

    async fn list_users(&self, page: PageRequest) -> ArcanaResult<UserListResponse> {
        debug!("HTTP ListUsers: page={}, size={}", page.page, page.size);

        let response = self
            .client
            .get(self.url("/api/v1/users"))
            .query(&[("page", page.page.to_string()), ("size", page.size.to_string())])
            .send()
            .await
            .map_err(|e| ArcanaError::Internal(format!("HTTP error: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            return Err(map_http_error(status, &response.text().await.unwrap_or_default()));
        }

        let page_response: HttpPageResponse<UserResponse> = response
            .json()
            .await
            .map_err(|e| ArcanaError::Internal(format!("JSON parse error: {}", e)))?;

        Ok(UserListResponse {
            users: page_response.data,
            page: page_response.page,
            size: page_response.size,
            total_elements: page_response.total_elements,
            total_pages: page_response.total_pages,
        })
    }

    async fn update_user(&self, id: UserId, request: UpdateUserRequest) -> ArcanaResult<UserResponse> {
        debug!("HTTP UpdateUser: {}", id);

        let http_request = HttpUpdateUserRequest {
            first_name: request.first_name,
            last_name: request.last_name,
            avatar_url: request.avatar_url,
        };

        let response = self
            .client
            .patch(self.url(&format!("/api/v1/users/{}", id)))
            .json(&http_request)
            .send()
            .await
            .map_err(|e| ArcanaError::Internal(format!("HTTP error: {}", e)))?;

        handle_response(response).await
    }

    async fn update_user_role(&self, id: UserId, request: UpdateUserRoleRequest) -> ArcanaResult<UserResponse> {
        debug!("HTTP UpdateUserRole: {}", id);

        let http_request = HttpUpdateRoleRequest {
            role: format!("{:?}", request.role),
        };

        let response = self
            .client
            .patch(self.url(&format!("/api/v1/users/{}/role", id)))
            .json(&http_request)
            .send()
            .await
            .map_err(|e| ArcanaError::Internal(format!("HTTP error: {}", e)))?;

        handle_response(response).await
    }

    async fn update_user_status(&self, id: UserId, request: UpdateUserStatusRequest) -> ArcanaResult<UserResponse> {
        debug!("HTTP UpdateUserStatus: {}", id);

        let http_request = HttpUpdateStatusRequest {
            status: format!("{:?}", request.status),
            reason: request.reason,
        };

        let response = self
            .client
            .patch(self.url(&format!("/api/v1/users/{}/status", id)))
            .json(&http_request)
            .send()
            .await
            .map_err(|e| ArcanaError::Internal(format!("HTTP error: {}", e)))?;

        handle_response(response).await
    }

    async fn change_password(&self, _id: UserId, _request: ChangePasswordRequest) -> ArcanaResult<()> {
        // Password changes should be handled by auth service
        Err(ArcanaError::Internal("Password change not supported via remote client".to_string()))
    }

    async fn delete_user(&self, id: UserId) -> ArcanaResult<()> {
        debug!("HTTP DeleteUser: {}", id);

        let response = self
            .client
            .delete(self.url(&format!("/api/v1/users/{}", id)))
            .send()
            .await
            .map_err(|e| ArcanaError::Internal(format!("HTTP error: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            return Err(map_http_error(status, &response.text().await.unwrap_or_default()));
        }

        Ok(())
    }

    async fn username_exists(&self, username: &str) -> ArcanaResult<bool> {
        let response = self
            .client
            .get(self.url(&format!("/api/v1/users/username/{}/exists", username)))
            .send()
            .await
            .map_err(|e| ArcanaError::Internal(format!("HTTP error: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            return Err(map_http_error(status, &response.text().await.unwrap_or_default()));
        }

        let exists_response: HttpExistsResponse = response
            .json()
            .await
            .map_err(|e| ArcanaError::Internal(format!("JSON parse error: {}", e)))?;

        Ok(exists_response.exists)
    }

    async fn email_exists(&self, email: &str) -> ArcanaResult<bool> {
        let response = self
            .client
            .get(self.url(&format!("/api/v1/users/email/{}/exists", email)))
            .send()
            .await
            .map_err(|e| ArcanaError::Internal(format!("HTTP error: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            return Err(map_http_error(status, &response.text().await.unwrap_or_default()));
        }

        let exists_response: HttpExistsResponse = response
            .json()
            .await
            .map_err(|e| ArcanaError::Internal(format!("JSON parse error: {}", e)))?;

        Ok(exists_response.exists)
    }
}

/// Creates a shareable HTTP user service client.
pub fn create_http_user_service(base_url: &str) -> ArcanaResult<Arc<dyn UserService>> {
    let client = HttpUserServiceClient::new(base_url)?;
    Ok(Arc::new(client))
}

async fn handle_response<T: serde::de::DeserializeOwned>(response: reqwest::Response) -> ArcanaResult<T> {
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(map_http_error(status, &body));
    }

    response
        .json()
        .await
        .map_err(|e| ArcanaError::Internal(format!("JSON parse error: {}", e)))
}

fn map_http_error(status: StatusCode, body: &str) -> ArcanaError {
    match status {
        StatusCode::NOT_FOUND => ArcanaError::NotFound {
            resource_type: "Resource",
            id: body.to_string(),
        },
        StatusCode::BAD_REQUEST => ArcanaError::Validation(body.to_string()),
        StatusCode::CONFLICT => ArcanaError::Conflict(body.to_string()),
        StatusCode::UNAUTHORIZED => ArcanaError::InvalidCredentials,
        StatusCode::FORBIDDEN => ArcanaError::Forbidden(body.to_string()),
        _ => ArcanaError::Internal(format!("HTTP error {}: {}", status, body)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_construction() {
        let client = HttpUserServiceClient::new("http://localhost:8080").unwrap();
        assert_eq!(client.url("/api/v1/users"), "http://localhost:8080/api/v1/users");

        let client_trailing = HttpUserServiceClient::new("http://localhost:8080/").unwrap();
        assert_eq!(client_trailing.url("/api/v1/users"), "http://localhost:8080/api/v1/users");
    }
}
