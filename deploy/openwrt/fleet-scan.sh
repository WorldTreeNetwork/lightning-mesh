#!/usr/bin/env bash
# Record which inventory nodes answer from this machine.
#
# Writes ~/.config/lightning-admin/fleet-reach.json (or LIGHTNING_FLEET_REACH).
# The working file (fleet-nodes.conf) stays the rollout list and the peer
# groups. Guild names stay in guilds.json. This file is only "dial here".
# A node that does not answer is left as it was; a scan from one laptop
# cannot see the whole mesh.
#
# Candidates are kernel neighbors (not FAILED) plus each member's fallback
# ssh_addr. Each candidate gets GET /api/node. A 64-hex node_id is kept.
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=fleet-file.sh
source "$DIR/fleet-file.sh"
CONF="$(fleet_conf "$DIR")"
OUT="$(fleet_reach_file)"
mkdir -p "$(dirname "$OUT")"

hits="$(mktemp)"
cands="$(mktemp)"
trap 'rm -f "$hits" "$cands"' EXIT

declare -A seen=()
# IPv4 neighbors.
ip -4 neigh show 2>/dev/null | while read -r ip _rest; do
	case "$ip" in
		[0-9]*) ;;
		*) continue ;;
	esac
	case " $_rest " in
		*" FAILED "*|*" INCOMPLETE "*) continue ;;
	esac
	printf '%s\n' "$ip"
done >"$cands"

# IPv6 neighbors. Link-local needs the device as a zone id.
ip -6 neigh show 2>/dev/null | while read -r ip dev iface _rest; do
	case "$ip" in
		fe80:*)
			case " $_rest " in
				*" FAILED "*|*" INCOMPLETE "*) continue ;;
			esac
			printf '[%s%%%s]\n' "$ip" "$iface"
			;;
		[0-9a-fA-F]*:*)
			case " $_rest " in
				*" FAILED "*|*" INCOMPLETE "*) continue ;;
			esac
			printf '%s\n' "$ip"
			;;
	esac
done >>"$cands"

while IFS='|' read -r name addr node_id _model _notes _group; do
	[ "$(fleet_kind "$name" "$addr" "$node_id")" = member ] || continue
	case "$addr" in
		''|-) continue ;;
		*:*) printf '[%s]\n' "$addr" ;;
		*) printf '%s\n' "$addr" ;;
	esac
done <"$CONF" >>"$cands"

: >"$hits"
n=0
while IFS= read -r addr; do
	[ -n "$addr" ] || continue
	[ -n "${seen[$addr]+x}" ] && continue
	seen[$addr]=1
	n=$((n + 1))
	[ "$n" -le 80 ] || break
	body="$(curl -g -m 2 -sf "http://${addr}/api/node" 2>/dev/null || true)"
	[ -n "$body" ] || continue
	python3 - "$addr" "$body" >>"$hits" <<'PY' || true
import json, sys
addr, body = sys.argv[1], sys.argv[2]
try:
    value = json.loads(body)
except ValueError:
    sys.exit(0)
nid = str(value.get("node_id") or "").strip().lower()
name = str(value.get("name") or "").strip()
if len(nid) == 64 and all(c in "0123456789abcdef" for c in nid):
    print(f"{nid}\t{name}\t{addr}")
PY
done <"$cands"

python3 - "$OUT" "$hits" <<'PY'
import json, sys
from datetime import datetime, timezone
path, hits = sys.argv[1], sys.argv[2]
try:
    data = json.load(open(path))
except (OSError, ValueError):
    data = {}
nodes = data.get("nodes") if isinstance(data.get("nodes"), dict) else {}
now = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
seen = 0
with open(hits) as fh:
    for line in fh:
        nid, name, addr = (line.rstrip("\n").split("\t") + ["", "", ""])[:3]
        if not nid or not addr:
            continue
        row = nodes.get(nid) if isinstance(nodes.get(nid), dict) else {}
        row["ssh"] = addr
        row["seen_at"] = now
        if name:
            row["name"] = name
        nodes[nid] = row
        seen += 1
data["scanned_at"] = now
data["nodes"] = nodes
with open(path, "w") as fh:
    json.dump(data, fh, indent=2)
    fh.write("\n")
print(f">> {seen} node(s) answered; wrote {path}")
PY
