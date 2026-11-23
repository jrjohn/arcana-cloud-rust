#!/usr/bin/env bash
# Generate ONE vertical feature slice the way the template's User feature is built:
#
#   entity (core) → DAO trait + MySQL DAO impl → repository trait + impl
#   → DTOs + mapper → service trait + impl → REST controller → DI + router + OpenAPI
#   + migration + unit tests at every layer.
#
#   tools/new-feature.sh Invoice            # PascalCase, singular
#   tools/new-feature.sh PurchaseOrder
#
# Fields: the slice starts with `name` + `description`. Add fields afterwards in the order
# entity → migration → DAO row → DTO → mapper (the compiler walks you through it).
# The slice is REST + monolithic. gRPC / layered deployment for the feature is a manual
# step (AGENTS.md §1): mirror proto/user_service.proto + crates/*-grpc/src/services/user_service.rs.
set -euo pipefail

[ $# -eq 1 ] || { echo "usage: $0 <FeatureName>   (PascalCase, singular, e.g. Invoice)" >&2; exit 1; }
P="$1"
[[ "$P" =~ ^[A-Z][a-zA-Z0-9]*$ ]] || { echo "FeatureName must be PascalCase (e.g. Invoice, PurchaseOrder)" >&2; exit 1; }
s="$(printf '%s' "$P" | perl -pe 's/(?<=[a-z0-9])([A-Z])/_$1/g; $_=lc')"   # PurchaseOrder → purchase_order
TABLE="${s}s"
URL="$(printf '%s' "$TABLE" | tr '_' '-')"
case "$s" in user|auth|health|job|jobs) echo "'$P' collides with a template feature" >&2; exit 1 ;; esac

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# Locate the layer crates by suffix: tools/new-app.sh renames the `arcana-` prefix.
crate_dir() {
    local d; d="$(ls -d crates/*-"$1" 2>/dev/null | head -1)"
    [ -n "$d" ] || { echo "new-feature: no crates/*-$1 crate found" >&2; exit 1; }
    echo "$d"
}
CORE_DIR="$(crate_dir core)"; REPO_DIR="$(crate_dir repository)"; SVC_DIR="$(crate_dir service)"
REST_DIR="$(crate_dir rest)"; SRV_DIR="$(crate_dir server)"
ident() { basename "$1" | tr '-' '_'; }
CORE="$(ident "$CORE_DIR")"; REPO="$(ident "$REPO_DIR")"; SVC="$(ident "$SVC_DIR")"

[ ! -e "$SVC_DIR/src/${s}_service.rs" ] || { echo "feature '$P' already exists ($SVC_DIR/src/${s}_service.rs)" >&2; exit 1; }

# render <dest>: stdin template → dest, with placeholders substituted.
render() {
    mkdir -p "$(dirname "$1")"
    sed -e "s/__P__/$P/g" -e "s/__s__/$s/g" -e "s/__TABLE__/$TABLE/g" -e "s/__URL__/$URL/g" \
        -e "s/__CORE__/$CORE/g" -e "s/__REPO__/$REPO/g" -e "s/__SVC__/$SVC/g" > "$1"
    echo "  + $1"
}
# insert_after <file> <perl-regex anchor> <text>: fails loudly when the anchor is missing,
# so a template change breaks this script instead of silently skipping a registration.
insert_after() {
    local f="$1" anchor="$2" text="$3"
    ANCHOR="$anchor" TEXT="$text" perl -0777 -i -pe '
        BEGIN { $n = 0 }
        $n += s/($ENV{ANCHOR})/$1$ENV{TEXT}/s;
        END { exit($n ? 0 : 3) }' "$f" \
        || { echo "new-feature: anchor not found in $f: $anchor — registration NOT done" >&2; exit 1; }
    echo "  ~ $f"
}
append() { printf '%s\n' "$2" >> "$1"; echo "  ~ $1"; }

echo "new-feature: $P (module ${s}, table ${TABLE}, route /api/v1/${URL})"

# ---------------------------------------------------------------- core: entity
render "$CORE_DIR/src/domain/entities/${s}.rs" <<'EOF'
//! `__P__` entity.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// `__P__` domain entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct __P__ {
    /// Unique identifier.
    pub id: Uuid,
    /// Display name.
    pub name: String,
    /// Optional free-text description.
    pub description: Option<String>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

impl __P__ {
    /// Creates a new `__P__` with a fresh id.
    #[must_use]
    pub fn new(name: String, description: Option<String>) -> Self {
        let now = Utc::now();
        Self { id: Uuid::new_v4(), name, description, created_at: now, updated_at: now }
    }

