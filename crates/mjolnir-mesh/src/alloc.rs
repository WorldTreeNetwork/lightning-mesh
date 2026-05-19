//! Subnet claim allocation: deterministic preference + collision walk.

use ipnet::Ipv4Net;
use std::collections::HashSet;
use std::net::Ipv4Addr;

/// The default mesh address space (10.42.0.0/16, 256 /24s).
pub const DEFAULT_MESH_SPACE: Ipv4Net =
    Ipv4Net::new_assert(Ipv4Addr::new(10, 42, 0, 0), 16);

/// Pick a /24 within `base` (which must itself be a /16).
///
/// Algorithm:
///   1. Preferred index = blake3(node_id).as_bytes()[0] % 256.
///   2. Walk indices `preferred, preferred+1, ..., preferred+255` (mod 256).
///   3. Return the first /24 within `base` not in `claimed`.
///   4. If all 256 /24s are claimed (full mesh exhaustion), return `None`.
///
/// `claimed` is the set of currently-known claimed subnets (read from the
/// CRDT `/subnets/` namespace by the caller).
pub fn pick_subnet(
    node_id: &str,
    claimed: &HashSet<Ipv4Net>,
    base: Ipv4Net,
) -> Option<Ipv4Net> {
    assert_eq!(
        base.prefix_len(),
        16,
        "pick_subnet currently requires a /16 base"
    );
    // u16 widening: `preferred + offset` stays ≤ 510, so the `% 256 as u8`
    // truncation is well-defined. Do not narrow this back to u8 — it would
    // wrap silently mid-walk.
    let preferred = blake3::hash(node_id.as_bytes()).as_bytes()[0] as u16;
    let base_octets = base.network().octets();
    for offset in 0u16..256 {
        let idx = ((preferred + offset) % 256) as u8;
        let candidate =
            Ipv4Net::new(Ipv4Addr::new(base_octets[0], base_octets[1], idx, 0), 24).ok()?;
        if !claimed.contains(&candidate) {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty() -> HashSet<Ipv4Net> {
        HashSet::new()
    }

    #[test]
    fn deterministic_preference() {
        let a = pick_subnet("node-alpha", &empty(), DEFAULT_MESH_SPACE);
        let b = pick_subnet("node-alpha", &empty(), DEFAULT_MESH_SPACE);
        assert_eq!(a, b);
        assert!(a.is_some());
    }

    #[test]
    fn distinct_nodes_get_distinct_preferences_usually() {
        let mut results = HashSet::new();
        for i in 0..50u32 {
            let node_id = format!("node-{i}");
            let subnet = pick_subnet(&node_id, &empty(), DEFAULT_MESH_SPACE).unwrap();
            results.insert(subnet);
        }
        assert!(
            results.len() >= 40,
            "expected at least 40 distinct /24s, got {}",
            results.len()
        );
    }

    #[test]
    fn skips_claimed_subnets() {
        // Find the preferred /24 for "node-x" on an empty set.
        let preferred = pick_subnet("node-x", &empty(), DEFAULT_MESH_SPACE).unwrap();

        // Pre-claim it.
        let mut claimed = HashSet::new();
        claimed.insert(preferred);

        // pick_subnet must return something different.
        let next = pick_subnet("node-x", &claimed, DEFAULT_MESH_SPACE).unwrap();
        assert_ne!(next, preferred);
    }

    #[test]
    fn three_routers_pick_non_overlapping() {
        let mut claimed = HashSet::new();

        let r1 = pick_subnet("router-1", &claimed, DEFAULT_MESH_SPACE).unwrap();
        claimed.insert(r1);

        let r2 = pick_subnet("router-2", &claimed, DEFAULT_MESH_SPACE).unwrap();
        claimed.insert(r2);

        let r3 = pick_subnet("router-3", &claimed, DEFAULT_MESH_SPACE).unwrap();

        assert_ne!(r1, r2);
        assert_ne!(r1, r3);
        assert_ne!(r2, r3);
    }

    #[test]
    fn exhaustion_returns_none() {
        let mut claimed = HashSet::new();
        let base_octets = DEFAULT_MESH_SPACE.network().octets();
        for idx in 0u8..=255 {
            let net =
                Ipv4Net::new(Ipv4Addr::new(base_octets[0], base_octets[1], idx, 0), 24).unwrap();
            claimed.insert(net);
        }
        assert_eq!(pick_subnet("any-node", &claimed, DEFAULT_MESH_SPACE), None);
    }
}
