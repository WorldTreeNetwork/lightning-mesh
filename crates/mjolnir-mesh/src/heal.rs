// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors
// Lightning Mesh is dual-licensed (AGPL-3.0-or-later or commercial); see LICENSE
// and COMMERCIAL-LICENSE.md at the repository root.

//! Pure checks for meshd's connectivity self-heal (mjolnir-mesh-70t).
//!
//! Kernel shared state (client `/24` address + connected route) can drift
//! after a claim is already on the iface — BusyBox `ip route flush proto`
//! was one way (dpn). The daemon loop re-asserts; these parsers decide
//! whether a restore is needed without netlink, so they unit-test everywhere.
//!
//! Also the merged-client-L2 detector: several nodes' copper in one switch
//! puts foreign `10.42.y.1` on `br-lan` as an ARP neighbor. That is forbidden
//! (one routed `/24` per node). The dataplane loop then detaches copper.

use std::net::Ipv4Addr;

use ipnet::Ipv4Net;

/// OpenWrt factory LAN address. meshd used to keep this as a second alias on
/// every `br-lan` (bead 659, wired recovery). Two nodes on one switch then
/// both answer it; AP3000 field notes already forbade that. Strip it.
pub const STOCK_LAN_ALIAS: Ipv4Addr = Ipv4Addr::new(192, 168, 1, 1);

/// True when `ip -4 route show dev <client>` already has the connected
/// prefix (BusyBox and iproute2 both print `10.42.242.0/24` as the first
/// token). Host `/32`s and `default` do not count.
pub fn connected_prefix_present(route_show: &str, net: Ipv4Net) -> bool {
    let want = net.to_string();
    route_show.lines().any(|line| {
        line.split_whitespace()
            .next()
            .is_some_and(|dest| dest == want)
    })
}

/// True when `ip -4 addr show` / `ip -4 -o addr show` lists `want` as an
/// `inet` address on the iface (prefix length ignored).
pub fn iface_has_ipv4(addr_show: &str, want: Ipv4Addr) -> bool {
    let want = want.to_string();
    addr_show.lines().any(|line| {
        let mut toks = line.split_whitespace();
        while let Some(t) = toks.next() {
            if t != "inet" {
                continue;
            }
            let Some(addr) = toks.next() else {
                return false;
            };
            return addr.split('/').next() == Some(want.as_str());
        }
        false
    })
}

/// True when `ip -4 neigh show dev <client>` lists another node's client
/// gateway as a live ARP neighbor on this bridge.
///
/// A `10.42.y.1` other than *our* `.1` on `br-lan` means that node's `br-lan`
/// is on the same L2 (LAN ports on one switch). Isolated nodes never see a
/// foreign `.1` as a neighbor — babel has a route, not an ARP entry.
/// STALE/FAILED/INCOMPLETE are ignored so a leftover probe does not flap.
pub fn foreign_gateway_on_bridge(neigh_show: &str, our_gw: Ipv4Addr) -> bool {
    neigh_show
        .lines()
        .any(|line| live_foreign_gateway(line, our_gw).is_some())
}

fn live_foreign_gateway(line: &str, our_gw: Ipv4Addr) -> Option<Ipv4Addr> {
    let mut toks = line.split_whitespace();
    let ip: Ipv4Addr = toks.next()?.parse().ok()?;
    if !neigh_nud_live(line) {
        return None;
    }
    if ip == our_gw {
        return None;
    }
    if is_claimed_slash24_gateway(ip) {
        return Some(ip);
    }
    None
}

fn neigh_nud_live(line: &str) -> bool {
    line.split_whitespace()
        .any(|t| matches!(t, "REACHABLE" | "DELAY" | "PROBE" | "PERMANENT"))
}

/// Client gateways we vend are `10.42.x.1` (claimed `/24`).
fn is_claimed_slash24_gateway(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    o[0] == 10 && o[1] == 42 && o[3] == 1
}

/// Copper (ethernet/DSA) ports enslaved to `bridge` in `ip -o link show`
/// / `bridge link` output. Wifi (`phy*-ap*`, `wlan*`) stays; those are the
/// client AP. Names are the token before the first colon after the index.
pub fn copper_bridge_ports(link_show: &str, bridge: &str) -> Vec<String> {
    let master = format!("master {bridge}");
    let mut out = Vec::new();
    for line in link_show.lines() {
        if !line.contains(&master) {
            continue;
        }
        let Some(name) = link_ifname(line) else {
            continue;
        };
        if is_copper_port(&name) {
            out.push(name);
        }
    }
    out.sort();
    out.dedup();
    out
}

