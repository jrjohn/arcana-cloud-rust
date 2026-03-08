#!/usr/bin/env bash
# ============================================================
# Kind K8s Smoke Test — Arcana Cloud Rust (3-Layer gRPC)
#
# Creates a temporary Kind cluster, loads the CI image,
# deploys the 3-layer gRPC architecture, waits for all pods
# to become Ready, runs the integration smoke test, then tears
# the cluster down.
#
# Usage:
#   bash scripts/kind-smoke-test.sh [SOURCE_IMAGE] [LABEL] [TIMEOUT_SECS]
#
# Arguments:
#   SOURCE_IMAGE  Docker image to tag as arcana-cloud-rust:ci
#                 e.g. localhost:5000/arcana/rust-app:build-42
#                 Default: arcana-cloud-rust:ci (use as-is)
#   LABEL         Protocol label for smoke test output  (default: k8s-grpc)
#   TIMEOUT_SECS  Seconds to wait for pods / smoke test (default: 480)
#
# Requires: kind, kubectl, docker, bash
# ============================================================
set -euo pipefail

# ── Config ────────────────────────────────────────────────────
SOURCE_IMAGE="${1:-arcana-cloud-rust:ci}"
LABEL="${2:-k8s-grpc}"
TIMEOUT="${3:-480}"

CLUSTER_NAME="arcana-ci-$(date +%s)"
NS="arcana-ci-kind-rust"
NODE_PORT="30094"
EXPECTED_PODS="3"
APP_LABELS="app in (arcana-ci-repository,arcana-ci-service,arcana-ci-controller)"
MANIFEST="deployment/kubernetes/ci/kind-ci-grpc.yaml"
CI_IMAGE="arcana-cloud-rust:ci"

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║   Arcana Cloud Rust — Kind K8s gRPC Smoke Test          ║"
echo "║   Cluster : ${CLUSTER_NAME}"
echo "║   Image   : ${SOURCE_IMAGE} → ${CI_IMAGE}"
echo "║   Timeout : ${TIMEOUT}s"
echo "╚══════════════════════════════════════════════════════════╝"

# ── Cleanup trap ──────────────────────────────────────────────
cleanup() {
    local EXIT_CODE=$?
    echo ""
    echo "▶ [cleanup] Deleting Kind cluster '${CLUSTER_NAME}' ..."
    kind delete cluster --name "${CLUSTER_NAME}" 2>/dev/null || true
    if [ "${EXIT_CODE}" -ne 0 ]; then
        echo "✗ Kind smoke test FAILED (exit ${EXIT_CODE})"
    fi
    exit "${EXIT_CODE}"
}
trap cleanup EXIT INT TERM

# ── 1. Verify kind is available ───────────────────────────────
echo ""
echo "▶ [1/7] Checking kind ..."
kind version
kubectl version --client --short 2>/dev/null || kubectl version --client

# ── 2. Tag/prepare the CI image ───────────────────────────────
echo ""
echo "▶ [2/7] Preparing image '${CI_IMAGE}' ..."
if [ "${SOURCE_IMAGE}" != "${CI_IMAGE}" ]; then
    echo "  Tagging ${SOURCE_IMAGE} → ${CI_IMAGE}"
    docker tag "${SOURCE_IMAGE}" "${CI_IMAGE}"
else
    echo "  Using existing image '${CI_IMAGE}'"
    docker inspect "${CI_IMAGE}" > /dev/null 2>&1 || {
        echo "✗ Image '${CI_IMAGE}' not found locally — cannot proceed"
        exit 1
    }
fi

# ── 3. Create Kind cluster ────────────────────────────────────
echo ""
echo "▶ [3/7] Creating Kind cluster '${CLUSTER_NAME}' ..."
kind create cluster --name "${CLUSTER_NAME}" --config - <<EOF
kind: Cluster
apiVersion: kind.x-k8s.io/v1alpha4
nodes:
  - role: control-plane
    extraPortMappings:
      - containerPort: ${NODE_PORT}
        hostPort: ${NODE_PORT}
        protocol: TCP
EOF

