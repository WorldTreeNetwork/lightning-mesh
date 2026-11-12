// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

//! IPv6 discovery: Unique Local (`fc00::/7`) and link-local (`fe80::/10`)
//! addresses on every interface, kernel neighbor-table dump, and an ICMPv6
//! all-nodes (`ff02::1`) probe. Addresses are also encoded as base58 of the
//! 16 raw octets for the admin list.

mod hello;
mod interfaces;
mod mdns;
mod merge;
mod reach;
mod types;

#[cfg(target_os = "linux")]
mod icmp;
#[cfg(not(target_os = "linux"))]
mod icmp {
    use super::types::{LinkLocalInterface, RawNeighbor};
    pub fn probe_all_nodes(_: &[LinkLocalInterface]) -> (Vec<RawNeighbor>, Option<String>) {
        (
            Vec::new(),
            Some("ICMPv6 all-nodes probe is Linux-only in v1".into()),
        )
    }
}

#[cfg(target_os = "linux")]
mod neigh;
#[cfg(not(target_os = "linux"))]
mod neigh {
    use super::types::{LinkLocalInterface, RawNeighbor};
    pub async fn dump_link_local(_: &[LinkLocalInterface]) -> Result<Vec<RawNeighbor>, String> {
        Ok(Vec::new())
    }
}

pub use hello::http_get_json;
pub use merge::merge_scan;
pub use types::{LinkLocalInterface, LinkLocalNeighbor, RawNeighbor, ScanResult};

pub async fn scan_link_local() -> Result<ScanResult, String> {
    let interfaces = interfaces::collect_interfaces()?;
    let iface_index = interfaces.clone();
    let from_neigh = neigh::dump_link_local(&interfaces)
        .await
        .unwrap_or_default();
    let (from_probe, probe_error) = icmp::probe_all_nodes(&interfaces);
    let names = mdns::browse().await;
    let mut scan = merge_scan(interfaces, from_neigh, from_probe, probe_error, true);
    mdns::apply_names(&mut scan.neighbors, &names);
    let candidates = hello::probe_candidates(&scan.neighbors);
    let mut nodes = probe_all(&candidates);
    let reach = reach::reach_targets(&iface_index, &candidates);
    let reach_hits = probe_all(&reach.iter().map(|item| item.target.clone()).collect::<Vec<_>>());
    for hit in &reach_hits {
        if let Some(item) = reach.iter().find(|item| item.target.address == hit.address) {
            ensure_hello_row(&mut scan.neighbors, &hit.address, &item.iface, item.ifindex);
        }
    }
    nodes.extend(reach_hits);
    let (seeded, heard) = directory_harvest(&nodes, &scan.neighbors);
    let seed_hits = probe_all(&seeded.iter().map(|item| item.target.clone()).collect::<Vec<_>>());
    for hit in &seed_hits {
        if let Some(item) = seeded.iter().find(|item| item.target.address == hit.address) {
            ensure_hello_row(&mut scan.neighbors, &hit.address, &item.iface, item.ifindex);
        }
    }
    nodes.extend(seed_hits);
    hello::apply_nodes(&mut scan.neighbors, &nodes);
    // A directory neighbor that did not answer hello is still a published router.
    insert_unanswered(&mut scan.neighbors, &heard);
    Ok(scan)
}

fn probe_all(targets: &[hello::ProbeTarget]) -> Vec<hello::HelloNode> {
    let mut found = Vec::new();
    for chunk in targets.chunks(64) {
        found.extend(hello::probe_nodes(chunk));
    }
    found
}

fn ensure_hello_row(neighbors: &mut Vec<types::LinkLocalNeighbor>, address: &str, iface: &str, ifindex: u32) {
    if neighbors.iter().any(|row| row.address.split('%').next().unwrap_or(&row.address) == address) {
        return;
    }
    let Ok(ip) = address.parse::<std::net::IpAddr>() else {
        return;
    };
    neighbors.push(types::LinkLocalNeighbor::from_ip(
        iface,
        ifindex,
        ip,
        None,
        "reachable",
        "neighbor",
        "hello",
    ));
}

