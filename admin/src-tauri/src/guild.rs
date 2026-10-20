// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

//! Lightning Admin guild verbs.
//!
//! Product copy says guild. The signed identikey-log kinds are
//! `keyspace.join`, `keyspace.change`, `keyspace.assign-node`, and
//! `keyspace.nest` (identikey-keyspace-v1). There is no rename kind.
//!
//! These ops are membership records. They do not wrap a guild PTK (that
//! stays in identikey-keys) and they do not write the client SSID.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use dcbor::prelude::*;
use identikey_log::{Author, Hlc, Op, codec, sign};
use serde::{Deserialize, Serialize};

pub const KIND_JOIN: &str = "keyspace.join";
pub const KIND_CHANGE: &str = "keyspace.change";
pub const KIND_ASSIGN: &str = "keyspace.assign-node";
pub const KIND_NEST: &str = "keyspace.nest";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuildVerb {
    Join,
    Change,
    AssignNode,
    Nest,
}

impl GuildVerb {
    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw.trim() {
            "join" => Ok(Self::Join),
            "change" => Ok(Self::Change),
            "assign-node" => Ok(Self::AssignNode),
            "nest" => Ok(Self::Nest),
            "rename" | "guild.rename" => Err("a guild has no rename verb".into()),
            other if other.starts_with("guild.") => {
                Err(format!("protocol kind is keyspace.*, not {other}"))
            }
            other => Err(format!("unknown guild verb {other}")),
        }
    }

    pub fn kind(self) -> &'static str {
        match self {
            Self::Join => KIND_JOIN,
            Self::Change => KIND_CHANGE,
            Self::AssignNode => KIND_ASSIGN,
            Self::Nest => KIND_NEST,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuildReceipt {
    pub kind: String,
    pub keyspace_id: String,
    pub content_hash: String,
    pub actor: String,
    pub log_path: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuildSignInput {
    pub verb: String,
    pub actor_seed_hex: String,
    pub guild_id_hex: Option<String>,
    pub node_id_hex: Option<String>,
}

pub fn default_log_path() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME is unset; cannot place the guild log")?;
    Ok(PathBuf::from(home)
        .join(".config")
        .join("lightning-admin")
        .join("guild-ops.log"))
}

/// Sign one guild verb and append the signed op to `log_path`.
pub fn sign_guild(input: &GuildSignInput, now_ms: u64, log_path: &Path) -> Result<GuildReceipt, String> {
    let verb = GuildVerb::parse(&input.verb)?;
    let seed = parse_hex32(&input.actor_seed_hex, "actor seed")?;
    let author = Author::from_seed(&seed);
    let actor = author.actor();
    let guild_id = match input.guild_id_hex.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(hex) => Some(parse_hex32(hex, "guild id")?),
    };
    let node_id = match input.node_id_hex.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(hex) => Some(parse_hex32(hex, "node id")?),
    };

    let (body, target) = match verb {
        GuildVerb::Join => encode_join(actor, guild_id)?,
        GuildVerb::Change => {
            let id = guild_id.ok_or("change needs a guild id")?;
            (encode_change(actor, id)?, Some(id))
        }
        GuildVerb::AssignNode => {
            let id = guild_id.ok_or("assign node needs a guild id")?;
            let node = node_id.ok_or("assign node needs a 32-byte node id")?;
            (encode_assign(id, node)?, Some(id))
        }
        GuildVerb::Nest => {
            let outer = guild_id.ok_or("nest needs the outer guild id")?;
            (encode_nest(actor, outer)?, Some(outer))
        }
    };

    let mut op = Op::new(verb.kind(), actor, Hlc::new(now_ms, 0)).with_body(body);
    if let Some(fp) = target {
        op = op.with_target_fp(fp);
    }
    if op.kind.starts_with("guild.") || op.kind.contains("rename") {
        return Err("refusing a non-protocol guild kind".into());
    }

    let signed = author.sign(op).map_err(|e| e.to_string())?;
    sign::verify(&signed).map_err(|e| e.to_string())?;
    let hash = codec::content_hash(&signed.op).map_err(|e| e.to_string())?;
    let bytes = codec::encode_signed(&signed).map_err(|e| e.to_string())?;
    append_log(log_path, &bytes)?;

    let keyspace_id = match verb {
        GuildVerb::Join if guild_id.is_none() => hash,
        GuildVerb::Nest => hash,
        _ => guild_id.or(target).unwrap_or(hash),
    };

    Ok(GuildReceipt {
        kind: verb.kind().to_owned(),
        keyspace_id: hex32(&keyspace_id),
        content_hash: hex32(&hash),
        actor: hex32(&actor),
        log_path: log_path.display().to_string(),
    })
}

pub fn sign_guild_now(input: &GuildSignInput) -> Result<GuildReceipt, String> {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis() as u64;
    sign_guild(input, now_ms, &default_log_path()?)
}

fn encode_join(actor: [u8; 32], guild_id: Option<[u8; 32]>) -> Result<(Vec<u8>, Option<[u8; 32]>), String> {
    let admin = guild_id.is_none();
    let mut body = Map::new();
    body.insert("roster", vec![roster_entry(actor, admin)]);
    body.insert("wraps", Vec::<CBOR>::new());
    body.insert("epoch", 0u64);
    if admin {
        body.insert("profile", "house");
    }
    if let Some(id) = guild_id {
        body.insert("keyspace", CBOR::to_byte_string(id));
    }
    Ok((CBOR::from(body).to_cbor_data(), guild_id))
}

