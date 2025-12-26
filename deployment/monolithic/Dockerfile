# Arcana Cloud Rust - Monolithic Dockerfile
# Multi-stage build for minimal image size

# Build stage - using bookworm for latest Rust
FROM rust:latest AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    protobuf-compiler \
    libprotobuf-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests and proto files
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
COPY plugins/ ./plugins/
COPY proto/ ./proto/
COPY migrations/ ./migrations/

# Build release binary
RUN cargo build --release --package arcana-server

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    wget \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/arcana-server /app/arcana-server

# Copy configuration
COPY config/ ./config/
COPY migrations/ ./migrations/

# Create non-root user
RUN useradd -m -s /bin/bash arcana
RUN chown -R arcana:arcana /app
USER arcana

# Expose ports
EXPOSE 8080 9090

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8080/health || exit 1

# Environment variables
ENV RUST_LOG=info
ENV ARCANA_ENVIRONMENT=production
ENV ARCANA_DEPLOYMENT_MODE=monolithic

# Run the server
ENTRYPOINT ["/app/arcana-server"]
