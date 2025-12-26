//! Deployment mode configuration.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Deployment mode for the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DeploymentMode {
    /// All layers in a single process.
    #[default]
    Monolithic,
    /// Distributed layers with HTTP communication.
    LayeredHttp,
    /// Distributed layers with gRPC communication.
    LayeredGrpc,
    /// Kubernetes deployment with HTTP.
    KubernetesHttp,
    /// Kubernetes deployment with gRPC.
    KubernetesGrpc,
}

impl DeploymentMode {
    /// Returns true if this is a monolithic deployment.
    #[must_use]
    pub const fn is_monolithic(&self) -> bool {
        matches!(self, Self::Monolithic)
    }

    /// Returns true if this is a layered deployment.
    #[must_use]
    pub const fn is_layered(&self) -> bool {
        matches!(self, Self::LayeredHttp | Self::LayeredGrpc)
    }

    /// Returns true if this is a Kubernetes deployment.
    #[must_use]
    pub const fn is_kubernetes(&self) -> bool {
        matches!(self, Self::KubernetesHttp | Self::KubernetesGrpc)
    }

    /// Returns true if gRPC is the inter-service protocol.
    #[must_use]
    pub const fn uses_grpc(&self) -> bool {
        matches!(self, Self::LayeredGrpc | Self::KubernetesGrpc)
    }
}

impl fmt::Display for DeploymentMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Monolithic => write!(f, "monolithic"),
            Self::LayeredHttp => write!(f, "layered-http"),
            Self::LayeredGrpc => write!(f, "layered-grpc"),
            Self::KubernetesHttp => write!(f, "kubernetes-http"),
            Self::KubernetesGrpc => write!(f, "kubernetes-grpc"),
        }
    }
}

/// Layer type for distributed deployments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DeploymentLayer {
    /// All layers (for monolithic deployment).
    #[default]
    All,
    /// Controller/Presentation layer only.
    Controller,
    /// Service/Application layer only.
    Service,
    /// Repository/Infrastructure layer only.
    Repository,
}

impl DeploymentLayer {
    /// Returns true if this layer includes the controller.
    #[must_use]
    pub const fn has_controller(&self) -> bool {
        matches!(self, Self::All | Self::Controller)
    }

    /// Returns true if this layer includes the service.
    #[must_use]
    pub const fn has_service(&self) -> bool {
        matches!(self, Self::All | Self::Service)
    }

    /// Returns true if this layer includes the repository.
    #[must_use]
    pub const fn has_repository(&self) -> bool {
        matches!(self, Self::All | Self::Repository)
    }
}

impl fmt::Display for DeploymentLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::All => write!(f, "all"),
            Self::Controller => write!(f, "controller"),
            Self::Service => write!(f, "service"),
            Self::Repository => write!(f, "repository"),
        }
    }
}

/// Communication protocol for inter-service communication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CommunicationProtocol {
    /// HTTP/REST protocol.
    #[default]
    Http,
    /// gRPC protocol.
    Grpc,
}

impl fmt::Display for CommunicationProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http => write!(f, "http"),
            Self::Grpc => write!(f, "grpc"),
        }
    }
}
