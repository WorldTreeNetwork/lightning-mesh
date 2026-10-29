// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

//! Runs `admin/scripts/guild/ceremony.rhai` against [`crate::guild::sign_guild_at`].
//!
//! The script is the order. This module holds the seed, the log, and the
//! local name index. The seed never enters the script scope.

use std::fs::{self, OpenOptions};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use rhai::{Dynamic, Engine, EvalAltResult, Map, Scope};
use serde_json::{json, Value};

use crate::guild::{sign_guild_at, GuildSignInput};

const SCRIPT: &str = include_str!("../../scripts/guild/ceremony.rhai");
const NOTE: &str =
    "Display names are local. identikey-keyspace-v1 has no name field and no rename.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CeremonyMode {
    Genesis,
    Assign,
}

pub struct CeremonyRequest {
    pub mode: CeremonyMode,
    pub workplace_name: String,
    pub workplace_id: String,
    pub node_name: String,
    pub node_id: String,
}

struct Ctx {
    config: PathBuf,
    seed_hex: String,
    now_ms: u64,
}

pub fn config_dir() -> Result<PathBuf, String> {
    if let Some(dir) = std::env::var_os("LIGHTNING_GUILD_CONFIG") {
        return Ok(PathBuf::from(dir));
    }
    let home = std::env::var("HOME").map_err(|_| "HOME is unset".to_string())?;
    Ok(PathBuf::from(home).join(".config").join("lightning-admin"))
}

pub fn run_ceremony(config: &Path, req: &CeremonyRequest) -> Result<Value, String> {
    validate(req)?;
    fs::create_dir_all(config).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(config)
            .map_err(|e| e.to_string())?
            .permissions();
        perms.set_mode(0o700);
        fs::set_permissions(config, perms).map_err(|e| e.to_string())?;
    }
    let seed_hex = load_or_create_seed(&config.join("actor-seed"))?;
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis() as u64;
    let ctx = Arc::new(Mutex::new(Ctx {
        config: config.to_path_buf(),
        seed_hex,
        now_ms,
    }));

    let mut engine = Engine::new();
    engine.set_max_operations(10_000);

    let lines_ctx = ctx.clone();
    engine.register_fn("log_lines", move || log_lines(&lines_ctx));

    let sign_ctx = ctx.clone();
    engine.register_fn(
        "sign_op",
        move |verb: String, guild_id: String, node_id: String, counter: i64| {
            sign_op(&sign_ctx, verb, guild_id, node_id, counter)
        },
    );

    let save_ctx = ctx.clone();
    engine.register_fn(
        "save_genesis",
        move |admin_id: String,
              work_id: String,
              access_id: String,
              actor: String,
              workplace_name: String,
              node_name: String,
              node_id: String| {
            save_genesis(
                &save_ctx,
                admin_id,
                work_id,
                access_id,
                actor,
                workplace_name,
                node_name,
                node_id,
            )
        },
    );

    let assign_ctx = ctx.clone();
    engine.register_fn(
        "save_assign",
        move |workplace_id: String,
              access_id: String,
              actor: String,
              node_name: String,
              node_id: String| {
            save_assign(
                &assign_ctx,
                workplace_id,
                access_id,
                actor,
                node_name,
                node_id,
            )
        },
    );

    let mut scope = Scope::new();
    let mode = match req.mode {
        CeremonyMode::Genesis => "genesis",
        CeremonyMode::Assign => "assign",
    };
    scope.push("mode", mode.to_string());
    scope.push("workplace_name", req.workplace_name.clone());
    scope.push("workplace_id", req.workplace_id.clone());
    scope.push("node_name", req.node_name.clone());
    scope.push("node_id", req.node_id.clone());

    let returned = engine
        .eval_with_scope::<Map>(&mut scope, SCRIPT)
        .map_err(|e| e.to_string())?;
    let mut value = map_to_json(&returned)?;
    if let Some(obj) = value.as_object_mut() {
        obj.insert(
            "log".into(),
            json!(config.join("guild-ops.log").display().to_string()),
        );
        obj.insert(
            "index".into(),
            json!(config.join("guilds.json").display().to_string()),
        );
        obj.insert(
            "seed".into(),
            json!(config.join("actor-seed").display().to_string()),
        );
    }
    Ok(value)
}

