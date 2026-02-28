//! # Arcana Service
//!
//! Business logic service layer for Arcana Cloud Rust.
//! Contains use cases and application services.
//!
//! ## Structure
//!
//! ```text
//! src/
//!   user_service.rs          ← UserService trait
//!   impl/
//!     mod.rs                 ← pub use declarations
//!     user_service_impl.rs   ← UserServiceImpl + UserServiceComponent
//! ```

pub mod cache;
pub mod dto;
pub mod mappers;
pub mod user_service;
pub mod auth_service;
pub mod r#impl;

pub use cache::*;
pub use dto::*;
pub use user_service::*;
pub use auth_service::*;
pub use r#impl::{UserServiceComponent, UserServiceImpl};
