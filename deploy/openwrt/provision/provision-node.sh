#!/usr/bin/env bash
# Stock OpenWrt → Lightning Mesh, one entry point.
# Lightning Admin calls this. The steps are also runnable on their own:
#   preflight.sh   SSH, board, local binary, package cache
#   install-node.sh --wireless   stage, apply, start meshd
#   enroll.sh      inventory line, this node's peers, meshd restart
#   health.sh      radios, identity ULA, UDP 49737, lamp
#
# Stock SSH is often root@192.168.1.1. Once meshd stays up it claims the
# client /24 and that address is gone. This script follows br-lan (tr3000-b
# became root@10.42.149.1) and enrolls that gateway. It does not roll any
# other node. A missing house guild is a healthy first node.
# The wpad/hostapd pin lives in mjolnir-apply, not here.
#
# Usage:
#   provision-node.sh --name tr3000-b --group bench root@192.168.1.1
#   provision-node.sh --check root@10.42.149.1
#   provision-node.sh --enroll-only --name tr3000-b --group bench root@10.42.149.1
#   provision-node.sh --self-test
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$DIR/lib.sh"
OW="$(provision_openwrt_dir)"

NAME=""
GROUP="bench"
HOST=""
CHECK=0
ENROLL_ONLY=0
NO_WIRELESS=0
WIRELESS="$OW/fleet-secrets/wireless.env"
while [ $# -gt 0 ]; do
	case "$1" in
		--name) NAME="${2:?--name needs a value}"; shift 2 ;;
		--group) GROUP="${2:?--group needs a value}"; shift 2 ;;
		--wireless) WIRELESS="${2:?--wireless needs a file}"; shift 2 ;;
		--bind-iface) PROVISION_BIND_IFACE="${2:?--bind-iface needs an interface}"; shift 2 ;;
		--no-wireless) NO_WIRELESS=1; shift ;;
		--check) CHECK=1; shift ;;
		--enroll-only) ENROLL_ONLY=1; shift ;;
		--self-test) provision_self_test; exit 0 ;;
		-*) echo "unknown option: $1" >&2; exit 2 ;;
		*) HOST="$1"; shift ;;
	esac
done
export PROVISION_BIND_IFACE
provision_install_ssh_wrapper

if [ "$CHECK" = 1 ]; then
	exec "$DIR/health.sh" ${PROVISION_BIND_IFACE:+--bind-iface "$PROVISION_BIND_IFACE"} "$HOST"
fi

provision_host_ok "$HOST" || { echo "usage: provision-node.sh --name NAME [--group bench] root@host" >&2; exit 2; }
provision_name_ok "$NAME" || { echo "--name is required (inventory name, e.g. tr3000-b)" >&2; exit 2; }
provision_name_ok "$GROUP" || { echo "--group must be letters, numbers, '.', '_' or '-'" >&2; exit 2; }

PROVISION_LAN_MAC="$(provision_ssh "$HOST" 'cat /sys/class/net/br-lan/address 2>/dev/null || cat /sys/class/net/eth0/address' || true)"
export PROVISION_LAN_MAC

run_install() {
	local target="$1"
	if [ "$NO_WIRELESS" = 1 ]; then
		"$DIR/preflight.sh" ${PROVISION_BIND_IFACE:+--bind-iface "$PROVISION_BIND_IFACE"} --no-wireless "$target"
		"$OW/install-node.sh" "$target"
	else
		"$DIR/preflight.sh" ${PROVISION_BIND_IFACE:+--bind-iface "$PROVISION_BIND_IFACE"} --wireless "$WIRELESS" "$target"
		"$OW/install-node.sh" --wireless "$WIRELESS" "$target"
	fi
}

if [ "$ENROLL_ONLY" = 0 ]; then
	set +e
	run_install "$HOST"
	install_rc=$?
	set -e
	PROVISION_ALLOW_RENEW=1
	NEW="$(provision_resolve_host "$HOST")"
	unset PROVISION_ALLOW_RENEW
	if [ "$install_rc" -ne 0 ]; then
		if [ "$NEW" != "$HOST" ]; then
			echo ">> install lost $HOST after the client /24 claim; retrying once at $NEW"
			run_install "$NEW"
		else
			exit "$install_rc"
		fi
	fi
	HOST="$NEW"
else
	HOST="$(provision_resolve_host "$HOST")"
fi

echo "dial: $HOST"
"$DIR/enroll.sh" ${PROVISION_BIND_IFACE:+--bind-iface "$PROVISION_BIND_IFACE"} --name "$NAME" --group "$GROUP" "$HOST"
"$DIR/health.sh" ${PROVISION_BIND_IFACE:+--bind-iface "$PROVISION_BIND_IFACE"} "$HOST"
echo "provision ok: $NAME"
