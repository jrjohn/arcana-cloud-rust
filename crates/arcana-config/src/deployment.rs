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
    /// Background job worker only (no inbound API surface).
    Worker,
    /// Cron scheduler only (leader-elected, exactly one active instance).
    Scheduler,
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

    /// Returns true if this layer is a background job role.
    ///
    /// Job roles never serve the public API and never open a database pool,
    /// so they are dispatched before the deployment-mode branch.
    #[must_use]
    pub const fn is_job_role(&self) -> bool {
        matches!(self, Self::Worker | Self::Scheduler)
    }

    /// Returns true if this layer may be scaled horizontally on demand.
    ///
    /// `Repository` holds the only database pool, so replica count multiplies
    /// database connections; `Scheduler` elects a single leader, so extra
    /// replicas are standby only. Neither should be attached to an HPA.
    #[must_use]
    pub const fn is_horizontally_scalable(&self) -> bool {
        matches!(self, Self::Controller | Self::Service | Self::Worker)
    }
}

impl fmt::Display for DeploymentLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::All => write!(f, "all"),
            Self::Controller => write!(f, "controller"),
            Self::Service => write!(f, "service"),
            Self::Repository => write!(f, "repository"),
            Self::Worker => write!(f, "worker"),
            Self::Scheduler => write!(f, "scheduler"),
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

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // DeploymentMode tests
    // =========================================================================

    #[test]
    fn test_deployment_mode_default_is_monolithic() {
        let mode = DeploymentMode::default();
        assert_eq!(mode, DeploymentMode::Monolithic);
    }

    #[test]
    fn test_deployment_mode_is_monolithic() {
        assert!(DeploymentMode::Monolithic.is_monolithic());
        assert!(!DeploymentMode::LayeredHttp.is_monolithic());
        assert!(!DeploymentMode::LayeredGrpc.is_monolithic());
        assert!(!DeploymentMode::KubernetesHttp.is_monolithic());
        assert!(!DeploymentMode::KubernetesGrpc.is_monolithic());
    }

    #[test]
    fn test_deployment_mode_is_layered() {
        assert!(!DeploymentMode::Monolithic.is_layered());
        assert!(DeploymentMode::LayeredHttp.is_layered());
        assert!(DeploymentMode::LayeredGrpc.is_layered());
        assert!(!DeploymentMode::KubernetesHttp.is_layered());
        assert!(!DeploymentMode::KubernetesGrpc.is_layered());
    }

    #[test]
    fn test_deployment_mode_is_kubernetes() {
        assert!(!DeploymentMode::Monolithic.is_kubernetes());
        assert!(!DeploymentMode::LayeredHttp.is_kubernetes());
        assert!(!DeploymentMode::LayeredGrpc.is_kubernetes());
        assert!(DeploymentMode::KubernetesHttp.is_kubernetes());
        assert!(DeploymentMode::KubernetesGrpc.is_kubernetes());
    }

    #[test]
    fn test_deployment_mode_uses_grpc() {
        assert!(!DeploymentMode::Monolithic.uses_grpc());
        assert!(!DeploymentMode::LayeredHttp.uses_grpc());
        assert!(DeploymentMode::LayeredGrpc.uses_grpc());
        assert!(!DeploymentMode::KubernetesHttp.uses_grpc());
        assert!(DeploymentMode::KubernetesGrpc.uses_grpc());
    }

    #[test]
    fn test_deployment_mode_display() {
        assert_eq!(DeploymentMode::Monolithic.to_string(), "monolithic");
        assert_eq!(DeploymentMode::LayeredHttp.to_string(), "layered-http");
        assert_eq!(DeploymentMode::LayeredGrpc.to_string(), "layered-grpc");
        assert_eq!(DeploymentMode::KubernetesHttp.to_string(), "kubernetes-http");
        assert_eq!(DeploymentMode::KubernetesGrpc.to_string(), "kubernetes-grpc");
    }

    #[test]
    fn test_deployment_mode_serialization() {
        let json = serde_json::to_string(&DeploymentMode::Monolithic).unwrap();
        assert_eq!(json, "\"monolithic\"");

        let json = serde_json::to_string(&DeploymentMode::LayeredGrpc).unwrap();
        assert_eq!(json, "\"layeredgrpc\"");
    }

    #[test]
    fn test_deployment_mode_deserialization() {
        let mode: DeploymentMode = serde_json::from_str("\"monolithic\"").unwrap();
        assert_eq!(mode, DeploymentMode::Monolithic);

        let mode: DeploymentMode = serde_json::from_str("\"layeredhttp\"").unwrap();
        assert_eq!(mode, DeploymentMode::LayeredHttp);
    }

    #[test]
    fn test_deployment_mode_roundtrip_serialization() {
        let modes = [
            DeploymentMode::Monolithic,
            DeploymentMode::LayeredHttp,
            DeploymentMode::LayeredGrpc,
            DeploymentMode::KubernetesHttp,
            DeploymentMode::KubernetesGrpc,
        ];
        for mode in &modes {
            let json = serde_json::to_string(mode).unwrap();
            let parsed: DeploymentMode = serde_json::from_str(&json).unwrap();
            assert_eq!(*mode, parsed, "Roundtrip failed for {:?}", mode);
        }
    }

    // =========================================================================
    // DeploymentLayer tests
    // =========================================================================

    #[test]
    fn test_deployment_layer_default_is_all() {
        let layer = DeploymentLayer::default();
        assert_eq!(layer, DeploymentLayer::All);
    }

    #[test]
    fn test_deployment_layer_has_controller() {
        assert!(DeploymentLayer::All.has_controller());
        assert!(DeploymentLayer::Controller.has_controller());
        assert!(!DeploymentLayer::Service.has_controller());
        assert!(!DeploymentLayer::Repository.has_controller());
    }

    #[test]
    fn test_deployment_layer_has_service() {
        assert!(DeploymentLayer::All.has_service());
        assert!(!DeploymentLayer::Controller.has_service());
        assert!(DeploymentLayer::Service.has_service());
        assert!(!DeploymentLayer::Repository.has_service());
    }

    #[test]
    fn test_deployment_layer_has_repository() {
        assert!(DeploymentLayer::All.has_repository());
        assert!(!DeploymentLayer::Controller.has_repository());
        assert!(!DeploymentLayer::Service.has_repository());
        assert!(DeploymentLayer::Repository.has_repository());
    }

    #[test]
    fn test_deployment_layer_display() {
        assert_eq!(DeploymentLayer::All.to_string(), "all");
        assert_eq!(DeploymentLayer::Controller.to_string(), "controller");
        assert_eq!(DeploymentLayer::Service.to_string(), "service");
        assert_eq!(DeploymentLayer::Repository.to_string(), "repository");
        assert_eq!(DeploymentLayer::Worker.to_string(), "worker");
        assert_eq!(DeploymentLayer::Scheduler.to_string(), "scheduler");
    }

    #[test]
    fn test_job_roles_carry_no_api_layer() {
        // A worker/scheduler pod must not serve the API or open a DB pool.
        for layer in [DeploymentLayer::Worker, DeploymentLayer::Scheduler] {
            assert!(layer.is_job_role(), "{layer} should be a job role");
            assert!(!layer.has_controller(), "{layer} must not serve REST");
            assert!(!layer.has_service(), "{layer} must not host services");
            assert!(!layer.has_repository(), "{layer} must not open a DB pool");
        }
    }

    #[test]
    fn test_api_layers_are_not_job_roles() {
        for layer in [
            DeploymentLayer::All,
            DeploymentLayer::Controller,
            DeploymentLayer::Service,
            DeploymentLayer::Repository,
        ] {
            assert!(!layer.is_job_role(), "{layer} should not be a job role");
        }
    }

    #[test]
    fn test_only_stateless_roles_are_horizontally_scalable() {
        // Repository owns the only DB pool; Scheduler elects a single leader.
        assert!(DeploymentLayer::Controller.is_horizontally_scalable());
        assert!(DeploymentLayer::Service.is_horizontally_scalable());
        assert!(DeploymentLayer::Worker.is_horizontally_scalable());
        assert!(!DeploymentLayer::Repository.is_horizontally_scalable());
        assert!(!DeploymentLayer::Scheduler.is_horizontally_scalable());
        assert!(!DeploymentLayer::All.is_horizontally_scalable());
    }

    #[test]
    fn test_deployment_layer_env_value_deserialization() {
        // These are exactly the strings the Kubernetes manifests set in
        // ARCANA_DEPLOYMENT__LAYER; a rename here silently breaks the pods.
        let cases = [
            ("all", DeploymentLayer::All),
            ("controller", DeploymentLayer::Controller),
            ("service", DeploymentLayer::Service),
            ("repository", DeploymentLayer::Repository),
            ("worker", DeploymentLayer::Worker),
            ("scheduler", DeploymentLayer::Scheduler),
        ];
        for (raw, expected) in cases {
            let parsed: DeploymentLayer =
                serde_json::from_str(&format!("\"{raw}\"")).unwrap();
            assert_eq!(parsed, expected, "failed to parse {raw}");
        }
    }

    #[test]
    fn test_deployment_layer_serialization_roundtrip() {
        let layers = [
            DeploymentLayer::All,
            DeploymentLayer::Controller,
            DeploymentLayer::Service,
            DeploymentLayer::Repository,
            DeploymentLayer::Worker,
            DeploymentLayer::Scheduler,
        ];
        for layer in &layers {
            let json = serde_json::to_string(layer).unwrap();
            let parsed: DeploymentLayer = serde_json::from_str(&json).unwrap();
            assert_eq!(*layer, parsed, "Roundtrip failed for {:?}", layer);
        }
    }

    // =========================================================================
    // CommunicationProtocol tests
    // =========================================================================

    #[test]
    fn test_communication_protocol_default_is_http() {
        let protocol = CommunicationProtocol::default();
        assert_eq!(protocol, CommunicationProtocol::Http);
    }

    #[test]
    fn test_communication_protocol_display() {
        assert_eq!(CommunicationProtocol::Http.to_string(), "http");
        assert_eq!(CommunicationProtocol::Grpc.to_string(), "grpc");
    }

    #[test]
    fn test_communication_protocol_serialization_roundtrip() {
        let protocols = [CommunicationProtocol::Http, CommunicationProtocol::Grpc];
        for proto in &protocols {
            let json = serde_json::to_string(proto).unwrap();
            let parsed: CommunicationProtocol = serde_json::from_str(&json).unwrap();
            assert_eq!(*proto, parsed, "Roundtrip failed for {:?}", proto);
        }
    }

    // =========================================================================
    // Combination / business logic tests
    // =========================================================================

    #[test]
    fn test_monolithic_mode_all_layer_combination() {
        let mode = DeploymentMode::Monolithic;
        let layer = DeploymentLayer::All;

        // Monolithic with All layer should have everything
        assert!(mode.is_monolithic());
        assert!(layer.has_controller());
        assert!(layer.has_service());
        assert!(layer.has_repository());
        assert!(!mode.uses_grpc());
    }

    #[test]
    fn test_layered_grpc_mode_service_layer_combination() {
        let mode = DeploymentMode::LayeredGrpc;
        let layer = DeploymentLayer::Service;

        assert!(!mode.is_monolithic());
        assert!(mode.is_layered());
        assert!(mode.uses_grpc());
        assert!(!layer.has_controller());
        assert!(layer.has_service());
        assert!(!layer.has_repository());
    }
}
