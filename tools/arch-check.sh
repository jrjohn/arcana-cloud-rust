#!/usr/bin/env bash
# Run the same arch-qube scan CI runs (Jenkinsfile stage "Architecture Qube").
# Exit 0 = pass. Anything else = not done (AGENTS.md §4).
#
#   tools/arch-check.sh
#   ARCH_QUBE_IMAGE=... tools/arch-check.sh
#
# Streams the source in with tar | docker cp (a bind mount scans an empty tree under
# Docker-in-Docker — that is how the old CI gate passed while checking nothing), and copies
# the report out to arch-qube-reports/.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE="${ARCH_QUBE_IMAGE:-arcana.boo/arcana/arch-qube:latest}"
THRESHOLD=90          # keep equal to the Jenkinsfile stage "Architecture Qube" (--threshold 90)
FRAMEWORK=rust        # keep equal to the Jenkinsfile (--framework rust)
NAME="arch-check-$$"

command -v docker >/dev/null || { echo "arch-check: docker not found — cannot check (this is NOT a pass)" >&2; exit 2; }
docker info >/dev/null 2>&1 || { echo "arch-check: docker daemon not reachable — cannot check (this is NOT a pass)" >&2; exit 2; }

cleanup() { docker rm -fv "$NAME" >/dev/null 2>&1 || true; }
trap cleanup EXIT

docker create --name "$NAME" -v /src -v /output "$IMAGE" \
    scan /src --framework "$FRAMEWORK" --no-ai --ci \
    --format json,markdown -o /output --threshold "$THRESHOLD" >/dev/null \
    || { echo "arch-check: could not create the arch-qube container (image $IMAGE) — NOT a pass" >&2; exit 2; }

# macOS bsdtar carries extended attributes (com.apple.provenance) that docker cp refuses.
if tar --version 2>/dev/null | grep -q bsdtar; then
    TAR_NOXATTR=(--no-xattrs --no-mac-metadata)
    export COPYFILE_DISABLE=1
else
    TAR_NOXATTR=(--no-xattrs)
fi
# Same exclusions as the Jenkinsfile stage.
tar "${TAR_NOXATTR[@]}" --exclude=./.git --exclude=./target --exclude=./.scannerwork \
    --exclude=./coverage --exclude=./arch-qube-reports \
    -C "$ROOT" -cf - . | docker cp - "$NAME":/src

set +e
docker start -a "$NAME"
RC=$?
set -e

mkdir -p "$ROOT/arch-qube-reports"
docker cp "$NAME":/output/. "$ROOT/arch-qube-reports/" >/dev/null 2>&1 || true

if [ "$RC" -eq 0 ]; then
    echo "arch-check: PASS (report: arch-qube-reports/)"
else
    echo "arch-check: FAIL rc=$RC — fix the structure; do not lower the threshold (AGENTS.md §4)" >&2
fi
exit "$RC"
