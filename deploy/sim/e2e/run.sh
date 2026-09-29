#!/usr/bin/env bash
# Sim e2e dispatcher (mjolnir-mesh-0wgr). Never calls `bd close`.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
usage() {
	echo "usage: deploy/sim/e2e/run.sh <health|nat|keep-ip|dhcp-crdt|dna|fade|apply-rollback|all>" >&2
	echo "  health          0wgr.1 lab gate" >&2
	echo "  nat             0wgr.2 household NAT" >&2
	echo "  keep-ip         0wgr.3 (dual proto 158 is FAIL)" >&2
	echo "  dhcp-crdt       0wgr.5 B ACKs A's first lease" >&2
	echo "  dna             0wgr.4 RFC 4436 probe (does not close sz9.1)" >&2
	echo "  fade            0wgr.6 vwifi distance (FAIL until loss model bites)" >&2
	echo "  apply-rollback  0wgr.7 snapshot, fail closed, restore" >&2
	exit 2
}
[ "${1:-}" != "" ] || usage
case "$1" in
	health) exec "$HERE/health.sh" ;;
	nat) exec "$HERE/nat.sh" ;;
	keep-ip) exec "$HERE/keep-ip.sh" ;;
	dhcp-crdt) exec "$HERE/dhcp-crdt.sh" ;;
	dna) exec "$HERE/dna.sh" ;;
	fade) exec "$HERE/fade.sh" ;;
	apply-rollback) exec "$HERE/apply-rollback.sh" ;;
	all)
		"$HERE/health.sh"
		"$HERE/nat.sh"
		set +e
		"$HERE/keep-ip.sh"
		echo "all: keep-ip may FAIL while proto 158 does not move"
		;;
	-h|--help) usage ;;
	*) usage ;;
esac
