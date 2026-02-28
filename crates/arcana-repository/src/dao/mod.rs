//! DAO (Data Access Object) layer.
//!
//! DAOs provide low-level, single-source data access abstractions.
//! Each DAO interface maps to one data source (MySQL, gRPC, REST, etc.).
//!
//! ## Structure
//!
//! ```text
//! dao/
//!   user_dao.rs              ← UserDao trait
//!   impl/
//!     mod.rs                 ← pub use declarations
//!     mysql/
//!       user_dao_impl.rs     ← MySqlUserDaoImpl
//! ```
//!
//! Hierarchy:
//! ```text
//! Service → Repository (interface + impl) → DAO (interface + impl) → DB/API
//! ```

pub mod user_dao;
pub mod r#impl;

pub use user_dao::UserDao;
pub use r#impl::MySqlUserDaoImpl;