struct HeardDirectory {
    record: reach::DirectoryRecord,
    iface: String,
    ifindex: u32,
}

fn directory_harvest(
    nodes: &[hello::HelloNode],
    neighbors: &[types::LinkLocalNeighbor],
) -> (Vec<reach::ReachTarget>, Vec<HeardDirectory>) {
    let known: std::collections::BTreeSet<String> = neighbors
        .iter()
        .map(|row| row.address.split('%').next().unwrap_or(&row.address).to_string())
        .collect();
    let (fallback_iface, fallback_index) = nodes
        .first()
        .and_then(|node| {
            neighbors
                .iter()
                .find(|row| row.address == node.address)
                .map(|row| (row.iface.clone(), row.ifindex))
        })
        .unwrap_or_else(|| ("network".into(), 0));
    let mut targets = Vec::new();
    let mut heard = Vec::new();
    let mut pushed = std::collections::BTreeSet::new();
    for node in nodes {
        let Ok(value) = hello::http_get_json(&node.address, 0, "/api/directory") else {
            continue;
        };
        let (iface, ifindex) = neighbors
            .iter()
            .find(|row| row.address == node.address)
            .map(|row| (row.iface.clone(), row.ifindex))
            .unwrap_or_else(|| (fallback_iface.clone(), fallback_index));
        for record in reach::directory_records(&value) {
            heard.push(HeardDirectory {
                record,
                iface: iface.clone(),
                ifindex,
            });
        }
        for ip in reach::directory_addresses(&value) {
            if matches!(ip, std::net::IpAddr::V6(v6) if v6.is_unicast_link_local()) {
                continue;
            }
            if !hello::is_probeable(ip) {
                continue;
            }
            let address = ip.to_string();
            if known.contains(&address) || !pushed.insert(address.clone()) {
                continue;
            }
            targets.push(reach::ReachTarget {
                target: hello::ProbeTarget { address, scope_id: 0 },
                iface: iface.clone(),
                ifindex,
            });
        }
    }
    (targets, heard)
}

fn bare_addr(address: &str) -> &str {
    address.split('%').next().unwrap_or(address)
}

fn paint_directory(row: &mut types::LinkLocalNeighbor, record: &reach::DirectoryRecord) {
    row.lightning = true;
    row.kind = "directory".into();
    row.source = "directory".into();
    row.state = "announced".into();
    if !record.name.is_empty() {
        row.mesh_name = Some(record.name.clone());
    }
    if !record.node_id.is_empty() {
        row.mesh_node_id = Some(record.node_id.clone());
    }
    if !record.subnet.is_empty() {
        row.mesh_subnet = Some(record.subnet.clone());
    }
    if !record.backhaul.is_empty() {
        row.mesh_backhaul = Some(record.backhaul.clone());
    }
}

/// Keep a directory neighbor when no hello answer already carries that node id.
fn insert_unanswered(neighbors: &mut Vec<types::LinkLocalNeighbor>, heard: &[HeardDirectory]) {
    let mut seen_ids: std::collections::BTreeSet<String> = neighbors
        .iter()
        .filter_map(|row| row.mesh_node_id.as_ref())
        .filter(|id| !id.is_empty())
        .map(|id| id.to_ascii_lowercase())
        .collect();
    for item in heard {
        let id = item.record.node_id.to_ascii_lowercase();
        if !id.is_empty() && seen_ids.contains(&id) {
            continue;
        }
        if let Some(row) = neighbors
            .iter_mut()
            .find(|row| bare_addr(&row.address) == item.record.address)
        {
            if !row.lightning {
                paint_directory(row, &item.record);
            }
            if !id.is_empty() {
                seen_ids.insert(id);
            }
            continue;
        }
        let Ok(ip) = item.record.address.parse::<std::net::IpAddr>() else {
            continue;
        };
        let mut row = types::LinkLocalNeighbor::from_ip(
            item.iface.clone(),
            item.ifindex,
            ip,
            None,
            "announced",
            "directory",
            "directory",
        );
        paint_directory(&mut row, &item.record);
        neighbors.push(row);
        if !id.is_empty() {
            seen_ids.insert(id);
        }
    }
}

