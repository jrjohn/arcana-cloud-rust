# Arcana Cloud Rust - Kubernetes Deployment

Microservice deployment for Kubernetes with layered architecture.

## Architecture

```
                    ┌─────────────────────────────────────────┐
                    │              Ingress                    │
                    │         (api.arcana.local)              │
                    └─────────────────┬───────────────────────┘
                                      │
                    ┌─────────────────▼───────────────────────┐
                    │        Controller Layer (REST)          │
                    │    arcana-controller (replicas: 3-20)   │
                    │           Port: 8080                    │
                    └─────────────────┬───────────────────────┘
                                      │ gRPC
                    ┌─────────────────▼───────────────────────┐
                    │         Service Layer (gRPC)            │
                    │     arcana-service (replicas: 2-10)     │
                    │           Port: 9090                    │
                    └─────────────────┬───────────────────────┘
                                      │ gRPC
                    ┌─────────────────▼───────────────────────┐
                    │        Repository Layer (gRPC)          │
                    │    arcana-repository (replicas: 2)      │
                    │           Port: 9090                    │
                    └─────────────────┬───────────────────────┘
                                      │ MySQL
                    ┌─────────────────▼───────────────────────┐
                    │            MySQL Database               │
                    └─────────────────────────────────────────┘
```

## Quick Start

### Prerequisites

- Kubernetes cluster (1.25+)
- kubectl configured
- Docker image built: `arcana-cloud-rust:latest`

### Build Docker Image

```bash
# From project root
docker build -t arcana-cloud-rust:latest -f deployment/layered/Dockerfile .
```

### Layout

```
deployment/kubernetes/
  kustomization.yaml   entry point -> base
  base/                the manifests (incl. base/jobs for worker + scheduler)
  overlays/            per-environment patches, each -> ../../base
  ci/                  standalone kind manifests, not part of kustomize
```

`base/` and `overlays/` are siblings because kustomize refuses to build an
overlay nested inside the directory it references.

### Deploy to Kubernetes

```bash
# Base deployment (development)
kubectl apply -k deployment/kubernetes/

# Or use overlays for specific environments
kubectl apply -k deployment/kubernetes/overlays/dev/
kubectl apply -k deployment/kubernetes/overlays/production/

# Render without applying -- do this before every change
kubectl kustomize deployment/kubernetes/ | less
```

### Environment variable spelling

One underscore after the `ARCANA` prefix, two between config levels:

```
ARCANA_DEPLOYMENT__LAYER=worker        # deployment.layer   (correct)
ARCANA_JOBS__WORKER__CONCURRENCY=4     # jobs.worker.concurrency
ARCANA__DEPLOYMENT__LAYER=worker       # ignored -- key "_deployment.layer"
ARCANA_SERVER__REST__PORT=8080         # ignored -- server.rest.port does not exist
```

An unmatched variable is discarded in silence: the pod starts, reports healthy,
and runs on file defaults. `crates/arcana-config/src/loader.rs` has tests that
pin both the working and the inert spelling.

### Verify Deployment

```bash
# Check pods
kubectl get pods -n arcana

# Check services
kubectl get svc -n arcana

# Check ingress
kubectl get ingress -n arcana

# View logs
kubectl logs -n arcana -l app.kubernetes.io/name=arcana-controller -f
```

## Components

| Component | Description | Replicas | Port |
|-----------|-------------|----------|------|
| `arcana-controller` | REST API layer | 3-20 (HPA) | 8080 |
| `arcana-service` | Business logic | 2-10 (HPA) | 9090 |
| `arcana-repository` | Database access | 2 (fixed) | 9090 |
| `arcana-job-worker` | Background jobs | 2-10 (HPA) | 8080 |
| `arcana-job-scheduler` | Cron entries | 2 (fixed) | 8080 |

All five run the **same image and the same binary**. The role is chosen only by
`ARCANA_DEPLOYMENT__LAYER`; there are no command-line flags, and an `args:`
entry in a manifest is silently ignored.

## Pod sizing

Split pods by what makes them grow, not by where the code lives. Each role
below has a different limiting resource, which is the whole reason it is a
separate pod:

