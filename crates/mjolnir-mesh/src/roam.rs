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

/// Linux `rtm_protocol` stamped on mobility `/32`s. Numeric: babeld 1.13
/// rejects named proto filters. **Do not** `ip route flush proto 158` on
/// OpenWrt: BusyBox `ip` silently ignores `proto` and deletes every route
/// out the client bridge, including the connected `/24` (clients DHCP then
/// blackhole). Delete host routes by destination instead.
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

/// Host `/32` destinations from `ip -4 route show dev <client>`.
///
/// BusyBox (OpenWrt) prints a host route as a bare IPv4 (`10.42.5.23`);
/// iproute2 prints `10.42.5.23/32`. Connected `/24`s and `default` are
/// skipped — those must survive a mobility flush.
pub fn parse_host_route_dests(output: &str) -> Vec<Ipv4Addr> {
    let mut out = Vec::new();
    for line in output.lines() {
        let Some(dest) = line.split_whitespace().next() else {
            continue;
        };
        if dest == "default" {
            continue;
        }
        if let Some((ip, plen)) = dest.split_once('/') {
            if plen == "32"
                && let Ok(addr) = ip.parse()
            {
                out.push(addr);
            }
            continue;
        }
        if let Ok(addr) = dest.parse::<Ipv4Addr>() {
            out.push(addr);
        }
    }
    out
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

/// Island-member addresses sit in `.2`..=`.99`. DHCP for phones starts at
/// `.100` (OpenWrt `dhcp.lan.start`). The claim owner is always `.1`.
pub const ISLAND_HOST_MIN: u32 = 2;
pub const ISLAND_HOST_MAX: u32 = 99;

/// One client-subnet claim, stripped to what [`join_island`] needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IslandClaim<'a> {
    pub owner_id: &'a str,
    pub subnet: Ipv4Net,
    /// Smaller is older. Caller maps HLC to a monotonic integer.
    pub claimed_at: u64,
}

/// How this node should sit on a household client island.
///
/// Same function on every node — no `ROLE=core`. The claim owner (first
/// writer) vends DHCP and holds `.1`; everyone else is a unique host in the
/// same `/24` and still runs meshd. `stitch_l2` means put the 802.11s
/// mesh-point in the client bridge so roam is one broadcast domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IslandJoin {
    pub subnet: Ipv4Net,
    pub owner_id: String,
    pub local_addr: Ipv4Addr,
    pub run_dhcp: bool,
    pub stitch_l2: bool,
}