# Export kubeconfig and fix server address for container-to-container access
kind export kubeconfig --name "${CLUSTER_NAME}"
KIND_CONTAINER="${CLUSTER_NAME}-control-plane"
KIND_IP=$(docker inspect -f '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' "${KIND_CONTAINER}" 2>/dev/null | tr ' ' '\n' | grep -v '^$' | head -1)
if [ -n "${KIND_IP}" ]; then
    echo "  Patching kubeconfig server to https://${KIND_IP}:6443 (kind node IP)"
    kubectl config set-cluster "kind-${CLUSTER_NAME}" --server="https://${KIND_IP}:6443"
else
    echo "  Warning: could not determine kind node IP, using default kubeconfig"
fi

# ── 4. Load Docker image into Kind ───────────────────────────
echo ""
echo "▶ [4/7] Loading image '${CI_IMAGE}' into cluster '${CLUSTER_NAME}' ..."
kind load docker-image "${CI_IMAGE}" --name "${CLUSTER_NAME}"

# ── 5. Apply Kubernetes manifest ─────────────────────────────
echo ""
echo "▶ [5/7] Applying manifest '${MANIFEST}' ..."
kubectl apply -f "${MANIFEST}"

# ── 6. Wait for pods ─────────────────────────────────────────
echo ""
echo "▶ [6/7] Waiting for ${EXPECTED_PODS} pods to be Ready (timeout ${TIMEOUT}s) ..."
echo "  Namespace: ${NS}"
echo "  Selector : ${APP_LABELS}"

ELAPSED=0
SLEEP_INTERVAL=10

while true; do
    READY=$(kubectl get pods -n "${NS}" \
        -l "${APP_LABELS}" \
        --field-selector=status.phase=Running \
        -o jsonpath='{.items[*].status.containerStatuses[*].ready}' 2>/dev/null \
        | tr ' ' '\n' | grep -c "true" || true)

    TOTAL=$(kubectl get pods -n "${NS}" \
        -l "${APP_LABELS}" \
        -o jsonpath='{.items[*].metadata.name}' 2>/dev/null \
        | tr ' ' '\n' | grep -c "." || true)

    echo "  ... ${ELAPSED}s — pods ready: ${READY}/${EXPECTED_PODS} (total found: ${TOTAL})"

    if [ "${READY}" -ge "${EXPECTED_PODS}" ]; then
        echo "  ✓ All ${EXPECTED_PODS} pods are Ready"
        break
    fi

    if [ "${ELAPSED}" -ge "${TIMEOUT}" ]; then
        echo ""
        echo "✗ Pods did not become Ready within ${TIMEOUT}s"
        echo ""
        echo "── Pod status ──────────────────────────────────────────"
        kubectl get pods -n "${NS}" -o wide || true
        echo ""
        echo "── Recent events ───────────────────────────────────────"
        kubectl get events -n "${NS}" --sort-by='.lastTimestamp' | tail -30 || true
        echo ""
        echo "── Pod logs (repository) ───────────────────────────────"
        kubectl logs -n "${NS}" -l "app=arcana-ci-repository" --tail=50 2>/dev/null || true
        echo "── Pod logs (service) ──────────────────────────────────"
        kubectl logs -n "${NS}" -l "app=arcana-ci-service" --tail=50 2>/dev/null || true
        echo "── Pod logs (controller) ───────────────────────────────"
        kubectl logs -n "${NS}" -l "app=arcana-ci-controller" --tail=50 2>/dev/null || true
        exit 1
    fi

    sleep "${SLEEP_INTERVAL}"
    ELAPSED=$((ELAPSED + SLEEP_INTERVAL))
done

# ── 7. Run integration smoke test ────────────────────────────
echo ""
echo "▶ [7/7] Running integration smoke test ..."

# Determine the Kind node IP for NodePort access
NODE_IP=$(kubectl get nodes -o jsonpath='{.items[0].status.addresses[?(@.type=="InternalIP")].address}' 2>/dev/null)
if [ -z "${NODE_IP}" ]; then
    NODE_IP="localhost"
fi
BASE_URL="http://${NODE_IP}:${NODE_PORT}"

echo "  Base URL : ${BASE_URL}"
echo "  Label    : ${LABEL}"
echo ""

# Run with remaining timeout budget
REMAINING=$((TIMEOUT - ELAPSED))
[ "${REMAINING}" -lt 60 ] && REMAINING=120

bash scripts/integration-smoke-test.sh "${BASE_URL}" "k8s-grpc" 120

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║   ✓ Kind K8s gRPC smoke test PASSED                     ║"
echo "╚══════════════════════════════════════════════════════════╝"
