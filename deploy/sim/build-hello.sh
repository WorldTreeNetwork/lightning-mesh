#!/usr/bin/env bash
# Optional x86_64 musl mjolnir-hello for sim guests. Same embed order as
# deploy/openwrt/build-hello.sh. Does not replace the aarch64 fleet binary.
#
# Usage:  deploy/sim/build-hello.sh
set -euo pipefail

TARGET="x86_64-unknown-linux-musl"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${REPO_ROOT}"
OUT="deploy/sim/mjolnir-hello-x86_64"
WEB_DIR="hello-mesh-web"
CROSS_TARGET="target/sim-cross"

if [ "${SKIP_WEB:-0}" != 1 ]; then
	command -v bun >/dev/null 2>&1 || { echo "bun not found — install it or set SKIP_WEB=1" >&2; exit 1; }
	echo ">> building hello-mesh-web frontend and syncing into crates/mjolnir-hello/static/"
	(cd "${WEB_DIR}" && bun install --frozen-lockfile && bun run build:embed)
else
	echo ">> SKIP_WEB=1 — assuming crates/mjolnir-hello/static/ is already fresh"
fi
[ -f "crates/mjolnir-hello/static/index.html" ] || { echo "crates/mjolnir-hello/static/index.html missing" >&2; exit 1; }

echo ">> building mjolnir-hello for ${TARGET}"
docker run --rm \
  -v "${REPO_ROOT}:/work" -w /work \
  -e CARGO_TARGET_DIR="/work/${CROSS_TARGET}" \
  messense/rust-musl-cross:x86_64-musl \
  cargo build --release --locked --target "${TARGET}" \
    -p mjolnir-hello --bin mjolnir-hello

mkdir -p deploy/sim
cp "${CROSS_TARGET}/${TARGET}/release/mjolnir-hello" "${OUT}"
echo ">> done -> ${OUT}  ($(du -h "${OUT}" | cut -f1))"
file "${OUT}" 2>/dev/null || true
