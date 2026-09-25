#!/usr/bin/env bash
# Instrument: Linux STA hops A→B. Does NOT close 5wc/wvg/sz9.1.
# Exit 0 only if the keep-IP contract in add-sim-roam-keep-ip/design.md holds.
set -euo pipefail
A="${SIM_A:-root@10.99.0.10}"
B="${SIM_B:-root@10.99.0.11}"
P="${SIM_PHONE:-root@10.99.0.14}"
S=(ssh -o BatchMode=yes -o ConnectTimeout=8 -o StrictHostKeyChecking=accept-new)
log() { printf '%s\n' "$*"; }

fail() { log "RED: $*"; exit 1; }

remote() { "${S[@]}" "$1" "$2"; }

ap_if() { remote "$1" "iw dev | awk '/Interface/{d=\$2} /type AP/{print d; exit}'"; }
ap_a=$(ap_if "$A")
ap_b=$(ap_if "$B")
bssid_a=$(remote "$A" "iw dev $ap_a info | awk '/addr/{print \$2; exit}'")
bssid_b=$(remote "$B" "iw dev $ap_b info | awk '/addr/{print \$2; exit}'")
[ -n "$bssid_a" ] && [ -n "$bssid_b" ] || fail "missing AP BSSIDs a=$bssid_a b=$bssid_b"

# STA: first hwsim managed iface
sta_dev=$(remote "$P" "iw dev | awk '/Interface/{d=\$2} /type managed/{print d; exit}'")
[ -n "$sta_dev" ] || fail "no managed iface on phone (did vwifi-client start?)"

sta_mac=$(remote "$P" "cat /sys/class/net/$sta_dev/address")
log "mechanism: iw-connect"
log "sta_mac: $sta_mac"
log "bssid_a: $bssid_a bssid_b: $bssid_b sta_dev: $sta_dev"

# Associate to A
remote "$P" "iw dev $sta_dev disconnect >/dev/null 2>&1 || true; sleep 1; iw dev $sta_dev connect LightningMesh 2437 $bssid_a || true"
sleep 3
remote "$P" "udhcpc -i $sta_dev -n -q -t 4 -T 1 >/tmp/sta-dhcp.log 2>&1 || true"
ipv4=$(remote "$P" "ip -4 -o addr show $sta_dev | awk '{print \$4}' | cut -d/ -f1 | head -1")
[ -n "$ipv4" ] || fail "STA got no IPv4 on $sta_dev"

assoc_a_before=$(remote "$A" "iw dev $ap_a station dump | grep -c $sta_mac || true")
assoc_b_before=$(remote "$B" "iw dev $ap_b station dump | grep -c $sta_mac || true")
neigh_a=$(remote "$A" "ip neigh show $ipv4 | head -1")
r158_a_before=$(remote "$A" "ip route show proto 158 | grep -c $ipv4 || true")
r158_b_before=$(remote "$B" "ip route show proto 158 | grep -c $ipv4 || true")
lease_before=$(remote "$A" "grep -i $sta_mac /etc/mjolnir/leases.state 2>/dev/null | head -c 200 || echo none")

log "ipv4: $ipv4"
log "assoc_a_before: $assoc_a_before assoc_b_before: $assoc_b_before"
log "neigh_nud_a: $neigh_a"
log "route158_a_before: $r158_a_before route158_b_before: $r158_b_before"
log "lease_before: $lease_before"

[ "$assoc_a_before" != 0 ] || fail "STA not associated to A before hop"
[ "$assoc_b_before" = 0 ] || fail "STA already on B before hop"

# Probe
probe_log=$(mktemp)
remote "$P" "for i in \$(seq 1 25); do ping -c1 -W1 10.42.69.1 >/dev/null && echo OK \$i || echo LOST \$i; sleep 1; done" >"$probe_log" &
probe_pid=$!
sleep 1

# Hop
log "hop: iw connect LightningMesh $bssid_b"
remote "$P" "iw dev $sta_dev disconnect >/dev/null 2>&1 || true; sleep 1; iw dev $sta_dev connect LightningMesh 2437 $bssid_b"
sleep 5
wait "$probe_pid" || true

ipv4_after=$(remote "$P" "ip -4 -o addr show $sta_dev | awk '{print \$4}' | cut -d/ -f1 | head -1")
assoc_a_after=$(remote "$A" "iw dev $ap_a station dump | grep -c $sta_mac || true")
assoc_b_after=$(remote "$B" "iw dev $ap_b station dump | grep -c $sta_mac || true")
r158_a_after=$(remote "$A" "ip route show proto 158 | grep -c $ipv4 || true")
r158_b_after=$(remote "$B" "ip route show proto 158 | grep -c $ipv4 || true")
lease_after=$(remote "$A" "grep -i $sta_mac /etc/mjolnir/leases.state 2>/dev/null | head -c 200 || echo none")
loss=$(grep -c LOST "$probe_log" || true)
ok=$(grep -c OK "$probe_log" || true)
rm -f "$probe_log"

log "ipv4_after: $ipv4_after"
log "assoc_a_after: $assoc_a_after assoc_b_after: $assoc_b_after"
log "route158_a_after: $r158_a_after route158_b_after: $r158_b_after"
log "lease_after: $lease_after"
log "probe: ok=$ok lost=$loss"
log "NOTE: this harness must not bd close 5wc/wvg/sz9.1"

[ "$assoc_b_after" != 0 ] || fail "no association transition to B"
[ "$assoc_a_after" = 0 ] || fail "STA still on A after hop"
[ "$ipv4_after" = "$ipv4" ] || fail "IPv4 changed $ipv4 -> $ipv4_after"
[ "$r158_a_after" = 0 ] || fail "A still has proto 158 /$ipv4"
[ "$r158_b_after" != 0 ] || fail "B missing proto 158 /$ipv4"
log "GREEN keep-IP hop"
exit 0
