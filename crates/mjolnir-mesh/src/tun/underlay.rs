//! Pure policy for the source-specific outer transport routing table.
//! Babel owns peer routes; the daemon copies only non-Babel underlay routes.
use std::net::Ipv6Addr;

/// Babeld copies this id into `rtmsg.rtm_table`, which is one byte, and
/// reserves 252..=255. An id above 255 is truncated (42696 becomes 200),
/// so the policy rule would look up a table Babel never writes.
pub const OUTER_TABLE: &str = "210";
pub const OUTER_MARK: &str = "0x4d4a";

/// A binding lease includes the interface incarnation as well as its usable
/// host. A changed or absent observation invalidates an existing endpoint.
pub fn usable_binding(
    node: &str,
    guild: Option<&str>,
    kernel: &str,
    iface: &str,
    index: Option<u32>,
) -> Option<(Ipv6Addr, u32)> {
    let addr = super::desired_router_ula(guild, node)?;
    let index = index.filter(|i| *i != 0)?;
    super::house_addr_usable(kernel, iface, addr).then_some((addr, index))
}

/// Backhaul bind. The identity ULA is enough: no house guild, and a guild
/// that is not this peer's guild, still produce the same address.
pub fn usable_backhaul_binding(
    node: &str,
    kernel: &str,
    iface: &str,
    index: Option<u32>,
) -> Option<(Ipv6Addr, u32)> {
    let addr = super::ula_addr(node);
    let index = index.filter(|i| *i != 0)?;
    let usable = super::parse_if_inet6(kernel).into_iter().any(|row| {
        row.iface == iface
            && row.addr == addr
            && (row.prefix_len == 64 || row.prefix_len == 128)
            && row.flags & super::IFA_F_TENTATIVE == 0
            && row.flags & super::IFA_F_DADFAILED == 0
    });
    usable.then_some((addr, index))
}

/// The outer table's copy of the identity `/64` on the mesh bridge. A Babel
/// host route is not required. `unreachable` does not count.
pub fn on_link_backhaul_route(text: &str, peer: Ipv6Addr, iface: &str) -> bool {
    text.lines().any(|line| {
        let t: Vec<_> = line.split_whitespace().collect();
        if t.iter().any(|w| *w == "unreachable") {
            return false;
        }
        let field = |k| {
            t.iter()
                .position(|s| *s == k)
                .and_then(|i| t.get(i + 1))
                .copied()
        };
        let prefix = t.first().and_then(|s| s.parse::<ipnet::Ipv6Net>().ok());
        prefix.is_some_and(|n| n.contains(&peer)) && field("dev") == Some(iface)
    })
}

/// A direct dial needs an exact physical Babel host route, not a covering
/// prefix, an overlay route, or a host route installed by an operator.
pub fn physical_peer_route(text: &str, peer: Ipv6Addr, iface: &str) -> bool {
    text.lines().any(|line| {
        let t: Vec<_> = line.split_whitespace().collect();
        let field = |k| {
            t.iter()
                .position(|s| *s == k)
                .and_then(|i| t.get(i + 1))
                .copied()
        };
        let prefix = t.first().and_then(|s| {
            s.parse::<ipnet::Ipv6Net>().ok().or_else(|| {
                s.parse::<Ipv6Addr>()
                    .ok()
                    .map(|a| ipnet::Ipv6Net::new(a, 128).unwrap())
            })
        });
        prefix.is_some_and(|n| n.prefix_len() == 128 && n.addr() == peer)
            && field("dev") == Some(iface)
            && field("proto") == Some("babel")
    })
}

