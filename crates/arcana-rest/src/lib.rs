//! # Arcana REST
//!
//! REST API layer using Axum for Arcana Cloud Rust.
//! Provides HTTP endpoints for user management, authentication, and health checks.

pub mod controllers;
pub mod extractors;
pub mod middleware;
pub mod responses;
pub mod router;
pub mod state;

pub use router::*;
pub use state::*;
