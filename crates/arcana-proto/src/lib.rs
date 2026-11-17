//! # Arcana Proto
//!
//! Generated protobuf / tonic code shared by every gRPC tier, plus the TLS
//! helpers used to build gRPC transports. Holding these in their own crate
//! lets both the gRPC presentation layer (`arcana-grpc`) and the remote
//! repository adapters (`arcana-repository::remote`) use them without a
//! dependency cycle.

pub mod tls;

pub use tls::{TlsConfigBuilder, build_client_tls_from_ca, build_client_tls_from_config};

/// Common types used across services.
#[allow(clippy::pedantic, reason = "tonic-build output, regenerated on every build; not hand-written code")]
pub mod common {
    include!(concat!(env!("OUT_DIR"), "/arcana.common.rs"));
}

/// User service definitions.
#[allow(clippy::pedantic, reason = "tonic-build output, regenerated on every build; not hand-written code")]
pub mod user {
    include!(concat!(env!("OUT_DIR"), "/arcana.user.rs"));
}

/// Authentication service definitions.
#[allow(clippy::pedantic, reason = "tonic-build output, regenerated on every build; not hand-written code")]
pub mod auth {
    include!(concat!(env!("OUT_DIR"), "/arcana.auth.rs"));
}

/// Health check service definitions.
#[allow(clippy::pedantic, reason = "tonic-build output, regenerated on every build; not hand-written code")]
pub mod health {
    include!(concat!(env!("OUT_DIR"), "/arcana.health.rs"));
}

/// Repository service definitions for layered deployment.
#[allow(clippy::pedantic, reason = "tonic-build output, regenerated on every build; not hand-written code")]
pub mod repository {
    include!(concat!(env!("OUT_DIR"), "/arcana.repository.rs"));
}

/// Job queue service definitions.
#[allow(clippy::pedantic, reason = "tonic-build output, regenerated on every build; not hand-written code")]
pub mod jobs {
    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/arcana.jobs.v1.rs"));
    }
}
