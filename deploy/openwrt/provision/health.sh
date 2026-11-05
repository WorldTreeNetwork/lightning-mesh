#!/usr/bin/env bash
# Read-only picture of a node after stock install. The lamp follows the
# identity ULA on br-mesh. 10.254 is not assigned. A missing house guild
# is success. UDP 49737 (0xC249) on that ULA is the backhaul socket.
# `creat(babeld.underlay.pid): File exists` in logread is babeld noise,
# not a failed install. This script does not read that log.
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$DIR/lib.sh"

HOST=""
while [ $# -gt 0 ]; do
	case "$1" in
		--bind-iface) PROVISION_BIND_IFACE="${2:?--bind-iface needs an interface}"; shift 2 ;;
		-*) echo "unknown option: $1" >&2; exit 2 ;;
		*) HOST="$1"; shift ;;
	esac
done
export PROVISION_BIND_IFACE
provision_install_ssh_wrapper
provision_host_ok "$HOST" || { echo "host must look like root@192.168.1.1 or root@10.42.x.1" >&2; exit 2; }
HOST="$(provision_resolve_host "$HOST")"
echo "dial: $HOST"

log="$(provision_ssh "$HOST" 'sh -s' <<'EOF'
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

if ip -4 addr show dev br-mesh 2>/dev/null | grep -q 'inet 10\.254\.'; then
	echo "legacy_10_254: present"
else
	echo "legacy_10_254: absent"
fi

# MESH_IROH_PORT is 49737, printed in /proc/net/udp6 as hex C249.
if grep -qi ':C249' /proc/net/udp6 2>/dev/null; then
	echo "backhaul_udp: bound"
else
	echo "backhaul_udp: absent"
fi
echo "house_guild_for_backhaul: not_required"

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
)"
printf '%s\n' "$log"

if printf '%s\n' "$log" | grep -qx 'node_id: unknown'; then
	echo "health: meshd status did not print a node id" >&2
	exit 1
fi
if printf '%s\n' "$log" | grep -qx 'lamp: flashing-red'; then
	echo "health: identity ULA is missing while mesh L2 is up" >&2
	exit 1
fi
if printf '%s\n' "$log" | grep -qx 'backhaul_udp: absent' \
	&& printf '%s\n' "$log" | grep -q 'backhaul_ula=1'; then
	echo "health: identity ULA is up and UDP 49737 is not bound" >&2
	exit 1
fi
echo "health: ok"