fn encode_change(actor: [u8; 32], id: [u8; 32]) -> Result<Vec<u8>, String> {
    let mut body = Map::new();
    body.insert("keyspace", CBOR::to_byte_string(id));
    body.insert("roster", vec![roster_entry(actor, true)]);
    Ok(CBOR::from(body).to_cbor_data())
}

fn encode_assign(id: [u8; 32], node: [u8; 32]) -> Result<Vec<u8>, String> {
    let mut body = Map::new();
    body.insert("keyspace", CBOR::to_byte_string(id));
    body.insert("node", CBOR::to_byte_string(node));
    Ok(CBOR::from(body).to_cbor_data())
}

fn encode_nest(actor: [u8; 32], outer: [u8; 32]) -> Result<Vec<u8>, String> {
    let mut body = Map::new();
    body.insert("outer", CBOR::to_byte_string(outer));
    body.insert("roster", vec![roster_entry(actor, true)]);
    body.insert("wraps", Vec::<CBOR>::new());
    body.insert("epoch", 0u64);
    body.insert("profile", "access-node");
    Ok(CBOR::from(body).to_cbor_data())
}

fn roster_entry(actor: [u8; 32], admin: bool) -> CBOR {
    let mut puk = Map::new();
    puk.insert("alg", "ed25519");
    puk.insert("key", CBOR::to_byte_string(actor));
    let mut entry = Map::new();
    entry.insert("admin", admin);
    entry.insert("actor", CBOR::to_byte_string(actor));
    entry.insert("puks", vec![CBOR::from(puk)]);
    CBOR::from(entry)
}

fn append_log(path: &Path, signed: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    writeln!(file, "{}", hex_bytes(signed)).map_err(|e| e.to_string())
}

fn parse_hex32(raw: &str, label: &str) -> Result<[u8; 32], String> {
    let s = raw.trim();
    if s.len() != 64 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("{label} must be 32 bytes as 64 hex characters"));
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).map_err(|e| e.to_string())?;
    }
    Ok(out)
}

fn hex32(bytes: &[u8; 32]) -> String {
    hex_bytes(bytes)
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEED: &str = "1111111111111111111111111111111111111111111111111111111111111111";
    const GUILD: &str = "2222222222222222222222222222222222222222222222222222222222222222";
    const NODE: &str = "3333333333333333333333333333333333333333333333333333333333333333";

    fn input(verb: &str, guild: bool, node: bool) -> GuildSignInput {
        GuildSignInput {
            verb: verb.into(),
            actor_seed_hex: SEED.into(),
            guild_id_hex: guild.then(|| GUILD.into()),
            node_id_hex: node.then(|| NODE.into()),
        }
    }

    fn sign_at(dir: &Path, verb: &str, guild: bool, node: bool) -> GuildReceipt {
        let path = dir.join("guild-ops.log");
        sign_guild(&input(verb, guild, node), 1_700_000_000_000, &path).unwrap()
    }

    #[test]
    fn four_verbs_use_keyspace_kinds() {
        let dir = std::env::temp_dir().join(format!(
            "lm-guild-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let join = sign_at(&dir, "join", false, false);
        let change = sign_at(&dir, "change", true, false);
        let assign = sign_at(&dir, "assign-node", true, true);
        let nest = sign_at(&dir, "nest", true, false);

        assert_eq!(join.kind, KIND_JOIN);
        assert_eq!(change.kind, KIND_CHANGE);
        assert_eq!(assign.kind, KIND_ASSIGN);
        assert_eq!(nest.kind, KIND_NEST);
        assert_eq!(change.keyspace_id, GUILD);
        assert_eq!(assign.keyspace_id, GUILD);
        assert_ne!(join.keyspace_id, GUILD);
        assert_ne!(nest.keyspace_id, GUILD);
        assert_eq!(join.keyspace_id, join.content_hash);
        assert_eq!(nest.keyspace_id, nest.content_hash);
        for receipt in [&join, &change, &assign, &nest] {
            assert!(receipt.kind.starts_with("keyspace."));
            assert!(!receipt.kind.starts_with("guild."));
            assert!(!receipt.kind.contains("rename"));
        }

        let log = fs::read_to_string(dir.join("guild-ops.log")).unwrap();
        let lines: Vec<_> = log.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), 4);
        for line in lines {
            let raw = decode_hex(line);
            let (signed, hash) = sign::decode_and_verify(&raw).unwrap();
            assert!(signed.op.kind.starts_with("keyspace."));
            assert!(!signed.op.body.as_deref().unwrap().windows(5).any(|w| w == b"guild"));
            assert_eq!(hex32(&hash).len(), 64);
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rename_and_guild_kinds_are_refused() {
        let path = std::env::temp_dir().join("lm-guild-refuse.log");
        let _ = fs::remove_file(&path);
        for verb in ["rename", "guild.join", "guild.change", "ssid"] {
            let err = sign_guild(&input(verb, true, true), 1, &path).unwrap_err();
            assert!(err.contains("rename") || err.contains("keyspace") || err.contains("unknown"));
        }
        assert!(!path.exists());
    }

    #[test]
    fn assign_requires_ids_and_does_not_touch_a_radio_string() {
        let path = std::env::temp_dir().join("lm-guild-assign-missing.log");
        let err = sign_guild(&input("assign-node", true, false), 1, &path).unwrap_err();
        assert!(err.contains("node id"));
        let err = sign_guild(&input("nest", false, false), 1, &path).unwrap_err();
        assert!(err.contains("outer guild"));
        let err = sign_guild(&input("change", false, false), 1, &path).unwrap_err();
        assert!(err.contains("guild id"));
    }

    fn decode_hex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }
}
