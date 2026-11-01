use std::net::{Ipv4Addr, Ipv6Addr};

/// The reserved link-addressing block for per-peer TUN /31s.
/// Devices on the mesh never see these addresses.
pub const LINK_BLOCK: (Ipv4Addr, u8) = (Ipv4Addr::new(10, 255, 0, 0), 16);

/// Prefix length of the shared mesh **backhaul** block (`10.254.0.0/16`).
///
/// Every node self-assigns one address in this block to its shared-segment
/// (container) interface; the host part is derived from the node id, so each
/// node's underlay address is stable and collision-free with zero coordination —
/// no DHCP, no server, works fully offline.
///
/// This is the node-to-node *underlay* that iroh/QUIC and mDNS run over (distinct
/// from the per-peer TUN /31s in [`LINK_BLOCK`], from the CRDT-assigned client
/// /24s, and from babeld's IPv6 link-local *overlay* adjacency on the TUNs).
/// Because every node shares this one `/16`, all peers are on-link to each other
/// on the shared L2 segment — direct neighbours, no routing.
///
/// IPv4 (not an IPv6 ULA) on purpose: iroh 1.0 surfaces private IPv4 addresses as
/// connection candidates and announces them over mDNS, but does *not* surface
/// IPv6 ULA addresses, and binding one directly trips IPv6 DAD. See the
/// `iroh-lan-backhaul-findings` memory for the empirical detail.
pub const BACKHAUL_PREFIX_LEN: u8 = 16;

/// Domain-separated input for the mesh ULA `/48` (add-identity-derived-ula).
/// ASCII bytes, not the CRDT gossip topic (`mjolnir/mesh/crdt/v0`) — bumping
/// gossip must not renumber every node. Federation (`yau`) may later replace
/// this with a per-mesh id; not this change.
pub const ULA_ID_INPUT: &[u8] = b"mjolnir/mesh/ula/v0";

/// Prefix length of the identity-derived ULA on `br-mesh`. Every node's
/// `/128` sits in this one `/64` (subnet id 0) so peers are on-link.
pub const ULA_PREFIX_LEN: u8 = 64;

/// RFC 4193 ULA `/48` for this software mesh: `fd` + first 40 bits of
/// `blake3(ULA_ID_INPUT)`.
pub fn ula_prefix() -> Ipv6NetPrefix {
    let p = blake3::hash(ULA_ID_INPUT);
    let b = p.as_bytes();
    Ipv6NetPrefix {
        octets: [
            0xfd, b[0], b[1], b[2], b[3], b[4], 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
    }
}

/// Tiny helper so tests can format the `/48` without depending on `ipnet` here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ipv6NetPrefix {
    pub octets: [u8; 16],
}

impl Ipv6NetPrefix {
    pub fn addr(self) -> Ipv6Addr {
        Ipv6Addr::from(self.octets)
    }
}

/// Derive this node's identity ULA from its node id.
///
/// `/48` from [`ula_prefix`], subnet id 0, interface id = first 64 bits of
/// `blake3(node_id)` (opaque; RFC 7136). IID 0 is clamped to 1 (subnet-router
/// anycast). Never hashed from the IPv4 `10.254` host — `pt9` may move that.
pub fn ula_addr(node_id: &str) -> Ipv6Addr {
    let p = blake3::hash(ULA_ID_INPUT);
    let pb = p.as_bytes();
    let i = blake3::hash(node_id.as_bytes());
    let ib = i.as_bytes();
    let mut iid = u64::from_be_bytes([ib[0], ib[1], ib[2], ib[3], ib[4], ib[5], ib[6], ib[7]]);
    if iid == 0 {
        iid = 1;
    }
    let idb = iid.to_be_bytes();
    Ipv6Addr::from([
        0xfd, pb[0], pb[1], pb[2], pb[3], pb[4], 0, 0, idb[0], idb[1], idb[2], idb[3], idb[4],
        idb[5], idb[6], idb[7],
    ])
}

/// True when `ip` is Unique Local (`fc00::/7`). Used to refuse iroh candidates.
pub fn is_unique_local(ip: Ipv6Addr) -> bool {
    ip.is_unique_local()
}

/// Domain-separated input for a house `/48` (add-house-ula-prefix).
/// Twenty ASCII bytes, no trailing NUL. Not [`ULA_ID_INPUT`] and not the
/// CRDT gossip topic: changing either of those must not renumber the house.
pub const HOUSE_ULA_DOMAIN: &[u8] = b"mjolnir/house/ula/v1";

