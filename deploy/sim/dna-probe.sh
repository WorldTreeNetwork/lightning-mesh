#!/usr/bin/env bash
# DNAv4-like STA personality (RFC 4436): when the gateway MAC the STA has
# cached changes, send a DHCPREQUEST for the current lease (busybox
# `udhcpc -r IP` = INIT-REBOOT). Record keep vs hole. Does NOT close sz9.1.
set -euo pipefail
P="${SIM_PHONE:-root@10.99.0.14}"
A="${SIM_A:-root@10.99.0.10}"
B="${SIM_B:-root@10.99.0.11}"
S=(ssh -o BatchMode=yes -o ConnectTimeout=8 -o StrictHostKeyChecking=accept-new)
log() { printf '%s\n' "$*"; }
fail() { log "RED: $*"; exit 1; }
remote() { "${S[@]}" "$1" "$2"; }

sta_dev=$(remote "$P" "iw dev | awk '/Interface/{d=\$2} /type managed/{print d; exit}'")
[ -n "$sta_dev" ] || fail "no managed iface on phone"
ipv4=$(remote "$P" "ip -4 -o addr show $sta_dev | awk '{print \$4}' | cut -d/ -f1 | head -1")
[ -n "$ipv4" ] || fail "STA has no IPv4"
gw=$(remote "$P" "ip -4 route show dev $sta_dev | awk '/default/{print \$3; exit}'")
[ -n "$gw" ] || gw="${ipv4%.*}.1"
gw_mac=$(remote "$P" "ip neigh show $gw | awk '{for(i=1;i<=NF;i++) if(\$i==\"lladdr\") print \$(i+1)}'")
[ -n "$gw_mac" ] || fail "no neigh MAC for gw $gw"
sta_mac=$(remote "$P" "cat /sys/class/net/$sta_dev/address")
ap_if() { remote "$1" "iw dev | awk '/Interface/{d=\$2} /type AP/{print d; exit}'"; }
home=""
for n in "$A" "$B"; do
	if remote "$n" "iw dev $(ap_if "$n") station dump 2>/dev/null | grep -q $sta_mac"; then
		home=$n
		break
	fi
done
[ -n "$home" ] || fail "STA $sta_mac not on A or B AP"
ap=$(ap_if "$home")
fake_mac="02:de:ad:be:ef:00"

log "rfc: 4436 DNAv4-like (DHCPREQUEST for current lease after gw MAC change)"
log "sta_dev: $sta_dev ipv4: $ipv4 gw: $gw gw_mac_before: $gw_mac"
log "associated: $home ap: $ap"
log "NOTE: informs sz9.1; must not bd close sz9.1 (needs iPhone)"

# BusyBox ip neigh has show/flush only — change the AP iface MAC instead
# (iPhone DNAv4 trigger: cached gateway MAC no longer matches).
t0=$(date +%s%3N)
remote "$home" "ip link set $ap down; ip link set $ap address $fake_mac; ip link set $ap up"
log "gw_mac_injected: $fake_mac on $home $ap"

# RFC 4436 §2.2: DHCP INIT-REBOOT — REQUEST the current address (broadcast).
# busybox: -r IP requests this address; -n -q exit after one lease attempt.
remote "$P" "udhcpc -i $sta_dev -n -q -t 3 -T 1 -r $ipv4 >/tmp/dna-dhcp.log 2>&1 || true"
dhcp_log=$(remote "$P" "cat /tmp/dna-dhcp.log")
log "dhcp_log: $dhcp_log"

ipv4_after=$(remote "$P" "ip -4 -o addr show $sta_dev | awk '{print \$4}' | cut -d/ -f1 | head -1")
gw_mac_after=$(remote "$P" "ip neigh show $gw | awk '{for(i=1;i<=NF;i++) if(\$i==\"lladdr\") print \$(i+1)}'")
kept=0
[ "$ipv4_after" = "$ipv4" ] && kept=1

hole_ms=-1
for i in $(seq 1 20); do
	if remote "$P" "ping -c1 -W1 $gw >/dev/null 2>&1"; then
		t1=$(date +%s%3N)
		hole_ms=$((t1 - t0))
		break
	fi
done

log "ipv4_after: $ipv4_after kept: $kept"
log "gw_mac_after: $gw_mac_after"
log "hole_ms: $hole_ms (from AP-MAC change to ping-ok; -1 = never)"
log "sz9.1: LEFT OPEN"

# restore AP MAC
remote "$home" "ip link set $ap down; ip link set $ap address $gw_mac; ip link set $ap up; /usr/sbin/hostapd -B /tmp/ap.conf 2>/dev/null || true"

[ "$kept" = 1 ] || log "INFO: lease not kept (Linux udhcpc, not iPhone DNAv4)"
exit 0
