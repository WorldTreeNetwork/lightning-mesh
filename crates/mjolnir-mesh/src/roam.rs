// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors
// Lightning Mesh is dual-licensed (AGPL-3.0-or-later or commercial); see LICENSE
// and COMMERCIAL-LICENSE.md at the repository root.

//! Guest-client roaming (bead mjolnir-mesh-sz9).
//!
//! When a phone keeps the IP another node vended and associates here, this
//! node holds a host `/32` on the client bridge so inbound mesh traffic
//! follows the client. Parsers are pure over `ip neigh` / `iw` text; the
//! daemon installs the routes.

use std::collections::BTreeSet;
use std::net::Ipv4Addr;

use ipnet::Ipv4Net;

/// Linux `rtm_protocol` stamped on mobility `/32`s so `ip route flush proto`
/// cannot touch anything else on the box. Numeric: babeld 1.13 rejects named
/// proto filters.
pub const MOBILITY_ROUTE_PROTO: u8 = 158;

/// One IPv4 neighbour on the client bridge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Neighbour {
    pub ip: Ipv4Addr,
    pub mac: String,
    pub nud: String,
}

/// Inputs to [`guest_routes`].
#[derive(Debug, Clone)]
pub struct RoamInputs<'a> {
    /// This node's claimed client subnet, if any. Addresses *inside* it are
    /// already covered by the connected /24 — they are not guests.
    pub own_subnet: Option<Ipv4Net>,
    /// The mesh client space (`10.42.0.0/16`). Anything outside is not ours.
    pub mesh_space: Ipv4Net,
    /// `ip -4 neigh show dev <client>` on this node.
    pub neighbours: &'a [Neighbour],
    /// Lowercased MACs currently associated to this node's AP ifaces.
    pub associated: &'a std::collections::HashSet<String>,
}

/// IPv4 neighbours whose NUD is usable (not FAILED / INCOMPLETE / NONE).
fn usable_nud(nud: &str) -> bool {
    matches!(
        nud,
        "REACHABLE" | "STALE" | "DELAY" | "PROBE" | "PERMANENT" | "NOARP"
    )
}

fn normalize_mac(mac: &str) -> String {
    mac.trim().to_ascii_lowercase()
}

/// Parse `ip -4 neigh show dev <iface>` into [`Neighbour`]s.
///
/// Typical line: `10.42.5.23 lladdr aa:bb:cc:dd:ee:ff REACHABLE`
pub fn parse_ip_neigh(output: &str) -> Vec<Neighbour> {
    let mut out = Vec::new();
    for line in output.lines() {
        let mut toks = line.split_whitespace();
        let Some(ip) = toks.next().and_then(|t| t.parse::<Ipv4Addr>().ok()) else {
            continue;
        };
        let mut mac = String::new();
        let mut nud = String::new();
        while let Some(t) = toks.next() {
            if t == "lladdr" {
                if let Some(m) = toks.next() {
                    mac = normalize_mac(m);
                }
            } else if t.chars().all(|c| c.is_ascii_uppercase()) {
                nud = t.to_string();
            }
        }
        if mac.is_empty() || nud.is_empty() {
            continue;
        }
        out.push(Neighbour { ip, mac, nud });
    }
    out
}

/// Parse `iw dev` for AP interface names (`type AP`, not mesh-point).
pub fn parse_ap_ifaces(iw_dev_output: &str) -> Vec<String> {
    let mut current: Option<String> = None;
    let mut aps = Vec::new();
    for line in iw_dev_output.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Interface ") {
            current = Some(rest.trim().to_string());
        } else if trimmed == "type AP"
            && let Some(name) = current.take()
        {
            aps.push(name);
        }
    }
    aps
}

/// Parse `iw dev <ap> station dump` into associated MACs (lowercased).
pub fn parse_associated_macs(dump: &str) -> Vec<String> {
    let mut macs = Vec::new();
    for line in dump.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Station ") {
            if let Some(mac) = rest.split_whitespace().next() {
                macs.push(normalize_mac(mac));
            }
        }
    }
    macs
}

/// Guest IPs that should have a mobility `/32` on this node: in mesh space,
/// not in our own subnet, ARP-reachable on the client bridge, and associated
/// to one of our APs.
pub fn guest_routes(inputs: &RoamInputs<'_>) -> BTreeSet<Ipv4Addr> {
    let mut desired = BTreeSet::new();
    for n in inputs.neighbours {
        if !usable_nud(&n.nud) {
            continue;
        }
        if !inputs.mesh_space.contains(&n.ip) {
            continue;
        }
        if inputs.own_subnet.is_some_and(|s| s.contains(&n.ip)) {
            continue;
        }
        if !inputs.associated.contains(&n.mac) {
            continue;
        }
        desired.insert(n.ip);
    }
    desired
}

