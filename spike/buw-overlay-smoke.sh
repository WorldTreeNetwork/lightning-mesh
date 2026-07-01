#!/usr/bin/env bash
#
# buw.7 single-node smoke: run the REAL `mjolnir-meshd mesh --overlay` binary and
# confirm the overlay data-plane WIRING starts — mjolnir0 comes up (buw.2), the
# reader/writer/FIB tasks start, and the static overlay babeld config is rendered
# (buw.5) — without a panic. No peers, so no iroh peering is exercised here; the
# two-node data path is proven separately (buw.1 spike + buw-multicast-spike.sh).
#
# Run:  sudo spike/buw-overlay-smoke.sh
set -euo pipefail
NS=buw-ov
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$ROOT/target/debug/mjolnir-meshd"
LOG=/tmp/buw-overlay-smoke
mkdir -p "$LOG"

cleanup() {
  ip netns pids "$NS" 2>/dev/null | xargs -r kill 2>/dev/null || true
  ip netns del "$NS" 2>/dev/null || true
}
trap cleanup EXIT
[[ $EUID -eq 0 ]] || { echo "run as root (sudo)"; exit 1; }
[[ -x "$BIN" ]] || { echo "build first: cargo build -p mjolnir-mesh --features daemon"; exit 1; }
cleanup

# Namespace + a dummy backhaul iface for the node's derived 10.254.x address.
ip netns add "$NS"
ip -n "$NS" link set lo up
ip -n "$NS" link add bh0 type dummy
ip -n "$NS" link set bh0 up

echo ">> running meshd mesh --overlay (8s, no peers)"
timeout 8 ip netns exec "$NS" "$BIN" \
  --secret-file "$LOG/secret" --lan mesh \
  --overlay --backhaul-iface bh0 \
  --babel-config "$LOG/babeld.conf" \
  --claims-file "$LOG/claims.state" \
  --client-iface bh0 >"$LOG/meshd.log" 2>&1 || true

echo "=== meshd overlay log lines ==="
grep -iE 'overlay|mjolnir0|babeld|panic|error' "$LOG/meshd.log" | head -20 | sed 's/^/  /'
echo "=== rendered babeld config ($LOG/babeld.conf) ==="
sed 's/^/  /' "$LOG/babeld.conf" 2>/dev/null || echo "  (no config rendered)"

echo "=== VERDICT ==="
up=$(grep -c 'overlay mode: single mjolnir0 up' "$LOG/meshd.log" || true)
cfg=$(grep -c 'interface mjolnir0 type tunnel' "$LOG/babeld.conf" 2>/dev/null || true)
panic=$(grep -c 'panicked' "$LOG/meshd.log" || true)
if [[ "$up" -ge 1 && "$cfg" -ge 1 && "$panic" -eq 0 ]]; then
  echo -e "\033[1;32mSMOKE PASS\033[0m: mjolnir0 up + static overlay babeld config rendered, no panic."
  exit 0
else
  echo -e "\033[1;31mSMOKE FAIL\033[0m: up=$up cfg=$cfg panic=$panic — see $LOG/meshd.log"
  exit 1
fi
