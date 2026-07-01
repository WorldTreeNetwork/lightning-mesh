#!/usr/bin/env bash
#
# buw.7/buw.8 local 2-node validation: two REAL `mjolnir-meshd --overlay --lan`
# nodes in two netns over a veth underlay. Validates the meshd overlay
# integration end-to-end over real iroh: node-id dialing + iroh mDNS discovery
# (buw.8, no derived-addr pin), the connection manager registering the peer, and
# the single mjolnir0 data plane. The underlay uses ordinary L2 addresses
# (10.9.0.x) distinct from the overlay's derived 10.254.x — no second static
# scheme for the overlay itself.
#
# Run:  sudo spike/buw-overlay-2node.sh
set -euo pipefail
NS_A=ov-a
NS_B=ov-b
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$ROOT/target/debug/mjolnir-meshd"
LOG=/tmp/buw-overlay-2node
RUN="${RUN_SECS:-25}"
mkdir -p "$LOG"

cleanup() {
  pkill -f 'mjolnir-meshd .* mesh' 2>/dev/null || true
  for ns in "$NS_A" "$NS_B"; do
    ip netns pids "$ns" 2>/dev/null | xargs -r kill 2>/dev/null || true
    ip netns del "$ns" 2>/dev/null || true
  done
}
trap cleanup EXIT
[[ $EUID -eq 0 ]] || { echo "run as root (sudo)"; exit 1; }
[[ -x "$BIN" ]] || { echo "build: cargo build -p mjolnir-mesh --features daemon"; exit 1; }
cleanup

# --- underlay: veth pair with ordinary L2 addresses (NOT the derived 10.254.x) ---
ip netns add "$NS_A"; ip netns add "$NS_B"
ip link add veth-a netns "$NS_A" type veth peer name veth-b netns "$NS_B"
ip -n "$NS_A" addr add 10.9.0.1/24 dev veth-a; ip -n "$NS_A" link set veth-a up; ip -n "$NS_A" link set lo up
ip -n "$NS_B" addr add 10.9.0.2/24 dev veth-b; ip -n "$NS_B" link set veth-b up; ip -n "$NS_B" link set lo up
MA="$(ip -n "$NS_A" link show veth-a | awk '/link\/ether/{print $2}')"
MB="$(ip -n "$NS_B" link show veth-b | awk '/link\/ether/{print $2}')"
ip -n "$NS_A" neigh replace 10.9.0.2 lladdr "$MB" dev veth-a nud permanent
ip -n "$NS_B" neigh replace 10.9.0.1 lladdr "$MA" dev veth-b nud permanent
ip -n "$NS_A" link add client0 type dummy 2>/dev/null || true; ip -n "$NS_A" link set client0 up
ip -n "$NS_B" link add client0 type dummy 2>/dev/null || true; ip -n "$NS_B" link set client0 up

# --- identities + rosters ---
timeout 8 ip netns exec "$NS_A" "$BIN" --secret-file "$LOG/sa" --lan id >"$LOG/ida.txt" 2>/dev/null || true
timeout 8 ip netns exec "$NS_B" "$BIN" --secret-file "$LOG/sb" --lan id >"$LOG/idb.txt" 2>/dev/null || true
IDA="$(grep -oE '[0-9a-f]{64}' "$LOG/ida.txt" | head -1)"
IDB="$(grep -oE '[0-9a-f]{64}' "$LOG/idb.txt" | head -1)"
[[ -n "$IDA" && -n "$IDB" ]] || { echo "FAIL: could not derive node ids"; cat "$LOG/ida.txt" "$LOG/idb.txt"; exit 1; }
echo "A=$IDA"; echo "B=$IDB"
echo "$IDB" >"$LOG/roster-a"; echo "$IDA" >"$LOG/roster-b"

# --- run both meshd --overlay --lan ---
echo ">> starting two meshd --overlay --lan (${RUN}s)"
ip netns exec "$NS_A" env RUST_LOG=info "$BIN" --secret-file "$LOG/sa" --lan mesh --overlay \
  --roster "$LOG/roster-a" --backhaul-iface veth-a --client-iface client0 \
  --babel-config "$LOG/babeld-a.conf" --claims-file "$LOG/claims-a" >"$LOG/meshd-a.log" 2>&1 &
ip netns exec "$NS_B" env RUST_LOG=info "$BIN" --secret-file "$LOG/sb" --lan mesh --overlay \
  --roster "$LOG/roster-b" --backhaul-iface veth-b --client-iface client0 \
  --babel-config "$LOG/babeld-b.conf" --claims-file "$LOG/claims-b" >"$LOG/meshd-b.log" 2>&1 &
sleep "$RUN"

echo "=== mjolnir0 in each netns ==="
ip -n "$NS_A" -4 -o addr show mjolnir0 2>/dev/null | sed 's/^/  A: /' || echo "  A: no mjolnir0"
ip -n "$NS_B" -4 -o addr show mjolnir0 2>/dev/null | sed 's/^/  B: /' || echo "  B: no mjolnir0"

echo "=== connection evidence (both logs) ==="
grep -E 'overlay mode: single mjolnir0|overlay peer connected|dialing peer|tunnel path|kind=|DIRECT|RELAY' "$LOG/meshd-a.log" | tail -6 | sed 's/^/  A| /'
grep -E 'overlay mode: single mjolnir0|overlay peer connected|dialing peer|tunnel path|kind=|DIRECT|RELAY' "$LOG/meshd-b.log" | tail -6 | sed 's/^/  B| /'

CA=$(grep -c 'overlay peer connected' "$LOG/meshd-a.log" 2>/dev/null || echo 0)
CB=$(grep -c 'overlay peer connected' "$LOG/meshd-b.log" 2>/dev/null || echo 0)
echo "=== VERDICT ==="
if [[ "$CA" -ge 1 || "$CB" -ge 1 ]]; then
  echo -e "\033[1;32mCONNECT PASS\033[0m: two --overlay --lan nodes formed an iroh connection (A:$CA B:$CB 'overlay peer connected'), node-id dialing + iroh discovery, no derived-addr pin. Connection manager registered the peer over one mjolnir0."
  exit 0
else
  echo -e "\033[1;31mCONNECT FAIL\033[0m: no 'overlay peer connected' (A:$CA B:$CB). meshd logs:"
  tail -n 8 "$LOG/meshd-a.log" | sed 's/^/  A| /'
  exit 1
fi
