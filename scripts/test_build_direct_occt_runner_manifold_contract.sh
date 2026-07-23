#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_SCRIPT="$ROOT/scripts/build_direct_occt_runner.sh"

grep -Fq 'MANIFOLD_DIR="${ECKY_MANIFOLD_RUNTIME_DIR:-$ROOT/.dist/runtime/manifold}"' "$BUILD_SCRIPT"
grep -Fq '"$MANIFOLD_DIR/include"' "$BUILD_SCRIPT"
grep -Fq '"$MANIFOLD_DIR/lib/libmanifold.a"' "$BUILD_SCRIPT"
grep -Fq 'Manifold runtime missing. Run scripts/prepare_manifold_runtime.sh first.' "$BUILD_SCRIPT"

missing_runtime="$(mktemp -d)/missing-manifold"
error_output="$(mktemp)"
if ECKY_OCCT_RUNTIME_DIR="$ROOT/.dist/runtime/occt" \
  ECKY_MANIFOLD_RUNTIME_DIR="$missing_runtime" \
  bash "$BUILD_SCRIPT" >"$error_output" 2>&1; then
  echo "Direct runner build unexpectedly accepted missing Manifold runtime" >&2
  exit 1
fi
grep -Fq 'Manifold runtime missing. Run scripts/prepare_manifold_runtime.sh first.' "$error_output"

echo "Direct runner Manifold build contract passed"