/// Pick the island `/24` (oldest client claim) and this node's address on it.
///
/// Empty input → `None` (caller claims a unique `/24` as today). One claim
/// we own, no peers → stay the unique-`/24` node, do not stitch. Two or more
/// owners, or a single foreign claim → join the winner and stitch L2.
pub fn join_island(self_id: &str, claims: &[IslandClaim<'_>]) -> Option<IslandJoin> {
    let winner = claims
        .iter()
        .min_by(|a, b| a.claimed_at.cmp(&b.claimed_at).then(a.owner_id.cmp(b.owner_id)))?;
    let owners: BTreeSet<&str> = claims.iter().map(|c| c.owner_id).collect();
    let stitch_l2 = owners.len() >= 2 || winner.owner_id != self_id;
    let run_dhcp = winner.owner_id == self_id;
    let local_addr = island_member_addr(winner.subnet, self_id, winner.owner_id);
    Some(IslandJoin {
        subnet: winner.subnet,
        owner_id: winner.owner_id.to_string(),
        local_addr,
        run_dhcp,
        stitch_l2,
    })
}

/// Address this node holds on `subnet`. Owner is `.1`; others hash into
/// [`ISLAND_HOST_MIN`]..=[`ISLAND_HOST_MAX`].
pub fn island_member_addr(subnet: Ipv4Net, node_id: &str, owner_id: &str) -> Ipv4Addr {
    let base = u32::from(subnet.network());
    let gateway = Ipv4Addr::from(base + 1);
    if node_id == owner_id {
        return gateway;
    }
    let hash = blake3::hash(node_id.as_bytes());
    let bytes = hash.as_bytes();
    let span = ISLAND_HOST_MAX - ISLAND_HOST_MIN + 1;
    let mut off = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) % span;
    for _ in 0..span {
        let addr = Ipv4Addr::from(base + ISLAND_HOST_MIN + off);
        if addr != gateway {
            return addr;
        }
        off = (off + 1) % span;
    }
    Ipv4Addr::from(base + ISLAND_HOST_MIN)
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

    fn net(s: &str) -> Ipv4Net {
        s.parse().unwrap()
    }

    #[test]
    fn lone_owner_keeps_unique_slash24_and_does_not_stitch() {
        let claims = [IslandClaim {
            owner_id: "wr3000s-a",
            subnet: net("10.42.242.0/24"),
            claimed_at: 1,
        }];
        let got = join_island("wr3000s-a", &claims).unwrap();
        assert_eq!(got.local_addr, "10.42.242.1".parse::<Ipv4Addr>().unwrap());
        assert!(got.run_dhcp);
        assert!(!got.stitch_l2, "a single node is not an island yet");
    }

    #[test]
    fn two_owners_join_the_oldest_claim_and_stitch() {
        let claims = [
            IslandClaim {
                owner_id: "indoor-c",
                subnet: net("10.42.203.0/24"),
                claimed_at: 200,
            },
            IslandClaim {
                owner_id: "wr3000s-a",
                subnet: net("10.42.242.0/24"),
                claimed_at: 100,
            },
        ];
        let core = join_island("wr3000s-a", &claims).unwrap();
        assert_eq!(core.subnet, net("10.42.242.0/24"));
        assert_eq!(core.local_addr, "10.42.242.1".parse::<Ipv4Addr>().unwrap());
        assert!(core.run_dhcp);
        assert!(core.stitch_l2);

        let leaf = join_island("indoor-c", &claims).unwrap();
        assert_eq!(leaf.subnet, net("10.42.242.0/24"));
        assert_eq!(leaf.owner_id, "wr3000s-a");
        assert!(!leaf.run_dhcp, "only the claim owner vends DHCP");
        assert!(leaf.stitch_l2);
        assert_ne!(leaf.local_addr, core.local_addr);
        let host = u32::from(leaf.local_addr) & 0xff;
        assert!(
            (ISLAND_HOST_MIN..=ISLAND_HOST_MAX).contains(&host),
            "member host {host} outside .2-.99"
        );
    }

    #[test]
    fn newcomer_with_no_own_claim_joins_foreign_and_stitches() {
        let claims = [IslandClaim {
            owner_id: "wr3000s-a",
            subnet: net("10.42.242.0/24"),
            claimed_at: 1,
        }];
        let got = join_island("indoor-b", &claims).unwrap();
        assert!(got.stitch_l2);
        assert!(!got.run_dhcp);
        assert_eq!(got.local_addr, island_member_addr(got.subnet, "indoor-b", "wr3000s-a"));
    }

    #[test]
    fn island_member_addr_is_stable_and_not_gateway() {
        let subnet = net("10.42.242.0/24");
        let a = island_member_addr(subnet, "indoor-c", "wr3000s-a");
        let b = island_member_addr(subnet, "indoor-c", "wr3000s-a");
        assert_eq!(a, b);
        assert_ne!(a, "10.42.242.1".parse::<Ipv4Addr>().unwrap());
        assert_eq!(
            island_member_addr(subnet, "wr3000s-a", "wr3000s-a"),
            "10.42.242.1".parse::<Ipv4Addr>().unwrap()
        );
    }

    #[test]
    fn empty_claims_mean_pick_a_unique_slash24() {
        assert!(join_island("me", &[]).is_none());
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

    #[test]
    fn parse_host_route_dests_keeps_connected_slash24() {
        // Live m3000 2026-09-11: BusyBox `ip route show dev br-lan` after a
        // proto-158 flush had already dropped the /24. This is the healthy
        // table — flush must delete only the /32s (bare IPv4 on BusyBox).
        let raw = "\
10.42.242.0/24 dev br-lan scope link  src 10.42.242.1
10.42.5.23 dev br-lan scope link
10.42.5.24/32 dev br-lan proto 158 scope link
192.168.1.0/24 dev br-lan scope link  src 192.168.1.1
default via 192.168.0.1 dev eth0
";
        let got = parse_host_route_dests(raw);
        assert_eq!(
            got,
            vec![Ipv4Addr::new(10, 42, 5, 23), Ipv4Addr::new(10, 42, 5, 24)]
        );
    }
}
