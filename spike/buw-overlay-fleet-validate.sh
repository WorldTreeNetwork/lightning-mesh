#!/usr/bin/env bash
#
# buw.7 fleet validation: bring up the single-overlay-TUN data plane on TWO real
# deployed nodes and confirm end-to-end operation over real iroh transport.
#
# Runs against --overlay --internet (or --overlay --lan once mjolnir-mesh-buw.8
# lands): the underlay is iroh's own transport, so there is no overlay/underlay
# addressing conflict. A purely-local two-node run is blocked by buw.8, so this
# validates on hardware where the pieces already work (iroh peering proven in
# auu; babel-over-overlay proven in buw.1 / buw-multicast-spike.sh).
#
# Prereqs: both nodes flashed + install-node.sh run + rosters set + `option
# overlay '1'` added to each node's `config meshd` in /etc/config/mjolnir.
#
# Usage:  spike/buw-overlay-fleet-validate.sh root@<nodeA-ip> root@<nodeB-ip>
set -euo pipefail
A="${1:?usage: buw-overlay-fleet-validate.sh root@<A> root@<B>}"
B="${2:?usage: buw-overlay-fleet-validate.sh root@<A> root@<B>}"

say() { printf '\n== %s ==\n' "$*"; }

say "restart meshd on both nodes"
ssh "$A" 'service mjolnir-meshd restart'
ssh "$B" 'service mjolnir-meshd restart'
sleep 20  # let iroh peer + babel converge

say "1) single overlay TUN up on each node (exactly one mjolnir0, addr in 10.254/16)"
for H in "$A" "$B"; do
  echo "--- $H ---"
  ssh "$H" 'ip -o link show mjolnir0; ip -4 -o addr show mjolnir0'
  # must NOT have any per-peer mj-peer-* interfaces (the churn buw removes)
  n=$(ssh "$H" 'ip -o link show | grep -c mj-peer- || true')
  echo "   per-peer mj-peer-* interfaces: $n (expect 0)"
done

say "2) connection manager registered the peer (real iroh connection)"
ssh "$A" 'logread -e mjolnir_meshd | grep -E "overlay peer connected|overlay mode: single mjolnir0" | tail -3'
ssh "$B" 'logread -e mjolnir_meshd | grep -E "overlay peer connected|overlay mode: single mjolnir0" | tail -3'

say "3) static overlay babeld config (one mjolnir0 interface, RTT metric)"
ssh "$A" 'cat /etc/mjolnir/babeld.conf'

say "4) babel adjacency + a learned client route over mjolnir0"
# babeld -g diagnostic port (if enabled) or the kernel route table:
ssh "$A" 'ip -4 route show proto babel; echo "---neighbours---"; babeld -g 33123 2>/dev/null || true'

say "5) DATA PLANE: client on A reaches a host on B across the overlay"
echo "   (run from A's client LAN: ping a host in B's claimed /24 — traffic must"
echo "    cross mjolnir0 via the connection manager + FIB demux over iroh)"

cat <<'EOF'

VERDICT (manual): PASS when, on BOTH nodes,
  - exactly one mjolnir0 exists and ZERO mj-peer-* interfaces,
  - the log shows "overlay peer connected" (iroh connection registered),
  - babeld installs the peer's client /24 as a REACHABLE route via 10.254.x dev mjolnir0,
  - and a client ping crosses the overlay.
Any per-peer mj-peer-* interface or config churn (procd restarting babeld on peer
change) is a FAIL — the whole point of buw is that neither happens.
EOF
