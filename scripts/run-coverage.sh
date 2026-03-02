#!/usr/bin/env sh
set -eu

mkdir -p coverage
rm -f coverage/lcov.info

cargo llvm-cov \
  --workspace \
  --lib \
  --exclude arcana-server \
  --exclude arcana-grpc \
  --exclude arcana-plugin-runtime \
  --exclude arcana-repository \
  --no-fail-fast \
  --lcov \
  --output-path coverage/lcov.info

echo 'Coverage generated:'
wc -l coverage/lcov.info
