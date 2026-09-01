#!/usr/bin/env bash
# Run one errand on the mesh client SSID, then restore known-good Wi-Fi.
#
# The laptop has one radio. Joining the test SSID drops Pirate Radio / Origami
# and the agent cannot report. EXIT/INT/TERM always restore HOME_CON.
#
# Usage:
#   deploy/openwrt/mesh-errand.sh
#   ERRAND='ssh -o BatchMode=yes -o ConnectTimeout=5 root@$GW logread | tail -40' \
#     deploy/openwrt/mesh-errand.sh
#   SSID='Lightning Mesh' HOME_CON='IdentiKey Pirate Radio' WAIT=30 \
#     deploy/openwrt/mesh-errand.sh
#
# Env exported into ERRAND: ADDR GW DNS IFACE SSID HOME_CON
# Default ERRAND (empty): DHCP + ping $GW (GATEWAY_OK / GATEWAY_BLACKHOLE).
#
# Do NOT `ip route replace default via 10.42.x.1` while the chat uplink is
# the working one.
set -u

SSID="${SSID:-Lightning Mesh}"
WAIT="${WAIT:-25}"
ERRAND="${ERRAND:-}"

active_wifi_con() {
	nmcli -t -f NAME,TYPE,DEVICE connection show --active \
		| awk -F: '$2=="802-11-wireless" && $3!="" {print $1; exit}'
}

HOME_CON="${HOME_CON:-$(active_wifi_con)}"
if [ -z "${HOME_CON}" ]; then
	echo "mesh-errand: no active Wi-Fi connection to restore to" >&2
	exit 2
fi

IFACE="$(nmcli -t -f DEVICE,TYPE device status | awk -F: '$2=="wifi"{print $1; exit}')"
RESTORED=0

restore() {
	[ "$RESTORED" = 1 ] && return 0
	RESTORED=1
	echo "==== restoring ${HOME_CON} ====" >&2
	nmcli -w "$WAIT" connection up "$HOME_CON" || \
		echo "mesh-errand: WARNING restore of ${HOME_CON} failed" >&2
	ip -4 -o addr show "$IFACE" 2>/dev/null || true
}

trap restore EXIT INT TERM

echo "home=${HOME_CON}  ssid=${SSID}  iface=${IFACE}" >&2
echo "==== joining ${SSID} (wait ${WAIT}s) ====" >&2
if ! nmcli -w "$WAIT" connection up "$SSID"; then
	echo "JOIN_FAILED ssid=${SSID}"
	exit 1
fi

sleep 2
ADDR="$(nmcli -g IP4.ADDRESS device show "$IFACE" 2>/dev/null | head -1)"
GW="$(nmcli -g IP4.GATEWAY device show "$IFACE" 2>/dev/null | head -1)"
DNS="$(nmcli -g IP4.DNS device show "$IFACE" 2>/dev/null | tr '\n' ' ')"
STATE="$(nmcli -g GENERAL.STATE device show "$IFACE" 2>/dev/null)"
[ "${ADDR:-}" = "--" ] && ADDR=""
[ "${GW:-}" = "--" ] && GW=""

echo "state=${STATE}"
echo "addr=${ADDR:-none}"
echo "gateway=${GW:-none}"
echo "dns=${DNS:-none}"

export ADDR GW DNS IFACE SSID HOME_CON

if [ -n "$ERRAND" ]; then
	echo "==== errand ====" >&2
	set +e
	bash -c "$ERRAND"
	ec=$?
	echo "ERRAND_EXIT=$ec"
	exit "$ec"
fi

if [ -z "${ADDR}" ]; then
	echo "DHCP_FAILED  (associated but no IPv4 lease — phone spinner is DHCP)"
	exit 1
fi
if [ -z "${GW}" ]; then
	echo "NO_GATEWAY  (lease without a router option)"
	exit 1
fi
if ping -c 2 -W 2 "$GW" >/dev/null 2>&1; then
	echo "GATEWAY_OK ${GW}"
else
	echo "GATEWAY_BLACKHOLE ${GW}  (DHCP ok, ping/SSH to .1 fail — see NOTES dual-LAN-IP)"
	exit 1
fi
