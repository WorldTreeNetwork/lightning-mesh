#!/usr/bin/env bash
# 0wgr.5 — B must ACK A's first lease. Does not close wvg.4.
set -euo pipefail
. "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib.sh"
E2E_SUITE=dhcp-crdt
e2e_log start "dhcp-crdt"

sta_dev=$(remote "$PHONE" "iw dev | awk '/Interface/{d=\$2} /type managed/{print d; exit}'" | tr -d '\r')
[ -n "$sta_dev" ] || e2e_fail "no STA iface"
sta_mac=$(remote "$PHONE" "cat /sys/class/net/$sta_dev/address" | tr -d '\r')
ipv4=$(remote "$PHONE" "ip -4 -o addr show $sta_dev | awk '{print \$4}' | cut -d/ -f1 | head -1" | tr -d '\r')
[ -n "$ipv4" ] || e2e_fail "STA has no IPv4"
e2e_log sta "mac=$sta_mac ip=$ipv4 dev=$sta_dev"

ap_if() { remote "$1" "iw dev | awk '/Interface/{d=\$2} /type AP/{print d; exit}'" | tr -d '\r'; }
bssid_b=$(remote "$B" "iw dev $(ap_if "$B") info | awk '/addr/{print \$2; exit}'" | tr -d '\r')
remote "$PHONE" "iw dev $sta_dev disconnect >/dev/null 2>&1 || true; sleep 1; iw dev $sta_dev connect LightningMesh 2437 $bssid_b || true"
sleep 3
remote "$PHONE" "udhcpc -i $sta_dev -n -q -t 4 -T 1 -r $ipv4 >/tmp/dhcp-crdt.log 2>&1 || true"
ipv4_after=$(remote "$PHONE" "ip -4 -o addr show $sta_dev | awk '{print \$4}' | cut -d/ -f1 | head -1" | tr -d '\r')
dhcp_log=$(remote "$PHONE" "tr '\n' ' ' </tmp/dhcp-crdt.log")
e2e_log dhcp "$dhcp_log"

[ "$ipv4_after" = "$ipv4" ] || e2e_fail "B offered different IP '$ipv4_after' want '$ipv4'"

roam=$(remote "$B" "cat /tmp/dnsmasq.cfg01411c.d/mjolnir-roam.conf 2>/dev/null || cat /tmp/dnsmasq*.d/mjolnir-roam.conf 2>/dev/null || true")
echo "$roam" | grep -qi "${sta_mac}" || e2e_fail "B mjolnir-roam.conf lacks STA MAC $sta_mac (LeaseBook/hostsfile empty?)"
echo "$roam" | grep -q "$ipv4" || e2e_fail "B mjolnir-roam.conf lacks IP $ipv4"
e2e_log hostsfile "B roam.conf has $sta_mac $ipv4"
e2e_log pass "dhcp-crdt"
echo "PASS dhcp-crdt"
