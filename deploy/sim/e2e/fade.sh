#!/usr/bin/env bash
# 0wgr.6 — fade 802.11s until overlay dies; restore. Does not replace two_site_netns.rs.
set -euo pipefail
. "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib.sh"
E2E_SUITE=fade
e2e_log start "fade"

restore() {
	vwifi-ctrl set 27914 0 0 0 2>/dev/null || true
	vwifi-ctrl set 28170 0 0 0 2>/dev/null || true
	vwifi-ctrl set 28938 0 0 0 2>/dev/null || true
}
trap restore EXIT

oa=$(overlay_ip "$A" | tr -d '\r')
ob=$(overlay_ip "$B" | tr -d '\r')
remote "$A" "ping -c1 -W3 $ob >/dev/null" || e2e_fail "overlay already down"
e2e_log baseline "ping $oa -> $ob ok"

# Move B far (cid from vwifi-ctrl ls; 28170 was B historically — apply to all non-zero)
while read -r cid rest; do
	[ -n "$cid" ] || continue
	vwifi-ctrl set "$cid" 0 50000 0 2>/dev/null || true
done < <(vwifi-ctrl ls 2>/dev/null | awk 'NF>=1 && $1 ~ /^[0-9]+$/ {print}')
e2e_log fade "vwifi-ctrl distance=50000"

down=0
for i in $(seq 1 35); do
	if ! remote "$A" "ping -c1 -W2 $ob >/dev/null 2>&1"; then
		down=1
		break
	fi
	sleep 1
done
[ "$down" = 1 ] || e2e_fail "overlay still up after fade"
remote "$A" "true" || e2e_fail "ssh died during fade"
e2e_log withdrawn "overlay down, ssh ok"

restore
up=0
for i in $(seq 1 20); do
	if remote "$A" "ping -c1 -W2 $ob >/dev/null 2>&1"; then
		up=1
		break
	fi
	sleep 1
done
[ "$up" = 1 ] || e2e_fail "overlay did not reconverge after restore"
e2e_log restored "overlay ping ok"
e2e_log pass "fade"
echo "PASS fade"