fn validate(req: &CeremonyRequest) -> Result<(), String> {
    parse_owned_hex32(&req.node_id, "node id")?;
    match req.mode {
        CeremonyMode::Genesis if req.workplace_name.trim().is_empty() => {
            Err("genesis needs a workplace name".into())
        }
        CeremonyMode::Assign => parse_owned_hex32(&req.workplace_id, "workplace id").map(|_| ()),
        CeremonyMode::Genesis => Ok(()),
    }
}

fn sign_op(
    ctx: &Arc<Mutex<Ctx>>,
    verb: String,
    guild_id: String,
    node_id: String,
    counter: i64,
) -> Result<Map, Box<EvalAltResult>> {
    if counter < 0 {
        return Err("HLC counter must be >= 0".into());
    }
    let ctx = ctx.lock().map_err(|_| "guild ceremony lock poisoned")?;
    let input = GuildSignInput {
        verb,
        actor_seed_hex: ctx.seed_hex.clone(),
        guild_id_hex: Some(guild_id),
        node_id_hex: Some(node_id),
    };
    let receipt = sign_guild_at(
        &input,
        ctx.now_ms,
        counter as u64,
        &ctx.config.join("guild-ops.log"),
    )
    .map_err(|e| e.to_string())?;
    let mut map = Map::new();
    map.insert("kind".into(), Dynamic::from(receipt.kind));
    map.insert("keyspace_id".into(), Dynamic::from(receipt.keyspace_id));
    map.insert("content_hash".into(), Dynamic::from(receipt.content_hash));
    map.insert("actor".into(), Dynamic::from(receipt.actor));
    Ok(map)
}

fn log_lines(ctx: &Arc<Mutex<Ctx>>) -> i64 {
    let Ok(ctx) = ctx.lock() else {
        return 0;
    };
    let path = ctx.config.join("guild-ops.log");
    let Ok(text) = fs::read_to_string(path) else {
        return 0;
    };
    text.lines().filter(|l| !l.trim().is_empty()).count() as i64
}

fn save_genesis(
    ctx: &Arc<Mutex<Ctx>>,
    admin_id: String,
    work_id: String,
    access_id: String,
    actor: String,
    workplace_name: String,
    node_name: String,
    node_id: String,
) -> Result<(), Box<EvalAltResult>> {
    let ctx = ctx.lock().map_err(|_| "guild ceremony lock poisoned")?;
    let index = json!({
        "actor": actor,
        "note": NOTE,
        "guilds": [
            {
                "name": "Admin",
                "profile": "house",
                "keyspaceId": admin_id,
                "admin": true
            },
            {
                "name": workplace_name,
                "workplace": true,
                "profile": "house",
                "keyspaceId": work_id,
                "admin": true,
                "nodes": [{
                    "name": node_name,
                    "nodeId": node_id,
                    "accessNodeKeyspaceId": access_id
                }]
            }
        ]
    });
    write_index(&ctx.config.join("guilds.json"), &index).map_err(|e| e.to_string())?;
    Ok(())
}

fn save_assign(
    ctx: &Arc<Mutex<Ctx>>,
    workplace_id: String,
    access_id: String,
    actor: String,
    node_name: String,
    node_id: String,
) -> Result<(), Box<EvalAltResult>> {
    let ctx = ctx.lock().map_err(|_| "guild ceremony lock poisoned")?;
    let path = ctx.config.join("guilds.json");
    let mut index: Value = fs::read_to_string(&path)
        .and_then(|text| serde_json::from_str(&text).map_err(|e| std::io::Error::other(e)))
        .map_err(|e| format!("read guild index: {e}"))?;
    let guilds = index
        .get_mut("guilds")
        .and_then(Value::as_array_mut)
        .ok_or("guild index has no guilds array")?;
    let guild = guilds
        .iter_mut()
        .find(|g| g.get("keyspaceId").and_then(Value::as_str) == Some(workplace_id.as_str()))
        .ok_or("workplace id is not in the guild index")?;
    let nodes = guild
        .as_object_mut()
        .and_then(|o| o.get_mut("nodes"))
        .and_then(Value::as_array_mut)
        .ok_or("workplace guild has no nodes array")?;
    nodes.push(json!({
        "name": node_name,
        "nodeId": node_id,
        "accessNodeKeyspaceId": access_id
    }));
    if let Some(obj) = index.as_object_mut() {
        obj.insert("actor".into(), json!(actor));
    }
    write_index(&path, &index).map_err(|e| e.to_string())?;
    Ok(())
}

