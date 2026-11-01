// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

//! Pairwise admission of a foreign house `/48`.
//!
//! A direction counts only from a verified `keyspace.admit-prefix` whose
//! target matches body `keyspace`, whose actor is an `admin` in the
//! caller-supplied snapshot, and whose prefix is `house_ula_prefix` of the
//! peer guild. One direction admits nothing. This module installs no route.

use dcbor::prelude::*;
use identikey_log::{sign, SignedOp};
use mjolnir_mesh::tun::house_ula_prefix;

use crate::guild::KIND_ADMIT;

/// One trusted (guild, actor, role) row. The caller authenticates it.
/// The guild log is not this snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorityGrant {
    pub guild_id: [u8; 32],
    pub actor: [u8; 32],
    pub role: String,
}

/// One direction that already passed the predicate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmitDirection {
    pub keyspace: [u8; 32],
    pub peer: [u8; 32],
    pub prefix: [u8; 6],
}

/// Both directions of one pair, ordered by guild id.
///
/// `prefix_a` is the house `/48` of `guild_a`. `prefix_b` is the house `/48`
/// of `guild_b`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmittedPair {
    pub guild_a: [u8; 32],
    pub prefix_a: [u8; 6],
    pub guild_b: [u8; 32],
    pub prefix_b: [u8; 6],
}

/// Return the direction when this signed op counts against `authority`.
///
/// A bare or forged op does not count: the signature is checked here.
/// An empty snapshot fails closed. There is no expiry and no withdrawal.
pub fn direction_from_signed(
    signed: &SignedOp,
    authority: &[AuthorityGrant],
) -> Option<AdmitDirection> {
    sign::verify(signed).ok()?;
    if signed.op.kind != KIND_ADMIT {
        return None;
    }
    let target = signed.op.target_fp?;
    let (keyspace, peer, prefix) = admit_fields(&signed.op)?;
    if target != keyspace || peer == keyspace {
        return None;
    }
    if prefix != house_prefix(&peer) {
        return None;
    }
    let admin = authority.iter().any(|grant| {
        grant.guild_id == keyspace && grant.actor == signed.op.actor && grant.role == "admin"
    });
    if !admin {
        return None;
    }
    Some(AdmitDirection {
        keyspace,
        peer,
        prefix,
    })
}

/// Pairs whose both directions are present. Duplicate directions count once.
/// The result is ordered by the lower guild id. Nothing is written.
pub fn admitted_pairs(directions: &[AdmitDirection]) -> Vec<AdmittedPair> {
    let mut pairs = Vec::new();
    for (i, left) in directions.iter().enumerate() {
        for right in directions.iter().skip(i + 1) {
            let (low, high) = if left.keyspace < left.peer {
                (left, right)
            } else {
                (right, left)
            };
            if low.keyspace != high.peer || low.peer != high.keyspace || low.keyspace >= low.peer {
                continue;
            }
            let pair = AdmittedPair {
                guild_a: low.keyspace,
                prefix_a: high.prefix,
                guild_b: low.peer,
                prefix_b: low.prefix,
            };
            if !pairs.iter().any(|seen: &AdmittedPair| {
                seen.guild_a == pair.guild_a && seen.guild_b == pair.guild_b
            }) {
                pairs.push(pair);
            }
        }
    }
    pairs.sort_by(|a, b| a.guild_a.cmp(&b.guild_a).then(a.guild_b.cmp(&b.guild_b)));
    pairs
}

fn house_prefix(guild_id: &[u8; 32]) -> [u8; 6] {
    let prefix = house_ula_prefix(guild_id);
    let mut out = [0u8; 6];
    out.copy_from_slice(&prefix.octets[..6]);
    out
}

fn admit_fields(op: &identikey_log::Op) -> Option<([u8; 32], [u8; 32], [u8; 6])> {
    let bytes = op.body.as_ref()?;
    let cbor = CBOR::try_from_data(bytes).ok()?;
    let CBORCase::Map(map) = cbor.into_case() else {
        return None;
    };
    let keyspace = bytes_exact(&map, "keyspace", 32)?;
    let peer = bytes_exact(&map, "peer", 32)?;
    let prefix = bytes_exact(&map, "prefix", 6)?;
    Some((
        keyspace.try_into().ok()?,
        peer.try_into().ok()?,
        prefix.try_into().ok()?,
    ))
}

