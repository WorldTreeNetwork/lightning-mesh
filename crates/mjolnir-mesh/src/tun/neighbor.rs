//! Authenticated overlay neighbor identities. Link-local addresses are observations,
//! never reversible encodings of a node identity.
use super::{desired_router_ula, router_ula};
use ipnet::IpNet;
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv6Addr};

#[derive(Debug, Default)]
pub struct Neighbors {
    peers: HashMap<u64, (Ipv6Addr, HashSet<Ipv6Addr>)>,
}
impl Neighbors {
    pub fn register(&mut self, connection: u64, guild: Option<&str>, node_id: &str) -> bool {
        let Some(key) = desired_router_ula(guild, node_id) else {
            return false;
        };
        self.peers.insert(connection, (key, HashSet::new()));
        true
    }
    pub fn remove(&mut self, connection: u64) {
        self.peers.remove(&connection);
    }
    pub fn observe(&mut self, connection: u64, packet: &[u8]) {
        if let (Some(source), Some((_, sources))) =
            (babel_source(packet), self.peers.get_mut(&connection))
        {
            sources.insert(source);
        }
    }
    /// Both directions must be unique. A colliding key also fails closed.
    pub fn connection(&self, via: Ipv6Addr) -> Option<u64> {
        let mut claims = self
            .peers
            .iter()
            .filter(|(_, (_, sources))| sources.contains(&via));
        let (id, (key, sources)) = claims.next()?;
        if claims.next().is_some()
            || sources.len() != 1
            || self.peers.values().filter(|(k, _)| k == key).count() != 1
        {
            return None;
        }
        Some(*id)
    }
    pub fn key(&self, connection: u64) -> Option<Ipv6Addr> {
        self.peers.get(&connection).map(|(key, _)| *key)
    }
}

pub fn link_local(addr: Ipv6Addr) -> bool {
    addr.segments()[0] & 0xffc0 == 0xfe80
}

/// IPv6/UDP Babel v2 packet containing a well-formed Hello or IHU TLV.
/// Extension headers/fragments are deliberately not association evidence.
pub fn babel_source(packet: &[u8]) -> Option<Ipv6Addr> {
    if packet.first()? >> 4 != 6 || *packet.get(6)? != 17 {
        return None;
    }
    let payload = u16::from_be_bytes(packet.get(4..6)?.try_into().ok()?) as usize;
    let udp = packet.get(40..40 + payload)?;
    if udp.len() < 12
        || u16::from_be_bytes(udp[0..2].try_into().ok()?) != 6696
        || u16::from_be_bytes(udp[2..4].try_into().ok()?) != 6696
    {
        return None;
    }
    let udp_len = u16::from_be_bytes(udp[4..6].try_into().ok()?) as usize;
    if udp_len != udp.len() || udp[8..10] != [42, 2] {
        return None;
    }
    let body_len = u16::from_be_bytes(udp[10..12].try_into().ok()?) as usize;
    let body = udp.get(12..12 + body_len)?;
    let source = Ipv6Addr::from(<[u8; 16]>::try_from(packet.get(8..24)?).ok()?);
    if !link_local(source) {
        return None;
    }
    let mut i = 0;
    let mut evidence = false;
    while i < body.len() {
        let typ = body[i];
        i += 1;
        if typ == 0 {
            continue;
        }
        let len = *body.get(i)? as usize;
        i += 1;
        let value = body.get(i..i + len)?;
        if typ == 4 && len >= 6 {
            evidence = true;
        }
        if typ == 5 && len >= 6 {
            let expected = match value[0] {
                0 => 6,
                1 => 10,
                2 => 22,
                3 => 14,
                _ => usize::MAX,
            };
            if len >= expected {
                evidence = true;
            }
        }
        i += len;
    }
    evidence.then_some(source)
}