    /// Changes the editable fields and bumps `updated_at`.
    pub fn update(&mut self, name: Option<String>, description: Option<String>) {
        if let Some(name) = name {
            self.name = name;
        }
        if description.is_some() {
            self.description = description;
        }
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sets_fields_and_timestamps() {
        let e = __P__::new("first".to_string(), None);
        assert_eq!(e.name, "first");
        assert_eq!(e.created_at, e.updated_at);
    }

    #[test]
    fn update_changes_only_given_fields() {
        let mut e = __P__::new("first".to_string(), Some("d".to_string()));
        e.update(None, None);
        assert_eq!(e.name, "first");
        assert_eq!(e.description.as_deref(), Some("d"));
        e.update(Some("second".to_string()), None);
        assert_eq!(e.name, "second");
    }
}
EOF
append "$CORE_DIR/src/domain/entities/mod.rs" "mod ${s};
pub use ${s}::*;"

# ---------------------------------------------------------------- repository: DAO trait
render "$REPO_DIR/src/dao/${s}_dao.rs" <<'EOF'
//! `__P__Dao` trait — low-level `__P__` data access (one data source per implementation).

use __CORE__::{ArcanaResult, Interface, Page, PageRequest, __P__};
use async_trait::async_trait;
use uuid::Uuid;

/// Low-level `__P__` data access object. Coordinated by `__P__RepositoryImpl`.
#[async_trait]
pub trait __P__Dao: Interface + Send + Sync {
    /// Finds one by id.
    async fn find_by_id(&self, id: Uuid) -> ArcanaResult<Option<__P__>>;
    /// Lists with pagination.
    async fn find_all(&self, page: PageRequest) -> ArcanaResult<Page<__P__>>;
    /// Inserts a new row.
    async fn save(&self, entity: &__P__) -> ArcanaResult<__P__>;
    /// Updates an existing row.
    async fn update(&self, entity: &__P__) -> ArcanaResult<__P__>;
    /// Deletes by id. Returns `true` if a row was deleted.
    async fn delete(&self, id: Uuid) -> ArcanaResult<bool>;
}
EOF
insert_after "$REPO_DIR/src/dao/mod.rs" 'pub mod r#impl;\n' "pub mod ${s}_dao;
"
append "$REPO_DIR/src/dao/mod.rs" "pub use ${s}_dao::${P}Dao;
pub use r#impl::MySql${P}DaoImpl;"

# ---------------------------------------------------------------- repository: MySQL DAO impl
render "$REPO_DIR/src/dao/impl/mysql/${s}_dao_impl.rs" <<'EOF'
//! `MySQL` `__P__Dao` implementation (table `__TABLE__`, see `migrations/`).

use crate::sql_convert::{count_to_u64, to_sql_bigint};
use crate::{dao::__P__Dao, DatabasePoolInterface};
use __CORE__::{ArcanaError, ArcanaResult, Page, PageRequest, __P__};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shaku::Component;
use sqlx::FromRow;
use std::sync::Arc;
use uuid::Uuid;

/// `MySQL` implementation of [`__P__Dao`].
#[derive(Component, Clone)]
#[shaku(interface = __P__Dao)]
pub struct MySql__P__DaoImpl {
    #[shaku(inject)]
    pool: Arc<dyn DatabasePoolInterface>,
}

impl MySql__P__DaoImpl {
    /// Creates a new `MySql__P__DaoImpl`.
    #[must_use]
    pub fn new(pool: Arc<dyn DatabasePoolInterface>) -> Self {
        Self { pool }
    }
}

/// Database row of `__TABLE__`.
#[derive(Debug, FromRow)]
struct __P__Row {
    id: String,
    name: String,
    description: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<__P__Row> for __P__ {
    type Error = ArcanaError;

    fn try_from(row: __P__Row) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&row.id)
            .map_err(|e| ArcanaError::Internal(format!("Invalid UUID in database: {e}")))?;
        Ok(Self { id, name: row.name, description: row.description, created_at: row.created_at, updated_at: row.updated_at })
    }
}

