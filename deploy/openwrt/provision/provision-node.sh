#!/usr/bin/env bash
# Stock OpenWrt → Lightning Mesh, one entry point.
# Lightning Admin calls this. The steps are also runnable on their own:
#   preflight.sh   SSH, board, local binary, package cache
#   install-node.sh --wireless   stage, apply, start meshd
#   enroll.sh      inventory line, peer list, meshd restart
#   health.sh      radios, plinks, lamp
#
# Usage:
#   provision-node.sh --name tr3000-b --group bench root@192.168.1.1
#   provision-node.sh --check root@192.168.1.1
#   provision-node.sh --enroll-only --name tr3000-b --group bench root@192.168.1.1
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
		--no-wireless) NO_WIRELESS=1; shift ;;
		--check) CHECK=1; shift ;;
		--enroll-only) ENROLL_ONLY=1; shift ;;
		--self-test) provision_self_test; exit 0 ;;
		-*) echo "unknown option: $1" >&2; exit 2 ;;
		*) HOST="$1"; shift ;;
	esac
done

if [ "$CHECK" = 1 ]; then
	exec "$DIR/health.sh" "$HOST"
fi

provision_host_ok "$HOST" || { echo "usage: provision-node.sh --name NAME [--group bench] root@host" >&2; exit 2; }
provision_name_ok "$NAME" || { echo "--name is required (inventory name, e.g. tr3000-b)" >&2; exit 2; }
provision_name_ok "$GROUP" || { echo "--group must be letters, numbers, '.', '_' or '-'" >&2; exit 2; }

if [ "$ENROLL_ONLY" = 0 ]; then
	if [ "$NO_WIRELESS" = 1 ]; then
		"$DIR/preflight.sh" --no-wireless "$HOST"
		"$OW/install-node.sh" "$HOST"
	else
		"$DIR/preflight.sh" --wireless "$WIRELESS" "$HOST"
		"$OW/install-node.sh" --wireless "$WIRELESS" "$HOST"
	fi
fi

"$DIR/enroll.sh" --name "$NAME" --group "$GROUP" "$HOST"
"$DIR/health.sh" "$HOST"
echo "provision ok: $NAME"
