// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

//! IPv6 link-local discovery: local `fe80::` addresses on every interface,
//! kernel neighbor-table dump, and an ICMPv6 all-nodes (`ff02::1`) probe.

mod icmp;
mod interfaces;
mod merge;
mod neigh;
mod types;

pub use merge::merge_scan;
pub use types::{LinkLocalInterface, LinkLocalNeighbor, RawNeighbor, ScanResult};

pub async fn scan_link_local() -> Result<ScanResult, String> {
    let interfaces = interfaces::collect_interfaces()?;
    let from_neigh = neigh::dump_link_local(&interfaces).await.unwrap_or_default();
    let (from_probe, probe_error) = icmp::probe_all_nodes(&interfaces);
    Ok(merge_scan(
        interfaces,
        from_neigh,
        from_probe,
        probe_error,
        true,
    ))
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
            assert!(
                n.address.starts_with("fe80:"),
                "non-link-local leaked: {}",
                n.address
            );
            assert!(n.scoped.contains('%'), "missing zone id: {}", n.scoped);
        }
    }
}
