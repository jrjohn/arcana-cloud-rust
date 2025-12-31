//! # Arcana Core
//!
//! Core types, traits, and error definitions for Arcana Cloud Rust.
//! This crate provides the foundational abstractions used across all layers
//! of the Clean Architecture implementation.
//!
//! This crate also includes the domain module containing entities, value objects,
//! and domain events.

pub mod domain;
pub mod error;
pub mod id;
pub mod pagination;
pub mod result;
pub mod telemetry;
pub mod traits;
pub mod validation;

pub use domain::*;
pub use error::*;
pub use id::*;
pub use pagination::*;
pub use result::*;
pub use traits::*;
pub use validation::*;

// Re-export shaku for dependency injection
pub use shaku::{module, Component, HasComponent, Interface, Module, ModuleBuildContext};
