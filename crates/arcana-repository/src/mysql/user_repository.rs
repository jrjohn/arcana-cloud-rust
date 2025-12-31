//! MySQL user repository implementation.

use crate::{traits::UserRepository, DatabasePoolInterface};
use arcana_core::{ArcanaError, ArcanaResult, Page, PageRequest, UserId};
use arcana_core::{Email, User, UserRole, UserStatus};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shaku::Component;
use sqlx::FromRow;
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

/// MySQL user repository implementation.
#[derive(Component, Clone)]
#[shaku(interface = UserRepository)]
pub struct MySqlUserRepository {
    #[shaku(inject)]
    pool: Arc<dyn DatabasePoolInterface>,
}

impl MySqlUserRepository {
    /// Creates a new MySQL user repository.
    #[must_use]
    pub fn new(pool: Arc<dyn DatabasePoolInterface>) -> Self {
        Self { pool }
    }
}

/// Database row representation of a user.
#[derive(Debug, FromRow)]
struct UserRow {
    id: String,  // MySQL stores UUID as CHAR(36)
    username: String,
    email: String,
    password_hash: String,
    first_name: Option<String>,
    last_name: Option<String>,
    role: String,
    status: String,
    email_verified: bool,
    avatar_url: Option<String>,
    last_login_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<UserRow> for User {
    type Error = ArcanaError;

    fn try_from(row: UserRow) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&row.id)
            .map_err(|e| ArcanaError::Internal(format!("Invalid UUID in database: {}", e)))?;

        Ok(User {
            id: UserId::from_uuid(id),
            username: row.username,
            email: Email::new_unchecked(row.email),
            password_hash: row.password_hash,
            first_name: row.first_name,
            last_name: row.last_name,
            role: parse_role(&row.role),
            status: parse_status(&row.status),
            email_verified: row.email_verified,
            avatar_url: row.avatar_url,
            last_login_at: row.last_login_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

fn parse_role(s: &str) -> UserRole {
    match s.to_lowercase().as_str() {
        "admin" => UserRole::Admin,
        "moderator" => UserRole::Moderator,
        "superadmin" => UserRole::SuperAdmin,
        _ => UserRole::User,
    }
}

fn parse_status(s: &str) -> UserStatus {
    match s.to_lowercase().as_str() {
        "active" => UserStatus::Active,
        "suspended" => UserStatus::Suspended,
        "locked" => UserStatus::Locked,
        "deleted" => UserStatus::Deleted,
        _ => UserStatus::PendingVerification,
    }
}

#[async_trait]
impl UserRepository for MySqlUserRepository {
    async fn find_by_id(&self, id: UserId) -> ArcanaResult<Option<User>> {
        debug!("Finding user by id: {}", id);

        let row = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT id, username, email, password_hash, first_name, last_name,
                   role, status, email_verified, avatar_url, last_login_at,
                   created_at, updated_at
            FROM users
            WHERE id = ? AND status != 'deleted'
            "#,
        )
        .bind(id.into_inner().to_string())
        .fetch_optional(self.pool.inner())
        .await?;

        row.map(User::try_from).transpose()
    }

    async fn find_by_username(&self, username: &str) -> ArcanaResult<Option<User>> {
        debug!("Finding user by username: {}", username);

        let row = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT id, username, email, password_hash, first_name, last_name,
                   role, status, email_verified, avatar_url, last_login_at,
                   created_at, updated_at
            FROM users
            WHERE username = ? AND status != 'deleted'
            "#,
        )
        .bind(username)
        .fetch_optional(self.pool.inner())
        .await?;

        row.map(User::try_from).transpose()
    }

    async fn find_by_email(&self, email: &str) -> ArcanaResult<Option<User>> {
        debug!("Finding user by email: {}", email);

        let row = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT id, username, email, password_hash, first_name, last_name,
                   role, status, email_verified, avatar_url, last_login_at,
                   created_at, updated_at
            FROM users
            WHERE LOWER(email) = LOWER(?) AND status != 'deleted'
            "#,
        )
        .bind(email)
        .fetch_optional(self.pool.inner())
        .await?;

        row.map(User::try_from).transpose()
    }

    async fn find_by_username_or_email(&self, identifier: &str) -> ArcanaResult<Option<User>> {
        debug!("Finding user by username or email: {}", identifier);

        let row = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT id, username, email, password_hash, first_name, last_name,
                   role, status, email_verified, avatar_url, last_login_at,
                   created_at, updated_at
            FROM users
            WHERE (username = ? OR LOWER(email) = LOWER(?)) AND status != 'deleted'
            "#,
        )
        .bind(identifier)
        .bind(identifier)
        .fetch_optional(self.pool.inner())
        .await?;