fn write_index(path: &Path, value: &Value) -> Result<(), String> {
    let mut opts = OpenOptions::new();
    opts.create(true).write(true).truncate(true);
    #[cfg(unix)]
    opts.mode(0o600);
    let mut file = opts.open(path).map_err(|e| e.to_string())?;
    writeln!(file, "{value}").map_err(|e| e.to_string())?;
    Ok(())
}

fn load_or_create_seed(path: &Path) -> Result<String, String> {
    if path.exists() {
        let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
        parse_owned_hex32(text.trim(), "actor seed")?;
        return Ok(text.trim().to_string());
    }
    let mut bytes = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut bytes);
    let hex = hex_encode(&bytes);
    let mut opts = OpenOptions::new();
    opts.create_new(true).write(true);
    #[cfg(unix)]
    opts.mode(0o600);
    let mut file = opts.open(path).map_err(|e| e.to_string())?;
    writeln!(file, "{hex}").map_err(|e| e.to_string())?;
    Ok(hex)
}

fn parse_owned_hex32(raw: &str, label: &str) -> Result<[u8; 32], String> {
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

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}

fn map_to_json(map: &Map) -> Result<Value, String> {
    let mut obj = serde_json::Map::new();
    for (k, v) in map {
        let text = v.clone().into_string().map_err(|e| e.to_string())?;
        obj.insert(k.to_string(), Value::String(text));
    }
    Ok(Value::Object(obj))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(mode: CeremonyMode, workplace_id: &str) -> CeremonyRequest {
        CeremonyRequest {
            mode,
            workplace_name: "Strawberry Creek".into(),
            workplace_id: workplace_id.into(),
            node_name: "ap3000-outdoor-b".into(),
            node_id: "a7a8841b13de5fbf20daec51318bad40dffc4425853edda6e9defc59af374037".into(),
        }
    }

    #[test]
    fn genesis_then_assign_keeps_the_seed_out_of_the_log() {
        let dir = std::env::temp_dir().join(format!(
            "lm-guild-ceremony-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let first = run_ceremony(&dir, &req(CeremonyMode::Genesis, "")).unwrap();
        let log = fs::read_to_string(dir.join("guild-ops.log")).unwrap();
        let index = fs::read_to_string(dir.join("guilds.json")).unwrap();
        let seed = fs::read_to_string(dir.join("actor-seed")).unwrap();
        assert_eq!(log.lines().filter(|l| !l.is_empty()).count(), 4);
        assert!(index.contains("Strawberry Creek"));
        assert!(index.contains("Admin"));
        assert!(!log.contains(seed.trim()));
        assert!(!index.contains(seed.trim()));
        assert_eq!(first["admin_guild"].as_str().unwrap().len(), 64);
        assert_eq!(first["workplace_guild"].as_str().unwrap().len(), 64);
        assert_eq!(first["access_node_guild"].as_str().unwrap().len(), 64);

        let again = run_ceremony(&dir, &req(CeremonyMode::Genesis, ""));
        assert!(again.unwrap_err().contains("already has ops"));

        let second = run_ceremony(
            &dir,
            &CeremonyRequest {
                mode: CeremonyMode::Assign,
                workplace_name: String::new(),
                workplace_id: first["workplace_guild"].as_str().unwrap().to_string(),
                node_name: "m3000-bench".into(),
                node_id: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
            },
        )
        .unwrap();
        let log = fs::read_to_string(dir.join("guild-ops.log")).unwrap();
        assert_eq!(log.lines().filter(|l| !l.is_empty()).count(), 6);
        assert!(second["access_node_guild"].as_str().unwrap().len() == 64);
        let index: Value =
            serde_json::from_str(&fs::read_to_string(dir.join("guilds.json")).unwrap()).unwrap();
        let nodes = index["guilds"][1]["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), 2);
        let _ = fs::remove_dir_all(&dir);
    }
}
