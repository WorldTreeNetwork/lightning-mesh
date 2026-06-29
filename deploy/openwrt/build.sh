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

# Build-identity stamp (mjolnir-mesh-0xu / -auu). build.rs prefers $MJOLNIR_BUILD
# over calling `git` itself — and in-container `git` FAILS on the bind-mounted repo
# ("fatal: detected dubious ownership"), so without this the banner silently
# stamps "unknown" and can no longer prove which source a node is running. Compute
# it on the HOST (where git trusts the repo) and pass it in, same as
# deploy/mikrotik/build.sh. Short SHA, `-dirty` suffix for an uncommitted tree.
GIT_SHA="$(git -C "${REPO_ROOT}" rev-parse --short HEAD 2>/dev/null || echo unknown)"
[ -n "$(git -C "${REPO_ROOT}" status --porcelain 2>/dev/null)" ] && GIT_SHA="${GIT_SHA}-dirty"

# Dedicated target dir for the cross-build (mjolnir-mesh-0xu). Cargo build scripts
# compile for the BUILD host, so a native `cargo build`/`clippy` on this host
# (glibc 2.39) leaves build-script binaries in target/ that the older-glibc build
# container cannot exec ("libc.so.6: version GLIBC_2.39 not found"). Isolating the
# cross-build's target dir means native and container builds never share host-side
# build artifacts. Lives under target/ so it stays git-ignored.
CROSS_TARGET="target/openwrt-cross"

echo ">> building ${BIN} for ${TARGET} (features=${FEATURES}, build=${GIT_SHA})"
# Same cross approach as deploy/mikrotik (messense rust-musl-cross), aarch64 image.
# CROSS_TARGET lives on the mounted repo, so deps compile once and incremental
# rebuilds are fast. No OCI packaging — we just want the static binary.
docker run --rm \
  -v "${REPO_ROOT}:/work" -w /work \
  -e CARGO_TARGET_DIR="/work/${CROSS_TARGET}" \
  -e MJOLNIR_BUILD="${GIT_SHA}" \
  messense/rust-musl-cross:aarch64-musl \
  cargo build --release --locked --target "${TARGET}" \
    -p mjolnir-mesh --bin "${BIN}" --features "${FEATURES}"

cp "${CROSS_TARGET}/${TARGET}/release/${BIN}" "${OUT}"
echo ">> done -> ${OUT}  ($(du -h "${OUT}" | cut -f1))"
file "${OUT}" 2>/dev/null || true
