#!/usr/bin/env bash
# Seed each node's human router name (mjolnir-mesh-t7i) from the fleet
# inventory. For every node in fleet-nodes.conf this sets UCI
# `mjolnir.meshd.name` to the inventory's `name` column and restarts meshd, so
# the node announces that name mesh-wide (gossiped, LWW) and adopts it as its
# system hostname. Idempotent — re-running just re-asserts the same value.
#
# Usage:  deploy/openwrt/seed-names.sh [node-name ...]
#   deploy/openwrt/seed-names.sh               # seed every node in the inventory
#   deploy/openwrt/seed-names.sh m3000 tr3000  # only these nodes
#
# Design choices a future operator/agent should know:
# - SSH is over the OVERLAY address (10.254.x, the inventory's backhaul_addr
#   column), same as update-fleet.sh — reaching it from the workstation needs
#   the jump-host ssh config from the README ("Fleet rollout" section).
# - Unreachable nodes (power-cycled / absent) are skipped and reported, not
#   fatal — the same tolerant posture as update-fleet.sh.
# - Restarts meshd so the name takes effect immediately. This is a UCI-config
#   restart of the daemon only (not mjolnir-apply); it briefly drops the CRDT
#   plane on that one node while babel keeps routing — cheap and in-band safe.
# - The inventory `name` column is the source of truth here; editing a node's
#   name means editing fleet-nodes.conf, then re-running this.
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONF="$DIR/fleet-nodes.conf"
[ -f "$CONF" ] || { echo "inventory missing: $CONF"; exit 1; }

ONLY=("$@")

want() {
	[ "${#ONLY[@]}" -eq 0 ] && return 0
	local n; for n in "${ONLY[@]}"; do [ "$n" = "$1" ] && return 0; done
	return 1
}

SEEDED=() SKIPPED=()
# Read the inventory on FD 3 — the ssh calls inside the loop would otherwise
# eat the remaining lines from stdin and end the walk early.
while IFS='|' read -r -u3 name addr node_id model notes; do
	case "$name" in ''|\#*) continue ;; esac
	want "$name" || continue

	echo
	echo "===== $name ($model) — root@$addr ====="
	if ! ssh -o BatchMode=yes -o ConnectTimeout=6 "root@$addr" true 2>/dev/null; then
		echo ">> UNREACHABLE — skipping ($notes)"
		SKIPPED+=("$name")
		continue
	fi

	# Set the name and restart meshd. Single-quote the value on the remote so a
	# name with shell-special chars is set literally; names are simple handles.
	if ssh -o BatchMode=yes "root@$addr" \
		"uci set mjolnir.meshd.name='$name' && uci commit mjolnir && /etc/init.d/mjolnir-meshd restart"; then
		echo ">> seeded name '$name'"
		SEEDED+=("$name")
	else
		echo ">> FAILED to seed '$name'" >&2
		SKIPPED+=("$name")
	fi
done 3<"$CONF"

echo
echo "seeded:  ${SEEDED[*]:-(none)}"
echo "skipped: ${SKIPPED[*]:-(none)}"
