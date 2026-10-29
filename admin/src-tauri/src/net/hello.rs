// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

//! Ask every unicast neighbor for `GET /api/node`.
//!
//! There is no address-family or subnet filter. IPv4 and IPv6, link-local
//! and global, are all candidates. Link-local IPv6 is connected with the
//! neighbor's interface index as the scope id. Multicast and unspecified
//! addresses are not TCP targets, so they are the only rows skipped.
//!
//! mjolnir-hello has to be listening on the address we probe. On a node
//! today that is the LAN gateway `10.42.<x>.1:80`.

use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, SocketAddrV6, TcpStream};
use std::time::Duration;

use serde::Deserialize;

use super::types::LinkLocalNeighbor;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProbeTarget {
    /// Address without a zone id. This is what [`HelloNode::address`] stores.
    pub address: String,
    /// IPv6 scope id. Nonzero for link-local; `0` otherwise.
    pub scope_id: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HelloNode {
    pub address: String,
    pub name: String,
    pub node_id: String,
    pub subnet: String,
    pub backhaul_addr: String,
    pub link_local_lan: String,
    pub ula: String,
}

#[derive(Deserialize)]
struct NodeBody {
    name: String,
    node_id: String,
    subnet: String,
    backhaul_addr: String,
    #[serde(default)]
    link_local_lan: String,
    #[serde(default)]
    ula: String,
}

/// Every unicast neighbor. Link-local IPv6 keeps `ifindex` as its scope id.
pub fn probe_candidates(neighbors: &[LinkLocalNeighbor]) -> Vec<ProbeTarget> {
    let mut out = Vec::new();
    for row in neighbors {
        let Ok(ip) = row.address.parse::<IpAddr>() else {
            continue;
        };
        if !is_probeable(ip) {
            continue;
        }
        let scope_id = match ip {
            IpAddr::V6(v6) if v6.is_unicast_link_local() => row.ifindex,
            _ => 0,
        };
        let target = ProbeTarget {
            address: row.address.clone(),
            scope_id,
        };
        if !out.contains(&target) {
            out.push(target);
        }
    }
    out
}

fn is_probeable(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            !v4.is_multicast() && !v4.is_broadcast() && !v4.is_unspecified() && !v4.is_loopback()
        }
        IpAddr::V6(v6) => !v6.is_multicast() && !v6.is_unspecified() && !v6.is_loopback(),
    }
}

pub fn probe_nodes(candidates: &[ProbeTarget]) -> Vec<HelloNode> {
    std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(candidates.len());
        for target in candidates {
            handles.push(scope.spawn(|| fetch_node(target)));
        }
        handles
            .into_iter()
            .filter_map(|handle| handle.join().ok().flatten())
            .collect()
    })
}

fn socket_addr(target: &ProbeTarget) -> Option<SocketAddr> {
    let ip: IpAddr = target.address.parse().ok()?;
    Some(match ip {
        IpAddr::V4(v4) => SocketAddr::from((v4, 80)),
        IpAddr::V6(v6) => SocketAddr::V6(SocketAddrV6::new(v6, 80, 0, target.scope_id)),
    })
}

fn fetch_node(target: &ProbeTarget) -> Option<HelloNode> {
    let addr = socket_addr(target)?;
    let mut sock = TcpStream::connect_timeout(&addr, Duration::from_millis(400)).ok()?;
    sock.set_read_timeout(Some(Duration::from_millis(500))).ok()?;
    sock.set_write_timeout(Some(Duration::from_millis(400))).ok()?;
    let host = if target.address.contains(':') {
        format!("[{}]", target.address)
    } else {
        target.address.clone()
    };
    sock.write_all(
        format!("GET /api/node HTTP/1.0\r\nHost: {host}\r\nConnection: close\r\n\r\n").as_bytes(),
    )
    .ok()?;
    let mut buf = Vec::new();
    sock.read_to_end(&mut buf).ok()?;
    let text = String::from_utf8_lossy(&buf);
    let body = text.split("\r\n\r\n").nth(1)?.trim();
    if !text.contains("200") {
        return None;
    }
    let parsed: NodeBody = serde_json::from_str(body).ok()?;
    if parsed.name.is_empty() || parsed.node_id.is_empty() {
        return None;
    }
    Some(HelloNode {
        address: target.address.clone(),
        name: parsed.name,
        node_id: parsed.node_id,
        subnet: parsed.subnet,
        backhaul_addr: parsed.backhaul_addr,
        link_local_lan: parsed.link_local_lan,
        ula: parsed.ula,
    })
}

pub fn apply_nodes(neighbors: &mut [LinkLocalNeighbor], nodes: &[HelloNode]) {
    for node in nodes {
        for row in neighbors.iter_mut() {
            let addr = row.address.split('%').next().unwrap_or(&row.address);
            let hit = addr == node.address
                || (!node.link_local_lan.is_empty() && addr == node.link_local_lan)
                || (!node.ula.is_empty() && addr == node.ula);
            if !hit {
                continue;
            }
            row.mesh_name = Some(node.name.clone());
            row.mesh_node_id = Some(node.node_id.clone());
            row.mesh_subnet = Some(node.subnet.clone());
            row.mesh_backhaul = Some(node.backhaul_addr.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, Ipv6Addr};

    use super::*;

    fn row(addr: IpAddr, ifindex: u32) -> LinkLocalNeighbor {
        LinkLocalNeighbor::from_ip("eth0", ifindex, addr, None, "reachable", "neighbor", "kernel")
    }

    #[test]
    fn every_unicast_neighbor_is_a_candidate() {
        let rows = vec![
            row("10.42.61.1".parse().unwrap(), 2),
            row("10.42.61.119".parse().unwrap(), 2),
            row("192.168.10.1".parse().unwrap(), 3),
            row("fe80::1".parse::<Ipv6Addr>().unwrap().into(), 3),
            row("fd89:ae6f:1b96::1".parse::<Ipv6Addr>().unwrap().into(), 3),
            row("ff02::1".parse::<Ipv6Addr>().unwrap().into(), 3),
            row(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 3),
        ];
        let got = probe_candidates(&rows);
        assert!(got.iter().any(|t| t.address == "10.42.61.1" && t.scope_id == 0));
        assert!(got.iter().any(|t| t.address == "10.42.61.119"));
        assert!(got.iter().any(|t| t.address == "192.168.10.1"));
        assert!(got.iter().any(|t| t.address == "fe80::1" && t.scope_id == 3));
        assert!(got
            .iter()
            .any(|t| t.address == "fd89:ae6f:1b96::1" && t.scope_id == 0));
        assert!(!got.iter().any(|t| t.address.contains("ff02")));
        assert!(!got.iter().any(|t| t.address == "0.0.0.0"));
    }

    #[test]
    fn link_local_socket_carries_the_interface_index() {
        let target = ProbeTarget {
            address: "fe80::82af:caff:fef2:21de".into(),
            scope_id: 19,
        };
        let SocketAddr::V6(addr) = socket_addr(&target).unwrap() else {
            panic!("expected ipv6");
        };
        assert_eq!(addr.scope_id(), 19);
        assert_eq!(addr.port(), 80);
    }
}
