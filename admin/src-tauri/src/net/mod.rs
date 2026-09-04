// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

//! IPv6 discovery: Unique Local (`fc00::/7`) and link-local (`fe80::/10`)
//! addresses on every interface, kernel neighbor-table dump, and an ICMPv6
//! all-nodes (`ff02::1`) probe. Addresses are also encoded as base58 of the
//! 16 raw octets for the admin list.

mod interfaces;
mod merge;
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

pub use merge::merge_scan;
pub use types::{LinkLocalInterface, LinkLocalNeighbor, RawNeighbor, ScanResult};

pub async fn scan_link_local() -> Result<ScanResult, String> {
    let interfaces = interfaces::collect_interfaces()?;
    let from_neigh = neigh::dump_link_local(&interfaces)
        .await
        .unwrap_or_default();
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
