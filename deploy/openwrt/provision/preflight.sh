#!/usr/bin/env bash
# Can this SSH host take a stock-OpenWrt Lightning install?
# Checks the operator side (binary, wireless env, package cache) and the
# router (OpenWrt release, board name). Does not change the router.
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$DIR/lib.sh"
OW="$(provision_openwrt_dir)"

HOST=""
WIRELESS="$OW/fleet-secrets/wireless.env"
NO_WIRELESS=0
while [ $# -gt 0 ]; do
	case "$1" in
		--wireless) WIRELESS="${2:?--wireless needs a file}"; shift 2 ;;
		--no-wireless) NO_WIRELESS=1; shift ;;
		-*) echo "unknown option: $1" >&2; exit 2 ;;
		*) HOST="$1"; shift ;;
	esac
done
provision_host_ok "$HOST" || { echo "host must look like root@192.168.1.1" >&2; exit 2; }

BIN="${MJOLNIR_MESHD_BIN:-$OW/mjolnir-meshd-aarch64}"
TXN="${MJOLNIR_TXN_BIN:-$OW/mjolnir-txn-aarch64}"
CACHE="$OW/pkg-cache"
[ -f "$BIN" ] || { echo "missing $BIN — run deploy/openwrt/build.sh" >&2; exit 1; }
[ -f "$TXN" ] || { echo "missing $TXN" >&2; exit 1; }
if [ "$NO_WIRELESS" = 0 ]; then
	[ -f "$WIRELESS" ] || { echo "missing wireless env $WIRELESS" >&2; exit 1; }
fi
need="babeld kmod-tun wpad-mesh-mbedtls wpad-basic-mbedtls hostapd-common libatomic1 iperf3"
for p in $need; do
	found=0
	for f in "$CACHE/$p"-[0-9]*.apk "$CACHE/$p"_*.ipk; do
		[ -f "$f" ] && found=1 && break
	done
	if [ "$found" = 0 ]; then
		echo "package cache has no $p (a stock box with no WAN cannot fetch it)" >&2
		exit 1
	fi
done

ssh -o BatchMode=yes -o ConnectTimeout=8 "$HOST" 'sh -s' <<'EOF'
set -e
[ -f /etc/openwrt_release ] || { echo "not OpenWrt: /etc/openwrt_release missing" >&2; exit 1; }
. /etc/openwrt_release
board=$(cat /tmp/sysinfo/board_name 2>/dev/null || true)
model=$(cat /tmp/sysinfo/model 2>/dev/null || true)
echo "release: ${DISTRIB_RELEASE:-unknown} ${DISTRIB_REVISION:-}"
echo "board: ${board:-unknown}"
echo "model: ${model:-unknown}"
echo "target: ${DISTRIB_TARGET:-unknown}"
EOF
echo "preflight ok: $HOST"
