// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

//! A Lightning node on its client LAN is the gateway at `10.42.<x>.1` and
//! answers `GET /api/node` from mjolnir-hello. That works with no upstream
//! internet: hello binds the LAN address.

use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;

use serde::Deserialize;

use super::types::LinkLocalNeighbor;

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

/// `10.42.0.0/16` host addresses that are the `.1` of their `/24`.
pub fn lan_gateway_candidates(neighbors: &[LinkLocalNeighbor]) -> Vec<String> {
    let mut out = Vec::new();
    for row in neighbors {
        let Ok(IpAddr::V4(ip)) = row.address.parse() else {
            continue;
        };
        if !is_mesh_lan_gateway(ip) {
            continue;
        }
        let text = ip.to_string();
        if !out.contains(&text) {
            out.push(text);
        }
    }
    out
}

pub fn is_mesh_lan_gateway(ip: Ipv4Addr) -> bool {
    let [a, b, _, d] = ip.octets();
    a == 10 && b == 42 && d == 1
}

pub fn probe_nodes(candidates: &[String]) -> Vec<HelloNode> {
    candidates.iter().filter_map(|ip| fetch_node(ip)).collect()
}

fn fetch_node(ip: &str) -> Option<HelloNode> {
    let addr: SocketAddr = format!("{ip}:80").parse().ok()?;
    let mut sock = TcpStream::connect_timeout(&addr, Duration::from_millis(400)).ok()?;
    sock.set_read_timeout(Some(Duration::from_millis(500))).ok()?;
    sock.set_write_timeout(Some(Duration::from_millis(400))).ok()?;
    sock.write_all(
        format!("GET /api/node HTTP/1.0\r\nHost: {ip}\r\nConnection: close\r\n\r\n").as_bytes(),
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
        address: ip.to_string(),
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
    use super::*;

    #[test]
    fn only_the_mesh_lan_gateway_is_a_candidate() {
        assert!(is_mesh_lan_gateway("10.42.61.1".parse().unwrap()));
        assert!(!is_mesh_lan_gateway("10.42.61.119".parse().unwrap()));
        assert!(!is_mesh_lan_gateway("10.43.1.1".parse().unwrap()));
        assert!(!is_mesh_lan_gateway("192.168.1.1".parse().unwrap()));
    }
}
