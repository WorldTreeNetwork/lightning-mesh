#!/usr/bin/env bash
# Probe the Lightning Mesh *client* SSID from the agent laptop without
# stranding the chat.
#
# The mesh SSID often has no internet (dead WAN, dual 192.168.1.1 on br-lan,
# stale babel default). Associating the laptop's only Wi-Fi radio to it drops
# the working uplink (Pirate Radio / Origami / …) and the agent cannot report.
# This script joins, prints DHCP + gateway reachability, then ALWAYS switches
# back. EXIT/INT/TERM all restore.
#
# Usage:
#   deploy/openwrt/probe-client-ssid.sh
#   SSID='Lightning Mesh' HOME_CON='IdentiKey Pirate Radio' \
#     deploy/openwrt/probe-client-ssid.sh
#   WAIT=30 deploy/openwrt/probe-client-ssid.sh   # seconds for nmcli up
#
# Do NOT `ip route replace default via 10.42.x.1` from the laptop while the
# working uplink is the chat path. This script never changes the default
# except by letting NetworkManager switch connections, then switching back.
set -u

SSID="${SSID:-Lightning Mesh}"
WAIT="${WAIT:-25}"

active_wifi_con() {
	nmcli -t -f NAME,TYPE,DEVICE connection show --active \
		| awk -F: '$2=="802-11-wireless" && $3!="" {print $1; exit}'
}

HOME_CON="${HOME_CON:-$(active_wifi_con)}"
if [ -z "${HOME_CON}" ]; then
	echo "probe-client-ssid: no active Wi-Fi connection to restore to" >&2
	exit 2
fi

IFACE="$(nmcli -t -f DEVICE,TYPE device status | awk -F: '$2=="wifi"{print $1; exit}')"
RESTORED=0

restore() {
	[ "$RESTORED" = 1 ] && return 0
	RESTORED=1
	echo "==== restoring ${HOME_CON} ====" >&2
	nmcli -w "$WAIT" connection up "$HOME_CON" || \
		echo "probe-client-ssid: WARNING restore of ${HOME_CON} failed" >&2
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

echo "state=${STATE}"
echo "addr=${ADDR:-none}"
echo "gateway=${GW:-none}"
echo "dns=${DNS:-none}"

if [ -z "${ADDR}" ] || [ "${ADDR}" = "--" ]; then
	echo "DHCP_FAILED  (associated but no IPv4 lease — phone spinner is DHCP)"
	exit 1
fi

if [ -z "${GW}" ] || [ "${GW}" = "--" ]; then
	echo "NO_GATEWAY  (lease without a router option)"
	exit 1
fi

if ping -c 2 -W 2 "$GW" >/dev/null 2>&1; then
	echo "GATEWAY_OK ${GW}"
else
	# Dual 192.168.1.1/24 + 10.42.x.1 on br-lan: DHCP works, unicast to .1
	# blackholes. Phone spinner is then "no usable gateway", not "no DHCP".
	echo "GATEWAY_BLACKHOLE ${GW}  (DHCP ok, ping/SSH to .1 fail — see NOTES dual-LAN-IP)"
	exit 1
fi
