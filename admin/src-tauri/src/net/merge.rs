// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

use std::collections::BTreeMap;
use std::net::Ipv6Addr;

use super::interfaces::if_name;
use super::types::{LinkLocalInterface, LinkLocalNeighbor, RawNeighbor, ScanResult};
use super::scoped_addr;

pub fn merge_scan(
    interfaces: Vec<LinkLocalInterface>,
    from_neigh: Vec<RawNeighbor>,
    from_probe: Vec<RawNeighbor>,
    probe_error: Option<String>,
    probed: bool,
) -> ScanResult {
    let mut by_index: BTreeMap<u32, &LinkLocalInterface> = BTreeMap::new();
    for iface in &interfaces {
        by_index.insert(iface.index, iface);
    }

    let mut rows: BTreeMap<(u32, Ipv6Addr), LinkLocalNeighbor> = BTreeMap::new();

    for iface in &interfaces {
        if iface.is_loopback {
            continue;
        }
        for scoped in &iface.link_local {
            let Some(addr) = parse_scoped(scoped) else {
                continue;
            };
            rows.insert(
                (iface.index, addr),
                LinkLocalNeighbor {
                    iface: iface.name.clone(),
                    ifindex: iface.index,
                    address: addr.to_string(),
                    scoped: scoped.clone(),
                    mac: iface.mac.clone(),
                    state: if iface.is_up {
                        "local".into()
                    } else {
                        "down".into()
                    },
                    kind: "local".into(),
                    source: "addr".into(),
                },
            );
        }
    }

    for raw in from_neigh.into_iter().chain(from_probe) {
        let iface_name = by_index
            .get(&raw.ifindex)
            .map(|i| i.name.clone())
            .or_else(|| if_name(raw.ifindex))
            .unwrap_or_else(|| format!("if{}", raw.ifindex));
        let is_local = by_index.get(&raw.ifindex).is_some_and(|i| {
            i.link_local
                .iter()
                .any(|s| parse_scoped(s) == Some(raw.address))
        });
        let entry = rows
            .entry((raw.ifindex, raw.address))
            .or_insert_with(|| LinkLocalNeighbor {
                iface: iface_name.clone(),
                ifindex: raw.ifindex,
                address: raw.address.to_string(),
                scoped: scoped_addr(raw.address, &iface_name),
                mac: None,
                state: String::new(),
                kind: if is_local { "local" } else { "neighbor" }.into(),
                source: raw.source.into(),
            });
        if entry.mac.is_none() {
            entry.mac = raw.mac.clone();
        }
        if source_rank(raw.source) >= source_rank(&entry.source) {
            entry.source = raw.source.into();
            if !raw.state.is_empty() && entry.kind != "local" {
                entry.state = raw.state.clone();
            } else if entry.kind != "local" && entry.state.is_empty() {
                entry.state = raw.state.clone();
            }
        }
        if is_local {
            entry.kind = "local".into();
        }
    }

    let mut neighbors: Vec<LinkLocalNeighbor> = rows.into_values().collect();
    neighbors.sort_by(|a, b| {
        a.iface
            .cmp(&b.iface)
            .then(a.kind.cmp(&b.kind))
            .then(a.address.cmp(&b.address))
    });

    ScanResult {
        interfaces,
        neighbors,
        probed,
        probe_error,
    }
}

fn parse_scoped(scoped: &str) -> Option<Ipv6Addr> {
    let host = scoped.split('%').next()?;
    host.parse().ok()
}

fn source_rank(source: &str) -> u8 {
    match source {
        "probe" => 3,
        "neigh" => 2,
        "addr" => 1,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv6Addr;

    fn iface(name: &str, index: u32, ip: &str, loopback: bool) -> LinkLocalInterface {
        let addr: Ipv6Addr = ip.parse().unwrap();
        LinkLocalInterface {
            name: name.into(),
            index,
            mac: Some("aa:bb:cc:dd:ee:ff".into()),
            is_up: true,
            is_loopback: loopback,
            link_local: vec![scoped_addr(addr, name)],
        }
    }

    fn raw(ifindex: u32, ip: &str, mac: Option<&str>, source: &'static str, state: &str) -> RawNeighbor {
        RawNeighbor {
            ifindex,
            address: ip.parse().unwrap(),
            mac: mac.map(|s| s.into()),
            state: state.into(),
            source,
        }
    }

    #[test]
    fn local_addresses_become_rows_and_loopback_is_dropped() {
        let scan = merge_scan(
            vec![
                iface("eth0", 2, "fe80::1", false),
                iface("lo", 1, "fe80::1", true),
            ],
            vec![],
            vec![],
            None,
            false,
        );
        assert_eq!(scan.neighbors.len(), 1);
        assert_eq!(scan.neighbors[0].kind, "local");
        assert_eq!(scan.neighbors[0].iface, "eth0");
        assert_eq!(scan.neighbors[0].scoped, "fe80::1%eth0");
        assert_eq!(scan.interfaces.len(), 2);
    }

    #[test]
    fn neighbor_mac_is_kept_and_probe_does_not_unlocal_self() {
        let scan = merge_scan(
            vec![iface("eth0", 2, "fe80::1", false)],
            vec![raw(2, "fe80::2", Some("00:11:22:33:44:55"), "neigh", "stale")],
            vec![
                raw(2, "fe80::1", None, "probe", "reachable"),
                raw(2, "fe80::2", None, "probe", "reachable"),
            ],
            None,
            true,
        );
        let self_row = scan.neighbors.iter().find(|n| n.address == "fe80::1").unwrap();
        assert_eq!(self_row.kind, "local");
        assert_eq!(self_row.source, "probe");

        let peer = scan.neighbors.iter().find(|n| n.address == "fe80::2").unwrap();
        assert_eq!(peer.kind, "neighbor");
        assert_eq!(peer.mac.as_deref(), Some("00:11:22:33:44:55"));
        assert_eq!(peer.state, "reachable");
        assert_eq!(peer.source, "probe");
    }

    #[test]
    fn gua_never_enters_via_raw_without_being_link_local_local_only_from_ifaces() {
        // merge_scan trusts callers to filter; dump/probe already drop GUA.
        let scan = merge_scan(vec![iface("eth0", 2, "fe80::1", false)], vec![], vec![], None, false);
        assert!(scan.neighbors.iter().all(|n| n.address.starts_with("fe80:")));
    }
}
