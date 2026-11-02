#!/bin/sh
# Re-run the iroh-ula-dial spike. Needs passwordless sudo for a temporary veth.
set -eu
cd "$(dirname "$0")"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/lm-iroh-ula-spike}"
cargo run --quiet --manifest-path Cargo.toml
