#!/usr/bin/env bash
# Build the IPQ5018 Caraboot/U-Boot (QCA u-boot-2016) as a 32-bit ARM ELF that
# MikroTik RouterBOOT can netboot (AArch32 hand-off; it then boots the aarch64
# OpenWrt FIT via QCA TrustZone). Output: caraboot-mango/u-boot (ELF @ 0x4A920000).
#
#   ./build-uboot.sh [defconfig=ipq5018_defconfig] [jobs=4]
set -euo pipefail
BUILD=/home/dorje/work/IdentiKey/openwrt-l23
UB="$BUILD/caraboot-mango"
DEFCONFIG="${1:-ipq5018_defconfig}"
JOBS="${2:-4}"
[ -d "$UB/.git" ] || { echo "!! caraboot tree missing at $UB"; exit 1; }

docker run --rm -v "$UB:/u" -w /u -u 0:0 debian:12 bash -c '
  set -e
  echo "=== install 32-bit ARM toolchain + u-boot deps ==="
  apt-get update -qq
  DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    build-essential gcc-arm-linux-gnueabi bison flex libssl-dev bc python3 \
    device-tree-compiler swig unzip file xxd >/tmp/apt.log 2>&1 || { tail -25 /tmp/apt.log; exit 1; }
  export CROSS_COMPILE=arm-linux-gnueabi-
  echo "=== configure '"$DEFCONFIG"' ==="
  make '"$DEFCONFIG"'
  echo "=== build (u-boot 2016 + modern gcc: tolerate warnings) ==="
  if ! make -j'"$JOBS"' KCFLAGS="-Wno-error" HOSTCFLAGS="-Wno-error" 2>/tmp/ub.log; then
    echo "--- parallel build failed; last 80 lines: ---"; tail -80 /tmp/ub.log
    echo "--- retry serial to pinpoint ---"
    make KCFLAGS="-Wno-error" HOSTCFLAGS="-Wno-error" 2>&1 | tail -60
  fi
  echo "=== artifacts ==="
  ls -la u-boot u-boot.bin u-boot.elf 2>/dev/null || true
  file u-boot 2>/dev/null || true
  chown -R 1000:1000 /u || true
  [ -f u-boot ] || { echo "!! u-boot ELF NOT produced"; exit 1; }
'
echo ">> build-uboot.sh done"
