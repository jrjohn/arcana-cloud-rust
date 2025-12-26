//! Server startup utilities.

use tracing::info;

/// Prints the startup banner.
pub fn print_banner() {
    info!(r#"
    ___                                   ________                __
   /   |  ______________ _____  ____ _   / ____/ /___  __  ______/ /
  / /| | / ___/ ___/ __ `/ __ \/ __ `/  / /   / / __ \/ / / / __  /
 / ___ |/ /  / /__/ /_/ / / / / /_/ /  / /___/ / /_/ / /_/ / /_/ /
/_/  |_/_/   \___/\__,_/_/ /_/\__,_/   \____/_/\____/\__,_/\__,_/

                         Rust Edition
    "#);
}

/// Prints server startup information.
pub fn print_startup_info(rest_port: u16, grpc_port: u16) {
    let separator = "=".repeat(60);
    info!("{}", separator);
    info!("REST API:  http://0.0.0.0:{}", rest_port);
    info!("gRPC API:  http://0.0.0.0:{}", grpc_port);
    info!("Health:    http://0.0.0.0:{}/health", rest_port);
    info!("API Docs:  http://0.0.0.0:{}/api/v1", rest_port);
    info!("{}", separator);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print_banner_does_not_panic() {
        // Initialize subscriber for testing
        let _ = tracing_subscriber::fmt::try_init();
        print_banner();
    }

    #[test]
    fn test_print_startup_info_does_not_panic() {
        let _ = tracing_subscriber::fmt::try_init();
        print_startup_info(8080, 9090);
    }

    #[test]
    fn test_print_startup_info_custom_ports() {
        let _ = tracing_subscriber::fmt::try_init();
        print_startup_info(3000, 50051);
    }
}
