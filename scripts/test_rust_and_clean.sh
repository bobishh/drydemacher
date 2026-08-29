#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
manifest="$repo_root/src-tauri/Cargo.toml"
test_target="$(mktemp -d "${TMPDIR:-/tmp}/ecky-rust-tests.XXXXXX")"

cleanup() {
  cargo clean --manifest-path "$manifest" --target-dir "$test_target" >/dev/null
  rmdir "$test_target"
}
trap cleanup EXIT

export CARGO_TARGET_DIR="$test_target"
cargo check --manifest-path "$manifest"
cargo test --manifest-path "$manifest" "$@"
