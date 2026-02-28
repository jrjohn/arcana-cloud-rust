//! User service implementations.
//!
//! This module contains the concrete implementations of service traits.
//! Trait definitions live in the parent module (e.g. `user_service.rs`).

pub mod user_service_impl;

pub use user_service_impl::{UserServiceComponent, UserServiceImpl};
