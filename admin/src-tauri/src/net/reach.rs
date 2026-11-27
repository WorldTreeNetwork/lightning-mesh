// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

//! Addresses to ask for `GET /api/node` beyond the kernel neighbor table.
//!
//! A Lightning node is often a routed LAN gateway (`10.42.<n>.1`) rather than
//! an ARP neighbor of this computer. Connected subnets of `/24` or smaller are
//! walked host by host. Every `10.42.x.1` is tried as well, because that is
//! the address hello binds. Docker, bridges, and tunnels are not walked.

use std::collections::BTreeSet;
use std::net::{IpAddr, Ipv4Addr};

use if_addrs::IfAddr;

use super::hello::ProbeTarget;
use super::types::LinkLocalInterface;

/// A probe that may not already be a neighbor row. `iface` is where a new row
/// should sit when the address answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReachTarget {
    pub target: ProbeTarget,
    pub iface: String,
    pub ifindex: u32,
}

pub fn reach_targets(interfaces: &[LinkLocalInterface], already: &[ProbeTarget]) -> Vec<ReachTarget> {
    let seen: BTreeSet<String> = already.iter().map(|target| target.address.clone()).collect();
    let (egress, egress_index) = default_iface(interfaces);
    let mut out = Vec::new();
    let mut pushed = BTreeSet::new();

    for (iface, index, ip) in connected_hosts() {
        push(&mut out, &mut pushed, &seen, iface, index, ip);
    }
    for third in 0..=255 {
        let ip = Ipv4Addr::new(10, 42, third, 1);
        push(&mut out, &mut pushed, &seen, egress.clone(), egress_index, ip);
    }
    out
}

fn push(
    out: &mut Vec<ReachTarget>,
    pushed: &mut BTreeSet<String>,
    seen: &BTreeSet<String>,
    iface: String,
    ifindex: u32,
    ip: Ipv4Addr,
) {
    let address = ip.to_string();
    if seen.contains(&address) || !pushed.insert(address.clone()) {
        return;
    }
    out.push(ReachTarget {
        target: ProbeTarget {
            address,
            scope_id: 0,
        },
        iface,
        ifindex,
    });
}

/// Hosts on directly connected IPv4 subnets of prefix 24 through 30.
pub fn connected_hosts() -> Vec<(String, u32, Ipv4Addr)> {
    let Ok(addrs) = if_addrs::get_if_addrs() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for iface in addrs {
        if !scannable_iface(&iface.name) || iface.is_loopback() {
            continue;
        }
        let IfAddr::V4(v4) = iface.addr else {
            continue;
        };
        let index = iface.index.unwrap_or(0);
        for ip in subnet_hosts(v4.ip, v4.netmask) {
            if ip == v4.ip {
                continue;
            }
            out.push((iface.name.clone(), index, ip));
        }
    }
    out
}

pub fn scannable_iface(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    !(n == "lo"
        || n.starts_with("docker")
        || n.starts_with("br-")
        || n.starts_with("veth")
        || n.starts_with("tailscale")
        || n.starts_with("tun")
        || n.starts_with("tap")
        || n.starts_with("wg")
        || n.starts_with("mj-")
        || n.starts_with("virbr")
        || n.starts_with("zt")
        || n.starts_with("cni")
        || n.starts_with("flannel"))
}

/// Hosts other than the network and broadcast addresses. Prefixes wider than
/// a `/24` are skipped so a `/16` is not walked.
pub fn subnet_hosts(ip: Ipv4Addr, mask: Ipv4Addr) -> Vec<Ipv4Addr> {
    let prefix = u32::from(mask).count_ones();
    if !(24..=30).contains(&prefix) {
        return Vec::new();
    }
    let mask_u = u32::from(mask);
    let network = u32::from(ip) & mask_u;
    let broadcast = network | !mask_u;
    let mut out = Vec::new();
    let mut cur = network.wrapping_add(1);
    while cur < broadcast {
        out.push(Ipv4Addr::from(cur));
        cur = cur.wrapping_add(1);
    }
    out
}

/// First hop of a claimed client subnet, `10.42.61.0/24` → `10.42.61.1`.
pub fn subnet_gateway(subnet: &str) -> Option<Ipv4Addr> {
    let (addr, prefix) = subnet.split_once('/')?;
    let ip: Ipv4Addr = addr.parse().ok()?;
    let prefix: u32 = prefix.parse().ok()?;
    if prefix == 0 || prefix > 32 {
        return None;
    }
    let mask = u32::MAX << (32 - prefix);
    let network = u32::from(ip) & mask;
    let gateway = network.wrapping_add(1);
    if gateway == network || gateway == (network | !mask) {
        return None;
    }
    Some(Ipv4Addr::from(gateway))
}

