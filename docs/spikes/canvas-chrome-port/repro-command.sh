#!/usr/bin/env bash
# Re-run the canvas-chrome-port compile probe.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/lm-spike-canvas}"
cd "$ROOT/repro"
echo "== go path: tauri 2.11.5 + macos-private-api =="
cargo check --offline --message-format=short
echo "== negative: same sources, no macos-private-api (must fail on macOS) =="
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
cp -R src "$tmp/src"
cat >"$tmp/Cargo.toml" <<'EOF'
[package]
name = "canvas-chrome-port-spike-neg"
version = "0.0.0"
edition = "2021"
publish = false

[workspace]

[dependencies]
tauri = { version = "=2.11.5", features = [] }
tauri-runtime = "=2.11.3"
tauri-runtime-wry = "=2.11.4"
tauri-utils = "=2.9.3"
tauri-macros = "=2.6.3"
wry = "=0.55.1"
tao = "=0.35.3"
EOF
set +e
(cd "$tmp" && CARGO_TARGET_DIR="$CARGO_TARGET_DIR-neg" cargo check --offline --message-format=short)
neg=$?
set -e
if [[ "$(uname -s)" == "Darwin" ]]; then
  if [[ $neg -eq 0 ]]; then
    echo "NEGATIVE CHECK FAILED: expected compile error without macos-private-api on macOS" >&2
    exit 1
  fi
  echo "negative path failed as expected (exit $neg)"
else
  echo "negative path exit $neg (transparent is un-gated off macOS)"
fi
