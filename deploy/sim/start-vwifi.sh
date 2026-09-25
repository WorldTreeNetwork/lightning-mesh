#!/usr/bin/env bash
# Host vwifi-server + guest vwifi-client (TCP on mgmt 10.99.0.1:8212).
set -euo pipefail
SERVER_IP="${VWIFI_SERVER:-10.99.0.1}"
A="${SIM_A:-10.99.0.10}"
B="${SIM_B:-10.99.0.11}"

if ! pgrep -x vwifi-server >/dev/null; then
	nohup vwifi-server -l >/tmp/vwifi-server.log 2>&1 &
	sleep 1
fi

guest() {
	local host="$1" mac="$2"
	ssh -o BatchMode=yes "root@${host}" "sh -s" <<EOF
set -e
wifi down 2>/dev/null || true
killall vwifi-client 2>/dev/null || true
rmmod mac80211_hwsim 2>/dev/null || true
insmod mac80211_hwsim radios=0
echo 'mac80211_hwsim radios=0' > /etc/modules.d/mac80211-hwsim
vwifi-add-interfaces 2 ${mac}
vwifi-client ${SERVER_IP} -n 2 >/tmp/vwifi-client.log 2>&1 &
sleep 1
iw phy phy2 interface add mesh0 type mp mesh_id mjolnir-mesh 2>/dev/null || true
iw dev mesh0 set channel 1 HT20 2>/dev/null || true
ip link set mesh0 up 2>/dev/null || true
iw phy phy3 interface add wlan-ap type __ap 2>/dev/null || true
iw dev wlan-ap set channel 6 HT20 2>/dev/null || true
ip link set wlan-ap up 2>/dev/null || true
EOF
}

guest "${A}" 02:00:0a:00:00
guest "${B}" 02:00:0b:00:00
echo ">> waiting for 802.11s ESTAB"
for i in $(seq 1 15); do
	if ssh -o BatchMode=yes "root@${A}" 'iw dev mesh0 station dump 2>/dev/null' | grep -q 'plink:ESTAB'; then
		ssh -o BatchMode=yes "root@${A}" 'iw dev mesh0 station dump | grep -E "Station|plink|signal"'
		vwifi-ctrl ls
		exit 0
	fi
	sleep 1
done
echo ">> no ESTAB yet" >&2
ssh -o BatchMode=yes "root@${A}" 'iw dev; cat /tmp/vwifi-client.log' || true
exit 1