#[async_trait]
impl __P__Dao for MySql__P__DaoImpl {
    async fn find_by_id(&self, id: Uuid) -> ArcanaResult<Option<__P__>> {
        let row = sqlx::query_as::<_, __P__Row>(
            "SELECT id, name, description, created_at, updated_at FROM __TABLE__ WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(self.pool.inner())
        .await?;
        row.map(__P__::try_from).transpose()
    }

    async fn find_all(&self, page: PageRequest) -> ArcanaResult<Page<__P__>> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM __TABLE__")
            .fetch_one(self.pool.inner())
            .await?;
        let rows = sqlx::query_as::<_, __P__Row>(
            "SELECT id, name, description, created_at, updated_at FROM __TABLE__ ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(to_sql_bigint(page.limit()))
        .bind(to_sql_bigint(page.offset()))
        .fetch_all(self.pool.inner())
        .await?;
        let items = rows.into_iter().map(__P__::try_from).collect::<ArcanaResult<Vec<_>>>()?;
        Ok(Page::new(items, page.page, page.size, count_to_u64(total)))
    }

    async fn save(&self, entity: &__P__) -> ArcanaResult<__P__> {
        sqlx::query(
            "INSERT INTO __TABLE__ (id, name, description, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(entity.id.to_string())
        .bind(&entity.name)
        .bind(&entity.description)
        .bind(entity.created_at)
        .bind(entity.updated_at)
        .execute(self.pool.inner())
        .await?;
        Ok(entity.clone())
    }

    async fn update(&self, entity: &__P__) -> ArcanaResult<__P__> {
        sqlx::query("UPDATE __TABLE__ SET name = ?, description = ?, updated_at = ? WHERE id = ?")
            .bind(&entity.name)
            .bind(&entity.description)
            .bind(entity.updated_at)
            .bind(entity.id.to_string())
            .execute(self.pool.inner())
            .await?;
        Ok(entity.clone())
    }

    async fn delete(&self, id: Uuid) -> ArcanaResult<bool> {
        let result = sqlx::query("DELETE FROM __TABLE__ WHERE id = ?")
            .bind(id.to_string())
            .execute(self.pool.inner())
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str) -> __P__Row {
        let now = Utc::now();
        __P__Row { id: id.to_string(), name: "n".to_string(), description: None, created_at: now, updated_at: now }
    }

    #[test]
    fn row_with_valid_uuid_converts() {
        let id = Uuid::new_v4();
        let entity = __P__::try_from(row(&id.to_string())).unwrap();
        assert_eq!(entity.id, id);
        assert_eq!(entity.name, "n");
    }

    #[test]
    fn row_with_invalid_uuid_is_an_error() {
        assert!(__P__::try_from(row("not-a-uuid")).is_err());
    }
}
EOF
append "$REPO_DIR/src/dao/impl/mysql/mod.rs" "
pub mod ${s}_dao_impl;
pub use ${s}_dao_impl::MySql${P}DaoImpl;"
append "$REPO_DIR/src/dao/impl/mod.rs" "pub use mysql::MySql${P}DaoImpl;"

# ---------------------------------------------------------------- repository: trait + impl
append "$REPO_DIR/src/traits.rs" "
/// \`${P}\` repository trait (domain-level access; coordinates one or more \`${P}Dao\`).
#[async_trait]
pub trait ${P}Repository: Interface + Send + Sync {
    /// Finds one by id.
    async fn find_by_id(&self, id: uuid::Uuid) -> ArcanaResult<Option<${CORE}::${P}>>;
    /// Lists with pagination.
    async fn find_all(&self, page: PageRequest) -> ArcanaResult<Page<${CORE}::${P}>>;
    /// Saves a new entity.
    async fn save(&self, entity: &${CORE}::${P}) -> ArcanaResult<${CORE}::${P}>;
    /// Updates an existing entity.
    async fn update(&self, entity: &${CORE}::${P}) -> ArcanaResult<${CORE}::${P}>;
    /// Deletes by id. Returns \`true\` if deleted.
    async fn delete(&self, id: uuid::Uuid) -> ArcanaResult<bool>;
}"

render "$REPO_DIR/src/impl/${s}_repository_impl.rs" <<'EOF'
//! `__P__RepositoryImpl` — implements [`__P__Repository`] by coordinating [`__P__Dao`]s.
//!
//! [`__P__Repository`]: crate::traits::__P__Repository
//! [`__P__Dao`]: crate::dao::__P__Dao

use crate::{dao::__P__Dao, traits::__P__Repository};
use __CORE__::{ArcanaResult, Page, PageRequest, __P__};
use async_trait::async_trait;
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

/// Repository implementation over a [`__P__Dao`].
#[derive(Component)]
#[shaku(interface = __P__Repository)]
pub struct __P__RepositoryImpl {
    #[shaku(inject)]
    __s___dao: Arc<dyn __P__Dao>,
}

impl __P__RepositoryImpl {
    /// Creates a new `__P__RepositoryImpl` with the given DAO.
    #[must_use]
    pub fn new(__s___dao: Arc<dyn __P__Dao>) -> Self {
        Self { __s___dao }
    }
}

#[async_trait]
impl __P__Repository for __P__RepositoryImpl {
    async fn find_by_id(&self, id: Uuid) -> ArcanaResult<Option<__P__>> {
        self.__s___dao.find_by_id(id).await
    }

    async fn find_all(&self, page: PageRequest) -> ArcanaResult<Page<__P__>> {
        self.__s___dao.find_all(page).await
    }

    async fn save(&self, entity: &__P__) -> ArcanaResult<__P__> {
        self.__s___dao.save(entity).await
    }

    async fn update(&self, entity: &__P__) -> ArcanaResult<__P__> {
        self.__s___dao.update(entity).await
    }

    async fn delete(&self, id: Uuid) -> ArcanaResult<bool> {
        self.__s___dao.delete(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    struct InMemory__P__Dao {
        rows: Mutex<HashMap<Uuid, __P__>>,
    }

    #[async_trait]
    impl __P__Dao for InMemory__P__Dao {
        async fn find_by_id(&self, id: Uuid) -> ArcanaResult<Option<__P__>> {
            Ok(self.rows.lock().unwrap().get(&id).cloned())
        }
        async fn find_all(&self, page: PageRequest) -> ArcanaResult<Page<__P__>> {
            let all: Vec<__P__> = self.rows.lock().unwrap().values().cloned().collect();
            let total = all.len() as u64;
            Ok(Page::new(all.into_iter().skip(page.offset()).take(page.limit()).collect(), page.page, page.size, total))
        }
        async fn save(&self, e: &__P__) -> ArcanaResult<__P__> {
            self.rows.lock().unwrap().insert(e.id, e.clone());
            Ok(e.clone())
        }
        async fn update(&self, e: &__P__) -> ArcanaResult<__P__> {
            self.save(e).await
        }
        async fn delete(&self, id: Uuid) -> ArcanaResult<bool> {
            Ok(self.rows.lock().unwrap().remove(&id).is_some())
        }
    }

    fn repo() -> __P__RepositoryImpl {
        __P__RepositoryImpl::new(Arc::new(InMemory__P__Dao::default()))
    }

    #[tokio::test]
    async fn save_then_find_by_id() {
        let repo = repo();
        let e = __P__::new("a".to_string(), None);
        repo.save(&e).await.unwrap();
        assert_eq!(repo.find_by_id(e.id).await.unwrap(), Some(e));
    }

    #[tokio::test]
    async fn find_all_pages_and_delete_removes() {
        let repo = repo();
        for i in 0..3 {
            repo.save(&__P__::new(format!("n{i}"), None)).await.unwrap();
        }
        let page = repo.find_all(PageRequest::new(0, 2)).await.unwrap();
        assert_eq!(page.content.len(), 2);
        assert_eq!(page.info.total_elements, 3);
        let id = page.content[0].id;
        assert!(repo.delete(id).await.unwrap());
        assert!(!repo.delete(id).await.unwrap());
    }
}
EOF
append "$REPO_DIR/src/impl/mod.rs" "
pub mod ${s}_repository_impl;
pub use ${s}_repository_impl::${P}RepositoryImpl;"
append "$REPO_DIR/src/lib.rs" "
// ${P} feature (generated by tools/new-feature.sh)
pub use dao::{${P}Dao, MySql${P}DaoImpl};
pub use r#impl::${P}RepositoryImpl;"

# ---------------------------------------------------------------- migration
TS="$(date -u +%Y%m%d%H%M%S)"
render "migrations/${TS}_create_${TABLE}_table.sql" <<'EOF'
-- Create __TABLE__ table (MySQL) — generated by tools/new-feature.sh
CREATE TABLE IF NOT EXISTS __TABLE__ (
    id CHAR(36) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE INDEX idx___TABLE___created_at ON __TABLE__(created_at DESC);
EOF

# ---------------------------------------------------------------- service: DTOs + mapper
render "$SVC_DIR/src/dto/${s}_dto.rs" <<'EOF'
//! `__P__` DTOs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Request to create a `__P__`.
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct Create__P__Request {
    #[validate(length(min = 1, max = 255, message = "Name must be 1-255 characters"))]
    pub name: String,
    #[validate(length(max = 2000))]
    pub description: Option<String>,
}

/// Request to update a `__P__` (absent fields are left unchanged).
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct Update__P__Request {
    #[validate(length(min = 1, max = 255, message = "Name must be 1-255 characters"))]
    pub name: Option<String>,
    #[validate(length(max = 2000))]
    pub description: Option<String>,
}

/// `__P__` returned to clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct __P__Response {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Page of `__P__Response`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct __P__ListResponse {
    pub items: Vec<__P__Response>,
    pub page: usize,
    pub size: usize,
    pub total_elements: u64,
    pub total_pages: u64,
}
EOF
append "$SVC_DIR/src/dto/mod.rs" "mod ${s}_dto;
pub use ${s}_dto::*;"

render "$SVC_DIR/src/mappers/${s}_mapper.rs" <<'EOF'
//! `__P__` entity → DTO mapping.

use crate::dto::{__P__ListResponse, __P__Response};
use __CORE__::{Page, __P__};

impl From<__P__> for __P__Response {
    fn from(e: __P__) -> Self {
        Self { id: e.id, name: e.name, description: e.description, created_at: e.created_at, updated_at: e.updated_at }
    }
}

impl From<Page<__P__>> for __P__ListResponse {
    fn from(page: Page<__P__>) -> Self {
        Self {
            items: page.content.into_iter().map(__P__Response::from).collect(),
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

    #[test]
    fn entity_maps_to_response() {
        let e = __P__::new("a".to_string(), Some("d".to_string()));
        let r = __P__Response::from(e.clone());
        assert_eq!(r.id, e.id);
        assert_eq!(r.description.as_deref(), Some("d"));
    }

    #[test]
    fn page_maps_to_list_response() {
        let page = Page::new(vec![__P__::new("a".to_string(), None)], 0, 10, 1);
        let r = __P__ListResponse::from(page);
        assert_eq!(r.items.len(), 1);
        assert_eq!(r.total_elements, 1);
    }
}
EOF
append "$SVC_DIR/src/mappers/mod.rs" "
mod ${s}_mapper;"

# ---------------------------------------------------------------- service: trait + impl
render "$SVC_DIR/src/${s}_service.rs" <<'EOF'
//! `__P__Service` trait definition.

use crate::dto::{Create__P__Request, Update__P__Request, __P__ListResponse, __P__Response};
use __CORE__::{ArcanaResult, Interface, PageRequest};
use async_trait::async_trait;
use uuid::Uuid;

/// `__P__` use cases. Controllers depend on this trait, never on `__P__ServiceImpl`.
#[async_trait]
pub trait __P__Service: Interface + Send + Sync {
    /// Creates a `__P__`.
    async fn create(&self, request: Create__P__Request) -> ArcanaResult<__P__Response>;
    /// Gets one by id (`NotFound` if absent).
    async fn get(&self, id: Uuid) -> ArcanaResult<__P__Response>;
    /// Lists with pagination.
    async fn list(&self, page: PageRequest) -> ArcanaResult<__P__ListResponse>;
    /// Updates one by id (`NotFound` if absent).
    async fn update(&self, id: Uuid, request: Update__P__Request) -> ArcanaResult<__P__Response>;
    /// Deletes one by id (`NotFound` if absent).
    async fn delete(&self, id: Uuid) -> ArcanaResult<()>;
}
EOF
render "$SVC_DIR/src/impl/${s}_service_impl.rs" <<'EOF'
//! `__P__ServiceImpl` — business rules for `__P__`.

use crate::dto::{Create__P__Request, Update__P__Request, __P__ListResponse, __P__Response};
use crate::__s___service::__P__Service;
use __CORE__::{ArcanaError, ArcanaResult, PageRequest, ValidateExt, __P__};
use __REPO__::__P__Repository;
use async_trait::async_trait;
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

/// Service implementation, injected with the repository **interface**.
#[derive(Component)]
#[shaku(interface = __P__Service)]
pub struct __P__ServiceImpl {
    #[shaku(inject)]
    repository: Arc<dyn __P__Repository>,
}

impl __P__ServiceImpl {
    /// Creates a new `__P__ServiceImpl`.
    #[must_use]
    pub fn new(repository: Arc<dyn __P__Repository>) -> Self {
        Self { repository }
    }

    async fn load(&self, id: Uuid) -> ArcanaResult<__P__> {
        self.repository.find_by_id(id).await?.ok_or_else(|| ArcanaError::not_found("__P__", id))
    }
}

#[async_trait]
impl __P__Service for __P__ServiceImpl {
    async fn create(&self, request: Create__P__Request) -> ArcanaResult<__P__Response> {
        request.validate_request()?;
        let entity = __P__::new(request.name, request.description);
        Ok(self.repository.save(&entity).await?.into())
    }

    async fn get(&self, id: Uuid) -> ArcanaResult<__P__Response> {
        Ok(self.load(id).await?.into())
    }

    async fn list(&self, page: PageRequest) -> ArcanaResult<__P__ListResponse> {
        Ok(self.repository.find_all(page).await?.into())
    }

    async fn update(&self, id: Uuid, request: Update__P__Request) -> ArcanaResult<__P__Response> {
        request.validate_request()?;
        let mut entity = self.load(id).await?;
        entity.update(request.name, request.description);
        Ok(self.repository.update(&entity).await?.into())
    }

    async fn delete(&self, id: Uuid) -> ArcanaResult<()> {
        if self.repository.delete(id).await? {
            Ok(())
        } else {
            Err(ArcanaError::not_found("__P__", id))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use __CORE__::Page;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    struct InMemory__P__Repository {
        rows: Mutex<HashMap<Uuid, __P__>>,
    }

    #[async_trait]
    impl __P__Repository for InMemory__P__Repository {
        async fn find_by_id(&self, id: Uuid) -> ArcanaResult<Option<__P__>> {
            Ok(self.rows.lock().unwrap().get(&id).cloned())
        }
        async fn find_all(&self, page: PageRequest) -> ArcanaResult<Page<__P__>> {
            let all: Vec<__P__> = self.rows.lock().unwrap().values().cloned().collect();
            let total = all.len() as u64;
            Ok(Page::new(all, page.page, page.size, total))
        }
        async fn save(&self, e: &__P__) -> ArcanaResult<__P__> {
            self.rows.lock().unwrap().insert(e.id, e.clone());
            Ok(e.clone())
        }
        async fn update(&self, e: &__P__) -> ArcanaResult<__P__> {
            self.save(e).await
        }
        async fn delete(&self, id: Uuid) -> ArcanaResult<bool> {
            Ok(self.rows.lock().unwrap().remove(&id).is_some())
        }
    }

    fn service() -> __P__ServiceImpl {
        __P__ServiceImpl::new(Arc::new(InMemory__P__Repository::default()))
    }

    fn create_request(name: &str) -> Create__P__Request {
        Create__P__Request { name: name.to_string(), description: None }
    }

    #[tokio::test]
    async fn create_then_get() {
        let svc = service();
        let created = svc.create(create_request("a")).await.unwrap();
        assert_eq!(svc.get(created.id).await.unwrap(), created);
    }

    #[tokio::test]
    async fn create_rejects_empty_name() {
        assert!(matches!(service().create(create_request("")).await, Err(ArcanaError::Validation(_))));
    }

    #[tokio::test]
    async fn get_missing_is_not_found() {
        assert!(matches!(service().get(Uuid::new_v4()).await, Err(ArcanaError::NotFound { .. })));
    }

    #[tokio::test]
    async fn update_changes_name_and_list_sees_it() {
        let svc = service();
        let created = svc.create(create_request("a")).await.unwrap();
        let req = Update__P__Request { name: Some("b".to_string()), description: None };
        assert_eq!(svc.update(created.id, req).await.unwrap().name, "b");
        assert_eq!(svc.list(PageRequest::new(0, 10)).await.unwrap().items[0].name, "b");
    }

    #[tokio::test]
    async fn delete_twice_second_is_not_found() {
        let svc = service();
        let created = svc.create(create_request("a")).await.unwrap();
        svc.delete(created.id).await.unwrap();
        assert!(svc.delete(created.id).await.is_err());
    }
}
EOF
append "$SVC_DIR/src/impl/mod.rs" "
pub mod ${s}_service_impl;
pub use ${s}_service_impl::${P}ServiceImpl;"
append "$SVC_DIR/src/lib.rs" "
// ${P} feature (generated by tools/new-feature.sh)
pub mod ${s}_service;
pub use ${s}_service::*;
pub use r#impl::${P}ServiceImpl;"

# ---------------------------------------------------------------- rest: controller
render "$REST_DIR/src/controllers/${s}_controller.rs" <<'EOF'
//! `__P__` REST controller — delegation only: extract, call `__P__Service`, wrap the response.
//!
//! The router carries its own state (`Arc<dyn __P__Service>`) and is mounted under
//! `/api/v1/__URL__` (behind the auth middleware) in `router.rs`.

use crate::{
    extractors::{AuthenticatedUser, PaginationQuery, ValidatedJson},
    responses::{created, no_content, ok, ApiResponse, ApiResult, AppError},
};
use __CORE__::ArcanaError;
use __SVC__::{Create__P__Request, Update__P__Request, __P__ListResponse, __P__Response, __P__Service};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use std::sync::Arc;
use uuid::Uuid;

/// State of this router: the service interface, resolved by the DI module.
pub type __P__State = Arc<dyn __P__Service>;

/// Creates the `__P__` router.
pub fn router() -> Router<__P__State> {
    Router::new()
        .route("/", get(list___s__).post(create___s__))
        .route("/{id}", get(get___s__).put(update___s__).delete(delete___s__))
}

fn parse_id(id: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(id).map_err(|_| AppError(ArcanaError::validation("Invalid __s__ id")))
}

/// List `__P__` items.
///
/// # Errors
///
/// Returns an [`AppError`] from the service.
#[utoipa::path(get, path = "/__URL__", tag = "__URL__", security(("bearer_auth" = [])),
    responses((status = 200, description = "Page of __P__", body = __P__ListResponse)))]
pub async fn list___s__(
    State(service): State<__P__State>,
    _user: AuthenticatedUser,
    Query(pagination): Query<PaginationQuery>,
) -> ApiResult<__P__ListResponse> {
    ok(service.list(pagination.into()).await?)
}

/// Create a `__P__`.
///
/// # Errors
///
/// Returns an [`AppError`] on validation failure or from the service.
#[utoipa::path(post, path = "/__URL__", tag = "__URL__", request_body = Create__P__Request,
    security(("bearer_auth" = [])), responses((status = 201, description = "Created", body = __P__Response)))]
pub async fn create___s__(
    State(service): State<__P__State>,
    _user: AuthenticatedUser,
    ValidatedJson(request): ValidatedJson<Create__P__Request>,
) -> Result<(StatusCode, Json<ApiResponse<__P__Response>>), AppError> {
    Ok(created(service.create(request).await?))
}

/// Get a `__P__` by id.
///
/// # Errors
///
/// Returns an [`AppError`] for an invalid id or when not found.
#[utoipa::path(get, path = "/__URL__/{id}", tag = "__URL__", params(("id" = String, Path, description = "UUID")),
    security(("bearer_auth" = [])), responses((status = 200, description = "Found", body = __P__Response)))]
pub async fn get___s__(
    State(service): State<__P__State>,
    _user: AuthenticatedUser,
    Path(id): Path<String>,
) -> ApiResult<__P__Response> {
    ok(service.get(parse_id(&id)?).await?)
}

/// Update a `__P__`.
///
/// # Errors
///
/// Returns an [`AppError`] for an invalid id, validation failure, or when not found.
#[utoipa::path(put, path = "/__URL__/{id}", tag = "__URL__", params(("id" = String, Path, description = "UUID")),
    request_body = Update__P__Request, security(("bearer_auth" = [])),
    responses((status = 200, description = "Updated", body = __P__Response)))]
