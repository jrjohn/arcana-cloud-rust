//! TLS configuration for gRPC.
//!
//! Provides helpers for configuring TLS on gRPC servers and clients.

use arcana_config::SecurityConfig;
use arcana_core::ArcanaResult;
use std::fs;
use std::path::Path;
use tonic::transport::{Certificate, ClientTlsConfig, Identity, ServerTlsConfig};
use tracing::{debug, info};

/// Builder for TLS configuration from security config.
#[derive(Debug, Clone)]
pub struct TlsConfigBuilder {
    cert_path: String,
    key_path: String,
    ca_cert_path: Option<String>,
}

impl TlsConfigBuilder {
    /// Creates a TLS config builder from security configuration.
    ///
    /// Returns None if TLS is disabled, or Some(builder) if enabled.
    pub fn from_security_config(config: &SecurityConfig) -> ArcanaResult<Option<Self>> {
        if !config.grpc_tls_enabled {
            debug!("gRPC TLS is disabled");
            return Ok(None);
        }

        let cert_path = config.tls_cert_path.as_ref().ok_or_else(|| {
            arcana_core::ArcanaError::Configuration(
                "TLS certificate path required when gRPC TLS is enabled".to_string(),
            )
        })?;

        let key_path = config.tls_key_path.as_ref().ok_or_else(|| {
            arcana_core::ArcanaError::Configuration(
                "TLS key path required when gRPC TLS is enabled".to_string(),
            )
        })?;

        info!("TLS enabled with cert: {}, key: {}", cert_path, key_path);

        Ok(Some(Self {
            cert_path: cert_path.clone(),
            key_path: key_path.clone(),
            ca_cert_path: None,
        }))
    }

    /// Creates a new TLS config builder with explicit paths.
    pub fn new(cert_path: impl Into<String>, key_path: impl Into<String>) -> Self {
        Self {
            cert_path: cert_path.into(),
            key_path: key_path.into(),
            ca_cert_path: None,
        }
    }

    /// Sets the CA certificate path for client certificate verification.
    #[must_use]
    pub fn with_ca_cert(mut self, ca_cert_path: impl Into<String>) -> Self {
        self.ca_cert_path = Some(ca_cert_path.into());
        self
    }

    /// Builds the server TLS configuration.
    pub fn build_server_config(&self) -> ArcanaResult<ServerTlsConfig> {
        let cert = read_file(&self.cert_path, "TLS certificate")?;
        let key = read_file(&self.key_path, "TLS private key")?;

        let identity = Identity::from_pem(cert, key);

        let mut config = ServerTlsConfig::new().identity(identity);

        // Add CA certificate for client verification if provided
        if let Some(ref ca_path) = self.ca_cert_path {
            let ca_cert = read_file(ca_path, "CA certificate")?;
            config = config.client_ca_root(Certificate::from_pem(ca_cert));
            info!("Server TLS configured with client certificate verification");
        } else {
            info!("Server TLS configured (no client cert verification)");
        }

        Ok(config)
    }

    /// Builds the client TLS configuration.
    ///
    /// For client connections, only the CA certificate is needed to verify
    /// the server. The cert and key are used for mutual TLS (mTLS).
    pub fn build_client_config(&self) -> ArcanaResult<ClientTlsConfig> {
        // For client, the cert_path is typically the CA cert to verify server
        let ca_cert = read_file(&self.cert_path, "CA certificate")?;

        let mut config = ClientTlsConfig::new().ca_certificate(Certificate::from_pem(ca_cert));

        // If we have a key, this is mTLS - client presents its own certificate
        if Path::new(&self.key_path).exists() && self.ca_cert_path.is_some() {
            if let Some(ref client_cert_path) = self.ca_cert_path {
                let client_cert = read_file(client_cert_path, "client certificate")?;
                let client_key = read_file(&self.key_path, "client private key")?;
                config = config.identity(Identity::from_pem(client_cert, client_key));
                info!("Client TLS configured with mutual TLS");
            }
        } else {
            info!("Client TLS configured (server verification only)");
        }

        Ok(config)
    }

    /// Builds a simple client TLS config that just trusts the given CA.
    pub fn build_simple_client_config(&self) -> ArcanaResult<ClientTlsConfig> {
        let ca_cert = read_file(&self.cert_path, "CA certificate")?;
        let config = ClientTlsConfig::new().ca_certificate(Certificate::from_pem(ca_cert));
        debug!("Simple client TLS configured");
        Ok(config)
    }
}

/// Builds a client TLS config for connecting to a TLS-enabled gRPC server.
///
/// This is a convenience function when you only need to configure the CA certificate
/// for server verification, without mutual TLS.
pub fn build_client_tls_from_ca(ca_cert_path: &str) -> ArcanaResult<ClientTlsConfig> {
    let ca_cert = read_file(ca_cert_path, "CA certificate")?;
    let config = ClientTlsConfig::new().ca_certificate(Certificate::from_pem(ca_cert));
    Ok(config)
}

/// Builds a client TLS config from security configuration.
///
/// Returns None if TLS is disabled.
pub fn build_client_tls_from_config(config: &SecurityConfig) -> ArcanaResult<Option<ClientTlsConfig>> {
    if !config.grpc_tls_enabled {
        return Ok(None);
    }

    // For client, we use the cert_path as the CA certificate to verify the server
    let cert_path = config.tls_cert_path.as_ref().ok_or_else(|| {
        arcana_core::ArcanaError::Configuration(
            "TLS certificate path required for client TLS verification".to_string(),
        )
    })?;

    let tls_config = build_client_tls_from_ca(cert_path)?;
    Ok(Some(tls_config))
}

