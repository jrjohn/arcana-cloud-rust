//! Remote repository adapters for layered deployments.
//!
//! - [`RemoteUserRepository`] implements [`crate::UserRepository`] by calling a
//!   repository tier over gRPC.
//! - [`RepositoryGrpcService`] exposes a local [`crate::UserRepository`] to
//!   remote service tiers over gRPC; [`RepositoryGrpcServer`] hosts it.

mod grpc_repository_server;
mod grpc_repository_service;
mod grpc_user_repository;

pub use grpc_repository_server::*;
pub use grpc_repository_service::*;
pub use grpc_user_repository::*;
