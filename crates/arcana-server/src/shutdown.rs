//! Process shutdown signalling, shared by every deployment role.

use tokio::signal;
use tracing::info;

/// Resolves when the process receives SIGINT or SIGTERM.
///
/// Kubernetes sends SIGTERM before the grace period starts, so every role
/// awaits this rather than running until it is killed.
pub async fn wait_for_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, initiating graceful shutdown...");
        }
        _ = terminate => {
            info!("Received terminate signal, initiating graceful shutdown...");
        }
    }
}
