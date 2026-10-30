// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

//! Read the local guild log back. The log is the source of truth; `guilds.json`
//! only supplies display names.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use dcbor::prelude::*;
use identikey_log::{sign, Author};
use serde_json::{json, Value};

use crate::net::encode_base58;

#[derive(Clone, Debug)]
struct Member {
    admin: bool,
    actor: String,
}

#[derive(Clone, Debug)]
struct Space {
    id: String,
    profile: String,
    outer: Option<String>,
    roster: Vec<Member>,
    nodes: Vec<String>,
    epoch: u64,
}

/// Fold `guild-ops.log` and overlay names from `guilds.json`.
pub fn inspect(config: &Path) -> Result<Value, String> {
    let log_path = config.join("guild-ops.log");
    let text = if log_path.exists() {
        fs::read_to_string(&log_path).map_err(|e| e.to_string())?
    } else {
        String::new()
    };
    let mut spaces: BTreeMap<String, Space> = BTreeMap::new();
    let mut op_rows = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let bytes = decode_hex(line)?;
        let (signed, hash) = sign::decode_and_verify(&bytes).map_err(|e| format!("op {}: {e}", n + 1))?;
        let id = hex32(&hash);
        let op = &signed.op;
        apply_op(&mut spaces, &id, op).map_err(|e| format!("op {} ({}): {e}", n + 1, op.kind))?;
        let body = op_map(op)?;
        let about = match op.kind.as_str() {
            "keyspace.join" => text_or_bytes(&body, "keyspace")
                .filter(|keyspace| !keyspace.is_empty())
                .unwrap_or_else(|| id.clone()),
            "keyspace.nest" => text_or_bytes(&body, "outer").unwrap_or_default(),
            _ => text_or_bytes(&body, "keyspace").unwrap_or_default(),
        };
        op_rows.push(json!({
            "n": n + 1,
            "kind": op.kind,
            "id": encode_base58(&hash),
            "hex": id,
            "actor": encode_base58(&op.actor),
            "hlc": [op.hlc.l, op.hlc.c],
            "about": about,
        }));
    }

    let you = actor_hex(config).ok();
    let names = read_names(config);
    let mut guilds = Vec::new();
    for space in spaces.values() {
        let meta = names.get(&space.id);
        let you_role = match &you {
            Some(actor) if space.roster.iter().any(|m| m.admin && m.actor == *actor) => "admin",
            Some(actor) if space.roster.iter().any(|m| m.actor == *actor) => "member",
            Some(_) => "absent",
            None => "no-seed",
        };
        let mut nodes = Vec::new();
        for node in &space.nodes {
            let named = meta.and_then(|m| m.nodes.iter().find(|n| n.node_id == *node));
            let access_space = named.and_then(|n| n.access.as_ref()).and_then(|id| spaces.get(id));
            nodes.push(json!({
                "name": named.map(|n| n.name.clone()),
                "nodeId": node,
                "accessId": access_space.map(|s| encode_base58(&parse_hex32(&s.id).unwrap_or([0; 32]))),
                "accessNodeKeyspaceId": access_space.map(|s| s.id.clone()),
            }));
        }
        guilds.push(json!({
            "name": meta.map(|m| m.name.clone()),
            "workplace": meta.map(|m| m.workplace).unwrap_or(false),
            "profile": space.profile,
            "id": encode_base58(&parse_hex32(&space.id)?),
            "keyspaceId": space.id,
            "outer": space.outer,
            "you": you_role,
            "epoch": space.epoch,
            "roster": space.roster.iter().map(|m| json!({
                "admin": m.admin,
                "actor": encode_base58(&parse_hex32(&m.actor).unwrap_or([0; 32])),
                "actorHex": m.actor,
            })).collect::<Vec<_>>(),
            "nodes": nodes,
        }));
    }

    let actor = you.map(|hex| {
        json!({
            "hex": hex,
            "id": encode_base58(&parse_hex32(&hex).unwrap_or([0; 32])),
        })
    });

    Ok(json!({
        "verified": true,
        "held": "local log",
        "actor": actor,
        "guilds": guilds,
        "ops": op_rows,
        "note": "A guild id is the base58 form of the keyspace content hash. It is not an XID.",
    }))
}