        row.map(User::try_from).transpose()
    }

    async fn exists_by_username(&self, username: &str) -> ArcanaResult<bool> {
        let result: Option<i32> = sqlx::query_scalar(
            "SELECT 1 FROM users WHERE username = ? LIMIT 1",
        )
        .bind(username)
        .fetch_optional(self.pool.inner())
        .await?;

        Ok(result.is_some())
    }

    async fn exists_by_email(&self, email: &str) -> ArcanaResult<bool> {
        let result: Option<i32> = sqlx::query_scalar(
            "SELECT 1 FROM users WHERE LOWER(email) = LOWER(?) LIMIT 1",
        )
        .bind(email)
        .fetch_optional(self.pool.inner())
        .await?;

        Ok(result.is_some())
    }

    async fn find_all(&self, page: PageRequest) -> ArcanaResult<Page<User>> {
        debug!("Finding all users, page: {}, size: {}", page.page, page.size);

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE status != 'deleted'")
            .fetch_one(self.pool.inner())
            .await?;

        let rows = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT id, username, email, password_hash, first_name, last_name,
                   role, status, email_verified, avatar_url, last_login_at,
                   created_at, updated_at
            FROM users
            WHERE status != 'deleted'
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .fetch_all(self.pool.inner())
        .await?;

        let users: Vec<User> = rows
            .into_iter()
            .map(User::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Page::new(users, page.page, page.size, total as u64))
    }

    async fn find_by_role(&self, role: UserRole, page: PageRequest) -> ArcanaResult<Page<User>> {
        debug!("Finding users by role: {}", role);

        let role_str = role.to_string();

        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM users WHERE role = ? AND status != 'deleted'",
        )
        .bind(&role_str)
        .fetch_one(self.pool.inner())
        .await?;

        let rows = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT id, username, email, password_hash, first_name, last_name,
                   role, status, email_verified, avatar_url, last_login_at,
                   created_at, updated_at
            FROM users
            WHERE role = ? AND status != 'deleted'
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(&role_str)
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .fetch_all(self.pool.inner())
        .await?;

        let users: Vec<User> = rows
            .into_iter()
            .map(User::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Page::new(users, page.page, page.size, total as u64))
    }

    async fn save(&self, user: &User) -> ArcanaResult<User> {
        debug!("Saving new user: {}", user.username);

        let id_str = user.id.into_inner().to_string();

        // MySQL doesn't support RETURNING, so insert then select
        sqlx::query(
            r#"
            INSERT INTO users (id, username, email, password_hash, first_name, last_name,
                              role, status, email_verified, avatar_url, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id_str)
        .bind(&user.username)
        .bind(user.email.as_str())
        .bind(&user.password_hash)
        .bind(&user.first_name)
        .bind(&user.last_name)
        .bind(user.role.to_string())
        .bind(user.status.to_string())
        .bind(user.email_verified)
        .bind(&user.avatar_url)
        .bind(user.created_at)
        .bind(user.updated_at)
        .execute(self.pool.inner())
        .await?;

        // Fetch the inserted row
        self.find_by_id(user.id)
            .await?
            .ok_or_else(|| ArcanaError::Internal("Failed to fetch inserted user".to_string()))
    }

    async fn update(&self, user: &User) -> ArcanaResult<User> {
        debug!("Updating user: {}", user.id);

        let id_str = user.id.into_inner().to_string();

        // MySQL doesn't support RETURNING, so update then select
        sqlx::query(
            r#"
            UPDATE users
            SET username = ?, email = ?, password_hash = ?, first_name = ?,
                last_name = ?, role = ?, status = ?, email_verified = ?,
                avatar_url = ?, last_login_at = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&user.username)
        .bind(user.email.as_str())
        .bind(&user.password_hash)
        .bind(&user.first_name)
        .bind(&user.last_name)
        .bind(user.role.to_string())
        .bind(user.status.to_string())
        .bind(user.email_verified)
        .bind(&user.avatar_url)
        .bind(user.last_login_at)
        .bind(user.updated_at)
        .bind(&id_str)
        .execute(self.pool.inner())
        .await?;

        // Fetch the updated row
        self.find_by_id(user.id)
            .await?
            .ok_or_else(|| ArcanaError::Internal("Failed to fetch updated user".to_string()))
    }

    async fn delete(&self, id: UserId) -> ArcanaResult<bool> {
        debug!("Soft deleting user: {}", id);

        let result = sqlx::query(
            "UPDATE users SET status = 'deleted', updated_at = NOW() WHERE id = ?",
        )
        .bind(id.into_inner().to_string())
        .execute(self.pool.inner())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn count(&self) -> ArcanaResult<u64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE status != 'deleted'")
            .fetch_one(self.pool.inner())
            .await?;

        Ok(count as u64)
    }

    async fn count_by_role(&self, role: UserRole) -> ArcanaResult<u64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM users WHERE role = ? AND status != 'deleted'",
        )
        .bind(role.to_string())
        .fetch_one(self.pool.inner())
        .await?;

        Ok(count as u64)
    }
}

impl std::fmt::Debug for MySqlUserRepository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MySqlUserRepository").finish_non_exhaustive()
    }
}

// Type alias for backwards compatibility
pub type PgUserRepository = MySqlUserRepository;