/// Helper to read a file and provide a descriptive error.
fn read_file(path: &str, description: &str) -> ArcanaResult<Vec<u8>> {
    fs::read(path).map_err(|e| {
        arcana_core::ArcanaError::Configuration(format!(
            "Failed to read {} from '{}': {}",
            description, path, e
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Test certificate and key (self-signed, for testing only)
    const TEST_CERT: &str = r#"-----BEGIN CERTIFICATE-----
MIIBkTCB+wIJAKHBfpegFjJgMA0GCSqGSIb3DQEBCwUAMBExDzANBgNVBAMMBnVu
dXNlZDAeFw0yMzAxMDEwMDAwMDBaFw0yNDAxMDEwMDAwMDBaMBExDzANBgNVBAMM
BnVudXNlZDBcMA0GCSqGSIb3DQEBAQUAA0sAMEgCQQC7o96FCFxHQ1rMGK7HrbkL
FJbNhVDGMnXUXaDthEF78+hg+bFnYH/s8HgFxFm/sDnBKiPP0y8O8E9bBOYzWY5d
AgMBAAGjUzBRMB0GA1UdDgQWBBRrT/K4C/NvJJEQs7W5VJvM1PZqpzAfBgNVHSME
GDAWgBRrT/K4C/NvJJEQs7W5VJvM1PZqpzAPBgNVHRMBAf8EBTADAQH/MA0GCSqG
SIb3DQEBCwUAA0EAV8/K1dKcMNwKEgH9tJnzGKSmjDCFPxJFdwFhpfTq3mGf0tMY
HKbT0LKsLK3KM8j5PDPj0g9Y5EqbZMXF0N5bPQ==
-----END CERTIFICATE-----"#;

    const TEST_KEY: &str = r#"-----BEGIN PRIVATE KEY-----
MIIBVQIBADANBgkqhkiG9w0BAQEFAASCAT8wggE7AgEAAkEAu6PehQhcR0NazBiu
x625CxSWzYVQxjJ11F2g7YRBe/PoYPmxZ2B/7PB4BcRZv7A5wSojz9MvDvBPWwTm
M1mOXQIDAQABAkB9E5P1F0DwRYTKL5UD/YB1JKxmfZZEqAFVfRHOJb3gVvMUEeQe
O/hL9B4xXD1a3H3sDM5OuwB3sI2sxmq5VxchAiEA38VvBsPqfLPBBVlT3AOZbJy1
iP6G9T9sHOJz7dcNhKUCIQDYRtk5N7xb/EPR/aRE0tN8Ov0jmYjqgL+x7bL9sUJq
OQIhAKeMQP1D5Dq+1E0Rf+v8b8y/2j3gOBVIyNZG1vfyiE0lAiAJDR/JrdPPbU0C
U2MFU5HiZ8yF+fW3xKX9y/ySb/N98QIhAKfQ0fDJVzC8aMQ9P/t/7xYXnsRG8T3t
T9v8i1PO0N0Y
-----END PRIVATE KEY-----"#;

    fn create_temp_file(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file
    }

    #[test]
    fn test_tls_disabled() {
        let config = SecurityConfig {
            grpc_tls_enabled: false,
            ..Default::default()
        };

        let result = TlsConfigBuilder::from_security_config(&config).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_tls_enabled_missing_cert() {
        let config = SecurityConfig {
            grpc_tls_enabled: true,
            tls_cert_path: None,
            tls_key_path: Some("/path/to/key".to_string()),
            ..Default::default()
        };

        let result = TlsConfigBuilder::from_security_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_tls_enabled_missing_key() {
        let config = SecurityConfig {
            grpc_tls_enabled: true,
            tls_cert_path: Some("/path/to/cert".to_string()),
            tls_key_path: None,
            ..Default::default()
        };

        let result = TlsConfigBuilder::from_security_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_tls_builder_creation() {
        let cert_file = create_temp_file(TEST_CERT);
        let key_file = create_temp_file(TEST_KEY);

        let config = SecurityConfig {
            grpc_tls_enabled: true,
            tls_cert_path: Some(cert_file.path().to_string_lossy().to_string()),
            tls_key_path: Some(key_file.path().to_string_lossy().to_string()),
            ..Default::default()
        };

        let result = TlsConfigBuilder::from_security_config(&config).unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_build_server_config() {
        let cert_file = create_temp_file(TEST_CERT);
        let key_file = create_temp_file(TEST_KEY);

        let builder = TlsConfigBuilder::new(
            cert_file.path().to_string_lossy().to_string(),
            key_file.path().to_string_lossy().to_string(),
        );

        let result = builder.build_server_config();
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_client_config() {
        let cert_file = create_temp_file(TEST_CERT);
        let key_file = create_temp_file(TEST_KEY);

        let builder = TlsConfigBuilder::new(
            cert_file.path().to_string_lossy().to_string(),
            key_file.path().to_string_lossy().to_string(),
        );

        let result = builder.build_simple_client_config();
        assert!(result.is_ok());
    }

    #[test]
    fn test_read_file_error() {
        let result = read_file("/nonexistent/path/to/file", "test file");
        assert!(result.is_err());
    }
}