/// Resolve a display name, base58 id, or hex prefix to one keyspace hex id.
pub fn resolve_guild(config: &Path, query: &str) -> Result<String, String> {
    let view = inspect(config)?;
    let q = query.trim();
    let guilds = view["guilds"].as_array().ok_or("no guilds")?;
    let mut hits: Vec<&Value> = guilds
        .iter()
        .filter(|g| {
            let name = g["name"].as_str().unwrap_or("");
            let id = g["id"].as_str().unwrap_or("");
            let hex = g["keyspaceId"].as_str().unwrap_or("");
            name.eq_ignore_ascii_case(q)
                || id == q
                || hex == q
                || (!q.is_empty() && (id.starts_with(q) || hex.starts_with(q) || name.to_ascii_lowercase().contains(&q.to_ascii_lowercase())))
        })
        .collect();
    // Prefer an exact name or exact id over a substring when both match.
    let exact: Vec<&Value> = hits
        .iter()
        .copied()
        .filter(|g| {
            g["name"].as_str().unwrap_or("").eq_ignore_ascii_case(q)
                || g["id"].as_str() == Some(q)
                || g["keyspaceId"].as_str() == Some(q)
        })
        .collect();
    if !exact.is_empty() {
        hits = exact;
    }
    match hits.len() {
        1 => Ok(hits[0]["keyspaceId"].as_str().unwrap().to_string()),
        0 => Err(format!("no guild matches {q}")),
        _ => Err(format!(
            "guild query {q} matches {}",
            hits.iter()
                .map(|g| g["name"].as_str().unwrap_or(g["id"].as_str().unwrap_or("?")))
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

struct Name {
    name: String,
    workplace: bool,
    nodes: Vec<NamedNode>,
}

struct NamedNode {
    name: String,
    node_id: String,
    access: Option<String>,
}

fn read_names(config: &Path) -> BTreeMap<String, Name> {
    let path = config.join("guilds.json");
    let Ok(text) = fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return BTreeMap::new();
    };
    let mut out = BTreeMap::new();
    let Some(guilds) = value.get("guilds").and_then(Value::as_array) else {
        return out;
    };
    for g in guilds {
        let Some(id) = g.get("keyspaceId").and_then(Value::as_str) else {
            continue;
        };
        let nodes = g
            .get("nodes")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|n| {
                        Some(NamedNode {
                            name: n.get("name")?.as_str()?.to_string(),
                            node_id: n.get("nodeId")?.as_str()?.to_string(),
                            access: n
                                .get("accessNodeKeyspaceId")
                                .and_then(Value::as_str)
                                .map(str::to_string),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        out.insert(
            id.to_string(),
            Name {
                name: g.get("name").and_then(Value::as_str).unwrap_or("").to_string(),
                workplace: g.get("workplace").and_then(Value::as_bool).unwrap_or(false),
                nodes,
            },
        );
    }
    out
}

fn actor_hex(config: &Path) -> Result<String, String> {
    let text = fs::read_to_string(config.join("actor-seed")).map_err(|e| e.to_string())?;
    let seed = parse_hex32(text.trim())?;
    Ok(hex32(&Author::from_seed(&seed).actor()))
}

fn apply_op(spaces: &mut BTreeMap<String, Space>, hash: &str, op: &identikey_log::Op) -> Result<(), String> {
    let body = op_map(op)?;
    match op.kind.as_str() {
        "keyspace.join" => {
            let id = text_or_bytes(&body, "keyspace").unwrap_or_else(|| hash.to_string());
            let profile = text_field(&body, "profile").unwrap_or_else(|| "house".into());
            let epoch = uint_field(&body, "epoch").unwrap_or(0);
            let roster = roster(&body)?;
            spaces.insert(
                id.clone(),
                Space {
                    id,
                    profile,
                    outer: None,
                    roster,
                    nodes: Vec::new(),
                    epoch,
                },
            );
        }
        "keyspace.assign-node" => {
            let id = text_or_bytes(&body, "keyspace").ok_or("assign-node missing keyspace")?;
            let node = text_or_bytes(&body, "node").ok_or("assign-node missing node")?;
            let space = spaces.get_mut(&id).ok_or("assign-node cites an unknown guild")?;
            if !space.nodes.iter().any(|n| n == &node) {
                space.nodes.push(node);
            }
        }
        "keyspace.nest" => {
            let outer = text_or_bytes(&body, "outer").ok_or("nest missing outer")?;
            if !spaces.contains_key(&outer) {
                return Err("nest cites an unknown outer guild".into());
            }
            let profile = text_field(&body, "profile").unwrap_or_else(|| "access-node".into());
            let roster = roster(&body)?;
            spaces.insert(
                hash.to_string(),
                Space {
                    id: hash.to_string(),
                    profile,
                    outer: Some(outer),
                    roster,
                    nodes: Vec::new(),
                    epoch: uint_field(&body, "epoch").unwrap_or(0),
                },
            );
        }
        "keyspace.change" => {
            let id = text_or_bytes(&body, "keyspace").ok_or("change missing keyspace")?;
            let space = spaces.get_mut(&id).ok_or("change cites an unknown guild")?;
            if map_has(&body, "roster") {
                space.roster = roster(&body)?;
            }
            if let Some(profile) = text_field(&body, "profile") {
                space.profile = profile;
            }
            if let Some(epoch) = uint_field(&body, "epoch") {
                space.epoch = epoch;
            }
        }
        other => return Err(format!("unknown kind {other}")),
    }
    Ok(())
}

fn op_map(op: &identikey_log::Op) -> Result<Map, String> {
    let bytes = op.body.as_ref().ok_or("op has no body")?;
    let cbor = CBOR::try_from_data(bytes).map_err(|e| e.to_string())?;
    match cbor.into_case() {
        CBORCase::Map(map) => Ok(map),
        _ => Err("op body is not a map".into()),
    }
}

fn roster(map: &Map) -> Result<Vec<Member>, String> {
    let Some(v) = map.get::<&str, CBOR>("roster") else {
        return Ok(Vec::new());
    };
    let CBORCase::Array(items) = v.into_case() else {
        return Err("roster is not an array".into());
    };
    let mut out = Vec::new();
    for item in items {
        let CBORCase::Map(m) = item.into_case() else {
            return Err("roster entry is not a map".into());
        };
        let admin = m
            .get::<&str, CBOR>("admin")
            .and_then(|v| v.try_into_bool().ok())
            .unwrap_or(false);
        let actor = text_or_bytes(&m, "actor").unwrap_or_default();
        out.push(Member { admin, actor });
    }
    Ok(out)
}

fn text_or_bytes(map: &Map, key: &str) -> Option<String> {
    let v = map.get::<&str, CBOR>(key)?;
    match v.into_case() {
        CBORCase::ByteString(b) if b.len() == 32 => {
            let v = b.to_vec();
            let arr: [u8; 32] = v.try_into().ok()?;
            Some(hex32(&arr))
        }
        CBORCase::Text(s) => Some(s),
        _ => None,
    }
}

fn map_has(map: &Map, key: &str) -> bool {
    map.get::<&str, CBOR>(key).is_some()
}

fn text_field(map: &Map, key: &str) -> Option<String> {
    let v = map.get::<&str, CBOR>(key)?;
    match v.into_case() {
        CBORCase::Text(s) => Some(s),
        _ => None,
    }
}

fn uint_field(map: &Map, key: &str) -> Option<u64> {
    let v = map.get::<&str, CBOR>(key)?;
    match v.into_case() {
        CBORCase::Unsigned(n) => Some(n),
        _ => None,
    }
}

fn decode_hex(s: &str) -> Result<Vec<u8>, String> {
    if s.len() % 2 != 0 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("guild log line is not hex".into());
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    for i in (0..s.len()).step_by(2) {
        out.push(u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string())?);
    }
    Ok(out)
}

fn parse_hex32(raw: &str) -> Result<[u8; 32], String> {
    if raw.len() != 64 {
        return Err(format!("expected 32 bytes, got {} hex chars", raw.len()));
    }
    let bytes = decode_hex(raw)?;
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn hex32(bytes: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}

/// One guild plus the access-node guilds nested under it.
pub fn project(view: &Value, keyspace_hex: &str) -> Result<Value, String> {
    let guilds = view["guilds"].as_array().ok_or("no guilds")?;
    let guild = guilds
        .iter()
        .find(|g| g["keyspaceId"].as_str() == Some(keyspace_hex))
        .cloned()
        .ok_or("guild disappeared during resolve")?;
    let nested: Vec<Value> = guilds
        .iter()
        .filter(|g| g["outer"].as_str() == Some(keyspace_hex))
        .cloned()
        .collect();
    let mut out = view.clone();
    out["guild"] = guild;
    out["nested"] = json!(nested);
    out.as_object_mut().map(|o| {
        o.remove("guilds");
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guild_ceremony::{run_ceremony, CeremonyMode, CeremonyRequest};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn show_reads_the_log_and_names_the_workplace() {
        let dir = std::env::temp_dir().join(format!(
            "lm-guild-view-{}",
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
        ));
        run_ceremony(
            &dir,
            &CeremonyRequest {
                mode: CeremonyMode::Genesis,
                workplace_name: "Strawberry Creek".into(),
                workplace_id: String::new(),
                node_name: "ap3000-outdoor-b".into(),
                node_id: "a7a8841b13de5fbf20daec51318bad40dffc4425853edda6e9defc59af374037".into(),
            },
        )
        .unwrap();
        let view = inspect(&dir).unwrap();
        assert_eq!(view["verified"], true);
        let creek = view["guilds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|g| g["name"] == "Strawberry Creek")
            .unwrap();
        assert_eq!(creek["you"], "admin");
        assert_eq!(creek["profile"], "house");
        assert_eq!(creek["id"].as_str().unwrap().len() > 20, true);
        assert_eq!(
            creek["nodes"][0]["nodeId"],
            "a7a8841b13de5fbf20daec51318bad40dffc4425853edda6e9defc59af374037"
        );
        assert!(creek["nodes"][0]["accessNodeKeyspaceId"].as_str().unwrap().len() == 64);
        let creek_hex = creek["keyspaceId"].as_str().unwrap();
        let ops = view["ops"].as_array().unwrap();
        assert!(ops.iter().any(|op| {
            op["kind"] == "keyspace.assign-node" && op["about"] == creek_hex
        }));
        assert!(ops.iter().any(|op| op["kind"] == "keyspace.nest" && op["about"] == creek_hex));
        let join = ops.iter().find(|op| op["n"] == 1).unwrap();
        assert_eq!(join["about"], join["hex"]);
        let resolved = resolve_guild(&dir, "straw").unwrap();
        assert_eq!(resolved, creek["keyspaceId"].as_str().unwrap());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
