#!/bin/sh
# One-off commit-confirm: single Ethernet port -> WAN (add-single-port-wan-uplink).
# snapshot -> apply -> health gate -> rollback. Detached; survives the SSH session.
PORT=eth0
D=/root/mjolnir-stage/wanport
RESULT=$D/result
mkdir -p $D/backup
exec >>$D/apply.log 2>&1
log() { echo "[$(date '+%H:%M:%S')] $*"; }
finish() { echo "$1" > $RESULT.tmp && mv $RESULT.tmp $RESULT; log "result: $1"; exit 0; }
mkdir -p /etc/mjolnir/txn
exec 9>/etc/mjolnir/txn/lock
flock -n 9 || { echo "FAILED: another network transaction owns the node lock" > $RESULT; exit 1; }
rm -f $RESULT

[ -d /sys/class/net/br-lan/brif/$PORT ] || finish "NOOP: $PORT is not on br-lan"
uci -q get network.wan >/dev/null && finish "FAILED: network.wan already exists; refusing to guess"
LAN_ADDR=$(uci -q get network.lan.ipaddr | cut -d/ -f1)
PEERS_BEFORE=$(iw dev 2>/dev/null | awk '/Interface/{i=$2} /type mesh point/{print i}' | while read m; do iw dev $m station dump | grep -c "ESTAB"; done | awk '{s+=$1} END{print s+0}')
log "pre-apply: lan=$LAN_ADDR mesh_estab=$PEERS_BEFORE"
[ "$PEERS_BEFORE" -gt 0 ] || finish "FAILED: no mesh peer before apply; nothing to gate against"

cp /etc/config/network $D/backup/network
# Watchdog: this node has no timeout(1). If the main path hangs, restore anyway.
( sleep 170; [ -f $RESULT ] || { cp $D/backup/network /etc/config/network; /etc/init.d/network reload; echo "ROLLED_BACK: watchdog fired at 170s (previous network config restored)" > $RESULT; /etc/init.d/mjolnir-meshd restart; } ) &
log "applying: $PORT -> wan (dhcp), br-lan wifi-only"
uci -q delete network.@device[0].ports
uci set network.@device[0].bridge_empty='1'
uci set network.wan=interface
uci set network.wan.device="$PORT"
uci set network.wan.proto='dhcp'
uci set network.wan6=interface
uci set network.wan6.device="$PORT"
uci set network.wan6.proto='dhcpv6'
uci commit network
/etc/init.d/network reload
sleep 5

healthy() {
	[ ! -d /sys/class/net/br-lan/brif/$PORT ] || return 1
	ip -4 -o addr show dev br-lan | grep -q "$LAN_ADDR/" || return 1
	ls /sys/class/net/br-lan/brif/ 2>/dev/null | grep -q ap || return 1
	ip -4 -o addr show dev $PORT scope global | grep -q inet || return 1
	ip -4 route show default dev $PORT | grep -q '^default' || return 1
	n=$(iw dev 2>/dev/null | awk '/Interface/{i=$2} /type mesh point/{print i}' | while read m; do iw dev $m station dump | grep -c "ESTAB"; done | awk '{s+=$1} END{print s+0}')
	[ "$n" -gt 0 ] || return 1
	ping -c1 -W2 -I $PORT 1.1.1.1 >/dev/null 2>&1 || return 1
	return 0
}
i=0
while [ $i -lt 30 ]; do
	if healthy; then
		fw4 reload >/dev/null 2>&1
		log "health gate passed after $((i*3+5))s: wan=$(ip -4 -o addr show dev $PORT scope global | awk '{print $4}')"
		/etc/init.d/mjolnir-meshd restart
		finish "OK: $PORT is wan"
	fi
	i=$((i+1)); sleep 3
done
log "health gate FAILED; state:"; ip -4 -br addr 2>/dev/null || ip -4 addr; ls /sys/class/net/br-lan/brif/; ip -4 route show default
cp $D/backup/network /etc/config/network
/etc/init.d/network reload
sleep 8
/etc/init.d/mjolnir-meshd restart
finish "ROLLED_BACK: health gate failed after 95s (previous network config restored)"
