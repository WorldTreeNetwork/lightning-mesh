#!/usr/bin/env bash
# Roll an update across the whole fleet, ONE NODE AT A TIME, in-band.
# Wraps install-node.sh (staged payload + detached apply + health-gated
# rollback — see deploy/openwrt/README.md) over the fleet-nodes.conf inventory.
#
# Usage:  deploy/openwrt/update-fleet.sh [install-node options] [node-name ...]
#   deploy/openwrt/update-fleet.sh                          # binary/deps to every member
#   deploy/openwrt/update-fleet.sh --wireless node.env      # + radio config everywhere
#   deploy/openwrt/update-fleet.sh m3000 tr3000             # only these nodes
#
# Design choices a future operator/agent should know:
# - SEQUENTIAL on purpose: never update two nodes at once — the mesh must stay
#   routable so the remaining nodes can still be reached (and so a health-gate
#   rollback has live neighbours to gate against).
# - HALTS on the first failed node instead of marching a broken update across
#   the fleet. The failed node has rolled itself back (or says FAILED with
#   nothing changed); fix, then re-run — applies are idempotent, already-updated
#   nodes are touched-nothing no-ops.
# - Walks one installation file top to bottom (fleet-file.sh): default
#   fleet-nodes.conf, or LIGHTNING_FLEET / LIGHTNING_FLEET_FILE. Leaf-first,
#   wired jump node last. Lines with no node id are recorded hardware, not
#   rollout targets.
# - Dial the address fleet-scan.sh recorded for that node id, otherwise the
#   file's ssh_addr column. Unreachable nodes are skipped and reported.
# - After each apply, rewrites `list peer` to the other members of the same
#   group and restarts mjolnir-meshd. A bench line is not a house peer.
set -euo pipefail

# install-node.sh writes the inventory peers and, on a direct run, starts
# meshd. During a rollout it leaves the start to the rewrite below so each
# node is restarted once, with the member id from this file.
export MJOLNIR_FLEET_ROLLOUT=1

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=fleet-file.sh
source "$DIR/fleet-file.sh"
CONF="$(fleet_conf "$DIR")"
if [ ! -f "$DIR/mjolnir-hello-aarch64" ]; then
	echo "WARNING: $DIR/mjolnir-hello-aarch64 is missing. This rollout will not ship the front desk." >&2
	echo "WARNING: build it with deploy/openwrt/build-hello.sh, then re-run." >&2
fi

INSTALL_ARGS=()
ONLY=()
STAGE_ONLY=0
while [ $# -gt 0 ]; do
	case "$1" in
		--wireless|--health-timeout|--pkg-cache) INSTALL_ARGS+=("$1" "${2:?$1 needs a value}"); shift 2 ;;
		--stage-only)                            INSTALL_ARGS+=("$1"); STAGE_ONLY=1; shift ;;
		-*)                                      echo "unknown option: $1" >&2; exit 2 ;;
		*)                                       ONLY+=("$1"); shift ;;
	esac
done

want() {
	[ "${#ONLY[@]}" -eq 0 ] && return 0
	local n; for n in "${ONLY[@]}"; do [ "$n" = "$1" ] && return 0; done
	return 1
}

UPDATED=() SKIPPED=() PENDING=()
# Read the inventory on FD 3 — the ssh/scp calls inside the loop would
# otherwise eat the remaining lines from stdin and end the rollout early.
while IFS='|' read -r -u3 name addr node_id model notes _group; do
	case "$(fleet_kind "$name" "$addr" "$node_id")" in
		skip) continue ;;
		pending)
			want "$name" || continue
			echo ">> $name: recorded, no node id yet — not in this rollout"
			PENDING+=("$name")
			continue
			;;
	esac
	want "$name" || continue

	dial="$(fleet_dial_addr "$node_id" "$addr")"
	echo
	echo "===== $name ($model) — root@$dial ====="
	if [ "$dial" != "$addr" ]; then
		echo ">> scan address $dial (inventory fallback $addr)"
	fi
	if ! ssh -o BatchMode=yes -o ConnectTimeout=6 "root@$dial" true 2>/dev/null; then
		echo ">> UNREACHABLE — skipping ($notes)"
		SKIPPED+=("$name")
		continue
	fi

	if ! "$DIR/install-node.sh" "${INSTALL_ARGS[@]+"${INSTALL_ARGS[@]}"}" "root@$dial"; then
		# install-node couldn't READ the result in time — but the applier writes
		# it even on rollback, and a bouncing transit path (ProxyJump through a
		# wifi-reloading node) can outlast the read window on a node that actually
		# applied fine (mjolnir-mesh-3wj). Before halting the whole rollout, do one
		# final direct read once the node has had a moment to settle: if it says
		# OK, the apply stuck — continue instead of stranding downstream nodes.
		sleep 15
		final=$(ssh -o BatchMode=yes -o ConnectTimeout=8 "root@$dial" \
			"cat /root/mjolnir-stage/result 2>/dev/null" 2>/dev/null || true)
		case "$final" in
			OK*)
				echo ">> $name: install-node timed out reading the result, but the node"
				echo ">> reports '$final' — apply stuck (slow rejoin). Continuing."
				;;
			*)
				echo
				echo ">> ROLLOUT HALTED at $name — result='${final:-<unreadable>}'. If unreadable,"
				echo ">> the node may just be slow to rejoin; verify directly before assuming failure:"
				echo ">>   ssh root@$dial 'cat /root/mjolnir-stage/result; tail -40 /root/mjolnir-stage/apply.log'"
				echo ">> Already updated this run: ${UPDATED[*]:-none}. Re-running is safe (idempotent)."
				exit 1
				;;
		esac
	fi

	if [ "$STAGE_ONLY" = 0 ]; then
		echo ">> $name: rendering this group's peer set and starting meshd"
		fleet_render_peers "$CONF" "$node_id" "root@$dial"
		ssh -o BatchMode=yes -o ConnectTimeout=8 "root@$dial" \
			'/etc/init.d/mjolnir-meshd enable; /etc/init.d/mjolnir-meshd restart'
	fi

	# Post-check beyond the node's own health gate: it still routes the mesh.
	routes=$(ssh -o BatchMode=yes -o ConnectTimeout=6 "root@$dial" \
		"ip -4 route | grep -c 'via 10\.254' || true" 2>/dev/null || echo 0)
	echo ">> $name: OK (babel routes to $routes neighbour(s))"
	UPDATED+=("$name")
done 3< "$CONF"

echo
echo "===== fleet rollout summary ====="
echo "updated:     ${UPDATED[*]:-none}"
echo "unreachable: ${SKIPPED[*]:-none}"
echo "pending:     ${PENDING[*]:-none}"
[ "${#SKIPPED[@]}" -eq 0 ] || echo "(skipped nodes keep their old version — re-run '$0 ${SKIPPED[*]}' when they're back)"