#[derive(Debug, Default, Clone)]
pub struct NeighborRoutes(Vec<(IpNet, Ipv6Addr)>);
impl NeighborRoutes {
    /// Parse Babel routes on the explicitly selected overlay interface. IPv4
    /// next hops must be the kernel's `via inet6 fe80::...`, never 10.254.
    pub fn parse(text: &str, iface: &str) -> Self {
        let mut routes = Vec::new();
        for line in text.lines() {
            let tokens: Vec<_> = line.split_whitespace().collect();
            let field = |name| {
                tokens
                    .iter()
                    .position(|s| *s == name)
                    .and_then(|i| tokens.get(i + 1))
                    .copied()
            };
            if field("dev") != Some(iface) || field("proto") != Some("babel") {
                continue;
            }
            let Some(first) = tokens.first() else {
                continue;
            };
            let prefix = if *first == "default" {
                Some("0.0.0.0/0".parse::<IpNet>().unwrap())
            } else {
                first
                    .parse::<IpNet>()
                    .ok()
                    .or_else(|| first.parse::<IpAddr>().ok().map(IpNet::from))
            };
            let Some(prefix) = prefix else { continue };
            if matches!(prefix, IpNet::V6(v6) if v6.prefix_len() != 128 || link_local(v6.addr())) {
                continue;
            }
            let Some(i) = tokens.iter().position(|s| *s == "via") else {
                continue;
            };
            let via = tokens
                .get(i + 1)
                .filter(|s| **s != "inet6")
                .or_else(|| tokens.get(i + 2))
                .and_then(|s| s.parse::<Ipv6Addr>().ok());
            if let Some(via) = via.filter(|v| link_local(*v)) {
                routes.push((prefix, via));
            }
        }
        Self(routes)
    }
    pub fn via(&self, dest: IpAddr) -> Option<Ipv6Addr> {
        if let IpAddr::V6(v6) = dest {
            if link_local(v6) {
                return Some(v6);
            }
        }
        let max = self
            .0
            .iter()
            .filter(|(prefix, _)| prefix.contains(&dest))
            .map(|(p, _)| p.prefix_len())
            .max()?;
        let vias: HashSet<_> = self
            .0
            .iter()
            .filter(|(p, _)| p.prefix_len() == max && p.contains(&dest))
            .map(|(_, via)| *via)
            .collect();
        (vias.len() == 1).then(|| *vias.iter().next().unwrap())
    }
    pub fn next_hop_key(&self, dest: IpAddr, neighbors: &Neighbors) -> Option<Ipv6Addr> {
        neighbors.key(self.select(dest, neighbors)?)
    }

    /// The two dumps have different default-route address families. Only IPv4
    /// prefixes and IPv6 /128s are forwarded by the overlay.
    pub fn snapshots(v4: &str, v6: &str, iface: &str) -> Self {
        let mut routes = Self::parse(v4, iface);
        let hosts = v6
            .lines()
            .filter(|line| !line.starts_with("default "))
            .collect::<Vec<_>>()
            .join("\n");
        routes.0.extend(Self::parse(&hosts, iface).0);
        routes
    }

    pub fn select(&self, dest: IpAddr, neighbors: &Neighbors) -> Option<u64> {
        neighbors.connection(self.via(dest)?)
    }
}