| Role | Limiting resource | Requests | Limits | Scale on |
|------|-------------------|----------|--------|----------|
| controller | CPU (TLS, JSON, routing) | 128Mi / 100m | 512Mi / 500m | request rate |
| service | CPU + Redis conns | 256Mi / 200m | 1Gi / 1000m | request rate |
| repository | **database connections** | 128Mi / 100m | 512Mi / 500m | **do not autoscale** |
| job worker | job payload x concurrency | 128Mi / 100m | 512Mi / 500m | queue depth |
| scheduler | negligible | 64Mi / 50m | 256Mi / 200m | **do not autoscale** |

Two of those five must never be attached to an HPA, and the reasons are not
symmetric:

- **repository** owns the only database pool
  (`ARCANA_DATABASE__MAX_CONNECTIONS`, 50 in production). Replica count
  multiplies it, so autoscaling this layer under load walks straight into the
  database connection limit -- at the exact moment the database is already the
  bottleneck. Scale it by raising the pool, then the replica count, deliberately.
- **scheduler** elects a single leader on a Redis key. Extra replicas are warm
  standby: they cost memory and produce no additional throughput.

`DeploymentLayer::is_horizontally_scalable()` encodes this in code, and a unit
test pins it, so the rule cannot drift away from the manifests silently.

### Sizing the worker

Worker memory is `concurrency x peak job payload`, so
`ARCANA_JOBS__WORKER__CONCURRENCY` and the pod memory limit move together.
Raising concurrency alone inside a fixed limit converts a queue backlog into
an OOMKill. Prefer more worker pods over a higher concurrency per pod: pods
fail independently, threads inside one pod do not.

### Roles not yet split out

`arcana-plugin-runtime` (Wasmtime) and `arcana-ssr-engine` are built but have
no callers in the workspace today. When they gain one, give each its own
deployment rather than folding it into `service`:

- a WASM sandbox reserves `plugins.max_memory_bytes` (64Mi) per instance and
  `plugins.runtime_pool_size` is 8 in production -- roughly 512Mi of floor,
  which would be multiplied by every `service` replica.
- SSR is bursty CPU, so it would distort the request-rate signal the `service`
  HPA scales on.

## Configuration

### Environment Variables

Configuration is managed via ConfigMap and Secrets:

```bash
# View current config
kubectl get configmap arcana-config -n arcana -o yaml

# View secrets (base64 encoded)
kubectl get secret arcana-secrets -n arcana -o yaml
```

### Update Configuration

```bash
# Edit configmap
kubectl edit configmap arcana-config -n arcana

# Restart pods to pick up changes
kubectl rollout restart deployment -n arcana
```

## Scaling

### Manual Scaling

```bash
# Scale controller
kubectl scale deployment arcana-controller -n arcana --replicas=5

# Scale service
kubectl scale deployment arcana-service -n arcana --replicas=5
```

### Horizontal Pod Autoscaler

HPAs are configured for Controller and Service layers:

```bash
# View HPA status
kubectl get hpa -n arcana

# Watch scaling events
kubectl describe hpa arcana-controller-hpa -n arcana
```

## Network Policies

Network policies restrict traffic flow:

- **Controller**: Receives traffic from Ingress only
- **Service**: Receives traffic from Controller only
- **Repository**: Receives traffic from Service only

```bash
# View network policies
kubectl get networkpolicy -n arcana
```

## Security Features

- **RBAC**: Limited ServiceAccount permissions
- **Network Policies**: Strict traffic isolation
- **Pod Security**: Non-root, read-only filesystem
- **Secrets**: Sensitive data in Kubernetes Secrets
- **TLS**: Ingress TLS termination (production)

## Monitoring

### Prometheus Metrics

All pods expose `/metrics` endpoint:

```bash
# Port-forward to view metrics
kubectl port-forward -n arcana svc/arcana-controller 8080:80
curl http://localhost:8080/metrics
```

### Health Checks

- Controller: `GET /health`
- Service/Repository: gRPC health check

## Troubleshooting

### Common Issues

**Pods not starting:**
```bash
kubectl describe pod -n arcana <pod-name>
kubectl logs -n arcana <pod-name> --previous
```

**Service connection issues:**
```bash
# Test connectivity from within cluster
kubectl run -n arcana debug --rm -it --image=busybox -- wget -O- http://arcana-controller/health
```

**Database connection:**
```bash
# Check repository layer logs
kubectl logs -n arcana -l app.kubernetes.io/name=arcana-repository
```

## Cleanup

```bash
# Delete all resources
kubectl delete -k deployment/kubernetes/

# Or delete namespace (removes everything)
kubectl delete namespace arcana
```
