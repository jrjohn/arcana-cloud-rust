//! Result type aliases for Arcana Cloud.

use crate::ArcanaError;

/// A specialized `Result` type for Arcana operations.
pub type ArcanaResult<T> = Result<T, ArcanaError>;

/// A boxed future returning an `ArcanaResult`.
pub type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = ArcanaResult<T>> + Send + 'a>>;
