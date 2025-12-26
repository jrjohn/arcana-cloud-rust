# Arcana Cloud Rust - Layered Deployment Dockerfile
# Multi-stage build supporting Controller, Service, and Repository layers
#
# Build with:
#   docker build --build-arg LAYER=controller -t arcana-controller .
#   docker build --build-arg LAYER=service -t arcana-service .
#   docker build --build-arg LAYER=repository -t arcana-repository .

# Build stage
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

# Layer argument (controller, service, or repository)
ARG LAYER=controller

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    wget \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/arcana-server /app/arcana-server

# Copy configuration and migrations
COPY config/ ./config/
COPY migrations/ ./migrations/

# Create non-root user
RUN useradd -m -s /bin/bash arcana
RUN chown -R arcana:arcana /app
USER arcana

# Expose ports based on layer
# Controller: REST 8080
# Service: gRPC 9090
# Repository: gRPC 9090
EXPOSE 8080 9090

# Health check - different endpoints for different layers
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8080/health || \
        wget --no-verbose --tries=1 --spider http://localhost:9090/health || exit 1

# Environment variables (use double underscore for nested config)
ENV RUST_LOG=info
ENV ARCANA_ENVIRONMENT=production
ENV ARCANA_DEPLOYMENT__MODE=layeredgrpc
ENV ARCANA_DEPLOYMENT__LAYER=${LAYER}

# Run the server
ENTRYPOINT ["/app/arcana-server"]
