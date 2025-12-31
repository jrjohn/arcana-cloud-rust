//! Integration tests for MySqlUserRepository.
//!
//! These tests run against a real MySQL database using testcontainers.
//! Requires Docker to be available on the system.

mod common;

use arcana_core::{Email, PageRequest, User, UserId, UserRole, UserStatus};
use arcana_repository::{MySqlUserRepository, UserRepository};
use common::TestDatabase;
use std::sync::Arc;

fn create_test_user(username: &str, email: &str) -> User {
    let mut user = User::new(
        username.to_string(),
        Email::new_unchecked(email.to_string()),
        "hashed_password_123".to_string(),
        Some("Test".to_string()),
        Some("User".to_string()),
    );
    user.activate();
    user
}

#[tokio::test]
async fn test_save_and_find_by_id() {
    let db = TestDatabase::new().await;
    let repo = MySqlUserRepository::new(db.pool());

    let user = create_test_user("testuser", "test@example.com");
    let user_id = user.id;

    let saved = repo.save(&user).await.expect("Failed to save user");
    assert_eq!(saved.username, "testuser");
    assert_eq!(saved.email.as_str(), "test@example.com");

    let found = repo
        .find_by_id(user_id)
        .await
        .expect("Failed to find user")
        .expect("User not found");

    assert_eq!(found.id, user_id);
    assert_eq!(found.username, "testuser");
    assert_eq!(found.email.as_str(), "test@example.com");
    assert_eq!(found.status, UserStatus::Active);
}

#[tokio::test]
async fn test_find_by_id_not_found() {
    let db = TestDatabase::new().await;
    let repo = MySqlUserRepository::new(db.pool());

    let result = repo
        .find_by_id(UserId::new())
        .await
        .expect("Query failed");

    assert!(result.is_none());
}

#[tokio::test]
async fn test_find_by_username() {
    let db = TestDatabase::new().await;
    let repo = MySqlUserRepository::new(db.pool());

    let user = create_test_user("findme", "findme@example.com");
    repo.save(&user).await.expect("Failed to save user");

    let found = repo
        .find_by_username("findme")
        .await
        .expect("Query failed")
        .expect("User not found");

    assert_eq!(found.username, "findme");
    assert_eq!(found.email.as_str(), "findme@example.com");
}

#[tokio::test]
async fn test_find_by_username_not_found() {
    let db = TestDatabase::new().await;
    let repo = MySqlUserRepository::new(db.pool());

    let result = repo
        .find_by_username("nonexistent")
        .await
        .expect("Query failed");

    assert!(result.is_none());
}

#[tokio::test]
async fn test_find_by_email() {
    let db = TestDatabase::new().await;
    let repo = MySqlUserRepository::new(db.pool());

    let user = create_test_user("emailuser", "email@example.com");
    repo.save(&user).await.expect("Failed to save user");

    let found = repo
        .find_by_email("email@example.com")
        .await
        .expect("Query failed")
        .expect("User not found");

    assert_eq!(found.username, "emailuser");
}

#[tokio::test]
async fn test_find_by_email_case_insensitive() {
    let db = TestDatabase::new().await;
    let repo = MySqlUserRepository::new(db.pool());

    let user = create_test_user("caseuser", "CaseSensitive@Example.COM");
    repo.save(&user).await.expect("Failed to save user");

    let found = repo
        .find_by_email("casesensitive@example.com")
        .await
        .expect("Query failed")
        .expect("User not found");

    assert_eq!(found.username, "caseuser");
}

#[tokio::test]
async fn test_find_by_username_or_email_with_username() {
    let db = TestDatabase::new().await;
    let repo = MySqlUserRepository::new(db.pool());

    let user = create_test_user("dualuser", "dual@example.com");
    repo.save(&user).await.expect("Failed to save user");

    let found = repo
        .find_by_username_or_email("dualuser")
        .await
        .expect("Query failed")
        .expect("User not found");

    assert_eq!(found.username, "dualuser");
}

#[tokio::test]
async fn test_find_by_username_or_email_with_email() {
    let db = TestDatabase::new().await;
    let repo = MySqlUserRepository::new(db.pool());

    let user = create_test_user("dualuser2", "dual2@example.com");
    repo.save(&user).await.expect("Failed to save user");

    let found = repo
        .find_by_username_or_email("dual2@example.com")
        .await
        .expect("Query failed")
        .expect("User not found");

    assert_eq!(found.email.as_str(), "dual2@example.com");
}

#[tokio::test]
async fn test_exists_by_username() {
    let db = TestDatabase::new().await;
    let repo = MySqlUserRepository::new(db.pool());

    let user = create_test_user("existsuser", "exists@example.com");
    repo.save(&user).await.expect("Failed to save user");

    assert!(repo.exists_by_username("existsuser").await.expect("Query failed"));
    assert!(!repo.exists_by_username("nonexistent").await.expect("Query failed"));
}

