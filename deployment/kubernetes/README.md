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

### Deploy to Kubernetes

```bash
# Base deployment (development)
kubectl apply -k deployment/kubernetes/

# Or use overlays for specific environments
kubectl apply -k deployment/kubernetes/overlays/dev/
kubectl apply -k deployment/kubernetes/overlays/production/
```

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
| `arcana-repository` | Database access | 2 | 9090 |

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
