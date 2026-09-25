#!/bin/sh
# Sim-lab wireless profile. MUST refuse unmarked metal before any UCI write.
# Then pin hwsim radios to 2g+5g and exec the stock setup-wireless.sh.
set -e
[ -f /etc/mjolnir/sim-guest ] || {
	echo "FATAL: sim wireless profile requires /etc/mjolnir/sim-guest (unmarked node, including non-Cudy metal)"
	exit 1
}

# Drop iw-created leftovers (start-vwifi.sh mesh0/wlan-ap) so netifd can own the phys.
wifi down 2>/dev/null || true
iw dev mesh0 del 2>/dev/null || true
iw dev wlan-ap del 2>/dev/null || true

# Pin first two mac80211 devices to 2g / 5g so stock setup-wireless.sh finds both.
# wifi config may have labeled them 6g.
n=0
for r in $(uci show wireless | sed -n 's/^wireless\.\([^.]*\)=wifi-device/\1/p'); do
	case $n in
		0)
			uci set "wireless.$r.band=2g"
			uci set "wireless.$r.channel=6"
			uci set "wireless.$r.htmode=HT20"
			uci set "wireless.$r.disabled=0"
			uci set "wireless.$r.country=US"
			;;
		1)
			uci set "wireless.$r.band=5g"
			uci set "wireless.$r.channel=36"
			uci set "wireless.$r.htmode=HT20"
			uci set "wireless.$r.disabled=0"
			uci set "wireless.$r.country=US"
			;;
	esac
	n=$((n + 1))
done
[ "$n" -ge 2 ] || { echo "FATAL: need two hwsim wifi-device sections (got $n)"; exit 1; }
uci commit wireless

STOCK="${SETUP_WIRELESS_STOCK:-/root/mjolnir-stage/setup-wireless.stock.sh}"
[ -x "$STOCK" ] || STOCK="$(dirname "$0")/../openwrt/setup-wireless.sh"
[ -x "$STOCK" ] || { echo "FATAL: stock setup-wireless.sh not staged at $STOCK"; exit 1; }
exec "$STOCK" "$@"