fn bytes_exact(map: &Map, key: &str, len: usize) -> Option<Vec<u8>> {
    let value = map.get::<&str, CBOR>(key)?;
    match value.into_case() {
        CBORCase::ByteString(bytes) if bytes.len() == len => Some(bytes.to_vec()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guild::{sign_guild, GuildSignInput};
    use identikey_log::{sign, Author, Hlc, Op};
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    const GUILD_A: [u8; 32] = [0x11; 32];
    const GUILD_B: [u8; 32] = [0x22; 32];
    const PREFIX_A: [u8; 6] = [0xfd, 0x81, 0x2b, 0x22, 0x8a, 0xca];
    const PREFIX_B: [u8; 6] = [0xfd, 0x8f, 0x78, 0xa9, 0x2c, 0xaa];
    const STOCK: [u8; 6] = [0xfd, 0x12, 0x34, 0x56, 0x78, 0x9a];

    fn hex(bytes: &[u8]) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut out = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            out.push(HEX[(b >> 4) as usize] as char);
            out.push(HEX[(b & 0xf) as usize] as char);
        }
        out
    }

    fn author(seed_byte: u8) -> Author {
        Author::from_seed(&[seed_byte; 32])
    }

    fn grant(guild: [u8; 32], actor: [u8; 32], role: &str) -> AuthorityGrant {
        AuthorityGrant {
            guild_id: guild,
            actor,
            role: role.into(),
        }
    }

    fn fixture_admins() -> Vec<AuthorityGrant> {
        vec![
            grant(GUILD_A, author(0xa1).actor(), "admin"),
            grant(GUILD_B, author(0xb2).actor(), "admin"),
        ]
    }

    fn sign_direction(
        seed_byte: u8,
        keyspace: [u8; 32],
        peer: [u8; 32],
        prefix: [u8; 6],
        target: Option<[u8; 32]>,
    ) -> SignedOp {
        let who = author(seed_byte);
        let mut body = Map::new();
        body.insert("keyspace", CBOR::to_byte_string(keyspace));
        body.insert("peer", CBOR::to_byte_string(peer));
        body.insert("prefix", CBOR::to_byte_string(prefix));
        let mut op = Op::new(KIND_ADMIT, who.actor(), Hlc::new(1, 0))
            .with_body(CBOR::from(body).to_cbor_data());
        if let Some(fp) = target {
            op = op.with_target_fp(fp);
        }
        who.sign(op).unwrap()
    }

    fn sign_short_prefix(seed_byte: u8, keyspace: [u8; 32], peer: [u8; 32]) -> SignedOp {
        let who = author(seed_byte);
        let mut body = Map::new();
        body.insert("keyspace", CBOR::to_byte_string(keyspace));
        body.insert("peer", CBOR::to_byte_string(peer));
        body.insert(
            "prefix",
            CBOR::to_byte_string([0xfd, 0x11, 0x22, 0x33, 0x44]),
        );
        let op = Op::new(KIND_ADMIT, who.actor(), Hlc::new(1, 0))
            .with_body(CBOR::from(body).to_cbor_data())
            .with_target_fp(keyspace);
        who.sign(op).unwrap()
    }

    fn both_directions() -> Vec<SignedOp> {
        vec![
            sign_direction(0xa1, GUILD_A, GUILD_B, PREFIX_B, Some(GUILD_A)),
            sign_direction(0xb2, GUILD_B, GUILD_A, PREFIX_A, Some(GUILD_B)),
        ]
    }

    fn dirs(ops: &[SignedOp], authority: &[AuthorityGrant]) -> Vec<AdmitDirection> {
        ops.iter()
            .filter_map(|op| direction_from_signed(op, authority))
            .collect()
    }

    #[test]
    fn both_fixture_directions_admit_the_pair_and_install_nothing() {
        let admins = fixture_admins();
        let ops = both_directions();
        let pairs = admitted_pairs(&dirs(&ops, &admins));
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].guild_a, GUILD_A);
        assert_eq!(pairs[0].prefix_a, PREFIX_A);
        assert_eq!(pairs[0].guild_b, GUILD_B);
        assert_eq!(pairs[0].prefix_b, PREFIX_B);
        let again = admitted_pairs(&[dirs(&ops, &admins), dirs(&ops, &admins)].concat());
        assert_eq!(again, pairs);
    }

    #[test]
    fn one_signature_admits_nothing() {
        let ops = [sign_direction(
            0xa1,
            GUILD_A,
            GUILD_B,
            PREFIX_B,
            Some(GUILD_A),
        )];
        assert!(admitted_pairs(&dirs(&ops, &fixture_admins())).is_empty());
    }

    #[test]
    fn unrelated_actor_signs_both_directions() {
        let ops = vec![
            sign_direction(0xee, GUILD_A, GUILD_B, PREFIX_B, Some(GUILD_A)),
            sign_direction(0xee, GUILD_B, GUILD_A, PREFIX_A, Some(GUILD_B)),
        ];
        assert!(dirs(&ops, &fixture_admins()).is_empty());
    }

    #[test]
    fn member_does_not_speak_for_the_house() {
        let op = sign_direction(0xa1, GUILD_A, GUILD_B, PREFIX_B, Some(GUILD_A));
        let member = [grant(GUILD_A, author(0xa1).actor(), "member")];
        assert!(direction_from_signed(&op, &member).is_none());
    }

    #[test]
    fn missing_snapshot_fails_closed() {
        assert!(dirs(&both_directions(), &[]).is_empty());
    }

    #[test]
    fn mismatched_or_missing_target_does_not_count() {
        let admins = fixture_admins();
        let missing = sign_direction(0xa1, GUILD_A, GUILD_B, PREFIX_B, None);
        let wrong = sign_direction(0xa1, GUILD_A, GUILD_B, PREFIX_B, Some(GUILD_B));
        assert!(direction_from_signed(&missing, &admins).is_none());
        assert!(direction_from_signed(&wrong, &admins).is_none());
    }

    #[test]
    fn stock_prefix_and_short_prefix_do_not_count() {
        let admins = fixture_admins();
        let stock_a = sign_direction(0xa1, GUILD_A, GUILD_B, STOCK, Some(GUILD_A));
        let stock_b = sign_direction(0xb2, GUILD_B, GUILD_A, STOCK, Some(GUILD_B));
        assert!(direction_from_signed(&stock_a, &admins).is_none());
        assert!(direction_from_signed(&stock_b, &admins).is_none());
        let short = sign_short_prefix(0xa1, GUILD_A, GUILD_B);
        assert!(direction_from_signed(&short, &admins).is_none());
    }

    #[test]
    fn the_same_guild_is_not_foreign() {
        let op = sign_direction(0xa1, GUILD_A, GUILD_A, PREFIX_A, Some(GUILD_A));
        assert!(direction_from_signed(&op, &fixture_admins()).is_none());
    }

    #[test]
    fn an_unsigned_op_does_not_count() {
        let who = author(0xa1);
        let mut body = Map::new();
        body.insert("keyspace", CBOR::to_byte_string(GUILD_A));
        body.insert("peer", CBOR::to_byte_string(GUILD_B));
        body.insert("prefix", CBOR::to_byte_string(PREFIX_B));
        let op = Op::new(KIND_ADMIT, who.actor(), Hlc::new(1, 0))
            .with_body(CBOR::from(body).to_cbor_data())
            .with_target_fp(GUILD_A);
        let forged = SignedOp::new(op, Vec::new());
        assert!(direction_from_signed(&forged, &fixture_admins()).is_none());
    }

    #[test]
    fn sign_guild_derives_the_prefix_and_does_not_stage_a_ssid() {
        let dir = std::env::temp_dir().join(format!(
            "lm-admit-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("guild-ops.log");
        let receipt = sign_admit(&path, 0xa1, &hex(&GUILD_A), &hex(&GUILD_B), None);
        assert_eq!(receipt.kind, KIND_ADMIT);
        assert_eq!(receipt.keyspace_id, hex(&GUILD_A));
        let wrong = sign_admit(
            &path,
            0xa1,
            &hex(&GUILD_A),
            &hex(&GUILD_B),
            Some(&hex(&STOCK)),
        );
        let raw = fs::read_to_string(&path).unwrap();
        assert!(!raw.contains("ssid"));
        assert!(!raw.contains("10.42"));
        let signed: Vec<_> = raw
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| {
                let bytes = decode_hex(line);
                sign::decode_and_verify(&bytes).unwrap().0
            })
            .collect();
        let derived = direction_from_signed(&signed[0], &fixture_admins()).unwrap();
        assert_eq!(derived.prefix, PREFIX_B);
        assert!(direction_from_signed(&signed[1], &fixture_admins()).is_none());
        let _ = (wrong, fs::remove_dir_all(&dir));
    }

    fn sign_admit(
        path: &Path,
        seed_byte: u8,
        guild: &str,
        peer: &str,
        prefix: Option<&str>,
    ) -> crate::guild::GuildReceipt {
        sign_guild(
            &GuildSignInput {
                verb: "admit-prefix".into(),
                actor_seed_hex: hex(&[seed_byte; 32]),
                guild_id_hex: Some(guild.into()),
                node_id_hex: None,
                peer_id_hex: Some(peer.into()),
                prefix_hex: prefix.map(str::to_string),
            },
            1_700_000_000_000,
            path,
        )
        .unwrap()
    }

    fn decode_hex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }
}