pub fn scoped_addr(ip: std::net::Ipv6Addr, iface: &str) -> String {
    format!("{ip}%{iface}")
}

pub fn format_mac(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 6 {
        return None;
    }
    Some(
        bytes[..6]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(":"),
    )
}

pub fn is_unicast_link_local(ip: std::net::Ipv6Addr) -> bool {
    ip.is_unicast_link_local()
}

pub fn is_unique_local(ip: std::net::Ipv6Addr) -> bool {
    ip.is_unique_local()
}

pub fn is_ula_or_link_local(ip: std::net::Ipv6Addr) -> bool {
    is_unicast_link_local(ip) || is_unique_local(ip)
}

pub fn ipv6_scope(ip: std::net::Ipv6Addr) -> &'static str {
    if is_unique_local(ip) {
        "unique-local"
    } else if is_unicast_link_local(ip) {
        "link-local"
    } else {
        "other"
    }
}

/// Bitcoin base58 alphabet. One IPv6 hex couplet is one byte; this encodes
/// the 16-octet address with leading-zero bytes preserved as `'1'`.
const B58: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

pub fn ipv6_to_base58(ip: std::net::Ipv6Addr) -> String {
    encode_base58(&ip.octets())
}

pub fn encode_base58(bytes: &[u8]) -> String {
    let zeros = bytes.iter().take_while(|b| **b == 0).count();
    let mut acc = bytes.to_vec();
    let mut digits = Vec::new();
    while acc.iter().any(|&b| b != 0) {
        let mut rem = 0u32;
        for b in acc.iter_mut() {
            let cur = (rem << 8) | u32::from(*b);
            *b = (cur / 58) as u8;
            rem = cur % 58;
        }
        digits.push(B58[rem as usize]);
    }
    let mut out = vec![b'1'; zeros];
    digits.reverse();
    out.extend_from_slice(&digits);
    String::from_utf8(out).expect("base58 alphabet is ASCII")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv6Addr;

    #[test]
    fn scoped_addr_uses_percent_zone() {
        let ip: Ipv6Addr = "fe80::1".parse().unwrap();
        assert_eq!(scoped_addr(ip, "eth0"), "fe80::1%eth0");
    }

    #[test]
    fn format_mac_colon_hex() {
        assert_eq!(
            format_mac(&[0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66]),
            Some("00:11:22:33:44:55".into())
        );
        assert_eq!(format_mac(&[1, 2, 3]), None);
    }

    #[test]
    fn fe80_is_link_local_gua_is_not() {
        assert!(is_unicast_link_local("fe80::abcd".parse().unwrap()));
        assert!(!is_unicast_link_local("2001:db8::1".parse().unwrap()));
        assert!(!is_unicast_link_local("::1".parse().unwrap()));
    }

    #[tokio::test]
    async fn live_scan_returns_interfaces() {
        let scan = super::scan_link_local().await.expect("scan");
        assert!(
            !scan.interfaces.is_empty(),
            "expected at least one local interface"
        );
        let has_local_ll = scan
            .interfaces
            .iter()
            .any(|i| !i.is_loopback && !i.link_local.is_empty());
        assert!(
            has_local_ll,
            "expected at least one non-loopback fe80:: on this host; ifaces={:?}",
            scan.interfaces
                .iter()
                .map(|i| (&i.name, &i.link_local))
                .collect::<Vec<_>>()
        );
        for n in &scan.neighbors {
            if n.scope == "ipv4" {
                assert!(n.address.parse::<std::net::Ipv4Addr>().is_ok(), "{}", n.address);
                continue;
            }
            let ip: Ipv6Addr = n.address.parse().expect("neighbor address");
            assert!(is_ula_or_link_local(ip), "non-ULA/LL leaked: {}", n.address);
            if is_unicast_link_local(ip) {
                assert!(n.scoped.contains('%'), "missing zone id: {}", n.scoped);
            }
            assert_eq!(n.base58, ipv6_to_base58(ip));
            assert_eq!(n.scope, ipv6_scope(ip));
        }
    }

    #[test]
    fn unanswered_directory_neighbor_stays_in_the_scan() {
        let mut neighbors = vec![LinkLocalNeighbor::from_ip(
            "wlp191s0",
            3,
            "10.42.203.1".parse().unwrap(),
            None,
            "reachable",
            "neighbor",
            "hello",
        )];
        neighbors[0].lightning = true;
        neighbors[0].mesh_name = Some("ap3000-indoor-c".into());
        neighbors[0].mesh_node_id = Some("indoor-c".into());
        let silent = reach::DirectoryRecord {
            name: "m3000".into(),
            node_id: "m3000".into(),
            subnet: "10.42.243.0/24".into(),
            backhaul: "10.254.242.172".into(),
            address: "10.42.243.1".into(),
        };
        let heard = vec![
            HeardDirectory {
                record: reach::DirectoryRecord {
                    name: "ap3000-indoor-c".into(),
                    node_id: "indoor-c".into(),
                    subnet: "10.42.203.0/24".into(),
                    backhaul: "10.254.203.27".into(),
                    address: "10.42.203.1".into(),
                },
                iface: "wlp191s0".into(),
                ifindex: 3,
            },
            HeardDirectory {
                record: silent.clone(),
                iface: "wlp191s0".into(),
                ifindex: 3,
            },
            HeardDirectory {
                record: silent,
                iface: "wlp191s0".into(),
                ifindex: 3,
            },
        ];
        insert_unanswered(&mut neighbors, &heard);
        assert_eq!(neighbors.len(), 2);
        assert_eq!(neighbors[0].state, "reachable");
        assert_eq!(neighbors[0].kind, "neighbor");
        assert_eq!(neighbors[1].mesh_name.as_deref(), Some("m3000"));
        assert_eq!(neighbors[1].state, "announced");
        assert_eq!(neighbors[1].kind, "directory");
        assert!(neighbors[1].lightning);
        assert_eq!(neighbors[1].mesh_backhaul.as_deref(), Some("10.254.242.172"));
    }

    #[test]
    fn unique_local_and_link_local_scopes() {
        assert!(is_unique_local("fd01:d28c:7e4a::1".parse().unwrap()));
        assert!(is_unique_local("fc00::".parse().unwrap()));
        assert!(!is_unique_local("fe80::1".parse().unwrap()));
        assert!(!is_unique_local("2001:db8::1".parse().unwrap()));
        assert_eq!(
            ipv6_scope("fd01:d28c:7e4a::1".parse().unwrap()),
            "unique-local"
        );
        assert_eq!(ipv6_scope("fe80::1".parse().unwrap()), "link-local");
    }

    #[test]
    fn ipv6_octets_encode_as_bitcoin_base58() {
        let cases: &[(&str, &str)] = &[
            ("fd01:d28c:7e4a::1", "YF4RhMGBc3LA1xkSVuXzpg"),
            ("fe80::1", "YRka4zYGRkixTpb4LjCkzL"),
            ("fe80::6dd0:82fe:420c:6779", "YRka4zYGRkjGqACkk8onBa"),
            ("fc00::", "Y7r4v4m4eqstVx6aWDjQjH"),
            ("::1", "1111111111111112"),
        ];
        for (ip, encoded) in cases {
            let addr: Ipv6Addr = ip.parse().unwrap();
            assert_eq!(ipv6_to_base58(addr), *encoded, "ip={ip}");
        }
    }
}
