//! Entity-DTO mappers.

// Mappers are typically implemented as From/Into traits on the DTOs themselves.
// This module can contain more complex mapping logic if needed.

use arcana_core::Page;
use crate::dto::{UserListResponse, UserResponse};
use arcana_core::User;

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

#[cfg(test)]
mod tests {
    use super::*;
    use arcana_core::{Email, UserRole, UserStatus};

    fn make_user(username: &str, email: &str) -> User {
        let mut u = User::new(
            username.to_string(),
            Email::new(email).unwrap(),
            "hash".to_string(),
            Some("First".to_string()),
            Some("Last".to_string()),
        );
        u.activate();
        u
    }

    #[test]
    fn test_user_list_response_from_empty_page() {
        let page: Page<User> = Page::empty(0, 10);
        let response = UserListResponse::from(page);
        assert!(response.users.is_empty());
        assert_eq!(response.total_elements, 0);
        assert_eq!(response.total_pages, 0);
    }

    #[test]
    fn test_user_list_response_from_page_with_users() {
        let user1 = make_user("alice", "alice@example.com");
        let user2 = make_user("bob", "bob@example.com");
        let page = Page::new(vec![user1, user2], 0, 10, 2);
        let response = UserListResponse::from(page);

        assert_eq!(response.users.len(), 2);
        assert_eq!(response.total_elements, 2);
        assert_eq!(response.page, 0);
        assert_eq!(response.size, 10);
    }

    #[test]
    fn test_user_response_from_user() {
        let user = make_user("alice", "alice@example.com");
        let response = UserResponse::from(user.clone());
        assert_eq!(response.id, user.id);
        assert_eq!(response.username, "alice");
        assert_eq!(response.email, "alice@example.com");
        assert_eq!(response.first_name, Some("First".to_string()));
        assert_eq!(response.last_name, Some("Last".to_string()));
        assert_eq!(response.role, UserRole::User);
        assert_eq!(response.status, UserStatus::Active);
    }

    #[test]
    fn test_user_response_verified() {
        let user = make_user("charlie", "charlie@example.com");
        let response = UserResponse::from(user);
        assert!(response.email_verified); // user.activate() sets email_verified = true
    }

    #[test]
    fn test_user_list_response_pagination_metadata() {
        let users: Vec<User> = (0..5).map(|i| make_user(&format!("user{}", i), &format!("user{}@example.com", i))).collect();
        let page = Page::new(users, 1, 5, 25);
        let response = UserListResponse::from(page);

        assert_eq!(response.users.len(), 5);
        assert_eq!(response.total_elements, 25);
        assert_eq!(response.total_pages, 5);
        assert_eq!(response.page, 1);
        assert_eq!(response.size, 5);
    }
}
