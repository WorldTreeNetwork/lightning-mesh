#!/usr/bin/env bash
# 0wgr.1 — lab health + vwifi partition. Does not bd close anything.
set -euo pipefail
# shellcheck source=lib.sh
. "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib.sh"
E2E_SUITE=health

restore_vwifi() {
	pgrep -x vwifi-server >/dev/null || nohup vwifi-server -l >/tmp/vwifi-server.log 2>&1 &
	sleep 1
	for h in "$A" "$B"; do
		remote "$h" 'pgrep vwifi-client >/dev/null || vwifi-client 10.99.0.1 -n 2 >/tmp/vwifi-client.log 2>&1 &' || true
	done
}

trap restore_vwifi EXIT

e2e_log start "health"

for spec in "$A:node-a" "$B:node-b" "$CPE:isp-cpe" "$NAT:house-nat" "$PHONE:phone"; do
	host="${spec%%:*}"
	name="${spec##*:}"
	remote "$host" "true" || e2e_fail "ssh $host ($name)"
	e2e_log ssh_ok "$name $host"
done

for h in "$A" "$B"; do
	remote "$h" "test -f /etc/mjolnir/sim-guest" || e2e_fail "missing sim-guest on $h"
done
e2e_log marker "sim-guest present on node-a/b"

mif=$(mesh_if "$A" | tr -d '\r' | awk 'NF{print; exit}')
[ -n "$mif" ] || e2e_fail "no mesh point iface on node-a"
plink=$(remote "$A" "iw dev $mif station dump 2>/dev/null | grep -cE 'plink:[[:space:]]*ESTAB' || true" | tr -d '\r')
[ "${plink:-0}" -ge 1 ] || e2e_fail "802.11s not ESTAB on $mif (plink='$plink')"
e2e_log estab "$mif ESTAB count=$plink"

oa=$(overlay_ip "$A")
ob=$(overlay_ip "$B")
[ -n "$oa" ] && [ -n "$ob" ] || e2e_fail "missing br-mesh overlay ip a=$oa b=$ob"
remote "$A" "ping -c1 -W3 $ob >/dev/null" || e2e_fail "overlay ping $oa -> $ob"
e2e_log overlay "ping $oa -> $ob ok"

# Partition: air down, overlay must die, SSH must live.
killall vwifi-server 2>/dev/null || true
sleep 3
if remote "$A" "ping -c1 -W2 $ob >/dev/null 2>&1"; then
	e2e_fail "overlay still works after vwifi-server kill (mgmt leak?)"
fi
remote "$A" "true" || e2e_fail "mgmt SSH died with vwifi"
e2e_log partition "overlay down, ssh ok"

restore_vwifi
sleep 4
# overlay may need a few seconds to ESTAB again
ok=0
for i in $(seq 1 15); do
	if remote "$A" "ping -c1 -W2 $ob >/dev/null 2>&1"; then
		ok=1
		break
	fi
	sleep 1
done
[ "$ok" = 1 ] || e2e_fail "overlay did not return after vwifi restore"
e2e_log restored "overlay ping ok"
e2e_log pass "health"
echo "PASS health"
