//! DAO implementations.
//!
//! Trait definitions live in the parent `dao/` module (e.g. `user_dao.rs`).
//! Implementations are organized by technology (mysql, postgres, grpc, etc.).

pub mod mysql;

pub use mysql::MySqlUserDaoImpl;
