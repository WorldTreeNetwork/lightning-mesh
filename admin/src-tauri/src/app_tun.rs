// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

//! App `/128` for the selected house, and the thin call into the app TUN crate.
//!
//! The guild surface only computes the address. `open_app_tun` is called by a
//! test or a second process that already holds one `DatagramConn`.

use std::net::Ipv6Addr;
use mjolnir_app_tun::{app_ula, house_ula_prefix, parse_house_guild_id, DatagramConn};
pub use mjolnir_app_tun::AppTunOpen;
use crate::permit::AdmittedPair;

/// Address to show for the guild the existing surface has selected.
///
/// A workplace house has no outer guild. A nested access guild does.
/// Missing inputs yield nothing. This function opens no TUN and writes no route.
pub fn display_app_address(
    outer: Option<&str>,
    guild_hex: &str,
    actor_public_hex: Option<&str>,
) -> Option<String> {
    if outer.is_some() {
        return None;
    }
    let guild = parse_house_guild_id(guild_hex)?;
    let actor = parse_house_guild_id(actor_public_hex?)?;
    Some(app_ula(&guild, &actor).to_string())
}

/// Open the reusable session on exactly the caller's bound connection.
pub async fn open_app_tun<C: DatagramConn + Clone>(
    guild_id: [u8; 32],
    actor_public: [u8; 32],
    peer_node_id: [u8; 32],
    bound_node_id: [u8; 32],
    peer_addr: Ipv6Addr,
    conn: C,
) -> AppTunOpen {
    mjolnir_app_tun::open_app_tun(
        guild_id, actor_public, peer_node_id, bound_node_id, peer_addr, conn,
    ).await
}

/// Admission policy stays in Admin, pinned to the house that opened the session.
pub fn sync_admitted_routes(session: &mut AppTunOpen, pairs: &[AdmittedPair]) -> Result<(), ()> {
    if let Some(guild) = session.guild_id() {
        session.sync_admitted_routes(&prefixes_for_house(guild, pairs))?;
    }
    Ok(())
}

/// Foreign `/48`s for `local_guild`, never that house's own prefix.
pub fn prefixes_for_house(local_guild: [u8; 32], pairs: &[AdmittedPair]) -> Vec<[u8; 6]> {
    let own = house_prefix(&local_guild);
    foreign_prefixes(local_guild, pairs)
        .into_iter()
        .filter(|prefix| prefix != &own)
        .collect()
}

fn house_prefix(guild: &[u8; 32]) -> [u8; 6] {
    let net = house_ula_prefix(guild);
    let mut prefix = [0u8; 6];
    prefix.copy_from_slice(&net.octets[..6]);
    prefix
}

/// Foreign `/48`s of pairs that include `local_guild`.
pub fn foreign_prefixes(local_guild: [u8; 32], pairs: &[AdmittedPair]) -> Vec<[u8; 6]> {
    let mut out = Vec::new();
    for pair in pairs {
        let prefix = if pair.guild_a == local_guild {
            Some(pair.prefix_b)
        } else if pair.guild_b == local_guild {
            Some(pair.prefix_a)
        } else {
            None
        };
        if let Some(prefix) = prefix {
            if !out.contains(&prefix) {
                out.push(prefix);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture_addr(guild: u8, key: u8) -> Ipv6Addr {
        app_ula(&[guild; 32], &[key; 32])
    }

    #[test]
    fn fixture_app_address_matches_the_house_vector() {
        let shown = display_app_address(None, &"11".repeat(32), Some(&"ab".repeat(32))).unwrap();
        assert_eq!(shown, "fd81:2b22:8aca:1:bd5e:fa7a:5460:697");
        assert_eq!(fixture_addr(0x11, 0xab).to_string(), shown);
    }

    #[test]
    fn missing_actor_or_nested_guild_shows_nothing() {
        assert!(display_app_address(None, &"11".repeat(32), None).is_none());
        assert!(display_app_address(
            Some("22".repeat(32).as_str()),
            &"11".repeat(32),
            Some(&"ab".repeat(32))
        )
        .is_none());
    }

    fn house_prefix(bytes: [u8; 6]) -> [u8; 6] {
        bytes
    }

    #[test]
    fn a_presented_pair_routes_only_the_foreign_prefix() {
        let local = [0x11u8; 32];
        let peer = [0x22u8; 32];
        let own = house_prefix([0xfd, 0x81, 0x2b, 0x22, 0x8a, 0xca]);
        let foreign = house_prefix([0xfd, 0x8f, 0x78, 0xa9, 0x2c, 0xaa]);
        let stock = house_prefix([0xfd, 0x12, 0x34, 0x56, 0x78, 0x9a]);
        let pair = AdmittedPair {
            guild_a: local,
            prefix_a: own,
            guild_b: peer,
            prefix_b: foreign,
        };
        assert_eq!(prefixes_for_house(local, &[pair]), vec![foreign]);
        assert!(prefixes_for_house(local, &[]).is_empty());
        assert!(prefixes_for_house([0x33; 32], &[pair]).is_empty());
        let unrelated = AdmittedPair {
            guild_a: peer,
            prefix_a: foreign,
            guild_b: [0x33; 32],
            prefix_b: stock,
        };
        assert!(prefixes_for_house(local, &[unrelated]).is_empty());
        let claims_own = AdmittedPair { prefix_b: own, ..pair };
        assert!(prefixes_for_house(local, &[claims_own]).is_empty());
    }

    #[test]
    fn workplace_house_shows_its_own_address() {
        let shown = display_app_address(None, &"22".repeat(32), Some(&"ab".repeat(32))).unwrap();
        assert_eq!(shown, fixture_addr(0x22, 0xab).to_string());
        assert_ne!(shown, "fd81:2b22:8aca:1:bd5e:fa7a:5460:697");
    }

}