/// Safe routes for relay sockets. In particular neither a learned default
/// nor a route on a TUN may turn an underlay socket back into an inner packet.
pub fn relay_routes(text: &str) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    for line in text.lines() {
        let t: Vec<_> = line.split_whitespace().collect();
        let field = |k| {
            t.iter()
                .position(|s| *s == k)
                .and_then(|i| t.get(i + 1))
                .copied()
        };
        let Some(prefix) = t.first() else { continue };
        let Some(dev) = field("dev") else { continue };
        if dev == "mjolnir0" || dev.starts_with("mj-peer-") || field("proto") == Some("babel") {
            continue;
        }
        if *prefix != "default"
            && prefix.parse::<ipnet::IpNet>().is_err()
            && prefix.parse::<std::net::IpAddr>().is_err()
        {
            continue;
        }
        // All house reachability comes exclusively from physical Babel. A
        // main-table covering ULA route must not defeat the terminal deny.
        let addr = prefix
            .split('/')
            .next()
            .and_then(|p| p.parse::<std::net::IpAddr>().ok());
        if matches!(addr, Some(std::net::IpAddr::V6(v)) if v.is_unique_local())
            || matches!(addr, Some(std::net::IpAddr::V4(v)) if v.octets()[..2] == [10,254])
        {
            continue;
        }
        let mut route = vec![prefix.to_string()];
        for key in ["via", "dev", "metric", "src"] {
            if let Some(value) = field(key) {
                route.push(key.to_string());
                route.push(value.to_string());
            }
        }
        if t.contains(&"onlink") {
            route.push("onlink".into());
        }
        out.push(route);
    }
    out.sort_by_key(|route| route.iter().any(|s| s == "via"));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn outer_table_is_a_babeld_installable_id() {
        let id: u32 = OUTER_TABLE.parse().unwrap();
        assert!((1..=251).contains(&id));
    }
    #[test]
    fn usable_binding_rejects_dad_and_invalidates_on_loss_replacement_or_recreation() {
        let guild = "11".repeat(32);
        let addr = super::super::router_ula(&[0x11; 32], "node-example");
        let row = format!("{:032x} 00000002 80 00 00 br-mesh", u128::from(addr));
        let observe = |body: &str, text: Option<&str>, index| {
            usable_binding("node-example", text, body, "br-mesh", index)
        };
        let bound = observe(&row, Some(&guild), Some(2));
        assert_eq!(bound, Some((addr, 2)));
        for body in [
            row.replace("80 00 00", "80 00 40"),
            row.replace("80 00 00", "80 00 08"),
            row.replace("80 00 00", "40 00 00"),
            row.replace("br-mesh", "br-lan"),
            String::new(),
        ] {
            assert_eq!(observe(&body, Some(&guild), Some(2)), None);
        }
        assert_eq!(observe(&row, None, Some(2)), None);
        assert_eq!(observe(&row, Some("bad"), Some(2)), None);
        assert_eq!(observe(&row, Some(&guild), None), None);
        assert_ne!(observe(&row, Some(&guild), Some(3)), bound);
        let next_guild = "22".repeat(32);
        let next = super::super::router_ula(&[0x22; 32], "node-example");
        assert_eq!(observe(&row, Some(&next_guild), Some(2)), None);
        let new_row = row.replace(
            &format!("{:032x}", u128::from(addr)),
            &format!("{:032x}", u128::from(next)),
        );
        assert_eq!(
            observe(&new_row, Some(&next_guild), Some(2)),
            Some((next, 2))
        );
        assert_ne!(observe(&new_row, Some(&next_guild), Some(2)), bound);
    }

    #[test]
    fn backhaul_binding_ignores_guild_and_rejects_dad() {
        let addr = super::super::ula_addr("node-example");
        let row = format!("{:032x} 00000002 40 00 00 br-mesh", u128::from(addr));
        let bound = usable_backhaul_binding("node-example", &row, "br-mesh", Some(2));
        assert_eq!(bound, Some((addr, 2)));
        assert_eq!(
            usable_backhaul_binding(
                "node-example",
                &row.replace("40 00 00", "40 00 40"),
                "br-mesh",
                Some(2)
            ),
            None
        );
        assert_eq!(
            usable_backhaul_binding(
                "node-example",
                &row.replace("40 00 00", "40 00 08"),
                "br-mesh",
                Some(2)
            ),
            None
        );
        assert_eq!(
            usable_backhaul_binding("node-example", &row, "br-lan", Some(2)),
            None
        );
        assert_eq!(
            usable_backhaul_binding("node-example", &row, "br-mesh", None),
            None
        );
        assert_ne!(
            usable_backhaul_binding("node-example", &row, "br-mesh", Some(3)),
            bound
        );
        let prefix = super::super::ula_link_prefix();
        let peer = super::super::ula_addr("peer-example");
        assert!(prefix.contains(&addr) && prefix.contains(&peer));
        let on_link = format!("{prefix} dev br-mesh metric 256");
        assert!(on_link_backhaul_route(&on_link, peer, "br-mesh"));
        assert!(!on_link_backhaul_route(
            "unreachable fc00::/7 metric 42760",
            peer,
            "br-mesh"
        ));
        assert!(!on_link_backhaul_route(&on_link, peer, "mjolnir0"));
    }

    #[test]
    fn competing_overlay_metrics_and_physical_path_loss() {
        let peer = "fd81:2b22:8aca:0:403d:2741:12c1:e5c0".parse().unwrap();
        let physical = format!("{peer}/128 via fe80::aa dev br-mesh proto babel metric 9000");
        let overlay =
            format!("{peer}/128 via fe80::403d:2741:12c1:e5c0 dev mjolnir0 proto babel metric 1");
        assert!(physical_peer_route(
            &format!("{overlay}\n{physical}"),
            peer,
            "br-mesh"
        ));
        assert!(!physical_peer_route(&overlay, peer, "br-mesh"));
        assert!(!physical_peer_route(
            &physical.replace("proto babel", "proto static"),
            peer,
            "br-mesh"
        ));
        assert!(!physical_peer_route(
            "fd81:2b22:8aca::/48 dev br-mesh proto babel",
            peer,
            "br-mesh"
        ));
        assert_eq!(
            relay_routes(
                "default via 10.42.1.1 dev mjolnir0 proto babel metric 1\ndefault via 192.0.2.1 dev eth0 proto dhcp metric 9000"
            ),
            vec![vec![
                "default",
                "via",
                "192.0.2.1",
                "dev",
                "eth0",
                "metric",
                "9000"
            ]]
        );
        assert!(relay_routes("default via 10.42.1.1 dev br-mesh proto babel").is_empty());
    }
}
