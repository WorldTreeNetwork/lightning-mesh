#!/usr/bin/env bash
# 0wgr.2 — household NAT. Does not close dsd.
set -euo pipefail
. "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib.sh"
E2E_SUITE=nat
e2e_log start "nat"

a_wan=$(remote "$A" "ip -4 -o addr show eth1 | awk '{print \$4}' | cut -d/ -f1 | head -1" | tr -d '\r')
b_wan=$(remote "$B" "ip -4 -o addr show eth2 | awk '{print \$4}' | cut -d/ -f1 | head -1" | tr -d '\r')
a_gw=$(remote "$A" "uci get mjolnir.meshd.gateway" | tr -d '\r')
b_gw=$(remote "$B" "uci get mjolnir.meshd.gateway" | tr -d '\r')
[[ "$a_wan" == 192.168.1.* ]] || e2e_fail "node-a WAN not 192.168.1.x (got '$a_wan')"
[[ "$b_wan" == 192.168.50.* ]] || e2e_fail "node-b WAN not 192.168.50.x (got '$b_wan')"
[ "$a_gw" = auto ] || e2e_fail "node-a gateway=$a_gw want auto"
[ "$b_gw" = 0 ] || e2e_fail "node-b gateway=$b_gw want 0"
e2e_log wan "a=$a_wan gw=$a_gw b=$b_wan gw=$b_gw"

oa=$(overlay_ip "$A" | tr -d '\r')
ob=$(overlay_ip "$B" | tr -d '\r')
hops=$(remote "$B" "traceroute -n -m 4 $oa 2>/dev/null | awk 'NR==2{print \$2; exit}'" | tr -d '\r')
[ "$hops" = "$oa" ] || e2e_fail "overlay traceroute hop1='$hops' want $oa"
e2e_log overlay "1 hop $ob -> $oa"

if remote "$A" "ping -c1 -W2 $b_wan >/dev/null 2>&1"; then
	e2e_fail "A can ping B WAN $b_wan (inbound NAT should fail)"
fi
e2e_log inbound "A cannot ping $b_wan"
e2e_log pass "nat"
echo "PASS nat"
