#!/bin/sh
set -eu
cd "$(dirname "$0")/../../.."
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/lm-spike-demux}"
cargo build --manifest-path docs/spikes/overlay-ula-demux/Cargo.toml
exec sudo -n "$CARGO_TARGET_DIR/debug/overlay-ula-demux-spike"
