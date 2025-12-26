//! # Arcana gRPC
//!
//! gRPC service layer using Tonic for Arcana Cloud Rust.
//! Provides gRPC endpoints for user management, authentication, and health checks.
//!
//! Also includes gRPC clients for inter-layer communication in distributed deployments.

pub mod clients;
pub mod interceptors;
pub mod proto;
pub mod server;
pub mod services;

pub use clients::*;
pub use server::*;
pub use services::*;