fn link_ifname(line: &str) -> Option<String> {
    // `5: eth1: <BROADCAST,...>` or `5: eth1@eth0: <...>`
    let rest = line.split_once(':')?.1.trim();
    let name = rest.split(':').next()?.trim();
    let name = name.split('@').next()?.trim();
    if name.is_empty() {
        return None;
    }
    Some(name.to_string())
}

pub fn is_copper_port(name: &str) -> bool {
    let n = name.trim();
    n == "wan"
        || n == "lan"
        || n.starts_with("eth")
        || n.starts_with("lan")
        || n.starts_with("wan")
        || n.starts_with("enp")
        || n.starts_with("ens")
        || n.starts_with("enx")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn missing_slash24_is_the_dpn_blackhole() {
        // Live m3000 after proto-158 flush: addresses still on br-lan,
        // connected /24 gone. Replies to clients follow WAN default.
        let routes = "\
default via 192.168.0.1 dev eth0  src 192.168.0.25
10.254.0.0/16 dev br-mesh scope link  src 10.254.242.172
192.168.0.0/24 dev eth0 scope link  src 192.168.0.25
";
        let net = Ipv4Net::from_str("10.42.242.0/24").unwrap();
        assert!(!connected_prefix_present(routes, net));
    }

    #[test]
    fn healthy_br_lan_has_the_connected_prefix() {
        let routes = "\
default via 192.168.0.1 dev eth0  src 192.168.0.25
10.42.242.0/24 dev br-lan scope link  src 10.42.242.1
10.42.5.23 dev br-lan scope link
10.254.0.0/16 dev br-mesh scope link  src 10.254.242.172
192.168.1.0/24 dev br-lan scope link  src 192.168.1.1
";
        let net = Ipv4Net::from_str("10.42.242.0/24").unwrap();
        assert!(connected_prefix_present(routes, net));
        assert!(!connected_prefix_present(
            routes,
            Ipv4Net::from_str("10.42.5.0/24").unwrap()
        ));
    }

    #[test]
    fn iface_has_ipv4_reads_busybox_addr_and_oneline() {
        let block = "\
4: br-lan: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500
    inet 10.42.242.1/24 brd 10.42.242.255 scope global br-lan
    inet 192.168.1.1/24 brd 192.168.1.255 scope global br-lan
";
        assert!(iface_has_ipv4(block, "10.42.242.1".parse().unwrap()));
        assert!(iface_has_ipv4(block, "192.168.1.1".parse().unwrap()));
        assert!(!iface_has_ipv4(block, "10.42.242.106".parse().unwrap()));

        let oneline = "4: br-lan    inet 10.42.242.1/24 brd 10.42.242.255 scope global br-lan";
        assert!(iface_has_ipv4(oneline, "10.42.242.1".parse().unwrap()));
    }

    #[test]
    fn foreign_10_42_gateway_on_bridge_is_merged_l2() {
        let our: Ipv4Addr = "10.42.242.1".parse().unwrap();
        // Live 2026-09-15: laptop on m3000 LAN saw indoor-c's .1 on the same L2.
        let neigh = "\
10.42.242.1 dev br-lan lladdr 80:af:ca:e7:bd:01 REACHABLE
10.42.203.1 dev br-lan lladdr d4:0d:ab:5b:f9:64 REACHABLE
10.42.242.156 dev br-lan lladdr 44:f7:9f:44:a7:33 REACHABLE
192.168.1.1 dev br-lan lladdr 80:af:ca:e7:bd:01 STALE
";
        assert!(foreign_gateway_on_bridge(neigh, our));
    }

    #[test]
    fn own_gateway_and_stale_foreign_are_not_a_merge() {
        let our: Ipv4Addr = "10.42.242.1".parse().unwrap();
        let neigh = "\
10.42.242.1 dev br-lan lladdr 80:af:ca:e7:bd:01 REACHABLE
10.42.203.1 dev br-lan lladdr d4:0d:ab:5b:f9:64 STALE
10.42.86.1 dev br-lan FAILED
10.42.242.156 dev br-lan lladdr 44:f7:9f:44:a7:33 DELAY
";
        assert!(!foreign_gateway_on_bridge(neigh, our));
    }

    #[test]
    fn copper_ports_are_eth_and_dsa_not_wifi() {
        let link = "\
2: eth0: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500 qdisc mq master br-wan state UP
3: eth1: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500 qdisc mq master br-lan state UP
4: lan1@eth0: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500 master br-lan state UP
5: phy0-ap0: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500 master br-lan state UP
6: wlan0: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500 master br-lan state UP
";
        assert_eq!(
            copper_bridge_ports(link, "br-lan"),
            vec!["eth1".to_string(), "lan1".to_string()]
        );
        assert!(is_copper_port("eth1"));
        assert!(is_copper_port("wan"));
        assert!(!is_copper_port("phy0-ap0"));
        assert!(!is_copper_port("wlan0"));
    }
}
