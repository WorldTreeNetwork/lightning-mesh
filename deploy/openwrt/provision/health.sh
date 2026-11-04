#!/usr/bin/env bash
# Read-only picture of a node after stock install. The lamp line is the
# same rule meshd uses: mesh L2 up and the legacy 10.254 address absent
# is flashing red (overlay-sick). This build does not assign 10.254.
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$DIR/lib.sh"

HOST=""
while [ $# -gt 0 ]; do
	case "$1" in
		-*) echo "unknown option: $1" >&2; exit 2 ;;
		*) HOST="$1"; shift ;;
	esac
done
provision_host_ok "$HOST" || { echo "host must look like root@192.168.1.1" >&2; exit 2; }

ssh -o BatchMode=yes -o ConnectTimeout=8 "$HOST" 'sh -s' <<'EOF'
set -e
model=$(cat /tmp/sysinfo/model 2>/dev/null || echo unknown)
board=$(cat /tmp/sysinfo/board_name 2>/dev/null || echo unknown)
. /etc/openwrt_release
echo "model: $model"
echo "board: $board"
echo "release: ${DISTRIB_RELEASE:-unknown}"

status=$(mjolnir-meshd status --secret-file /etc/mjolnir/secret 2>/dev/null || true)
node_id=$(printf '%s\n' "$status" | sed -n 's/^  node id:  \([0-9a-fA-F]\{64\}\)$/\1/p' | head -n 1)
build=$(printf '%s\n' "$status" | sed -n 's/^  build:    //p' | head -n 1)
echo "node_id: ${node_id:-unknown}"
echo "build: ${build:-unknown}"

if [ -s /etc/mjolnir/house-guild-id ]; then
	echo "house_guild: present"
else
	echo "house_guild: absent"
fi

if /etc/init.d/wpad running 2>/dev/null; then
	echo "wpad: running"
else
	echo "wpad: stopped"
fi

iw dev 2>/dev/null | awk '
	$1=="Interface" { iface=$2 }
	$1=="addr" { addr=$2 }
	$1=="type" { kind=$2; if (NF>=3 && $2=="mesh") kind="mesh" }
	$1=="ssid" { ssid=$2; for (i=3;i<=NF;i++) ssid=ssid " " $i }
	$1=="channel" {
		if (kind=="AP") printf "client_ap: %s %s %s ch %s\n", iface, addr, ssid, $2
		if (kind=="mesh") printf "mesh_if: %s %s ch %s\n", iface, addr, $2
	}
'

estab=0
for iface in $(iw dev 2>/dev/null | awk '$1=="Interface"{i=$2} $1=="type" && $2=="mesh"{print i}'); do
	n=$(iw dev "$iface" station dump 2>/dev/null | grep -c 'mesh plink:.*ESTAB' || true)
	estab=$((estab + n))
done
echo "mesh_plinks: $estab"

overlay=0
ip -6 addr show dev br-mesh 2>/dev/null | grep -q 'inet6 fd' && overlay=1
oper=$(cat /sys/class/net/br-mesh/operstate 2>/dev/null || echo down)
echo "br_mesh: $oper backhaul_ula=$overlay"

# The lamp follows the identity ULA. 10.254 is not assigned. A missing house
# guild is not overlay-sick once that ULA is on br-mesh.
if [ "$overlay" = 0 ] && { [ "$estab" -gt 0 ] || [ "$oper" = up ]; }; then
	echo "lamp: flashing-red"
	echo "lamp_reason: mesh L2 is up and the identity ULA is not on br-mesh. The backhaul endpoint binds that address and does not need a house guild."
elif [ "$estab" -gt 0 ]; then
	echo "lamp: mesh"
	echo "lamp_reason: at least one 802.11s peer is established and the identity ULA is on br-mesh."
elif [ "$oper" != up ]; then
	echo "lamp: radios-down"
	echo "lamp_reason: br-mesh is $oper. If wpad is stopped, wifi-scripts waits on ubus wpa_supplicant and the radios never leave pending."
else
	echo "lamp: alone"
	echo "lamp_reason: the radio is up and no mesh peer is established."
fi
EOF