#[tokio::test]
async fn test_exists_by_email() {
    let db = TestDatabase::new().await;
    let repo = MySqlUserRepository::new(db.pool());

    let user = create_test_user("existsemail", "existsemail@example.com");
    repo.save(&user).await.expect("Failed to save user");

    assert!(repo.exists_by_email("existsemail@example.com").await.expect("Query failed"));
    assert!(repo.exists_by_email("EXISTSEMAIL@EXAMPLE.COM").await.expect("Query failed"));
    assert!(!repo.exists_by_email("nonexistent@example.com").await.expect("Query failed"));
}

#[tokio::test]
async fn test_find_all_empty() {
    let db = TestDatabase::new().await;
    let repo = MySqlUserRepository::new(db.pool());

    let page = repo
        .find_all(PageRequest::new(0, 10))
        .await
        .expect("Query failed");

    assert_eq!(page.content.len(), 0);
    assert_eq!(page.info.total_elements, 0);
}

#[tokio::test]
async fn test_find_all_with_users() {
    let db = TestDatabase::new().await;
    let repo = MySqlUserRepository::new(db.pool());

    for i in 1..=5 {
        let user = create_test_user(&format!("user{}", i), &format!("user{}@example.com", i));
        repo.save(&user).await.expect("Failed to save user");
    }

    let page = repo
        .find_all(PageRequest::new(0, 10))
        .await
        .expect("Query failed");

    assert_eq!(page.content.len(), 5);
    assert_eq!(page.info.total_elements, 5);
}

#[tokio::test]
async fn test_find_all_with_pagination() {
    let db = TestDatabase::new().await;
    let repo = MySqlUserRepository::new(db.pool());

    for i in 1..=10 {
        let user = create_test_user(&format!("pageuser{}", i), &format!("pageuser{}@example.com", i));
        repo.save(&user).await.expect("Failed to save user");
    }

    let page1 = repo
        .find_all(PageRequest::new(0, 3))
        .await
        .expect("Query failed");

    assert_eq!(page1.content.len(), 3);
    assert_eq!(page1.info.total_elements, 10);
    assert_eq!(page1.info.total_pages, 4);

    let page2 = repo
        .find_all(PageRequest::new(1, 3))
        .await
        .expect("Query failed");

    assert_eq!(page2.content.len(), 3);

    let page4 = repo
        .find_all(PageRequest::new(3, 3))
        .await
        .expect("Query failed");

    assert_eq!(page4.content.len(), 1);
}

#[tokio::test]
async fn test_find_by_role() {
    let db = TestDatabase::new().await;
    let repo = MySqlUserRepository::new(db.pool());

    let mut admin1 = create_test_user("admin1", "admin1@example.com");
    admin1.change_role(UserRole::Admin);
    repo.save(&admin1).await.expect("Failed to save admin1");

    let mut admin2 = create_test_user("admin2", "admin2@example.com");
    admin2.change_role(UserRole::Admin);
    repo.save(&admin2).await.expect("Failed to save admin2");

    let user = create_test_user("regularuser", "regular@example.com");
    repo.save(&user).await.expect("Failed to save user");

    let admins = repo
        .find_by_role(UserRole::Admin, PageRequest::new(0, 10))
        .await
        .expect("Query failed");

    assert_eq!(admins.content.len(), 2);
    assert!(admins.content.iter().all(|u| u.role == UserRole::Admin));

    let users = repo
        .find_by_role(UserRole::User, PageRequest::new(0, 10))
        .await
        .expect("Query failed");

    assert_eq!(users.content.len(), 1);
    assert_eq!(users.content[0].username, "regularuser");
}

#[tokio::test]
async fn test_update_user() {
    let db = TestDatabase::new().await;
    let repo = MySqlUserRepository::new(db.pool());

    let mut user = create_test_user("updateme", "updateme@example.com");
    let user_id = user.id;
    repo.save(&user).await.expect("Failed to save user");

    user.update_profile(Some("Updated".to_string()), Some("Name".to_string()), None);
    let updated = repo.update(&user).await.expect("Failed to update user");

    assert_eq!(updated.first_name, Some("Updated".to_string()));
    assert_eq!(updated.last_name, Some("Name".to_string()));

    let found = repo
        .find_by_id(user_id)
        .await
        .expect("Query failed")
        .expect("User not found");

    assert_eq!(found.first_name, Some("Updated".to_string()));
    assert_eq!(found.last_name, Some("Name".to_string()));
}

#[tokio::test]
async fn test_update_user_role() {
    let db = TestDatabase::new().await;
    let repo = MySqlUserRepository::new(db.pool());

    let mut user = create_test_user("rolechange", "rolechange@example.com");
    let user_id = user.id;
    repo.save(&user).await.expect("Failed to save user");

    user.change_role(UserRole::Moderator);
    repo.update(&user).await.expect("Failed to update user");

    let found = repo
        .find_by_id(user_id)
        .await
        .expect("Query failed")
        .expect("User not found");

    assert_eq!(found.role, UserRole::Moderator);
}