/// Exactly 64 hex digits (either case) of the raw 32-byte house guild id.
pub const HOUSE_GUILD_PATH: &str = "/etc/mjolnir/house-guild-id";

/// One line: the last house `/128` this daemon observed as usable.
/// The only address the daemon may delete.
pub const HOUSE_ULA_STATE_PATH: &str = "/var/lib/mjolnir/house-ula";

/// Router hosts sit in subnet 0. App hosts sit in subnet 1.
pub const HOUSE_ROUTER_SUBNET: u16 = 0;
pub const HOUSE_APP_SUBNET: u16 = 1;

/// `IFA_F_TENTATIVE`. A tentative address is not usable.
pub const IFA_F_TENTATIVE: u32 = 0x40;
/// `IFA_F_DADFAILED`. A DAD-failed address is not usable.
pub const IFA_F_DADFAILED: u32 = 0x08;

/// A house `/128` is usable when it is present at prefix length 128 and is
/// neither tentative nor DAD-failed. Command status is not part of this.
pub fn host_route_usable(prefix_len: u8, flags: u32) -> bool {
    prefix_len == 128 && flags & IFA_F_TENTATIVE == 0 && flags & IFA_F_DADFAILED == 0
}

fn trim_ascii_ws(text: &str) -> &str {
    text.trim_matches(|c: char| matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{000b}' | '\u{000c}'))
}

/// Accept a guild file body: ASCII trim, then exactly 64 hex digits.
/// Display-form base58 and anything else are absent.
pub fn parse_house_guild_id(text: &str) -> Option<[u8; 32]> {
    let text = trim_ascii_ws(text);
    if text.len() != 64 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

/// Parse the state file. One IPv6 address. Anything else means no owned address.
pub fn parse_house_state(text: &str) -> Option<Ipv6Addr> {
    let text = trim_ascii_ws(text);
    if text.is_empty() || text.chars().any(char::is_whitespace) {
        return None;
    }
    text.parse().ok()
}

fn iid64(input: &[u8]) -> [u8; 8] {
    let hashed = blake3::hash(input);
    let mut iid = u64::from_be_bytes(hashed.as_bytes()[..8].try_into().expect("8 bytes"));
    if iid == 0 {
        iid = 1;
    }
    iid.to_be_bytes()
}

fn house_host(guild_id: &[u8; 32], subnet: u16, iid: [u8; 8]) -> Ipv6Addr {
    let digest = blake3::hash(&[HOUSE_ULA_DOMAIN, guild_id].concat());
    let prefix = digest.as_bytes();
    let sub = subnet.to_be_bytes();
    Ipv6Addr::from([
        0xfd, prefix[0], prefix[1], prefix[2], prefix[3], prefix[4], sub[0], sub[1], iid[0],
        iid[1], iid[2], iid[3], iid[4], iid[5], iid[6], iid[7],
    ])
}

/// House `/48`: `fd` plus the first five bytes of `blake3(domain || guild_id)`.
pub fn house_ula_prefix(guild_id: &[u8; 32]) -> Ipv6NetPrefix {
    let digest = blake3::hash(&[HOUSE_ULA_DOMAIN, guild_id].concat());
    let b = digest.as_bytes();
    Ipv6NetPrefix {
        octets: [
            0xfd, b[0], b[1], b[2], b[3], b[4], 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
    }
}

/// Router `/128` in subnet 0. The interface id is `blake3` of the node id's
/// UTF-8, the same input as [`ula_addr`], clamped off 0.
pub fn router_ula(guild_id: &[u8; 32], node_id: &str) -> Ipv6Addr {
    house_host(guild_id, HOUSE_ROUTER_SUBNET, iid64(node_id.as_bytes()))
}

/// App `/128` in subnet 1. The interface id is `blake3` of the raw 32-byte key.
pub fn app_ula(guild_id: &[u8; 32], app_key: &[u8; 32]) -> Ipv6Addr {
    house_host(guild_id, HOUSE_APP_SUBNET, iid64(app_key))
}

/// Desired router `/128` from a guild file body, or none when the file is absent
/// or malformed.
pub fn desired_router_ula(guild_text: Option<&str>, node_id: &str) -> Option<Ipv6Addr> {
    let guild_id = parse_house_guild_id(guild_text?)?;
    Some(router_ula(&guild_id, node_id))
}

/// One IPv6 row from `/proc/net/if_inet6`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IfInet6Addr {
    pub addr: Ipv6Addr,
    pub prefix_len: u8,
    pub flags: u32,
    pub iface: String,
}

/// Parse `/proc/net/if_inet6`. Prefix length, scope, and flags are hex.
pub fn parse_if_inet6(body: &str) -> Vec<IfInet6Addr> {
    let mut out = Vec::new();
    for line in body.lines() {
        let mut parts = line.split_whitespace();
        let Some(hex) = parts.next() else { continue };
        let Some(_idx) = parts.next() else { continue };
        let Some(plen) = parts.next() else { continue };
        let Some(_scope) = parts.next() else { continue };
        let Some(flags) = parts.next() else { continue };
        let Some(iface) = parts.next() else { continue };
        if hex.len() != 32 {
            continue;
        }
        let Ok(prefix_len) = u8::from_str_radix(plen, 16) else {
            continue;
        };
        let Ok(flags) = u32::from_str_radix(flags, 16) else {
            continue;
        };
        let mut bytes = [0u8; 16];
        let mut ok = true;
        for (i, chunk) in hex.as_bytes().chunks_exact(2).enumerate() {
            match std::str::from_utf8(chunk)
                .ok()
                .and_then(|s| u8::from_str_radix(s, 16).ok())
            {
                Some(b) => bytes[i] = b,
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            continue;
        }
        out.push(IfInet6Addr {
            addr: Ipv6Addr::from(bytes),
            prefix_len,
            flags,
            iface: iface.to_string(),
        });
    }
    out
}

/// Whether `want` is a usable `/128` on `iface` in a `/proc/net/if_inet6` body.
pub fn house_addr_usable(body: &str, iface: &str, want: Ipv6Addr) -> bool {
    parse_if_inet6(body).into_iter().any(|row| {
        row.iface == iface && row.addr == want && host_route_usable(row.prefix_len, row.flags)
    })
}

/// Publish the house address only when the desired `/128` is usable on `iface`.
pub fn published_house_ula(
    node_id: &str,
    guild_text: Option<&str>,
    body: &str,
    iface: &str,
) -> Option<Ipv6Addr> {
    let desired = desired_router_ula(guild_text, node_id)?;
    house_addr_usable(body, iface, desired).then_some(desired)
}

/// What to do with `/var/lib/mjolnir/house-ula` after the post-add observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HouseStateAction {
    /// Desired address is usable. Record it.
    Write(Ipv6Addr),
    /// No desired address. Remove the state file.
    Remove,
    /// Add did not leave a usable address. Leave the file bytes alone.
    Unchanged,
}

/// Delete and add decided from the pre-add observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HouseIntent {
    pub owned: Option<Ipv6Addr>,
    pub desired: Option<Ipv6Addr>,
    pub delete: Option<Ipv6Addr>,
    pub try_add: Option<Ipv6Addr>,
}

/// `usable_before` is whether the desired address (not the owned one) is usable.
pub fn house_intent(
    state_text: Option<&str>,
    guild_text: Option<&str>,
    node_id: &str,
    usable_before: bool,
) -> HouseIntent {
    let owned = state_text.and_then(parse_house_state);
    let desired = desired_router_ula(guild_text, node_id);
    let delete = owned.filter(|addr| Some(*addr) != desired);
    let try_add = desired.filter(|_| !usable_before);
    HouseIntent {
        owned,
        desired,
        delete,
        try_add,
    }
}

/// Publication and the state file follow the observation after delete and add.
/// An add error is irrelevant when `usable_after` is true.
pub fn house_after(
    desired: Option<Ipv6Addr>,
    usable_after: bool,
) -> (Option<Ipv6Addr>, HouseStateAction) {
    match desired {
        Some(addr) if usable_after => (Some(addr), HouseStateAction::Write(addr)),
        Some(_) => (None, HouseStateAction::Unchanged),
        None => (None, HouseStateAction::Remove),
    }
}

/// Return whether the wanted backhaul address needs to be restored.
///
/// Kept independent of netlink so address-reconciliation decisions can be
/// tested on every supported development platform.
pub fn backhaul_addr_missing(present: &[Ipv4Addr], want: Ipv4Addr) -> bool {
    !present.contains(&want)
}

/// First two octets of the backhaul block `10.254.0.0/16`. Chosen to avoid the
/// TUN-link block (`10.255.0.0/16`, [`LINK_BLOCK`]) and the client mesh space
/// (`10.42.0.0/16`).
const BACKHAUL_BLOCK: [u8; 2] = [10, 254];

/// Derive this node's stable IPv4 backhaul address from its node id.
///
/// Address = `10.254.<h>.<l>`, where `<h>.<l>` is the first 16 bits of
/// `blake3(node_id)`. Deterministic (same node id → same address every boot) and
/// collision-resistant over the 16-bit host space — negligible for any realistic
/// same-site mesh, and the same trade-off [`pick_link_31`] already makes for the
/// TUN /31s. The host part is clamped off the `/16` network (`.0.0`) and
/// broadcast (`.255.255`) addresses.
pub fn backhaul_addr(node_id: &str) -> Ipv4Addr {
    backhaul_addr_salted(node_id, 0)
}

/// Derive the `attempt`-th candidate backhaul address for a node.
///
/// Attempt 0 is byte-identical to [`backhaul_addr`] (the pre-pt9 derivation, so
/// deployed nodes keep their addresses). Higher attempts re-hash with the
/// attempt number mixed in — the deterministic "next slot" a node moves to when
/// it loses a backhaul-address claim conflict (mjolnir-mesh-pt9). 16 bits of
/// host space makes collisions rare but real at fleet scale (~50% odds by ~300
/// nodes); the claim CRDT detects them and the HLC loser re-derives here.
pub fn backhaul_addr_salted(node_id: &str, attempt: u32) -> Ipv4Addr {
    let hash = if attempt == 0 {
        blake3::hash(node_id.as_bytes())
    } else {
        blake3::hash(format!("{node_id}#backhaul-{attempt}").as_bytes())
    };
    let bytes = hash.as_bytes();
    let mut host = u16::from_be_bytes([bytes[0], bytes[1]]);
    if host == 0 {
        host = 1; // avoid the network address 10.254.0.0
    } else if host == u16::MAX {
        host = u16::MAX - 1; // avoid the broadcast address 10.254.255.255
    }
    let [h, l] = host.to_be_bytes();
    Ipv4Addr::new(BACKHAUL_BLOCK[0], BACKHAUL_BLOCK[1], h, l)
}

/// Whether a claimed net lives in the backhaul block `10.254.0.0/16` — i.e. it
/// is a backhaul *address* claim (pt9), not a client-subnet claim. The two kinds
/// share the CRDT store and gossip lane; every consumer that means "client /24"
/// must exclude these.
pub fn in_backhaul_block(net: &ipnet::Ipv4Net) -> bool {
    net.prefix_len() >= BACKHAUL_PREFIX_LEN && net.network().octets()[..2] == BACKHAUL_BLOCK
}

/// Derive a /31 for a peer-pair, symmetrically.
///
/// Returns `(self_addr, peer_addr)` from the perspective of `self_id`.
/// Calling `pick_link_31(B, A)` on the other side returns the swapped pair, so both
/// endpoints agree on which /31 the link uses without coordination.
///
/// The /31 is selected by hashing the sorted (lower, higher) node-id pair into the
/// `10.255.0.0/16` address space. Collisions across distinct pairs are bounded by
/// the size of the /16 (32,768 distinct /31s) — sufficient for any realistic mesh.
pub fn pick_link_31(self_id: &str, peer_id: &str) -> (Ipv4Addr, Ipv4Addr) {
    // 1. Sort the pair lexicographically (lower, higher).
    let (lower, higher) = if self_id <= peer_id {
        (self_id, peer_id)
    } else {
        (peer_id, self_id)
    };

    // 2. Hash with blake3 over the concatenation (lower || "\0" || higher).
    let mut input = String::with_capacity(lower.len() + 1 + higher.len());
    input.push_str(lower);
    input.push('\0');
    input.push_str(higher);
    let hash = blake3::hash(input.as_bytes());
    let bytes = hash.as_bytes();

    // 3. Take 15 bits → offset within 10.255.0.0/16 chunked into /31s (15-bit space).
    //    Use first two bytes of hash for the 15-bit offset.
    let raw = u16::from_be_bytes([bytes[0], bytes[1]]);
    let offset = raw & 0x7FFF; // 15 bits

    // 4. Lower IP of the /31:
    //    10.255.{(offset >> 7) & 0xff}.{(offset & 0x7f) << 1}
    //    Upper IP = lower + 1.
    let third_octet = ((offset >> 7) & 0xFF) as u8;
    let fourth_octet_lower = ((offset & 0x7F) << 1) as u8;
    let lower_ip = Ipv4Addr::new(10, 255, third_octet, fourth_octet_lower);
    let upper_ip = Ipv4Addr::new(10, 255, third_octet, fourth_octet_lower + 1);

    // 5. Lower node-id gets the lower IP; higher gets the higher IP.
    // 6. From self_id's perspective: if self_id is the lower of the pair, return
    //    (lower_ip, upper_ip); else (upper_ip, lower_ip).
    if self_id <= peer_id {
        (lower_ip, upper_ip)
    } else {
        (upper_ip, lower_ip)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backhaul_addr_missing_when_absent() {
        let want = Ipv4Addr::new(10, 254, 1, 2);
        let present = [Ipv4Addr::new(192, 168, 1, 1), Ipv4Addr::new(10, 254, 3, 4)];
        assert!(backhaul_addr_missing(&present, want));
    }

    #[test]
    fn backhaul_addr_missing_is_false_when_present_among_several() {
        let want = Ipv4Addr::new(10, 254, 1, 2);
        let present = [
            Ipv4Addr::new(192, 168, 1, 1),
            want,
            Ipv4Addr::new(10, 254, 3, 4),
        ];
        assert!(!backhaul_addr_missing(&present, want));
    }

    #[test]
    fn backhaul_addr_missing_from_empty_list() {
        assert!(backhaul_addr_missing(&[], Ipv4Addr::new(10, 254, 1, 2)));
    }

    #[test]
    fn deterministic_same_pair() {
        let first = pick_link_31("alpha", "beta");
        let second = pick_link_31("alpha", "beta");
        assert_eq!(first, second);
    }

    #[test]
    fn symmetric_across_endpoints() {
        let (self_a, peer_a) = pick_link_31("alpha", "beta");
        let (self_b, peer_b) = pick_link_31("beta", "alpha");
        assert_eq!(self_a, peer_b);
        assert_eq!(peer_a, self_b);
        // Both ends agree on the link addresses.
    }

    #[test]
    fn addresses_form_valid_31() {
        let (self_addr, peer_addr) = pick_link_31("node-aabbccdd", "node-eeff0011");

        // Both within 10.255.0.0/16
        let octets_self = self_addr.octets();
        let octets_peer = peer_addr.octets();
        assert_eq!(octets_self[0], 10);
        assert_eq!(octets_self[1], 255);
        assert_eq!(octets_peer[0], 10);
        assert_eq!(octets_peer[1], 255);

        // self_addr and peer_addr differ by exactly 1 in the last octet,
        // with the lower being even (start of /31).
        let self_u32 = u32::from(self_addr);
        let peer_u32 = u32::from(peer_addr);
        assert_eq!(self_u32.abs_diff(peer_u32), 1);

        // The lower address of the pair must be even (bit 0 clear).
        let lower_u32 = self_u32.min(peer_u32);
        assert_eq!(lower_u32 & 1, 0);
    }

    #[test]
    fn backhaul_addr_is_deterministic() {
        let id = "fd7691128f2bb615d56cf2f0e202fa01472890dd8af89f9132d34d566776ed45";
        assert_eq!(backhaul_addr(id), backhaul_addr(id));
    }

    #[test]
    fn backhaul_addr_in_shared_block() {
        // Every node lands in the same 10.254.0.0/16, so all peers are on-link.
        let a = backhaul_addr("alpha");
        let b = backhaul_addr("beta");
        assert_eq!(&a.octets()[..2], &[10, 254]);
        assert_eq!(&b.octets()[..2], &[10, 254]);
        // Distinct node ids → distinct addresses.
        assert_ne!(a, b);
        // Never the /16 network or broadcast address.
        assert_ne!(a, Ipv4Addr::new(10, 254, 0, 0));
        assert_ne!(a, Ipv4Addr::new(10, 254, 255, 255));
    }

    #[test]
    fn salted_attempt_zero_matches_unsalted() {
        // Wire/deploy compat: attempt 0 must be byte-identical to the pre-pt9
        // derivation so already-deployed nodes keep their addresses.
        let id = "fd7691128f2bb615d56cf2f0e202fa01472890dd8af89f9132d34d566776ed45";
        assert_eq!(backhaul_addr_salted(id, 0), backhaul_addr(id));
    }

    #[test]
    fn salted_attempts_are_distinct_and_in_block() {
        use std::collections::HashSet;
        let id = "some-node";
        let mut seen = HashSet::new();
        for attempt in 0..8 {
            let a = backhaul_addr_salted(id, attempt);
            assert_eq!(&a.octets()[..2], &[10, 254]);
            assert_ne!(a, Ipv4Addr::new(10, 254, 0, 0));
            assert_ne!(a, Ipv4Addr::new(10, 254, 255, 255));
            seen.insert(a);
        }
        // 8 attempts in a 16-bit space: all distinct, statistically certain.
        assert_eq!(
            seen.len(),
            8,
            "salted attempts must yield distinct addresses"
        );
    }

    #[test]
    fn backhaul_block_membership() {
        use std::str::FromStr;
        let yes = ipnet::Ipv4Net::from_str("10.254.3.43/32").unwrap();
        let whole = ipnet::Ipv4Net::from_str("10.254.0.0/16").unwrap();
        let client = ipnet::Ipv4Net::from_str("10.42.1.0/24").unwrap();
        let link = ipnet::Ipv4Net::from_str("10.255.0.0/31").unwrap();
        assert!(in_backhaul_block(&yes));
        assert!(in_backhaul_block(&whole));
        assert!(!in_backhaul_block(&client));
        assert!(!in_backhaul_block(&link));
    }

    #[test]
    fn ula_addr_is_deterministic_unique_local_and_on_link() {
        let a = ula_addr("alpha");
        let b = ula_addr("beta");
        assert_eq!(a, ula_addr("alpha"));
        assert!(a.is_unique_local());
        assert!(b.is_unique_local());
        assert_ne!(a, b);
        // Same /64 (subnet id 0): first 8 octets match; last 8 are the IID.
        assert_eq!(&a.octets()[..8], &b.octets()[..8]);
        assert_eq!(&a.octets()[6..8], &[0, 0]);
        assert_eq!(a.octets()[0], 0xfd);
    }

    #[test]
    fn ula_addr_golden_vector() {
        // Exact input bytes: b"mjolnir/mesh/ula/v0". Locked so Admin / apps
        // can compute the same address without reading meshd.
        let id = "fd7691128f2bb615d56cf2f0e202fa01472890dd8af89f9132d34d566776ed45";
        let got = ula_addr(id);
        assert_eq!(
            got,
            "fd53:6213:4797:0:1478:922e:5c61:d39f"
                .parse::<Ipv6Addr>()
                .unwrap()
        );
    }

    #[test]
    fn backhaul_addrs_distinct_across_many_nodes() {
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        for i in 0u32..100 {
            seen.insert(backhaul_addr(&format!("node-{i:08x}")));
        }
        // 100 nodes in a 16-bit host space: collisions are statistically negligible.
        assert_eq!(
            seen.len(),
            100,
            "all 100 node backhaul addrs must be unique"
        );
    }

    #[test]
    fn distinct_pairs_get_distinct_links_usually() {
        use std::collections::HashSet;

        let pairs: Vec<(String, String)> = (0u32..100)
            .map(|i| (format!("node-{i:08x}"), format!("node-{:08x}", i + 1000)))
            .collect();

        let mut seen = HashSet::new();
        for (a, b) in &pairs {
            let (self_addr, _) = pick_link_31(a, b);
            // Use the lower IP of the /31 as the key (both ends share the same /31)
            let lower = u32::from(self_addr).min(u32::from({
                let (_, p) = pick_link_31(a, b);
                p
            }));
            seen.insert(lower);
        }

        // >95% should be distinct
        assert!(
            seen.len() >= 95,
            "only {} distinct /31s out of 100 pairs",
            seen.len()
        );
    }

    fn guild(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    fn guild_text(byte: u8) -> String {
        format!("{:02x}", byte).repeat(32)
    }

    fn inet_line(addr: Ipv6Addr, plen: u8, flags: u32, iface: &str) -> String {
        let hex: String = addr.octets().iter().map(|b| format!("{b:02x}")).collect();
        format!("{hex} 01 {plen:02x} 00 {flags:02x} {iface}\n")
    }

    fn plan(
        state: Option<&str>,
        guild_body: Option<&str>,
        node_id: &str,
        before: &str,
        after: &str,
    ) -> (HouseIntent, Option<Ipv6Addr>, HouseStateAction) {
        let desired = desired_router_ula(guild_body, node_id);
        let usable_before = desired.is_some_and(|addr| house_addr_usable(before, "br-mesh", addr));
        let intent = house_intent(state, guild_body, node_id, usable_before);
        let usable_after = desired.is_some_and(|addr| house_addr_usable(after, "br-mesh", addr));
        let (publish, action) = house_after(desired, usable_after);
        (intent, publish, action)
    }

    #[test]
    fn house_ula_fixture_vectors() {
        assert_eq!(HOUSE_ULA_DOMAIN.len(), 20);
        let node_id = "node-example";
        let guild_a = guild(0x11);
        let guild_b = guild(0x22);
        let app_key = [0xab; 32];
        let router_a: Ipv6Addr = "fd81:2b22:8aca::ddcd:86a6:64e9:f2df".parse().unwrap();
        let app_a: Ipv6Addr = "fd81:2b22:8aca:1:bd5e:fa7a:5460:697".parse().unwrap();
        let router_b: Ipv6Addr = "fd8f:78a9:2caa::ddcd:86a6:64e9:f2df".parse().unwrap();
        let legacy: Ipv6Addr = "fd53:6213:4797::ddcd:86a6:64e9:f2df".parse().unwrap();

        assert_eq!(
            house_ula_prefix(&guild_a).addr(),
            "fd81:2b22:8aca::".parse::<Ipv6Addr>().unwrap()
        );
        assert_eq!(
            house_ula_prefix(&guild_b).addr(),
            "fd8f:78a9:2caa::".parse::<Ipv6Addr>().unwrap()
        );
        assert_eq!(router_ula(&guild_a, node_id), router_a);
        assert_eq!(router_ula(&guild_b, node_id), router_b);
        assert_eq!(app_ula(&guild_a, &app_key), app_a);
        assert_eq!(ula_addr(node_id), legacy);
        assert_eq!(&router_a.octets()[6..8], &[0, 0]);
        assert_eq!(&app_a.octets()[6..8], &[0, 1]);
        assert_ne!(&router_a.octets()[..6], &router_b.octets()[..6]);
        assert_ne!(&router_a.octets()[..8], &legacy.octets()[..8]);
        // The compiled prefix and the gossip topic are not inputs.
        assert_ne!(house_ula_prefix(&guild_a).octets, ula_prefix().octets);
        assert_eq!(router_ula(&guild_a, node_id), router_ula(&guild_a, node_id));
    }

    #[test]
    fn house_guild_parser_accepts_hex_and_rejects_display_form() {
        let raw = guild_text(0x11);
        assert_eq!(parse_house_guild_id(&raw), Some(guild(0x11)));
        assert_eq!(parse_house_guild_id(&raw.to_uppercase()), Some(guild(0x11)));
        assert_eq!(
            parse_house_guild_id(&format!("  {raw}\n")),
            Some(guild(0x11))
        );
        assert_eq!(parse_house_guild_id(""), None);
        assert_eq!(parse_house_guild_id(&raw[..63]), None);
        assert_eq!(parse_house_guild_id(&format!("{raw}zz")), None);
        assert_eq!(parse_house_guild_id("2bE2k7mhouseguildidnothex"), None);
        let router: Ipv6Addr = "fd81:2b22:8aca::ddcd:86a6:64e9:f2df".parse().unwrap();
        assert_eq!(parse_house_state(&router.to_string()), Some(router));
        assert_eq!(parse_house_state(&format!("  {router}\n")), Some(router));
        assert_eq!(parse_house_state(""), None);
        assert_eq!(parse_house_state("not-an-address"), None);
        assert_eq!(parse_house_state(&format!("{router}/128")), None);
        assert_eq!(parse_house_state(&format!("{router}\n{router}")), None);
    }

    #[test]
    fn house_assignment_transitions() {
        let node_id = "node-example";
        let guild_a = guild_text(0x11);
        let guild_b = guild_text(0x22);
        let router_a = router_ula(&guild(0x11), node_id);
        let router_b = router_ula(&guild(0x22), node_id);
        let legacy = ula_addr(node_id);
        let state_a = router_a.to_string();
        let usable_a = inet_line(router_a, 128, 0x80, "br-mesh");
        let usable_b = inet_line(router_b, 128, 0x80, "br-mesh");
        let legacy_line = inet_line(legacy, 64, 0x80, "br-mesh");

        // Fresh assign: empty kernel, then the /128 is usable.
        let (intent, publish, action) = plan(None, Some(&guild_a), node_id, "", &usable_a);
        assert_eq!(intent.delete, None);
        assert_eq!(intent.try_add, Some(router_a));
        assert_eq!(publish, Some(router_a));
        assert_eq!(action, HouseStateAction::Write(router_a));

        // Already usable. No add. Still recorded.
        let (intent, publish, action) = plan(
            Some(&state_a),
            Some(&guild_a),
            node_id,
            &usable_a,
            &usable_a,
        );
        assert_eq!(intent.try_add, None);
        assert_eq!(intent.delete, None);
        assert_eq!(publish, Some(router_a));
        assert_eq!(action, HouseStateAction::Write(router_a));

        // Add was attempted and reported an error, but the address is usable
        // on the second look. That still counts as assigned.
        let (intent, publish, action) = plan(None, Some(&guild_a), node_id, "", &usable_a);
        assert_eq!(intent.try_add, Some(router_a));
        assert_eq!(publish, Some(router_a));
        assert_eq!(action, HouseStateAction::Write(router_a));

        // Failed add leaves no usable address: do not publish, do not replace state.
        let (intent, publish, action) = plan(Some(&state_a), Some(&guild_b), node_id, "", "");
        assert_eq!(intent.delete, Some(router_a));
        assert_eq!(intent.try_add, Some(router_b));
        assert_eq!(publish, None);
        assert_eq!(action, HouseStateAction::Unchanged);

        // Tentative and DAD-failed are not published and are not recorded.
        for flags in [
            IFA_F_TENTATIVE,
            IFA_F_DADFAILED,
            IFA_F_TENTATIVE | IFA_F_DADFAILED,
        ] {
            let line = inet_line(router_a, 128, flags, "br-mesh");
            let (intent, publish, action) = plan(None, Some(&guild_a), node_id, &line, &line);
            assert_eq!(intent.try_add, Some(router_a));
            assert_eq!(publish, None);
            assert_eq!(action, HouseStateAction::Unchanged);
        }

        // A /64 with the same bits is not the house host route.
        let covering = inet_line(router_a, 64, 0x80, "br-mesh");
        let (_, publish, action) = plan(None, Some(&guild_a), node_id, &covering, &covering);
        assert_eq!(publish, None);
        assert_eq!(action, HouseStateAction::Unchanged);

        // The same address on another interface is not this node's assignment.
        let other = inet_line(router_a, 128, 0x80, "mjolnir0");
        let (_, publish, _) = plan(None, Some(&guild_a), node_id, &other, &other);
        assert_eq!(publish, None);
        assert_eq!(
            published_house_ula(node_id, Some(&guild_a), &other, "br-mesh"),
            None
        );

        // Interface loss, then the reconcile restores the current host.
        let (intent, publish, action) =
            plan(Some(&state_a), Some(&guild_a), node_id, "", &usable_a);
        assert_eq!(intent.try_add, Some(router_a));
        assert_eq!(publish, Some(router_a));
        assert_eq!(action, HouseStateAction::Write(router_a));

        // Bad guild file adds nothing and omits publication.
        let (intent, publish, action) = plan(
            None,
            Some("not-hex-and-not-64"),
            node_id,
            &usable_a,
            &usable_a,
        );
        assert_eq!(intent.desired, None);
        assert_eq!(intent.try_add, None);
        assert_eq!(intent.delete, None);
        assert_eq!(publish, None);
        assert_eq!(action, HouseStateAction::Remove);

        // Missing state deletes nothing, even if some other /128 is present.
        let stranger: Ipv6Addr = "fd99::1".parse().unwrap();
        let stranger_line = inet_line(stranger, 128, 0x80, "br-mesh");
        let (intent, _, _) = plan(None, Some(&guild_a), node_id, &stranger_line, &usable_a);
        assert_eq!(intent.delete, None);
        assert_eq!(intent.try_add, Some(router_a));

        // Guild change or removal deletes only the recorded address.
        let both = format!("{legacy_line}{usable_a}");
        let (intent, publish, action) = plan(Some(&state_a), None, node_id, &both, &legacy_line);
        assert_eq!(intent.delete, Some(router_a));
        assert_ne!(intent.delete, Some(legacy));
        assert_eq!(publish, None);
        assert_eq!(action, HouseStateAction::Remove);

        let (intent, publish, action) =
            plan(Some(&state_a), Some(&guild_b), node_id, &both, &usable_b);
        assert_eq!(intent.delete, Some(router_a));
        assert_ne!(intent.delete, Some(legacy));
        assert_eq!(intent.try_add, Some(router_b));
        assert_eq!(publish, Some(router_b));
        assert_eq!(action, HouseStateAction::Write(router_b));

        // Unparseable state also deletes nothing.
        let (intent, _, _) = plan(Some("???"), Some(&guild_a), node_id, &usable_a, &usable_a);
        assert_eq!(intent.owned, None);
        assert_eq!(intent.delete, None);
    }
}
