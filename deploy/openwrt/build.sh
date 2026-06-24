#!/usr/bin/env bash
# Build mjolnir-meshd as a static aarch64 musl binary for OpenWrt mt76 nodes
# (MT7981 / MT7986, Cortex-A53 / ARM64). Unlike the MikroTik target there's no
# container: OpenWrt is real Linux, so the static musl binary runs natively. Pair
# it with babeld from OpenWrt's package repo. See mjolnir-mesh-dkb.
#
# Usage:  deploy/openwrt/build.sh [BIN]   (default BIN=mjolnir-meshd). Run anywhere.
set -euo pipefail

BIN="${1:-mjolnir-meshd}"
TARGET="aarch64-unknown-linux-musl"
FEATURES="${FEATURES:-daemon}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${REPO_ROOT}"
OUT="deploy/openwrt/${BIN}-aarch64"

echo ">> building ${BIN} for ${TARGET} (features=${FEATURES})"
# Same cross approach as deploy/mikrotik (messense rust-musl-cross), aarch64 image.
# target/ lives on the mounted repo, so deps compile once and incremental rebuilds
# are fast. No OCI packaging — we just want the static binary.
docker run --rm \
  -v "${REPO_ROOT}:/work" -w /work \
  messense/rust-musl-cross:aarch64-musl \
  cargo build --release --locked --target "${TARGET}" \
    -p mjolnir-mesh --bin "${BIN}" --features "${FEATURES}"

cp "target/${TARGET}/release/${BIN}" "${OUT}"
echo ">> done -> ${OUT}  ($(du -h "${OUT}" | cut -f1))"
file "${OUT}" 2>/dev/null || true
