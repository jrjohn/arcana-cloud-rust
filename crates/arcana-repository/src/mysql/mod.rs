//! MySQL repository implementations (backward-compatible).
//!
//! `MySqlUserDaoImpl` has moved to `dao/impl/mysql/user_dao_impl.rs` per the
//! unified Arcana impl/interface standard. It remains accessible as
//! `arcana_repository::MySqlUserDaoImpl` via the top-level re-export.

mod user_repository;

pub use user_repository::MySqlUserRepository;
pub use user_repository::PgUserRepository;
