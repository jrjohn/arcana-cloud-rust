#!/bin/bash
# Integration smoke test for arcana-cloud-rust
# Usage: bash scripts/integration-smoke-test.sh <BASE_URL> <LABEL> [TIMEOUT_SECONDS]
set -euo pipefail

BASE_URL="${1:-http://localhost:8080}"
LABEL="${2:-test}"
TIMEOUT="${3:-180}"
TS=$(date +%s%3N)
USERNAME="ci_${LABEL}_${TS}"
EMAIL="${USERNAME}@ci.test"
PASSWORD="CiPassword1!"

echo "=== Integration Smoke Test [${LABEL}] → ${BASE_URL} ==="

# ── 1. Health check ──────────────────────────────────────────
echo "▶ [1/4] Health check ..."
DEADLINE=$(($(date +%s) + TIMEOUT))
while true; do
  if curl -sf "${BASE_URL}/health" > /dev/null 2>&1; then
    echo "  ✓ Health OK"
    break
  fi
  [[ $(date +%s) -ge $DEADLINE ]] && echo "  ✗ Health timeout after ${TIMEOUT}s" && exit 1
  sleep 5
done

# ── 2. Register ──────────────────────────────────────────────
echo ""
echo "▶ [2/4] Register (POST /api/v1/auth/register) ..."
REG_HTTP_CODE=$(curl -s -o /tmp/smoke-reg-${LABEL}.json -w "%{http_code}" \
    -X POST "${BASE_URL}/api/v1/auth/register" \
    -H "Content-Type: application/json" \
    -d "{\"username\":\"${USERNAME}\",\"email\":\"${EMAIL}\",\"password\":\"${PASSWORD}\"}" \
    2>/dev/null || echo "000")

# Rust returns 200 (not 201) for register
if [ "${REG_HTTP_CODE}" -lt 200 ] || [ "${REG_HTTP_CODE}" -gt 201 ]; then
  echo "  ✗ Register failed — HTTP ${REG_HTTP_CODE}"
  cat /tmp/smoke-reg-${LABEL}.json 2>/dev/null || true
  exit 1
fi
echo "  ✓ Register OK — HTTP ${REG_HTTP_CODE}"

# ── 3. Login ─────────────────────────────────────────────────
echo ""
echo "▶ [3/4] Login (POST /api/v1/auth/login) ..."
LOGIN_HTTP_CODE=$(curl -s -o /tmp/smoke-login-${LABEL}.json -w "%{http_code}" \
    -X POST "${BASE_URL}/api/v1/auth/login" \
    -H "Content-Type: application/json" \
    -d "{\"username_or_email\":\"${USERNAME}\",\"password\":\"${PASSWORD}\"}" \
    2>/dev/null || echo "000")

if [ "${LOGIN_HTTP_CODE}" != "200" ]; then
  echo "  ✗ Login failed — HTTP ${LOGIN_HTTP_CODE}"
  cat /tmp/smoke-login-${LABEL}.json 2>/dev/null || true
  exit 1
fi

TOKEN=$(node -e "const d=require('fs').readFileSync('/tmp/smoke-login-${LABEL}.json','utf8');const j=JSON.parse(d);console.log((j.data||{}).access_token||'')" 2>/dev/null || echo "")
USER_ID=$(node -e "const d=require('fs').readFileSync('/tmp/smoke-login-${LABEL}.json','utf8');const j=JSON.parse(d);console.log(((j.data||{}).user||{}).id||'')" 2>/dev/null || echo "")
if [ -z "${TOKEN}" ]; then
  echo "  ✗ No access_token in login response"
  cat /tmp/smoke-login-${LABEL}.json 2>/dev/null || true
  exit 1
fi
echo "  ✓ Login OK — token=${TOKEN:0:20}... user_id=${USER_ID}"

# ── 4. Authenticated call (GET /api/v1/users/{id}) ───────────
echo ""
echo "▶ [4/4] Authenticated call (GET /api/v1/users/${USER_ID}) ..."
ME_HTTP_CODE=$(curl -s -o /tmp/smoke-me-${LABEL}.json -w "%{http_code}" \
    "${BASE_URL}/api/v1/users/${USER_ID}" \
    -H "Authorization: Bearer ${TOKEN}" \
    2>/dev/null || echo "000")

if [ "${ME_HTTP_CODE}" != "200" ]; then
  echo "  ✗ Authenticated call failed — HTTP ${ME_HTTP_CODE}"
  cat /tmp/smoke-me-${LABEL}.json 2>/dev/null || true
  exit 1
fi

ME_USER=$(node -e "const d=require('fs').readFileSync('/tmp/smoke-me-${LABEL}.json','utf8');const j=JSON.parse(d);console.log((j.data||{}).username||'?')" 2>/dev/null || echo "?")
echo "  ✓ Auth call OK — user: ${ME_USER}"

echo ""
echo "=== ✅ All 4 smoke tests PASSED [${LABEL}] ==="
