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

use std::net::Ipv4Addr;

use ipnet::Ipv4Net;

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

        let oneline =
            "4: br-lan    inet 10.42.242.1/24 brd 10.42.242.255 scope global br-lan";
        assert!(iface_has_ipv4(oneline, "10.42.242.1".parse().unwrap()));
    }
}