/// One neighbor from `GET /api/directory` that this computer may not have reached.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectoryRecord {
    pub name: String,
    pub node_id: String,
    pub subnet: String,
    pub backhaul: String,
    /// Address to show: the subnet gateway, else another published IPv4 or ULA.
    pub address: String,
}

/// Neighbors in a directory document. The answering node itself is `node`, not this list.
pub fn directory_records(value: &serde_json::Value) -> Vec<DirectoryRecord> {
    let Some(neighbors) = value.get("neighbors").and_then(|item| item.as_array()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for neighbor in neighbors {
        let Some(obj) = neighbor.as_object() else {
            continue;
        };
        let Some(address) = record_address(obj) else {
            continue;
        };
        out.push(DirectoryRecord {
            name: json_text(obj, "name"),
            node_id: json_text(obj, "node_id"),
            subnet: json_text(obj, "subnet"),
            backhaul: json_text(obj, "backhaul_addr"),
            address,
        });
    }
    out
}

fn json_text(obj: &serde_json::Map<String, serde_json::Value>, key: &str) -> String {
    obj.get(key)
        .and_then(|item| item.as_str())
        .unwrap_or("")
        .trim()
        .to_string()
}

/// Prefer the client gateway, then the transport pin, then another published address.
/// Link-local and global unicast IPv6 are not a row address: the scan keeps those
/// for probes, and a row has to be an address the list already knows how to show.
fn record_address(obj: &serde_json::Map<String, serde_json::Value>) -> Option<String> {
    let mut found = Vec::new();
    if let Some(subnet) = obj.get("subnet").and_then(|item| item.as_str()) {
        if let Some(gateway) = subnet_gateway(subnet) {
            found.push(IpAddr::V4(gateway));
        }
    }
    if let Some(text) = obj.get("backhaul_addr").and_then(|item| item.as_str()) {
        if let Some(ip) = parse_ip_token(text) {
            found.push(ip);
        }
    }
    if let Some(addrs) = obj.get("addrs").and_then(|item| item.as_array()) {
        for addr in addrs {
            if let Some(text) = addr.as_str() {
                if let Some(ip) = parse_ip_token(text) {
                    found.push(ip);
                }
            }
        }
    }
    found.into_iter().find(|ip| showable_address(*ip)).map(|ip| ip.to_string())
}

fn showable_address(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => !v4.is_multicast() && !v4.is_broadcast() && !v4.is_unspecified() && !v4.is_loopback(),
        IpAddr::V6(v6) => v6.is_unique_local(),
    }
}

/// IPs mentioned by a directory document: this node's subnet gateway and
/// backhaul, plus each neighbor's addresses and subnet gateway.
pub fn directory_addresses(value: &serde_json::Value) -> Vec<IpAddr> {
    let mut out = Vec::new();
    if let Some(node) = value.get("node").and_then(|item| item.as_object()) {
        push_directory_object(&mut out, node);
    }
    if let Some(neighbors) = value.get("neighbors").and_then(|item| item.as_array()) {
        for neighbor in neighbors {
            if let Some(obj) = neighbor.as_object() {
                push_directory_object(&mut out, obj);
            }
        }
    }
    out
}

fn push_directory_object(out: &mut Vec<IpAddr>, obj: &serde_json::Map<String, serde_json::Value>) {
    if let Some(subnet) = obj.get("subnet").and_then(|item| item.as_str()) {
        if let Some(gateway) = subnet_gateway(subnet) {
            out.push(IpAddr::V4(gateway));
        }
    }
    for key in ["backhaul_addr", "link_local_lan", "ula", "house_ula"] {
        if let Some(text) = obj.get(key).and_then(|item| item.as_str()) {
            if let Some(ip) = parse_ip_token(text) {
                out.push(ip);
            }
        }
    }
    if let Some(addrs) = obj.get("addrs").and_then(|item| item.as_array()) {
        for addr in addrs {
            if let Some(text) = addr.as_str() {
                if let Some(ip) = parse_ip_token(text) {
                    out.push(ip);
                }
            }
        }
    }
}

fn parse_ip_token(text: &str) -> Option<IpAddr> {
    let text = text.split('%').next().unwrap_or(text).trim();
    if text.is_empty() {
        return None;
    }
    if let Ok(ip) = text.parse::<IpAddr>() {
        return Some(ip);
    }
    text.parse::<std::net::SocketAddr>().ok().map(|addr| addr.ip())
}

