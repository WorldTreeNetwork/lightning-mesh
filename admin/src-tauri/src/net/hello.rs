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

use std::io::{ErrorKind, Read, Write};
use std::net::{IpAddr, SocketAddr, SocketAddrV6, TcpStream};
use std::time::Duration;

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
    pub house_ula: Option<String>,
    pub link_local_lan: String,
    pub ula: String,
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

pub(crate) fn is_probeable(ip: IpAddr) -> bool {
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
    parse_node_body(&target.address, body)
}

fn json_text(obj: &serde_json::Map<String, serde_json::Value>, key: &str) -> String {
    obj.get(key)
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .trim()
        .to_string()
}

/// A hello document. `{}` is the unpublished node (`node: null` in the
/// directory). A JSON object without hello's keys is not a node.
fn parse_node_body(address: &str, body: &str) -> Option<HelloNode> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    let obj = value.as_object()?;
    let known = obj.is_empty()
        || obj.contains_key("node_id")
        || obj.contains_key("backhaul_addr")
        || obj.contains_key("subnet")
        || obj.contains_key("link_local_lan")
        || obj.contains_key("ula");
    if !known {
        return None;
    }
    let house = json_text(obj, "house_ula");
    Some(HelloNode {
        address: address.to_string(),
        name: json_text(obj, "name"),
        node_id: json_text(obj, "node_id"),
        subnet: json_text(obj, "subnet"),
        backhaul_addr: json_text(obj, "backhaul_addr"),
        house_ula: if house.is_empty() { None } else { Some(house) },
        link_local_lan: json_text(obj, "link_local_lan"),
        ula: json_text(obj, "ula"),
    })
}

const LAN_PATHS: &[&str] = &["/api/node", "/api/radio", "/api/directory", "/api/health"];
const LAN_BODY_LIMIT: usize = 256 * 1024;

/// GET one hello document. The path is limited to the node, radio, and directory reads.
pub fn http_get_json(address: &str, scope_id: u32, path: &str) -> Result<serde_json::Value, String> {
    if !LAN_PATHS.contains(&path) {
        return Err("that path is not a node document".into());
    }
    let address = address.split('%').next().unwrap_or(address).trim();
    let ip: IpAddr = address
        .parse()
        .map_err(|_| "LAN address is not an IP".to_string())?;
    if !is_probeable(ip) {
        return Err("that address is not a unicast LAN target".into());
    }
    let scope_id = match ip {
        IpAddr::V6(v6) if v6.is_unicast_link_local() => scope_id,
        _ => 0,
    };
    let target = ProbeTarget {
        address: address.to_string(),
        scope_id,
    };
    let addr = socket_addr(&target).ok_or_else(|| "could not address that host".to_string())?;
    let mut sock = TcpStream::connect_timeout(&addr, Duration::from_millis(800))
        .map_err(|e| format!("timed out or refused: {e}"))?;
    sock.set_read_timeout(Some(Duration::from_millis(1200)))
        .map_err(|e| e.to_string())?;
    sock.set_write_timeout(Some(Duration::from_millis(800)))
        .map_err(|e| e.to_string())?;
    let host = if address.contains(':') {
        format!("[{address}]")
    } else {
        address.to_string()
    };
    sock.write_all(
        format!("GET {path} HTTP/1.0\r\nHost: {host}\r\nConnection: close\r\n\r\n").as_bytes(),
    )
    .map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        match sock.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                if buf.len() + n > LAN_BODY_LIMIT {
                    return Err("the node document is too large".into());
                }
                buf.extend_from_slice(&chunk[..n]);
            }
            Err(e) if e.kind() == ErrorKind::TimedOut || e.kind() == ErrorKind::WouldBlock => {
                if buf.is_empty() {
                    return Err(format!("timed out waiting for {path}"));
                }
                break;
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    parse_http_json(&String::from_utf8_lossy(&buf))
}

