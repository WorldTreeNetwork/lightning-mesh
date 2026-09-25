#!/usr/bin/env bash
# Turn four sim guests into: ISP CPE, aftermarket NAT, mesh node-a (ISP LAN),
# mesh node-b (double-NAT). Overlay stays 802.11s, not WAN hole-punch.
set -euo pipefail
SSH=(ssh -o BatchMode=yes -o ConnectTimeout=8 -o StrictHostKeyChecking=accept-new)

remote() { "${SSH[@]}" "$1" "sh -s"; }

# ISP CPE: eth0 mgmt DHCP, eth1 WAN (libvirt default), eth2 LAN 192.168.1.1 DHCP+masq
remote root@10.99.0.12 <<'EOF'
set -e
uci -q delete network.wan || true
uci set network.wan=interface
uci set network.wan.proto=dhcp
uci set network.wan.device=eth1
uci -q delete network.isplan || true
uci set network.isplan=interface
uci set network.isplan.proto=static
uci set network.isplan.device=eth2
uci set network.isplan.ipaddr=192.168.1.1
uci set network.isplan.netmask=255.255.255.0
uci -q delete dhcp.isplan || true
uci set dhcp.isplan=dhcp
uci set dhcp.isplan.interface=isplan
uci set dhcp.isplan.start=100
uci set dhcp.isplan.limit=100
uci set dhcp.isplan.leasetime=12h
uci -q delete firewall.isplan || true
uci add firewall zone
uci set firewall.@zone[-1].name=isplan
uci set firewall.@zone[-1].network=isplan
uci set firewall.@zone[-1].input=ACCEPT
uci set firewall.@zone[-1].output=ACCEPT
uci set firewall.@zone[-1].forward=ACCEPT
uci add firewall forwarding
uci set firewall.@forwarding[-1].src=isplan
uci set firewall.@forwarding[-1].dest=wan
uci set firewall.@zone[1].masq=1
uci set firewall.@zone[1].mtu_fix=1
uci commit
/etc/init.d/network restart
/etc/init.d/dnsmasq restart
/etc/init.d/firewall restart
EOF

# Aftermarket NAT: eth0 mgmt, eth1 WAN on ISP LAN, eth2 LAN 192.168.50.1
remote root@10.99.0.13 <<'EOF'
set -e
uci -q delete network.wan || true
uci set network.wan=interface
uci set network.wan.proto=dhcp
uci set network.wan.device=eth1
uci -q delete network.natlan || true
uci set network.natlan=interface
uci set network.natlan.proto=static
uci set network.natlan.device=eth2
uci set network.natlan.ipaddr=192.168.50.1
uci set network.natlan.netmask=255.255.255.0
uci -q delete dhcp.natlan || true
uci set dhcp.natlan=dhcp
uci set dhcp.natlan.interface=natlan
uci set dhcp.natlan.start=100
uci set dhcp.natlan.limit=50
uci set dhcp.natlan.leasetime=12h
uci add firewall zone
uci set firewall.@zone[-1].name=natlan
uci set firewall.@zone[-1].network=natlan
uci set firewall.@zone[-1].input=ACCEPT
uci set firewall.@zone[-1].output=ACCEPT
uci set firewall.@zone[-1].forward=ACCEPT
uci add firewall forwarding
uci set firewall.@forwarding[-1].src=natlan
uci set firewall.@forwarding[-1].dest=wan
uci set firewall.@zone[1].masq=1
uci set firewall.@zone[1].mtu_fix=1
uci commit
/etc/init.d/network restart
/etc/init.d/dnsmasq restart
/etc/init.d/firewall restart
EOF

# node-a: WAN on ISP LAN (eth1), gateway=auto
remote root@10.99.0.10 <<'EOF'
set -e
uci -q delete network.wan || true
uci set network.wan=interface
uci set network.wan.proto=dhcp
uci set network.wan.device=eth1
uci set firewall.@zone[1].network='wan'
uci set mjolnir.meshd.gateway=auto
uci commit
/etc/init.d/network restart
/etc/init.d/mjolnir-meshd restart
EOF

# node-b: WAN on aftermarket LAN (eth2), do not export default
remote root@10.99.0.11 <<'EOF'
set -e
uci -q delete network.wan || true
uci set network.wan=interface
uci set network.wan.proto=dhcp
uci set network.wan.device=eth2
uci set firewall.@zone[1].network='wan'
uci set mjolnir.meshd.gateway=0
uci commit
/etc/init.d/network restart
/etc/init.d/mjolnir-meshd restart
EOF

echo ">> NAT roles applied. Check: node-a eth1=192.168.1.x, node-b eth2=192.168.50.x"