fn default_iface(interfaces: &[LinkLocalInterface]) -> (String, u32) {
    if let Some((name, index)) = route_default_iface() {
        if let Some(iface) = interfaces.iter().find(|iface| iface.name == name) {
            return (iface.name.clone(), iface.index);
        }
        return (name, index);
    }
    interfaces
        .iter()
        .find(|iface| iface.is_up && !iface.is_loopback && scannable_iface(&iface.name))
        .map(|iface| (iface.name.clone(), iface.index))
        .unwrap_or_else(|| ("network".into(), 0))
}

fn route_default_iface() -> Option<(String, u32)> {
    let text = std::fs::read_to_string("/proc/net/route").ok()?;
    for line in text.lines().skip(1) {
        let mut parts = line.split_whitespace();
        let name = parts.next()?;
        let destination = parts.next()?;
        if destination == "00000000" {
            return Some((name.to_string(), 0));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_24_is_walked_and_a_16_is_not() {
        let mask24 = Ipv4Addr::new(255, 255, 255, 0);
        let hosts = subnet_hosts(Ipv4Addr::new(192, 168, 1, 20), mask24);
        assert_eq!(hosts.len(), 254);
        assert_eq!(hosts[0], Ipv4Addr::new(192, 168, 1, 1));
        assert!(subnet_hosts(Ipv4Addr::new(10, 42, 0, 1), Ipv4Addr::new(255, 255, 0, 0)).is_empty());
    }

    #[test]
    fn lightning_lan_gateways_are_candidates() {
        let targets = reach_targets(&[], &[]);
        let got: BTreeSet<_> = targets.iter().map(|item| item.target.address.as_str()).collect();
        // Every 10.42.x.1 is a candidate. A laptop already on one of those
        // LANs also walks that /24, so the total can be larger than 256.
        for octet in 0..256 {
            assert!(got.contains(format!("10.42.{octet}.1").as_str()), "missing 10.42.{octet}.1");
        }
    }

    #[test]
    fn neighbor_addresses_are_not_probed_twice() {
        let already = vec![ProbeTarget {
            address: "10.42.61.1".into(),
            scope_id: 0,
        }];
        let targets = reach_targets(&[], &already);
        assert!(targets.iter().all(|item| item.target.address != "10.42.61.1"));
    }

    #[test]
    fn directory_lists_gateways_and_neighbor_addresses() {
        let value = serde_json::json!({
            "node": { "subnet": "10.42.61.0/24", "backhaul_addr": "10.254.61.115" },
            "neighbors": [{
                "addrs": ["10.254.181.221:49737"],
                "subnet": "10.42.181.0/24"
            }]
        });
        let got = directory_addresses(&value);
        assert!(got.contains(&IpAddr::V4(Ipv4Addr::new(10, 42, 61, 1))));
        assert!(got.contains(&IpAddr::V4(Ipv4Addr::new(10, 254, 61, 115))));
        assert!(got.contains(&IpAddr::V4(Ipv4Addr::new(10, 254, 181, 221))));
        assert!(got.contains(&IpAddr::V4(Ipv4Addr::new(10, 42, 181, 1))));
    }

    #[test]
    fn directory_records_keep_a_neighbor_that_only_published_an_overlay() {
        let value = serde_json::json!({
            "node": { "name": "ap3000-indoor-c", "node_id": "self", "subnet": "10.42.203.0/24" },
            "neighbors": [
                {
                    "name": "ap3000-indoor-b",
                    "node_id": "indoor-b",
                    "subnet": "10.42.86.0/24",
                    "backhaul_addr": "10.254.86.252"
                },
                {
                    "node_id": "tr3000",
                    "addrs": ["10.254.61.115:49737", "[2001:db8::19]:47042"]
                }
            ]
        });
        let got = directory_records(&value);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].name, "ap3000-indoor-b");
        assert_eq!(got[0].address, "10.42.86.1");
        assert_eq!(got[0].backhaul, "10.254.86.252");
        assert_eq!(got[1].name, "");
        assert_eq!(got[1].address, "10.254.61.115");
    }

    #[test]
    fn virtual_ifaces_are_not_walked() {
        assert!(scannable_iface("wlp191s0"));
        assert!(scannable_iface("enp196s0f0u2u2"));
        assert!(!scannable_iface("docker0"));
        assert!(!scannable_iface("br-4fbd877829e3"));
        assert!(!scannable_iface("tailscale0"));
    }
}
