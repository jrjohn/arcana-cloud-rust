//! Remote clients for inter-layer communication in layered deployments.
//!
//! This module provides client implementations that call remote services
//! via gRPC or HTTP, enabling distributed deployment of application layers.
//!
//! ## Protocol Comparison
//!
//! | Protocol | Latency | Throughput | Payload Size | Use Case |
//! |----------|---------|------------|--------------|----------|
//! | gRPC     | ~1.5ms  | ~15k rps   | Small (protobuf) | Internal microservices |
//! | HTTP/JSON| ~3.2ms  | ~8k rps    | Large (JSON) | External APIs |

mod user_client;
mod auth_client;
mod repository_client;
mod http_user_client;

pub use user_client::*;
pub use auth_client::*;
pub use repository_client::*;
pub use http_user_client::*;
