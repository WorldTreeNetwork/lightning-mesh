#!/usr/bin/env bash
# Read the node id from a box that already runs mjolnir-meshd, add one
# member line to the selected fleet inventory, render that group's peers
# onto the box, and restart meshd so the process is using the list.
# Does not install packages or rewrite the radio.
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$DIR/lib.sh"
OW="$(provision_openwrt_dir)"
# shellcheck source=../fleet-file.sh
source "$OW/fleet-file.sh"

NAME=""
GROUP=""
HOST=""
NOTES=""
while [ $# -gt 0 ]; do
	case "$1" in
		--name) NAME="${2:?--name needs a value}"; shift 2 ;;
		--group) GROUP="${2:?--group needs a value}"; shift 2 ;;
		--notes) NOTES="${2:?--notes needs a value}"; shift 2 ;;
		-*) echo "unknown option: $1" >&2; exit 2 ;;
		*) HOST="$1"; shift ;;
	esac
done
provision_host_ok "$HOST" || { echo "host must look like root@192.168.1.1" >&2; exit 2; }
provision_name_ok "$NAME" || { echo "name must be letters, numbers, '.', '_' or '-'" >&2; exit 2; }
provision_name_ok "$GROUP" || { echo "group must be letters, numbers, '.', '_' or '-'" >&2; exit 2; }

CONF="$(fleet_conf "$OW")"
ADDR="${HOST#*@}"

echo ">> reading identity on $HOST"
status="$(ssh -o BatchMode=yes -o ConnectTimeout=8 "$HOST" \
	'mjolnir-meshd status --secret-file /etc/mjolnir/secret')"
node_id="$(printf '%s\n' "$status" | provision_parse_node_id || true)"
fleet_valid_node_id "$node_id" || { echo "status did not print a node id" >&2; exit 1; }
echo "node_id: $node_id"

facts="$(ssh -o BatchMode=yes -o ConnectTimeout=8 "$HOST" 'sh -s' <<'EOF'
model=$(cat /tmp/sysinfo/model 2>/dev/null || echo OpenWrt)
board=$(cat /tmp/sysinfo/board_name 2>/dev/null || echo unknown)
rel=$(. /etc/openwrt_release; printf '%s' "${DISTRIB_RELEASE:-unknown}")
mac=$(cat /sys/class/net/br-lan/address 2>/dev/null || cat /sys/class/net/eth0/address 2>/dev/null || echo unknown)
build=$(mjolnir-meshd status --secret-file /etc/mjolnir/secret | sed -n 's/^  build:    //p' | head -n 1)
printf 'model=%s\nboard=%s\nrelease=%s\nmac=%s\nbuild=%s\n' "$model" "$board" "$rel" "$mac" "$build"
EOF
)"
model="$(printf '%s\n' "$facts" | sed -n 's/^model=//p')"
board="$(printf '%s\n' "$facts" | sed -n 's/^board=//p')"
release="$(printf '%s\n' "$facts" | sed -n 's/^release=//p')"
mac="$(printf '%s\n' "$facts" | sed -n 's/^mac=//p')"
build="$(printf '%s\n' "$facts" | sed -n 's/^build=//p')"
if [ -z "$NOTES" ]; then
	NOTES="enrolled $(date -u +%Y-%m-%d) board ${board} eth ${mac} OpenWrt ${release} build ${build}"
fi
case "$NOTES" in
	*'|'*) echo "notes must not contain '|'" >&2; exit 2 ;;
esac

tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT
rc=0
provision_insert_member "$NAME" "$ADDR" "$node_id" "$model" "$NOTES" "$GROUP" <"$CONF" >"$tmp" || rc=$?
if [ "$rc" -eq 2 ]; then
	echo ">> $node_id is already in $CONF"
elif [ "$rc" -eq 3 ]; then
	echo "name $NAME is already a different node in $CONF" >&2
	exit 1
elif [ "$rc" -ne 0 ]; then
	echo "could not insert the inventory line (rc=$rc)" >&2
	exit 1
else
	cp -f "$tmp" "$CONF"
	echo ">> added $NAME to $CONF (group $GROUP, dial $ADDR)"
fi

echo ">> rendering peers"
fleet_render_peers "$CONF" "$node_id" "$HOST"
echo ">> restarting mjolnir-meshd so the peer list is the one running"
ssh -o BatchMode=yes -o ConnectTimeout=8 "$HOST" \
	'/etc/init.d/mjolnir-meshd enable; /etc/init.d/mjolnir-meshd restart'
echo "enroll ok: $NAME $node_id"