pub fn overlay_link_local(node_id: &str) -> Ipv6Addr {
    let iid = router_ula(&[0; 32], node_id).octets();
    let mut out = [0; 16];
    out[0] = 0xfe;
    out[1] = 0x80;
    out[8..].copy_from_slice(&iid[8..]);
    Ipv6Addr::from(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn hello(source: &str, typ: u8) -> Vec<u8> {
        let mut packet = vec![0; 60];
        packet[0] = 0x60;
        packet[6] = 17;
        packet[4..6].copy_from_slice(&20u16.to_be_bytes());
        packet[8..24].copy_from_slice(&source.parse::<Ipv6Addr>().unwrap().octets());
        packet[40..42].copy_from_slice(&6696u16.to_be_bytes());
        packet[42..44].copy_from_slice(&6696u16.to_be_bytes());
        packet[44..46].copy_from_slice(&20u16.to_be_bytes());
        packet[48..52].copy_from_slice(&[42, 2, 0, 8]);
        packet[52] = typ;
        packet[53] = 6;
        packet
    }
    #[test]
    fn authenticated_transit_ipv4_return_withdrawal_and_disconnect() {
        let guild = "11".repeat(32);
        let ll = "fe80::403d:2741:12c1:e5c0";
        assert_eq!(overlay_link_local("peer-example").to_string(), ll);
        let mut n = Neighbors::default();
        assert!(n.register(7, Some(&guild), "peer-example"));
        assert_eq!(
            n.key(7).unwrap().to_string(),
            "fd81:2b22:8aca:0:403d:2741:12c1:e5c0"
        );
        let local = router_ula(&[0x11; 32], "node-example");
        let r = NeighborRoutes::parse(
            &format!(
                "fd81:2b22:8aca::2/128 via {ll} dev mjolnir0 proto babel\nfd8f:78a9:2caa::2 via {ll} dev mjolnir0 proto babel\n{local}/128 via {ll} dev mjolnir0 proto babel\n10.42.1.0/24 via inet6 {ll} dev mjolnir0 proto babel"
            ),
            "mjolnir0",
        );
        assert_eq!(r.select("10.42.1.50".parse().unwrap(), &n), None);
        n.observe(7, &hello(ll, 4));
        assert_eq!(
            r.next_hop_key("fd8f:78a9:2caa::2".parse().unwrap(), &n),
            Some(router_ula(&[0x11; 32], "peer-example"))
        );
        for dest in [
            "fd81:2b22:8aca::2",
            "fd8f:78a9:2caa::2",
            "10.42.1.50",
            &local.to_string(),
            ll,
        ] {
            assert_eq!(r.select(dest.parse().unwrap(), &n), Some(7));
        }
        for dest in ["fd12:3456:789a::1", "fd81:2b22:8aca::99"] {
            assert_eq!(r.select(dest.parse().unwrap(), &n), None);
        }
        assert_eq!(
            NeighborRoutes::default().select("10.42.1.50".parse().unwrap(), &n),
            None
        );
        assert_eq!(
            NeighborRoutes::default().select(ll.parse().unwrap(), &n),
            Some(7)
        );
        n.remove(7);
        assert_eq!(r.select(ll.parse().unwrap(), &n), None);
    }
    #[test]
    fn ambiguous_claims_and_bad_guild_fail_closed() {
        let guild = "11".repeat(32);
        let ll = "fe80::403d:2741:12c1:e5c0";
        let via = ll.parse().unwrap();
        let mut n = Neighbors::default();
        assert!(!n.register(1, None, "peer-example"));
        assert!(!n.register(1, Some("bad"), "peer-example"));
        n.register(1, Some(&guild), "peer-example");
        n.observe(1, &hello(ll, 5));
        assert_eq!(n.connection(via), Some(1));
        n.register(2, Some(&guild), "other");
        n.observe(2, &hello(ll, 4));
        assert_eq!(n.connection(via), None);
        n.remove(2);
        assert_eq!(n.connection(via), Some(1));
        n.observe(1, &hello("fe80::ddcd:86a6:64e9:f2df", 4));
        assert_eq!(n.connection(via), None);
    }
    #[test]
    fn rejects_non_control_and_ipv4_gateway() {
        assert_eq!(babel_source(&hello("fd81::2", 4)), None);
        assert_eq!(babel_source(&hello("fe80::2", 8)), None);
        let mut truncated = hello("fe80::2", 4);
        truncated.pop();
        assert_eq!(babel_source(&truncated), None);
        assert_eq!(
            NeighborRoutes::parse(
                "10.42.1.0/24 via 10.254.0.2 dev mjolnir0 proto babel",
                "mjolnir0"
            )
            .via("10.42.1.50".parse().unwrap()),
            None
        );
        assert_eq!(
            NeighborRoutes::parse("fd81::2 via fe80::2 dev br-mesh proto babel", "mjolnir0")
                .via("fd81::2".parse().unwrap()),
            None
        );
    }
}
