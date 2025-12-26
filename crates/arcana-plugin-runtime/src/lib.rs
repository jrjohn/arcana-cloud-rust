//! # Arcana Plugin Runtime
//!
//! Plugin runtime for Arcana Cloud Rust using Wasmtime.
//! Manages loading, execution, and lifecycle of WASM plugins.

pub mod manager;

pub use manager::*;
