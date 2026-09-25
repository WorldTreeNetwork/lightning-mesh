#!/usr/bin/env bash
# Build mjolnir-meshd as a static x86_64 musl binary for sim-lab OpenWrt
# guests (q35). Does not replace deploy/openwrt/mjolnir-meshd-aarch64.
#
# Usage:  deploy/sim/build-meshd.sh [BIN]   (default BIN=mjolnir-meshd)
set -euo pipefail

BIN="${1:-mjolnir-meshd}"
TARGET="x86_64-unknown-linux-musl"
FEATURES="${FEATURES:-daemon}"
# mjolnir-txn lives in crate mjolnir-apply, not mjolnir-mesh.
PKG="${MJOLNIR_PKG:-mjolnir-mesh}"
[ "$BIN" = mjolnir-txn ] && PKG=mjolnir-apply && FEATURES=""

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${REPO_ROOT}"
OUT="deploy/sim/${BIN}-x86_64"

GIT_SHA="$(git -C "${REPO_ROOT}" rev-parse --short HEAD 2>/dev/null || echo unknown)"
[ -n "$(git -C "${REPO_ROOT}" status --porcelain 2>/dev/null)" ] && GIT_SHA="${GIT_SHA}-dirty"

# Isolated from target/openwrt-cross (aarch64 glibc mix).
CROSS_TARGET="target/sim-cross"

echo ">> building ${BIN} for ${TARGET} (features=${FEATURES}, build=${GIT_SHA})"
docker run --rm \
  -v "${REPO_ROOT}:/work" -w /work \
  -e CARGO_TARGET_DIR="/work/${CROSS_TARGET}" \
  -e MJOLNIR_BUILD="${GIT_SHA}" \
  messense/rust-musl-cross:x86_64-musl \
  cargo build --release --locked --target "${TARGET}" \
    -p "${PKG}" --bin "${BIN}" ${FEATURES:+--features "${FEATURES}"}

mkdir -p deploy/sim
cp "${CROSS_TARGET}/${TARGET}/release/${BIN}" "${OUT}"
echo ">> done -> ${OUT}  ($(du -h "${OUT}" | cut -f1))"
file "${OUT}" 2>/dev/null || true