pub async fn update___s__(
    State(service): State<__P__State>,
    _user: AuthenticatedUser,
    Path(id): Path<String>,
    ValidatedJson(request): ValidatedJson<Update__P__Request>,
) -> ApiResult<__P__Response> {
    ok(service.update(parse_id(&id)?, request).await?)
}

/// Delete a `__P__`.
///
/// # Errors
///
/// Returns an [`AppError`] for an invalid id or when not found.
#[utoipa::path(delete, path = "/__URL__/{id}", tag = "__URL__", params(("id" = String, Path, description = "UUID")),
    security(("bearer_auth" = [])), responses((status = 204, description = "Deleted")))]
pub async fn delete___s__(
    State(service): State<__P__State>,
    _user: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    service.delete(parse_id(&id)?).await?;
    Ok(no_content())
}

#[cfg(test)]
mod tests {
    use super::*;
    use __CORE__::{ArcanaResult, PageRequest};
    use async_trait::async_trait;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    struct Stub__P__Service;

    #[async_trait]
    impl __P__Service for Stub__P__Service {
        async fn create(&self, r: Create__P__Request) -> ArcanaResult<__P__Response> {
            Ok(__P__Response { id: Uuid::nil(), name: r.name, description: r.description, created_at: chrono::Utc::now(), updated_at: chrono::Utc::now() })
        }
        async fn get(&self, id: Uuid) -> ArcanaResult<__P__Response> {
            Err(ArcanaError::not_found("__P__", id))
        }
        async fn list(&self, page: PageRequest) -> ArcanaResult<__P__ListResponse> {
            Ok(__P__ListResponse { items: vec![], page: page.page, size: page.size, total_elements: 0, total_pages: 0 })
        }
        async fn update(&self, id: Uuid, _r: Update__P__Request) -> ArcanaResult<__P__Response> {
            Err(ArcanaError::not_found("__P__", id))
        }
        async fn delete(&self, _id: Uuid) -> ArcanaResult<()> {
            Ok(())
        }
    }

