//! Telemetry module for OpenTelemetry distributed tracing.
//!
//! This module provides initialization and configuration for distributed tracing
//! using OpenTelemetry with OTLP export.

#[cfg(feature = "telemetry")]
use opentelemetry::trace::TracerProvider;
#[cfg(feature = "telemetry")]
use opentelemetry::KeyValue;
#[cfg(feature = "telemetry")]
use opentelemetry_otlp::WithExportConfig;
#[cfg(feature = "telemetry")]
use opentelemetry_sdk::{
    runtime,
    trace::{RandomIdGenerator, Sampler},
    Resource,
};
#[cfg(feature = "telemetry")]
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
#[cfg(feature = "telemetry")]
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::ArcanaResult;
use serde::{Deserialize, Serialize};

/// Telemetry configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Whether telemetry is enabled.
    #[serde(default)]
    pub enabled: bool,

    /// Service name for tracing.
    #[serde(default = "default_service_name")]
    pub service_name: String,

    /// OTLP endpoint URL (e.g., "http://localhost:4317").
    #[serde(default)]
    pub otlp_endpoint: Option<String>,

    /// Sampling ratio (0.0 to 1.0).
    #[serde(default = "default_sampling_ratio")]
    pub sampling_ratio: f64,

    /// Whether to enable console output.
    #[serde(default = "default_console_output")]
    pub console_output: bool,
}

fn default_service_name() -> String {
    "arcana-cloud-rust".to_string()
}

fn default_sampling_ratio() -> f64 {
    1.0
}

fn default_console_output() -> bool {
    true
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            service_name: default_service_name(),
            otlp_endpoint: None,
            sampling_ratio: default_sampling_ratio(),
            console_output: default_console_output(),
        }
    }
}

/// Initialize telemetry with the given configuration.
///
/// This sets up:
/// - OpenTelemetry tracer with OTLP exporter (if endpoint configured)
/// - tracing subscriber with OpenTelemetry layer
/// - Console output layer (if enabled)
#[cfg(feature = "telemetry")]
pub fn init_telemetry(config: &TelemetryConfig) -> ArcanaResult<()> {
    if !config.enabled {
        // Just initialize basic tracing without OpenTelemetry
        init_basic_tracing(config.console_output)?;
        return Ok(());
    }

    let sampler = if config.sampling_ratio >= 1.0 {
        Sampler::AlwaysOn
    } else if config.sampling_ratio <= 0.0 {
        Sampler::AlwaysOff
    } else {
        Sampler::TraceIdRatioBased(config.sampling_ratio)
    };

    let resource = Resource::new(vec![
        KeyValue::new(SERVICE_NAME, config.service_name.clone()),
    ]);

    // Build the tracer provider
    let tracer_provider = if let Some(endpoint) = &config.otlp_endpoint {
        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(endpoint)
            .build()
            .map_err(|e| crate::ArcanaError::Internal(format!("Failed to create OTLP exporter: {}", e)))?;

        opentelemetry_sdk::trace::TracerProvider::builder()
            .with_batch_exporter(exporter, runtime::Tokio)
            .with_sampler(sampler)
            .with_id_generator(RandomIdGenerator::default())
            .with_resource(resource)
            .build()
    } else {
        // No OTLP endpoint, just create a basic provider
        opentelemetry_sdk::trace::TracerProvider::builder()
            .with_sampler(sampler)
            .with_id_generator(RandomIdGenerator::default())
            .with_resource(resource)
            .build()
    };

    let tracer = tracer_provider.tracer("arcana-cloud-rust");

    // Set global provider
    opentelemetry::global::set_tracer_provider(tracer_provider);

    // Build the subscriber with OpenTelemetry layer
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,arcana=debug,tower_http=debug"));

    if config.console_output {
        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer().with_target(true))
            .with(tracing_opentelemetry::layer().with_tracer(tracer))
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_opentelemetry::layer().with_tracer(tracer))
            .init();
    }

    tracing::info!(
        service_name = %config.service_name,
        sampling_ratio = %config.sampling_ratio,
        otlp_endpoint = ?config.otlp_endpoint,
        "Telemetry initialized"
    );

    Ok(())
}

/// Initialize basic tracing without OpenTelemetry.
#[cfg(feature = "telemetry")]
fn init_basic_tracing(console_output: bool) -> ArcanaResult<()> {
    if !console_output {
        return Ok(());
    }

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,arcana=debug,tower_http=debug"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_target(true))
        .init();

    Ok(())
}

/// Shutdown telemetry, flushing any pending spans.
#[cfg(feature = "telemetry")]
pub fn shutdown_telemetry() {
    opentelemetry::global::shutdown_tracer_provider();
    tracing::info!("Telemetry shutdown complete");
}

/// Placeholder for when telemetry feature is disabled.
#[cfg(not(feature = "telemetry"))]
pub fn init_telemetry(_config: &TelemetryConfig) -> ArcanaResult<()> {
    Ok(())
}

/// Placeholder for when telemetry feature is disabled.
#[cfg(not(feature = "telemetry"))]
pub fn shutdown_telemetry() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TelemetryConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.service_name, "arcana-cloud-rust");
        assert_eq!(config.sampling_ratio, 1.0);
        assert!(config.console_output);
    }
}
