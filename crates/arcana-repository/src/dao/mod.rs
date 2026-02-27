//! DAO (Data Access Object) layer.
//!
//! DAOs provide low-level, single-source data access abstractions.
//! Each DAO interface maps to one data source (MySQL, gRPC, REST, etc.).
//!
//! Hierarchy:
//! ```text
//! Service → Repository (interface + impl) → DAO (interface + impl) → DB/API
//! ```

pub mod user_dao;

pub use user_dao::UserDao;