    fn app() -> Router {
        router().with_state(Arc::new(Stub__P__Service) as __P__State)
    }

    fn authed(method: &str, uri: &str) -> Request<Body> {
        let claims = __SECURITY__::Claims::new_access(
            __CORE__::UserId::new(), "u".to_string(), "u@example.com".to_string(),
            __CORE__::UserRole::User, "iss".to_string(), "aud".to_string(),
            chrono::Utc::now() + chrono::Duration::hours(1),
        );
        let mut req = Request::builder().method(method).uri(uri)
            .header("authorization", "Bearer test").body(Body::empty()).unwrap();
        req.extensions_mut().insert(claims);
        req
    }

    #[tokio::test]
    async fn list_without_token_is_unauthorized() {
        let res = app().oneshot(Request::builder().uri("/").body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn list_with_token_is_ok() {
        let res = app().oneshot(authed("GET", "/")).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn get_with_bad_id_is_rejected_and_missing_is_not_found() {
        let res = app().oneshot(authed("GET", "/not-a-uuid")).await.unwrap();
        assert!(res.status().is_client_error());
        let res = app().oneshot(authed("GET", &format!("/{}", Uuid::new_v4()))).await.unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn delete_is_no_content() {
        let res = app().oneshot(authed("DELETE", &format!("/{}", Uuid::new_v4()))).await.unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);
    }
}
EOF
SECURITY="$(ident "$(crate_dir security)")"
sed -i.bak "s/__SECURITY__/$SECURITY/g" "$REST_DIR/src/controllers/${s}_controller.rs" && rm -f "$REST_DIR/src/controllers/${s}_controller.rs.bak"
append "$REST_DIR/src/controllers/mod.rs" "pub mod ${s}_controller;"

# ---------------------------------------------------------------- rest: router + OpenAPI
insert_after "$REST_DIR/src/router.rs" '\+ HasComponent<dyn TokenProviderInterface>' "
        + HasComponent<dyn ${SVC}::${P}Service>"
insert_after "$REST_DIR/src/router.rs" '\.nest\("/jobs", jobs_controller::router\(\)\)' "
        .nest(\"/${URL}\", crate::controllers::${s}_controller::router().with_state(module.resolve()))"
insert_after "$REST_DIR/src/openapi.rs" 'crate::controllers::health_controller::liveness_check,' "
        crate::controllers::${s}_controller::list_${s},
        crate::controllers::${s}_controller::create_${s},
        crate::controllers::${s}_controller::get_${s},
        crate::controllers::${s}_controller::update_${s},
        crate::controllers::${s}_controller::delete_${s},"
insert_after "$REST_DIR/src/openapi.rs" 'UserListResponse,(?=\s*\)\s*\),\s*modifiers)' "
            ${SVC}::Create${P}Request,
            ${SVC}::Update${P}Request,
            ${SVC}::${P}Response,
            ${SVC}::${P}ListResponse,"

# ---------------------------------------------------------------- server: DI container
DI="$SRV_DIR/src/di.rs"
insert_after "$DI" '\nuse std::sync::Arc;\n' "use ${REPO}::{MySql${P}DaoImpl, ${P}RepositoryImpl};
use ${SVC}::${P}ServiceImpl;
"
insert_after "$DI" 'mod shaku_modules \{\n    use super::\{.*?\};\n' "    use super::{MySql${P}DaoImpl, ${P}RepositoryImpl, ${P}ServiceImpl};
"
insert_after "$DI" 'pub MonolithicModule \{\n\s*components = \[\n' "                MySql${P}DaoImpl,
                ${P}RepositoryImpl,
                ${P}ServiceImpl,
"

echo
echo "new-feature: $P generated. Next:"
echo "  cargo test --workspace --lib --bins && cargo clippy --workspace --lib --bins -- -D warnings"
echo "  tools/arch-check.sh"
echo "Monolithic mode only: gRPC/layered wiring for $P is manual (AGENTS.md §1)."
