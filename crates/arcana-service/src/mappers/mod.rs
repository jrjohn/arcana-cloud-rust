//! Entity-DTO mappers.

// Mappers are typically implemented as From/Into traits on the DTOs themselves.
// This module can contain more complex mapping logic if needed.

use arcana_core::Page;
use crate::dto::{UserListResponse, UserResponse};
use arcana_domain::User;

/// Converts a page of users to a user list response.
impl From<Page<User>> for UserListResponse {
    fn from(page: Page<User>) -> Self {
        Self {
            users: page.content.into_iter().map(UserResponse::from).collect(),
            page: page.info.page,
            size: page.info.size,
            total_elements: page.info.total_elements,
            total_pages: page.info.total_pages,
        }
    }
}
