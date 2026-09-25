#!/usr/bin/env bash
# Build an OpenWrt x86_64 q35 guest image for the sim lab (imagebuilder).
# Does not boot Filogic/Cudy sysupgrade. Artifacts stay under deploy/sim/.
#
# Usage:  deploy/sim/build-image.sh
set -euo pipefail

RELEASE="${OPENWRT_RELEASE:-25.12.5}"
TARGET="x86"
SUBTARGET="64"
PROFILE="${OPENWRT_PROFILE:-generic}"
IB_NAME="openwrt-imagebuilder-${RELEASE}-${TARGET}-${SUBTARGET}.Linux-x86_64"
IB_URL="https://downloads.openwrt.org/releases/${RELEASE}/targets/${TARGET}/${SUBTARGET}/${IB_NAME}.tar.zst"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${REPO_ROOT}"
IB_ROOT="${REPO_ROOT}/deploy/sim/imagebuilder"
FILES="${REPO_ROOT}/deploy/sim/files"
OUT_DIR="${REPO_ROOT}/deploy/sim"
OUT_IMG="${OUT_DIR}/openwrt-x86-64-generic-ext4-combined-efi.img.gz"

# wpad-mesh replaces default wpad-basic. hwsim + babel + tun for the lab.
PACKAGES="${OPENWRT_PACKAGES:-babeld kmod-tun wpad-mesh-mbedtls -wpad-basic-mbedtls kmod-mac80211-hwsim}"

mkdir -p "${IB_ROOT}" "${FILES}/etc/mjolnir"
[ -f "${FILES}/etc/mjolnir/sim-guest" ] || echo sim-lab > "${FILES}/etc/mjolnir/sim-guest"

if [ ! -d "${IB_ROOT}/${IB_NAME}" ]; then
	echo ">> fetching ${IB_URL}"
	curl -fsSL "${IB_URL}" -o "${IB_ROOT}/${IB_NAME}.tar.zst"
	tar -C "${IB_ROOT}" --zstd -xf "${IB_ROOT}/${IB_NAME}.tar.zst"
fi

echo ">> imagebuilder ${IB_NAME} PROFILE=${PROFILE} PACKAGES=${PACKAGES}"
make -C "${IB_ROOT}/${IB_NAME}" image \
	PROFILE="${PROFILE}" \
	PACKAGES="${PACKAGES}" \
	FILES="${FILES}"

BIN_DIR="${IB_ROOT}/${IB_NAME}/bin/targets/${TARGET}/${SUBTARGET}"
SRC="$(ls -1 "${BIN_DIR}"/openwrt-*-${TARGET}-${SUBTARGET}-${PROFILE}-ext4-combined-efi.img.gz 2>/dev/null | head -1 || true)"
if [ -z "${SRC}" ]; then
	echo ">> no ext4-combined-efi.img.gz in ${BIN_DIR}; listing:" >&2
	ls -la "${BIN_DIR}" >&2 || true
	exit 1
fi
cp -f "${SRC}" "${OUT_IMG}"
echo ">> done -> ${OUT_IMG}  ($(du -h "${OUT_IMG}" | cut -f1))"
file "${OUT_IMG}" 2>/dev/null || true
