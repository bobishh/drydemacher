#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUNTIME_DIR="$(mktemp -d)/manifold-runtime"

ECKY_MANIFOLD_RUNTIME_DIR="$RUNTIME_DIR" \
  bash "$ROOT/scripts/prepare_manifold_runtime.sh"

test -f "$RUNTIME_DIR/lib/libmanifold.a"
test -f "$RUNTIME_DIR/include/manifold/manifold.h"
test -f "$RUNTIME_DIR/licenses/LICENSE"

python3 - "$RUNTIME_DIR/manifest.json" <<'PY'
import json
import sys

manifest = json.load(open(sys.argv[1]))
assert manifest["schemaVersion"] == 1
assert manifest["manifoldVersion"] == "3.5.2"
assert manifest["sourceCommit"] == "11235e6b8ebea2dbed8aec4285685aafd3d95667"
assert manifest["sourceArchiveSha256"] == "4fa8ba091b4b905fe19f9acd550484640426c5f10ccc0e0100143aa89fa8f5b9"
assert manifest["license"] == "Apache-2.0"
assert manifest["parallel"] is False
assert manifest["library"] == "lib/libmanifold.a"
assert len(manifest["librarySha256"]) == 64
PY

echo "Manifold runtime contract passed: $RUNTIME_DIR"