pub fn parse_http_json(raw: &str) -> Result<serde_json::Value, String> {
    let (head, body) = raw
        .split_once("\r\n\r\n")
        .ok_or_else(|| "the node response has no body".to_string())?;
    let status = head.lines().next().unwrap_or("");
    if !status.contains(" 200 ") && !status.ends_with(" 200") {
        let line = status.trim();
        return Err(if line.is_empty() {
            "the node did not answer".into()
        } else {
            format!("the node answered {line}")
        });
    }
    let body = body.trim();
    if body.is_empty() {
        return Err("the node document is empty".into());
    }
    serde_json::from_str(body).map_err(|e| format!("the node document is not JSON: {e}"))
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
            row.lightning = true;
            if !node.name.is_empty() {
                row.mesh_name = Some(node.name.clone());
            }
            if !node.node_id.is_empty() {
                row.mesh_node_id = Some(node.node_id.clone());
            }
            if !node.subnet.is_empty() {
                row.mesh_subnet = Some(node.subnet.clone());
            }
            if !node.backhaul_addr.is_empty() {
                row.mesh_backhaul = Some(node.backhaul_addr.clone());
            }
            row.house_ula = node.house_ula.clone();
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
    fn empty_node_document_is_still_hello() {
        let node = parse_node_body("192.168.1.1", "{}").unwrap();
        assert!(node.name.is_empty());
        assert!(node.node_id.is_empty());
        let mut rows = vec![row("192.168.1.1".parse().unwrap(), 2)];
        apply_nodes(&mut rows, &[node]);
        assert!(rows[0].lightning);
        assert!(rows[0].mesh_name.is_none());
        assert!(parse_node_body("10.0.0.1", r#"{"status":"ok"}"#).is_none());
    }

    #[test]
    fn house_ula_survives_the_probe_and_discovery_row() {
        let address = "10.42.61.1";
        let reference = "fd81:2b22:8aca:0:ddcd:86a6:64e9:f2df";
        let mut body = serde_json::json!({
            "name": "example",
            "node_id": "node-example",
            "subnet": "10.42.61.0/24",
            "backhaul_addr": "10.254.0.2",
            "link_local_lan": "fe80::1",
            "house_ula": reference,
        });
        let mut rows = vec![
            row(address.parse().unwrap(), 2),
            row("fe80::1".parse().unwrap(), 2),
        ];
        let node = parse_node_body(address, &body.to_string()).unwrap();
        apply_nodes(&mut rows, &[node]);
        for row in &rows {
            assert_eq!(row.house_ula.as_deref(), Some(reference));
            assert_eq!(row.mesh_backhaul.as_deref(), Some("10.254.0.2"));
        }

        // Older or unconfigured nodes omit the field; a fresh probe also clears
        // a previously published reference without dropping the transport pin.
        body.as_object_mut().unwrap().remove("house_ula");
        let node = parse_node_body(address, &body.to_string()).unwrap();
        assert_eq!(node.house_ula, None);
        apply_nodes(&mut rows, &[node]);
        for row in &rows {
            assert_eq!(row.house_ula, None);
            assert_eq!(row.mesh_backhaul.as_deref(), Some("10.254.0.2"));
        }
        body["house_ula"] = serde_json::Value::Null;
        assert_eq!(parse_node_body(address, &body.to_string()).unwrap().house_ula, None);
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
    fn http_json_reads_a_node_document_and_rejects_other_paths() {
        let doc = parse_http_json("HTTP/1.0 200 OK\r\nContent-Type: application/json\r\n\r\n{\"node_id\":\"abc\"}")
            .unwrap();
        assert_eq!(doc["node_id"], "abc");
        let err = parse_http_json("HTTP/1.0 404 Not Found\r\n\r\n{}").unwrap_err();
        assert!(err.contains("404"));
        let err = http_get_json("10.42.1.1", 0, "/api/owner-claim").unwrap_err();
        assert!(err.contains("not a node document"));
        let err = http_get_json("255.255.255.255", 0, "/api/radio").unwrap_err();
        assert!(err.contains("not a unicast"));
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
