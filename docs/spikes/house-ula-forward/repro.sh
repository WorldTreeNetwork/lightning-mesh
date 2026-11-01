#!/bin/bash
# Isolated netns spike. Does not touch the live mesh.
# Claim: a house /128 is reachable from the other site through Babel on a
# tunnel, and the local kernel does not treat that address as on-link.
set -u
ROOT="$(cd "$(dirname "$0")" && pwd)"
LOG="$ROOT/evidence.log"
: >"$LOG"
exec > >(tee -a "$LOG") 2>&1

NS_A=ula-spike-a
NS_B=ula-spike-b
HOUSE_A='fd81:2b22:8aca::1'
HOUSE_B='fd81:2b22:8aca::2'
OTHER='fd81:2b22:8aca::99'
STOCK='fd12:3456:789a::1'
LEGACY='fd53:6213:4797::1'

cleanup() {
  ip netns del "$NS_A" 2>/dev/null || true
  ip netns del "$NS_B" 2>/dev/null || true
}
trap cleanup EXIT
cleanup

ip netns add "$NS_A"
ip netns add "$NS_B"
ip link add ulaA type veth peer name ulaB
ip link set ulaA netns "$NS_A"
ip link set ulaB netns "$NS_B"

for ns in "$NS_A" "$NS_B"; do
  ip netns exec "$ns" ip link add br-mesh type dummy
  ip netns exec "$ns" ip link set br-mesh up
  ip netns exec "$ns" sysctl -w net.ipv6.conf.all.forwarding=1 >/dev/null
  ip netns exec "$ns" sysctl -w net.ipv6.conf.br-mesh.forwarding=1 >/dev/null
done
ip netns exec "$NS_A" ip link set ulaA up
ip netns exec "$NS_B" ip link set ulaB up
ip netns exec "$NS_A" ip -6 addr add "$HOUSE_A/128" dev br-mesh
ip netns exec "$NS_B" ip -6 addr add "$HOUSE_B/128" dev br-mesh
ip netns exec "$NS_B" ip -6 addr add "$STOCK/128" dev br-mesh
# Dual-assign stand-in: today's compiled /64 is a different /48.
ip netns exec "$NS_A" ip -6 addr add "$LEGACY/64" dev br-mesh

echo "=== on-link probe before babel (expect NOT dev br-mesh) ==="
ip netns exec "$NS_A" ip -6 route get "$HOUSE_B" || true
echo "=== random /64 address (expect NOT dev br-mesh) ==="
ip netns exec "$NS_A" ip -6 route get "$OTHER" || true
echo "=== legacy /64 sibling IS on-link (contrast, different /48) ==="
ip netns exec "$NS_A" ip -6 route get 'fd53:6213:4797::99' || true

cat >"$ROOT/a.conf" <<EOF
random-id true
interface ulaA type wired
redistribute local ip ${HOUSE_A}/128 allow
redistribute local deny
redistribute deny
in ip ${HOUSE_A}/128 allow
in ip ${HOUSE_B}/128 allow
in ip fc00::/7 deny
out ip ${HOUSE_A}/128 allow
out ip ${HOUSE_B}/128 allow
out ip fc00::/7 deny
EOF
cat >"$ROOT/b.conf" <<EOF
random-id true
interface ulaB type wired
redistribute local ip ${HOUSE_B}/128 allow
redistribute local deny
redistribute deny
in ip ${HOUSE_A}/128 allow
in ip ${HOUSE_B}/128 allow
in ip fc00::/7 deny
out ip ${HOUSE_A}/128 allow
out ip ${HOUSE_B}/128 allow
out ip fc00::/7 deny
EOF

# Second pair of configs tests whether `eq 128` on the /48 matches.
cat >"$ROOT/a-eq.conf" <<EOF
random-id true
interface ulaA type wired
redistribute local ip ${HOUSE_A}/128 allow
redistribute local deny
redistribute deny
in ip fd81:2b22:8aca::/48 eq 128 allow
in ip fc00::/7 deny
out ip fd81:2b22:8aca::/48 eq 128 allow
out ip fc00::/7 deny
EOF

ip netns exec "$NS_A" babeld -I "" -S "$ROOT/state-a" -h 1 -H 1 -c "$ROOT/a.conf" -d 1 &
PID_A=$!
ip netns exec "$NS_B" babeld -I "" -S "$ROOT/state-b" -h 1 -H 1 -c "$ROOT/b.conf" -d 1 &
PID_B=$!

echo "=== waiting for babel host route ==="
ok=0
for _ in $(seq 1 25); do
  if ip netns exec "$NS_A" ip -6 route show "$HOUSE_B/128" | grep -q ulaA; then
    ok=1
    break
  fi
  sleep 1
done
echo "babel_host_route=$ok"
echo "=== A's route to B ==="
ip netns exec "$NS_A" ip -6 route show "$HOUSE_B/128" || true
echo "=== route get B ==="
ip netns exec "$NS_A" ip -6 route get "$HOUSE_B" || true
echo "=== stock prefix must be absent ==="
ip netns exec "$NS_A" ip -6 route show "$STOCK/128" || true
echo "=== ping B ==="
ip netns exec "$NS_A" ping -6 -c 2 -W 2 "$HOUSE_B" || true
echo "=== ping random (must fail) ==="
if ip netns exec "$NS_A" ping -6 -c 1 -W 1 "$OTHER"; then
  echo "random_answered=yes"
else
  echo "random_answered=no"
fi

kill "$PID_A" "$PID_B" 2>/dev/null || true
wait "$PID_A" "$PID_B" 2>/dev/null || true
sleep 1

echo "=== eq-128 filter trial ==="
ip netns exec "$NS_A" ip -6 route del "$HOUSE_B/128" 2>/dev/null || true
ip netns exec "$NS_A" babeld -I "" -S "$ROOT/state-a2" -h 1 -H 1 -c "$ROOT/a-eq.conf" -d 1 &
PID_A=$!
ip netns exec "$NS_B" babeld -I "" -S "$ROOT/state-b2" -h 1 -H 1 -c "$ROOT/b.conf" -d 1 &
PID_B=$!
eq=0
for _ in $(seq 1 20); do
  if ip netns exec "$NS_A" ip -6 route show "$HOUSE_B/128" | grep -q ulaA; then
    eq=1
    break
  fi
  sleep 1
done
echo "eq128_installed=$eq"
kill "$PID_A" "$PID_B" 2>/dev/null || true
wait "$PID_A" "$PID_B" 2>/dev/null || true
echo "=== done ==="
