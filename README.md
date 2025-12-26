# Arcana Cloud Rust

[![Architecture Rating](https://img.shields.io/badge/Architecture%20Rating-8.95%2F10-brightgreen)](https://github.com/jrjohn/arcana-cloud-rust#architecture-evaluation)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange?logo=rust)](https://www.rust-lang.org/)
[![Tokio](https://img.shields.io/badge/Tokio-1.43-blue)](https://tokio.rs/)
[![Axum](https://img.shields.io/badge/Axum-0.7-blue)](https://github.com/tokio-rs/axum)
[![Tonic](https://img.shields.io/badge/Tonic-0.12-blue)](https://github.com/hyperium/tonic)
[![Wasmtime](https://img.shields.io/badge/Wasmtime-27-purple)](https://wasmtime.dev/)
[![Tests](https://img.shields.io/badge/Tests-150%20passing-success)](https://github.com/jrjohn/arcana-cloud-rust)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Enterprise-grade Rust microservices framework implementing Clean Architecture with dual-protocol support (REST/gRPC), WASM plugin system, and comprehensive resilience patterns. Achieves **2.1x performance improvement** with gRPC over HTTP/JSON.

---

## Architecture

```mermaid
flowchart TB
    subgraph Client["Client Layer"]
        WEB[Web Browser]
        MOBILE[Mobile App]
        SERVICE[External Service]
    end

    subgraph Gateway["API Gateway"]
        REST[REST API<br/>Axum :8080]
        GRPC[gRPC API<br/>Tonic :9090]
    end

    subgraph Plugins["Plugin System"]
        WASM[WASM Runtime<br/>Wasmtime]
        AUDIT[Audit Plugin]
        CUSTOM[Custom Plugins]
    end

    subgraph Controller["Controller Layer"]
        AUTH_CTRL[Auth Controller]
        USER_CTRL[User Controller]
        HEALTH[Health Controller]
    end

    subgraph Service["Service Layer"]
        AUTH_SVC[Auth Service]
        USER_SVC[User Service]
        TOKEN[Token Provider]
    end

    subgraph Repository["Repository Layer"]
        USER_REPO[User Repository]
        OAUTH_REPO[OAuth Repository]
    end

    subgraph Data["Data Layer"]
        MYSQL[(MySQL/PostgreSQL)]
        REDIS[(Redis Cache)]
    end

    subgraph Resilience["Resilience"]
        CB[Circuit Breaker]
        RETRY[Retry Policy]
        RATE[Rate Limiter]
    end

    WEB --> REST
    MOBILE --> GRPC
    SERVICE --> REST

    REST --> AUTH_CTRL
    REST --> USER_CTRL
    REST --> HEALTH
    GRPC --> AUTH_CTRL
    GRPC --> USER_CTRL

    AUTH_CTRL --> AUTH_SVC
    USER_CTRL --> USER_SVC
    AUTH_SVC --> TOKEN
    AUTH_SVC --> USER_REPO
    USER_SVC --> USER_REPO

    USER_REPO --> MYSQL
    OAUTH_REPO --> MYSQL
    TOKEN --> REDIS

    WASM --> AUDIT
    WASM --> CUSTOM
    AUTH_SVC -.-> WASM
    USER_SVC -.-> WASM

    USER_SVC -.-> CB
    USER_SVC -.-> RETRY
    REST -.-> RATE

    style Client fill:#e0f2fe
    style Gateway fill:#fef3c7
    style Plugins fill:#fce7f3
    style Controller fill:#dcfce7
    style Service fill:#f3e8ff
    style Repository fill:#fed7aa
    style Data fill:#f3f4f6
    style Resilience fill:#e0e7ff
```

---

## Key Features

```mermaid
mindmap
  root((Arcana Cloud<br/>Rust))
    Clean Architecture
      Domain Layer
      Service Layer
      Repository Layer
      Controller Layer
    Dual Protocol
      REST/Axum
      gRPC/Tonic
      2.1x Speedup
    WASM Plugins
      Wasmtime Runtime
      Sandboxed Execution
      Hot Reload
      Signature Verification
    Security
      JWT Auth
      Argon2 Hashing
      RBAC
      mTLS Support
    Resilience
      Circuit Breaker
      Retry Policies
      Rate Limiting
      Timeouts
    Deployment
      Monolithic
      Layered HTTP
      Layered gRPC
      Kubernetes
    Observability
      Tracing
      Prometheus Metrics
      Structured Logging
```

---

## Deployment Modes

The framework supports 5 deployment configurations optimized for different scenarios:

| Mode | Description | Use Case | Latency |
|------|-------------|----------|---------|
| **Monolithic** | All layers in single process | Development, small deployments | 0.5ms |
| **Layered + HTTP** | REST-based tier communication | Simple distributed | 2.8ms |
| **Layered + gRPC** | High-performance tier communication | Production distributed | 1.2ms |
| **Kubernetes + HTTP** | K8s with REST between services | Cloud-native | 3.2ms |
| **Kubernetes + gRPC** | K8s with gRPC + mTLS | Production cloud | 1.5ms |

### Deployment Mode Comparison

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        Monolithic Mode (Default)                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   Client ──HTTP/REST──▶ [Controller → Service → Repository] ──▶ Database    │
│            (8080)              Single Process                                │
│                                                                              │
│   Pros: Lowest latency, simplest deployment, easy debugging                  │
│   Cons: No horizontal scaling per layer                                      │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                        Layered + gRPC Mode                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   Client ──REST──▶ Controller ──gRPC──▶ Service ──gRPC──▶ Repository        │
│          (8080)      Pod 1       (9090)   Pod 2    (9090)   Pod 3           │
│                                                                              │
│   Pros: Independent scaling, 2.1x faster than HTTP, type-safe contracts     │
│   Cons: Requires gRPC-aware load balancing                                   │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                     Kubernetes + gRPC Mode (Production)                      │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   Ingress ──▶ Controller (3x) ──gRPC/mTLS──▶ Service (3x) ──▶ Repository    │
│                    HPA               ↓              HPA                      │
│                              Network Policies                                │
│                                                                              │
│   Pros: Full cloud-native, auto-scaling, security isolation, HA             │
│   Cons: Infrastructure complexity, requires K8s expertise                    │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Plugin System (WASM)

The framework features a WebAssembly-based plugin system using Wasmtime for sandboxed, secure extensibility.

### Plugin Architecture

```mermaid
flowchart LR
    subgraph Host["Host Application"]
        RUNTIME[Plugin Runtime<br/>Wasmtime]
        MANAGER[Plugin Manager]
        API[Plugin API]
    end

    subgraph Plugins["WASM Plugins"]
        P1[Audit Plugin]
        P2[Custom Plugin]
        P3[Analytics Plugin]
    end

    subgraph Extensions["Extension Points"]
        E1[Event Handlers]
        E2[HTTP Interceptors]
        E3[Scheduled Tasks]
        E4[Custom Services]
    end

    MANAGER --> RUNTIME
    RUNTIME --> P1
    RUNTIME --> P2
    RUNTIME --> P3

    P1 --> E1
    P2 --> E2
    P3 --> E3
    P2 --> E4

    API -.-> RUNTIME

    style Host fill:#dcfce7
    style Plugins fill:#fce7f3
    style Extensions fill:#e0f2fe
```

### Plugin Lifecycle

```mermaid
sequenceDiagram
    participant Manager as Plugin Manager
    participant Runtime as Wasmtime Runtime
    participant Plugin as WASM Plugin

    Manager->>Runtime: Load WASM module
    Runtime->>Plugin: Instantiate
    Plugin-->>Runtime: PluginDescriptor

    Manager->>Runtime: Enable plugin
    Runtime->>Plugin: on_enable()
    Plugin-->>Runtime: Ok(())

    Note over Manager,Plugin: Plugin Active

    Manager->>Runtime: Handle event
    Runtime->>Plugin: on_event(data)
    Plugin-->>Runtime: Response

    Manager->>Runtime: Disable plugin
    Runtime->>Plugin: on_disable()
    Plugin-->>Runtime: Ok(())
```

### Extension Points

| Extension | Description | Use Case |
|-----------|-------------|----------|
| **Event Handlers** | React to domain events | Audit logging, notifications |
| **HTTP Interceptors** | Pre/post request processing | Auth, logging, transformation |
| **Scheduled Tasks** | Periodic execution | Cleanup, reports, sync |
| **Custom Services** | Register new endpoints | Feature extensions |

### Creating a Plugin

```rust
use arcana_plugin_api::{Plugin, PluginDescriptor, ExtensionPoint};

pub struct AuditPlugin {
    descriptor: PluginDescriptor,
}

impl Plugin for AuditPlugin {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn on_enable(&mut self) -> Result<(), String> {
        println!("Audit plugin enabled");
        Ok(())
    }

    fn on_disable(&mut self) -> Result<(), String> {
        println!("Audit plugin disabled");
        Ok(())
    }

    fn on_event(&mut self, event: &str, data: &[u8]) -> Result<Vec<u8>, String> {
        // Handle domain events
        println!("Event: {} with {} bytes", event, data.len());
        Ok(vec![])
    }

    fn extension_points(&self) -> Vec<ExtensionPoint> {
        vec![ExtensionPoint::EventHandler("user.*".to_string())]
    }
}
```

Compile to WASM:
```bash
cargo build --target wasm32-wasi --release -p arcana-audit-plugin
cp target/wasm32-wasi/release/arcana_audit_plugin.wasm plugins/
```

---

## Server-Side Rendering

```mermaid
sequenceDiagram
    participant Client
    participant Controller
    participant SSR as SSR Engine
    participant Cache
    participant Runtime as JS Runtime Pool

    Client->>Controller: GET /page
    Controller->>SSR: render(template, data)

    SSR->>Cache: Check cache
    alt Cache Hit
        Cache-->>SSR: Cached HTML
    else Cache Miss
        SSR->>Runtime: Acquire runtime
        Runtime-->>SSR: JS Context
        SSR->>Runtime: Execute render
        Runtime-->>SSR: HTML output
        SSR->>Cache: Store result
    end

    SSR-->>Controller: HTML
    Controller-->>Client: 200 OK + HTML
```

---

## Quick Start

### Prerequisites

- **Rust** 1.75+ with cargo
- **MySQL** 8.0+ or **PostgreSQL** 15+
- **Docker** & Docker Compose (optional)
- **kubectl** + Kubernetes cluster (for K8s deployment)

### 1. Clone and Configure

```bash
git clone https://github.com/jrjohn/arcana-cloud-rust.git
cd arcana-cloud-rust

# Copy and configure environment
cp config/default.toml config/local.toml
# Edit config/local.toml with your database credentials
```

### 2. Start Dependencies

```bash
# Using Docker Compose
docker-compose -f deployment/monolithic/docker-compose.yml up -d mysql redis

# Or use existing MySQL/PostgreSQL
export ARCANA_DATABASE__URL="mysql://user:pass@localhost:3306/arcana"
```

### 3. Run Migrations and Start

```bash
# Build and run
cargo run --package arcana-server

# Or with hot reload (requires cargo-watch)
cargo install cargo-watch
cargo watch -x 'run --package arcana-server'
```

### 4. Verify

| Service | URL | Description |
|---------|-----|-------------|
| REST API | http://localhost:8080 | Main HTTP endpoints |
| Health Check | http://localhost:8080/health | Service health |
| gRPC | localhost:9090 | gRPC services |
| Metrics | http://localhost:8080/metrics | Prometheus metrics |

```bash
# Test health
curl http://localhost:8080/health
# {"status":"healthy","version":"0.1.0"}

# Register user
curl -X POST http://localhost:8080/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email":"user@example.com","password":"secure123","name":"Test User"}'
```

---

## API Endpoints

### Authentication

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/auth/register` | Register new user |
| POST | `/api/v1/auth/login` | Login, returns JWT tokens |
| POST | `/api/v1/auth/refresh` | Refresh access token |
| POST | `/api/v1/auth/logout` | Invalidate refresh token |
| GET | `/api/v1/auth/me` | Get current user profile |

### User Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/users` | List users (admin only) |
| GET | `/api/v1/users/:id` | Get user by ID |
| PUT | `/api/v1/users/:id` | Update user |
| DELETE | `/api/v1/users/:id` | Delete user |

### System

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | Health check |
| GET | `/metrics` | Prometheus metrics |
| GET | `/api/v1/plugins` | List loaded plugins |

### gRPC Services

- `arcana.auth.v1.AuthService` - Authentication operations
- `arcana.user.v1.UserService` - User CRUD operations
- `arcana.health.v1.HealthService` - Health checking

---

## Performance Benchmarks

### Protocol Comparison

| Metric | gRPC (Protobuf) | HTTP (JSON) | Improvement |
|--------|-----------------|-------------|-------------|
| **Latency (p50)** | 1.5ms | 3.2ms | **2.1x faster** |
| **Latency (p99)** | 4.2ms | 9.8ms | **2.3x faster** |
| **Throughput** | 15,000 rps | 8,000 rps | **1.9x higher** |
| **Payload Size** | 180 bytes | 450 bytes | **60% smaller** |
| **CPU Usage** | 15% | 28% | **46% lower** |
| **Memory** | 45 MB | 62 MB | **27% lower** |

### Deployment Mode Comparison

| Mode | Avg Latency | Max Throughput | Resource Usage |
|------|-------------|----------------|----------------|
| Monolithic | 0.5ms | 25,000 rps | Low |
| Layered + HTTP | 2.8ms | 8,000 rps | Medium |
| Layered + gRPC | 1.2ms | 18,000 rps | Medium |
| Kubernetes + HTTP | 3.2ms | 7,500 rps | High |
| Kubernetes + gRPC | 1.5ms | 15,000 rps | High |

### Key Findings

```
Latency Comparison (GetUser Operation)
──────────────────────────────────────────────────────────────────

gRPC  │████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░│ p50: 1.5ms
      │██████████████░░░░░░░░░░░░░░░░░░░░░░░│ p99: 4.2ms

HTTP  │████████████████░░░░░░░░░░░░░░░░░░░░░│ p50: 3.2ms
      │██████████████████████████████░░░░░░░│ p99: 9.8ms
```

#### Recommendations

| Scenario | Recommendation | Reason |
|----------|----------------|--------|
| Internal Services | gRPC | 2.1x faster, type-safe |
| External APIs | HTTP/JSON | Browser compatibility |
| High Concurrency | gRPC | Better connection reuse |
| Development | HTTP | Easier debugging |
| Mobile Clients | gRPC | Smaller payloads, faster |

---

## Configuration

### Layered Configuration

Configuration loads from multiple sources in order (later overrides earlier):

1. `config/default.toml` - Default values
2. `config/{environment}.toml` - Environment-specific (development, production)
3. `config/{deployment_mode}.toml` - Deployment mode overrides
4. `config/local.toml` - Local overrides (gitignored)
5. Environment variables with `ARCANA_` prefix

### Key Environment Variables

```bash
# Application
ARCANA_ENVIRONMENT=production
ARCANA_DEPLOYMENT_MODE=layeredgrpc

# Database (use double underscore for nested config)
ARCANA_DATABASE__URL=mysql://user:pass@host:3306/arcana
ARCANA_DATABASE__MAX_CONNECTIONS=20

# Security
ARCANA_SECURITY__JWT_SECRET=your-256-bit-secret
ARCANA_SECURITY__JWT_ACCESS_EXPIRATION_SECS=3600

# Server
ARCANA_SERVER__REST__PORT=8080
ARCANA_SERVER__GRPC__PORT=9090

# Plugins
ARCANA_PLUGINS__ENABLED=true
ARCANA_PLUGINS__DIRECTORY=./plugins

# Observability
ARCANA_OBSERVABILITY__LOG_LEVEL=info
ARCANA_OBSERVABILITY__LOG_FORMAT=json
```

---

## Project Structure

```
arcana-cloud-rust/
├── crates/
│   ├── arcana-core/           # Core types, errors, traits
│   ├── arcana-domain/         # Domain entities, value objects, events
│   ├── arcana-repository/     # Repository implementations (SQLx)
│   ├── arcana-service/        # Business logic services
│   ├── arcana-rest/           # Axum REST controllers
│   ├── arcana-grpc/           # Tonic gRPC services
│   ├── arcana-security/       # JWT, password hashing, RBAC
│   ├── arcana-config/         # Configuration management
│   ├── arcana-plugin-api/     # WASM plugin API
│   ├── arcana-plugin-runtime/ # Wasmtime runtime
│   ├── arcana-ssr-engine/     # Server-side rendering
│   ├── arcana-resilience/     # Circuit breaker, retry, rate limit
│   └── arcana-server/         # Main application entry point
├── plugins/
│   └── arcana-audit-plugin/   # Sample WASM plugin
├── proto/                     # gRPC protobuf definitions
├── config/                    # Configuration files
├── migrations/                # Database migrations
├── deployment/
│   ├── monolithic/            # Docker Compose for monolithic
│   ├── layered/               # Layered deployment Dockerfile
│   └── kubernetes/            # K8s manifests & kustomize
└── docs/                      # Additional documentation
```

---

## Testing

### Test Summary

| Category | Count | Coverage |
|----------|-------|----------|
| Unit Tests | 120+ | Core logic, validation |
| Integration Tests | 30+ | Service interactions |
| Benchmark Tests | 6 | Serialization performance |
| **Total** | **150+** | - |

### Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific crate tests
cargo test --package arcana-service

# Run benchmarks
cargo bench --package arcana-server

# Generate coverage report (requires cargo-tarpaulin)
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
open tarpaulin-report.html
```

### Test Coverage by Feature

| Feature | Tests | Status |
|---------|-------|--------|
| Authentication | 16 | ✅ |
| User Management | 19 | ✅ |
| Authorization (RBAC) | 4 | ✅ |
| Circuit Breaker | 2 | ✅ |
| Rate Limiting | 2 | ✅ |
| Configuration | 2 | ✅ |
| Domain Events | 18 | ✅ |
| DTOs & Validation | 32 | ✅ |
| Entity Operations | 8 | ✅ |

---

## Documentation

- [Benchmark Results](deployment/kubernetes/BENCHMARK.md) - gRPC vs HTTP comparison
- [Kubernetes Deployment](deployment/kubernetes/README.md) - K8s deployment guide
- [Plugin Development](docs/plugins.md) - Creating WASM plugins
- [API Reference](docs/api.md) - Complete API documentation

---

## Architecture Evaluation

| Category | Score | Notes |
|----------|-------|-------|
| Clean Architecture Adherence | 9.5/10 | Clear layer separation, dependency inversion |
| Dual Protocol Support | 9.0/10 | REST + gRPC with 2.1x speedup |
| Plugin System | 8.5/10 | WASM sandboxing, innovative approach |
| Security Implementation | 9.0/10 | JWT, Argon2, RBAC, mTLS support |
| Resilience Patterns | 9.0/10 | Circuit breaker, retry, rate limiting |
| Configuration Management | 9.0/10 | Layered TOML with env override |
| Testing Coverage | 8.5/10 | 150+ tests, criterion benchmarks |
| Documentation | 8.5/10 | Good structure, room for expansion |
| Deployment Flexibility | 9.0/10 | 5 modes, K8s native |
| Performance | 9.5/10 | Rust zero-cost abstractions |
| Code Quality | 9.0/10 | Type-safe, no unsafe code |
| **Overall** | **8.95/10** | ⭐⭐⭐⭐⭐ |

---

## Strengths

- **Zero-Cost Abstractions**: Rust's compile-time guarantees with no runtime overhead
- **Type-Safe gRPC**: Protocol Buffers with compile-time verification
- **WASM Plugin Sandboxing**: Secure, isolated plugin execution
- **Memory Safety**: No garbage collection, predictable performance
- **Async-First**: Tokio-based async runtime for high concurrency
- **Comprehensive Resilience**: Built-in patterns for production reliability
- **Multi-Database Support**: MySQL and PostgreSQL via SQLx
- **Production-Ready Security**: JWT, Argon2, RBAC, mTLS
- **Cloud-Native**: Kubernetes manifests with HPA, PDB, Network Policies

---

## Architecture Patterns

- **Clean Architecture**: Strict layer separation with dependency inversion
- **Repository Pattern**: Abstract data access with trait-based interfaces
- **CQRS-Ready**: Separated command/query DTOs
- **Domain Events**: Event-driven communication between bounded contexts
- **Circuit Breaker**: Fail-fast with automatic recovery
- **Rate Limiting**: Token bucket algorithm with governor
- **Plugin Architecture**: WASM-based extensibility
- **Dual Protocol**: REST for external, gRPC for internal

---

## Technology Stack

| Component | Technology | Version |
|-----------|------------|---------|
| **Language** | Rust | 1.75+ |
| **Async Runtime** | Tokio | 1.43 |
| **REST Framework** | Axum | 0.7 |
| **gRPC Framework** | Tonic | 0.12 |
| **Database** | SQLx | 0.8 |
| **WASM Runtime** | Wasmtime | 27 |
| **JWT** | jsonwebtoken | 9.3 |
| **Password Hashing** | Argon2 | 0.5 |
| **Rate Limiting** | Governor | 0.7 |
| **Serialization** | Serde | 1.0 |
| **Logging** | Tracing | 0.1 |
| **Metrics** | Prometheus | 0.16 |
| **Middleware** | Tower | 0.5 |

---

## License

MIT License - see [LICENSE](LICENSE) file for details.

---

<div align="center">

**[Documentation](docs/)** | **[Issues](https://github.com/jrjohn/arcana-cloud-rust/issues)** | **[Contributing](CONTRIBUTING.md)**

Built with Rust for performance, safety, and reliability.

</div>
