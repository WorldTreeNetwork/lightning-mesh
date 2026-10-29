#!/usr/bin/env bash
# Which installation a fleet tool walks.
#
# One file is one installation: the nodes updated, flashed, and upgraded
# together, and (once peer render lands) one gossip peer set. A second
# household is another file, not more lines in this one.
#
#   LIGHTNING_FLEET_FILE=/path/to.conf    explicit file
#   LIGHTNING_FLEET=name                  deploy/openwrt/fleet.d/name.conf
#   (unset)                               deploy/openwrt/fleet-nodes.conf
#
# Columns: name|backhaul_addr|node_id|model|notes
# A line is a member only when name, backhaul address, and node id are set.
# `name|-|-|model|notes` records hardware in this installation that has no
# node id yet. Rollout tools skip those lines.
#
# Source this file. `fleet_conf <deploy/openwrt dir>` prints the path.
# `fleet_kind <name> <addr> <node_id>` prints member, pending, or skip.

fleet_conf() {
	local dir="$1"
	local name path
	if [ -n "${LIGHTNING_FLEET_FILE:-}" ]; then
		if [ ! -f "$LIGHTNING_FLEET_FILE" ]; then
			echo "inventory missing: $LIGHTNING_FLEET_FILE" >&2
			return 1
		fi
		printf '%s\n' "$LIGHTNING_FLEET_FILE"
		return 0
	fi
	if [ -n "${LIGHTNING_FLEET:-}" ]; then
		name="$LIGHTNING_FLEET"
		case "$name" in
			''|*[!A-Za-z0-9._-]*)
				echo "LIGHTNING_FLEET must be a file stem (letters, numbers, '.', '_', '-'): '$name'" >&2
				return 1
				;;
		esac
		path="$dir/fleet.d/${name}.conf"
		if [ ! -f "$path" ]; then
			echo "inventory missing: $path" >&2
			return 1
		fi
		printf '%s\n' "$path"
		return 0
	fi
	path="$dir/fleet-nodes.conf"
	if [ ! -f "$path" ]; then
		echo "inventory missing: $path" >&2
		return 1
	fi
	printf '%s\n' "$path"
}

fleet_kind() {
	local name="$1" addr="$2" node_id="$3"
	case "$name" in
		''|\#*) printf 'skip\n'; return 0 ;;
	esac
	case "$addr" in
		''|-) printf 'pending\n'; return 0 ;;
	esac
	case "$node_id" in
		''|-) printf 'pending\n'; return 0 ;;
	esac
	printf 'member\n'
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
	set -euo pipefail
	DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
	got="$(fleet_conf "$DIR")"
	[ "$got" = "$DIR/fleet-nodes.conf" ]
	members=0
	pending=0
	while IFS='|' read -r name addr node_id _model _notes; do
		case "$(fleet_kind "$name" "$addr" "$node_id")" in
			member) members=$((members + 1)) ;;
			pending) pending=$((pending + 1)) ;;
		esac
	done <"$got"
	[ "$members" -ge 1 ]
	[ "$pending" -ge 1 ]
	tmp="$(mktemp)"
	trap 'rm -f "$tmp"' EXIT
	printf 'only|-|-|model|note\n' >"$tmp"
	LIGHTNING_FLEET_FILE="$tmp"
	[ "$(fleet_conf "$DIR")" = "$tmp" ]
	[ "$(fleet_kind only - -)" = pending ]
	unset LIGHTNING_FLEET_FILE
	if LIGHTNING_FLEET='../fleet-nodes' fleet_conf "$DIR" >/dev/null 2>&1; then
		echo "path escape was accepted" >&2
		exit 1
	fi
	echo "fleet-file ok ($members members, $pending pending in $got)"
fi