#[tokio::test]
async fn test_update_user_status() {
    let db = TestDatabase::new().await;
    let repo = MySqlUserRepository::new(db.pool());

    let mut user = create_test_user("statuschange", "statuschange@example.com");
    let user_id = user.id;
    repo.save(&user).await.expect("Failed to save user");

    user.suspend();
    repo.update(&user).await.expect("Failed to update user");

    let found = repo
        .find_by_id(user_id)
        .await
        .expect("Query failed")
        .expect("User not found");

    assert_eq!(found.status, UserStatus::Suspended);
}

#[tokio::test]
async fn test_delete_user() {
    let db = TestDatabase::new().await;
    let repo = MySqlUserRepository::new(db.pool());

    let user = create_test_user("deleteme", "deleteme@example.com");
    let user_id = user.id;
    repo.save(&user).await.expect("Failed to save user");

    assert!(repo.find_by_id(user_id).await.expect("Query failed").is_some());

    let deleted = repo.delete(user_id).await.expect("Failed to delete user");
    assert!(deleted);

    // Soft delete - user should not be found via normal queries
    assert!(repo.find_by_id(user_id).await.expect("Query failed").is_none());
}

#[tokio::test]
async fn test_delete_nonexistent_user() {
    let db = TestDatabase::new().await;
    let repo = MySqlUserRepository::new(db.pool());

    let deleted = repo.delete(UserId::new()).await.expect("Query failed");
    assert!(!deleted);
}

#[tokio::test]
async fn test_count_users() {
    let db = TestDatabase::new().await;
    let repo = MySqlUserRepository::new(db.pool());

    assert_eq!(repo.count().await.expect("Query failed"), 0);

    for i in 1..=3 {
        let user = create_test_user(&format!("countuser{}", i), &format!("countuser{}@example.com", i));
        repo.save(&user).await.expect("Failed to save user");
    }

    assert_eq!(repo.count().await.expect("Query failed"), 3);
}

#[tokio::test]
async fn test_count_by_role() {
    let db = TestDatabase::new().await;
    let repo = MySqlUserRepository::new(db.pool());

    let mut admin = create_test_user("countadmin", "countadmin@example.com");
    admin.change_role(UserRole::Admin);
    repo.save(&admin).await.expect("Failed to save admin");

    let user1 = create_test_user("countuser1", "countuser1@example.com");
    repo.save(&user1).await.expect("Failed to save user1");

    let user2 = create_test_user("countuser2", "countuser2@example.com");
    repo.save(&user2).await.expect("Failed to save user2");

    assert_eq!(repo.count_by_role(UserRole::Admin).await.expect("Query failed"), 1);
    assert_eq!(repo.count_by_role(UserRole::User).await.expect("Query failed"), 2);
    assert_eq!(repo.count_by_role(UserRole::Moderator).await.expect("Query failed"), 0);
}

#[tokio::test]
async fn test_deleted_users_excluded_from_queries() {
    let db = TestDatabase::new().await;
    let repo = MySqlUserRepository::new(db.pool());

    let user = create_test_user("todelete", "todelete@example.com");
    let user_id = user.id;
    repo.save(&user).await.expect("Failed to save user");

    assert_eq!(repo.count().await.expect("Query failed"), 1);

    repo.delete(user_id).await.expect("Failed to delete user");

    // All queries should exclude the deleted user
    assert_eq!(repo.count().await.expect("Query failed"), 0);
    assert!(repo.find_by_id(user_id).await.expect("Query failed").is_none());
    assert!(repo.find_by_username("todelete").await.expect("Query failed").is_none());
    assert!(repo.find_by_email("todelete@example.com").await.expect("Query failed").is_none());

    let page = repo.find_all(PageRequest::new(0, 10)).await.expect("Query failed");
    assert_eq!(page.content.len(), 0);
}

#[tokio::test]
async fn test_concurrent_saves() {
    let db = TestDatabase::new().await;
    let pool = db.pool();

    let handles: Vec<_> = (0..5)
        .map(|i| {
            let pool = Arc::clone(&pool);
            tokio::spawn(async move {
                let repo = MySqlUserRepository::new(pool);
                let user = create_test_user(
                    &format!("concurrent{}", i),
                    &format!("concurrent{}@example.com", i),
                );
                repo.save(&user).await.expect("Failed to save user");
            })
        })
        .collect();

    for handle in handles {
        handle.await.expect("Task panicked");
    }

    let repo = MySqlUserRepository::new(db.pool());
    assert_eq!(repo.count().await.expect("Query failed"), 5);
}