/// `add` = in desired but not installed; `del` = installed but not desired.
pub fn route_delta(
    installed: &BTreeSet<Ipv4Addr>,
    desired: &BTreeSet<Ipv4Addr>,
) -> (Vec<Ipv4Addr>, Vec<Ipv4Addr>) {
    let add: Vec<_> = desired.difference(installed).copied().collect();
    let del: Vec<_> = installed.difference(desired).copied().collect();
    (add, del)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::str::FromStr;

    #[test]
    fn parse_neigh_skips_failed_and_ipv6() {
        let raw = "\
10.42.5.23 lladdr aa:bb:cc:dd:ee:ff REACHABLE
10.42.5.24 lladdr 11:22:33:44:55:66 STALE
fe80::1 lladdr aa:bb:cc:dd:ee:ff REACHABLE
10.42.5.25 FAILED
10.0.0.1 lladdr 5c:7d:7d:4b:51:44 REACHABLE
";
        let n = parse_ip_neigh(raw);
        assert_eq!(n.len(), 3);
        assert_eq!(n[0].ip, Ipv4Addr::new(10, 42, 5, 23));
        assert_eq!(n[0].mac, "aa:bb:cc:dd:ee:ff");
        assert_eq!(n[1].nud, "STALE");
        assert_eq!(n[2].ip, Ipv4Addr::new(10, 0, 0, 1));
    }

    #[test]
    fn parse_ap_ifaces_skips_mesh_point() {
        let raw = "\
phy#0
	Interface phy0-ap0
		type AP
	Interface phy0-mesh0
		type mesh point
phy#1
	Interface phy1-ap0
		type AP
";
        assert_eq!(parse_ap_ifaces(raw), vec!["phy0-ap0", "phy1-ap0"]);
    }

    #[test]
    fn parse_stations() {
        let raw = "\
Station AA:BB:CC:DD:EE:FF (on phy0-ap0)
	signal:	-60 dBm
Station 11:22:33:44:55:66 (on phy0-ap0)
";
        assert_eq!(
            parse_associated_macs(raw),
            vec!["aa:bb:cc:dd:ee:ff", "11:22:33:44:55:66"]
        );
    }

    #[test]
    fn guests_are_associated_foreign_mesh_addrs() {
        let own = Ipv4Net::from_str("10.42.1.0/24").unwrap();
        let mesh = Ipv4Net::from_str("10.42.0.0/16").unwrap();
        let neighbours = vec![
            Neighbour {
                ip: Ipv4Addr::new(10, 42, 1, 20),
                mac: "aa:aa:aa:aa:aa:aa".into(),
                nud: "REACHABLE".into(),
            },
            Neighbour {
                ip: Ipv4Addr::new(10, 42, 5, 23),
                mac: "bb:bb:bb:bb:bb:bb".into(),
                nud: "REACHABLE".into(),
            },
            Neighbour {
                ip: Ipv4Addr::new(10, 42, 5, 24),
                mac: "cc:cc:cc:cc:cc:cc".into(),
                nud: "REACHABLE".into(),
            },
            Neighbour {
                ip: Ipv4Addr::new(10, 0, 0, 50),
                mac: "bb:bb:bb:bb:bb:bb".into(),
                nud: "REACHABLE".into(),
            },
        ];
        let associated: HashSet<String> = ["aa:aa:aa:aa:aa:aa", "bb:bb:bb:bb:bb:bb"]
            .into_iter()
            .map(str::to_string)
            .collect();
        let desired = guest_routes(&RoamInputs {
            own_subnet: Some(own),
            mesh_space: mesh,
            neighbours: &neighbours,
            associated: &associated,
        });
        assert_eq!(
            desired.into_iter().collect::<Vec<_>>(),
            vec![Ipv4Addr::new(10, 42, 5, 23)]
        );
    }

    #[test]
    fn route_delta_add_and_del() {
        let installed: BTreeSet<_> = [Ipv4Addr::new(10, 42, 5, 1), Ipv4Addr::new(10, 42, 5, 2)]
            .into_iter()
            .collect();
        let desired: BTreeSet<_> = [Ipv4Addr::new(10, 42, 5, 2), Ipv4Addr::new(10, 42, 5, 3)]
            .into_iter()
            .collect();
        let (add, del) = route_delta(&installed, &desired);
        assert_eq!(add, vec![Ipv4Addr::new(10, 42, 5, 3)]);
        assert_eq!(del, vec![Ipv4Addr::new(10, 42, 5, 1)]);
    }
}
