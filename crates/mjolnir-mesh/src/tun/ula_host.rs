//! Exact IPv6 host routes to already-connected overlay peers.

use std::collections::HashMap;
use std::net::{Ipv4Addr, Ipv6Addr};

pub type UlaHostRoutes = HashMap<Ipv6Addr, Ipv4Addr>;

/// Parse one `ip -6 route show dev <overlay iface>` snapshot. The caller must
/// replace the previous map, so withdrawn routes cannot keep selecting peers.
pub fn parse_ula_host_routes(text: &str) -> UlaHostRoutes {
    let mut next = UlaHostRoutes::new();
    for line in text.lines() {
        let toks: Vec<&str> = line.split_whitespace().collect();
        let Some(prefix) = toks.first() else { continue };
        if prefix.starts_with("fe80:") || *prefix == "any" || *prefix == "default" {
            continue;
        }
        let (ip, plen) = prefix.split_once('/').unwrap_or((prefix, "128"));
        let (Ok(dest), Ok(128)) = (ip.parse::<Ipv6Addr>(), plen.parse::<u8>()) else {
            continue;
        };
        let via = toks
            .iter()
            .position(|t| *t == "via")
            .and_then(|i| toks.get(i + 1))
            .and_then(|s| s.parse::<Ipv6Addr>().ok());
        let Some(peer) = via.and_then(fe80_to_overlay) else {
            continue;
        };
        next.insert(dest, peer);
    }
    next
}

/// Reverse `overlay_link_local` only for its exact `fe80::H` shape.
fn fe80_to_overlay(addr: Ipv6Addr) -> Option<Ipv4Addr> {
    let s = addr.segments();
    if s[0] != 0xfe80 || s[1..7].iter().any(|x| *x != 0) {
        return None;
    }
    let host = s[7];
    Some(Ipv4Addr::new(10, 254, (host >> 8) as u8, host as u8))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admitted_host_route_selects_ipv4_peer_and_withdraws() {
        let dest: Ipv6Addr = "fd8f:78a9:2caa::2".parse().unwrap();
        let peer = Ipv4Addr::new(10, 254, 0, 2);
        let routes =
            parse_ula_host_routes("fd8f:78a9:2caa::2/128 via fe80::2 dev mjolnir0 proto babel\n");
        assert_eq!(routes.get(&dest), Some(&peer));
        // Demux yields the existing IPv4 transport peer, never an fd bind.
        assert_ne!(routes.get(&dest), Some(&Ipv4Addr::new(10, 254, 0, 3)));
        assert_eq!(parse_ula_host_routes("").get(&dest), None);
    }

    #[test]
    fn house_forward_host_routes_select_only_the_named_peer_and_withdraw() {
        let host: Ipv6Addr = "fd81:2b22:8aca::2".parse().unwrap();
        let other: Ipv6Addr = "fd81:2b22:8aca::99".parse().unwrap();
        let peer = Ipv4Addr::new(10, 254, 0, 2);
        let other_peer = Ipv4Addr::new(10, 254, 0, 3);
        let mut routes = parse_ula_host_routes(
            "fd81:2b22:8aca::2/128 via fe80::2 dev mjolnir0 proto babel\n\
             fd81:2b22:8aca::99 via fe80::3 dev mjolnir0 proto babel\n",
        );
        assert_eq!(routes.get(&host), Some(&peer));
        assert_ne!(routes.get(&host), Some(&other_peer));
        assert_eq!(routes.get(&other), Some(&other_peer));

        routes = parse_ula_host_routes("fd81:2b22:8aca::99/128 via fe80::3\n");
        assert_eq!(routes.get(&host), None);
        assert_eq!(routes.get(&other), Some(&other_peer));
        routes = parse_ula_host_routes("");
        assert!(routes.is_empty());
    }

    #[test]
    fn house_forward_rejects_covering_routes_and_non_overlay_next_hops() {
        for line in [
            "fd81:2b22:8aca::2/128 via fe80:1::2",
            "fd81:2b22:8aca::2/128 via fe80::1:2",
            "fd81:2b22:8aca::2/128 via fd81::2",
            "fd81:2b22:8aca::2/128 via 10.254.0.2",
            "fd81:2b22:8aca::2/128 via",
            "fd81:2b22:8aca::2/128 dev mjolnir0",
            "fd81:2b22:8aca::/48 via fe80::2",
            "fd81:2b22:8aca::/64 via fe80::2",
            "fd81:2b22:8aca::2/127 via fe80::2",
            "fd81:2b22:8aca::2/129 via fe80::2",
            "fe80::2/128 via fe80::3",
            "any via fe80::2",
            "default via fe80::2",
            "garbage via fe80::2",
            "",
        ] {
            assert!(parse_ula_host_routes(line).is_empty(), "{line}");
        }
    }
}
