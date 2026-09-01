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

use std::collections::{BTreeMap, BTreeSet, HashSet};
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

// --- island formation (read-only, mjolnir-mesh-77f / 190 / 3kd) -----------

/// Knobs for [`islands`].
#[derive(Debug, Clone, Copy)]
pub struct IslandConfig {
    /// Below this many nodes the mesh is *always* one island. A household fleet
    /// never splits: splitting answers a broadcast/coordination cost that
    /// simply is not there at single-digit node counts, and a wrong split costs
    /// more than no split.
    pub min_split_nodes: usize,
    /// Backhaul links at or above this RSSI (dBm) hold an island together.
    /// Anything weaker is a candidate cut.
    pub weak_link_dbm: i32,
}

impl Default for IslandConfig {
    fn default() -> Self {
        Self {
            min_split_nodes: 8,
            weak_link_dbm: -75,
        }
    }
}

/// Partition nodes into islands: connected components over the backhaul links
/// strong enough to hold, once the mesh is big enough to be worth splitting.
///
/// `links` are undirected `(a, b, signal_dbm)` backhaul edges. Nodes with no
/// surviving strong link fall out as singleton islands. Output is sorted both
/// within and across islands, so every node computing it from the same inputs
/// gets byte-identical results — the prerequisite for ever acting on this
/// without an election.
///
/// **Read-only today.** Since roaming is solved at L3 by the mobility `/32`s
/// above, an island is no longer "the set of nodes sharing a subnet" but *the
/// scope broadcast and ARP propagate across* — a much safer thing to get wrong
/// (a mis-drawn broadcast boundary degrades discovery; a mis-drawn subnet
/// boundary renumbers someone's phone). Acting on it still needs the per-link
/// signal view gossiped into the CRDT so members agree on the partition. See
/// `docs/network-coordination/island-formation.md` and bead `77f`.
pub fn islands(
    nodes: &[String],
    links: &[(String, String, i32)],
    cfg: IslandConfig,
) -> Vec<Vec<String>> {
    let mut sorted: Vec<String> = nodes.to_vec();
    sorted.sort();
    sorted.dedup();
    if sorted.is_empty() {
        return Vec::new();
    }
    if sorted.len() < cfg.min_split_nodes {
        return vec![sorted];
    }

    let mut adj: BTreeMap<&str, Vec<&str>> =
        sorted.iter().map(|n| (n.as_str(), Vec::new())).collect();
    for (a, b, dbm) in links {
        if *dbm < cfg.weak_link_dbm {
            continue;
        }
        if let Some(e) = adj.get_mut(a.as_str()) {
            e.push(b.as_str());
        }
        if let Some(e) = adj.get_mut(b.as_str()) {
            e.push(a.as_str());
        }
    }

    let mut seen: HashSet<&str> = HashSet::new();
    let mut out: Vec<Vec<String>> = Vec::new();
    for start in sorted.iter() {
        if !seen.insert(start.as_str()) {
            continue;
        }
        let mut component = vec![start.as_str()];
        let mut stack = vec![start.as_str()];
        while let Some(cur) = stack.pop() {
            for next in adj.get(cur).into_iter().flatten() {
                if seen.insert(next) {
                    component.push(next);
                    stack.push(next);
                }
            }
        }
        component.sort();
        out.push(component.into_iter().map(String::from).collect());
    }
    out.sort();
    out
}

#[cfg(test)]
mod island_tests {
    use super::*;

    fn ids(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("node-{i:02}")).collect()
    }

    #[test]
    fn small_fleet_is_always_one_island_even_with_a_weak_link() {
        // The household case: four nodes, one barely reachable. Still one
        // island — splitting buys nothing at this size.
        let nodes = ids(4);
        let links = vec![
            (nodes[0].clone(), nodes[1].clone(), -40),
            (nodes[1].clone(), nodes[2].clone(), -45),
            (nodes[2].clone(), nodes[3].clone(), -92),
        ];
        let got = islands(&nodes, &links, IslandConfig::default());
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].len(), 4);
    }

    #[test]
    fn large_fleet_splits_on_the_weak_link() {
        let nodes = ids(10);
        let mut links: Vec<(String, String, i32)> = Vec::new();
        for w in nodes[0..5].windows(2) {
            links.push((w[0].clone(), w[1].clone(), -50));
        }
        for w in nodes[5..10].windows(2) {
            links.push((w[0].clone(), w[1].clone(), -50));
        }
        links.push((nodes[4].clone(), nodes[5].clone(), -88)); // the cut
        let got = islands(&nodes, &links, IslandConfig::default());
        assert_eq!(got.len(), 2, "weak hop should cut: {got:?}");
        assert_eq!(got[0], nodes[0..5].to_vec());
        assert_eq!(got[1], nodes[5..10].to_vec());
    }

    #[test]
    fn large_fleet_stays_one_island_when_every_link_is_strong() {
        let nodes = ids(10);
        let links: Vec<(String, String, i32)> = nodes
            .windows(2)
            .map(|w| (w[0].clone(), w[1].clone(), -55))
            .collect();
        let got = islands(&nodes, &links, IslandConfig::default());
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].len(), 10);
    }

    #[test]
    fn isolated_node_in_a_large_fleet_is_its_own_island() {
        let nodes = ids(10);
        let links: Vec<(String, String, i32)> = nodes[0..9]
            .windows(2)
            .map(|w| (w[0].clone(), w[1].clone(), -55))
            .collect();
        let got = islands(&nodes, &links, IslandConfig::default());
        assert_eq!(got.len(), 2);
        assert_eq!(got[1], vec![nodes[9].clone()]);
    }

    #[test]
    fn island_output_is_deterministic_regardless_of_input_order() {
        // Every node must compute byte-identical islands from the same facts —
        // the precondition for ever acting on this without an election.
        let nodes = ids(10);
        let links: Vec<(String, String, i32)> = nodes
            .windows(2)
            .map(|w| (w[0].clone(), w[1].clone(), -55))
            .collect();
        let a = islands(&nodes, &links, IslandConfig::default());
        let mut shuffled = nodes.clone();
        shuffled.reverse();
        let mut links_rev = links.clone();
        links_rev.reverse();
        assert_eq!(a, islands(&shuffled, &links_rev, IslandConfig::default()));
    }

    #[test]
    fn empty_fleet_has_no_islands() {
        assert!(islands(&[], &[], IslandConfig::default()).is_empty());
    }
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
