//! Repository layer implementations.
//!
//! Trait definitions live in the parent module (`traits.rs`).
//! This module contains concrete structs that implement those traits.

pub mod user_repository_impl;

pub use user_repository_impl::UserRepositoryImpl;
