// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

//! Guild verbs. Signs identikey-log keyspace ops. Does not call
//! `apply_network_name` and does not stage `CLIENT_SSID`.

use crate::guild::{GuildReceipt, GuildSignInput, sign_guild_now};
use crate::guild_ceremony::{config_dir, run_ceremony, CeremonyMode, CeremonyRequest};
use serde_json::{json, Value};
use std::fs::{self, OpenOptions};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

#[tauri::command]
pub fn sign_guild_op(input: GuildSignInput) -> Result<GuildReceipt, String> {
    sign_guild_now(&input)
}

#[tauri::command]
pub fn guild_snapshot() -> Result<Value, String> {
    crate::guild_view::inspect(&config_dir()?)
}

fn validate_id(id: &str) -> Result<(), String> {
    if id.len() != 64 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("expected a 64-hex id".into());
    }
    Ok(())
}

#[tauri::command]
pub fn guild_rename(keyspace_id: String, name: String) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("guild name must not be empty".into());
    }
    validate_id(&keyspace_id)?;
    let config = config_dir()?;
    let path = config.join("guilds.json");
    let mut index: Value = match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).map_err(|e| e.to_string())?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => json!({"guilds": []}),
        Err(e) => return Err(e.to_string()),
    };
    let guilds = index.get_mut("guilds").and_then(Value::as_array_mut)
        .ok_or("guild index has no guilds array")?;
    if let Some(guild) = guilds
        .iter_mut()
        .find(|g| g["keyspaceId"].as_str() == Some(keyspace_id.as_str()))
    {
        guild["name"] = json!(name);
    } else {
        guilds.push(json!({"name": name, "keyspaceId": keyspace_id, "profile": "house"}));
    }
    write_guild_index(&path, &index)
}

/// Assign signs before it updates the index. A house with no `nodes` array,
/// or an access guild that only exists in the log, would sign and then fail
/// the save. Make the index row ready first.
fn prepare_assign_index(config: &std::path::Path, keyspace_id: &str) -> Result<(), String> {
    let path = config.join("guilds.json");
    let mut index: Value = match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).map_err(|e| e.to_string())?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => json!({"guilds": []}),
        Err(e) => return Err(e.to_string()),
    };
    let guilds = index
        .get_mut("guilds")
        .and_then(Value::as_array_mut)
        .ok_or("guild index has no guilds array")?;
    let mut dirty = false;
    if let Some(guild) = guilds.iter_mut().find(|g| g["keyspaceId"].as_str() == Some(keyspace_id)) {
        let obj = guild.as_object_mut().ok_or("guild entry is not an object")?;
        if obj.get("nodes").and_then(Value::as_array).is_none() {
            obj.insert("nodes".into(), json!([]));
            dirty = true;
        }
    } else {
        guilds.push(json!({
            "name": "access-node",
            "profile": "access-node",
            "keyspaceId": keyspace_id,
            "nodes": []
        }));
        dirty = true;
    }
    if dirty {
        write_guild_index(&path, &index)?;
    }
    Ok(())
}

fn write_guild_index(path: &std::path::Path, index: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut opts = OpenOptions::new();
    opts.create(true).write(true).truncate(true);
    #[cfg(unix)]
    opts.mode(0o600);
    let mut file = opts.open(path).map_err(|e| e.to_string())?;
    writeln!(file, "{index}").map_err(|e| e.to_string())
}

#[tauri::command]
pub fn guild_assign(keyspace_id: String, node_name: String, node_id: String) -> Result<Value, String> {
    let node_name = node_name.trim();
    if node_name.is_empty() {
        return Err("node name must not be empty".into());
    }
    validate_id(&keyspace_id)?;
    validate_id(&node_id)?;
    let config = config_dir()?;
    prepare_assign_index(&config, &keyspace_id)?;
    run_ceremony(&config, &CeremonyRequest {
        mode: CeremonyMode::Assign,
        workplace_name: String::new(),
        workplace_id: keyspace_id,
        node_name: node_name.to_string(),
        node_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assign_prep_adds_nodes_and_does_not_open_a_log() {
        let dir = std::env::temp_dir().join(format!(
            "lm-guild-prep-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let house = "11".repeat(32);
        fs::write(
            dir.join("guilds.json"),
            format!(r#"{{"guilds":[{{"name":"Admin","profile":"house","keyspaceId":"{house}"}}]}}"#),
        )
        .unwrap();
        prepare_assign_index(&dir, &house).unwrap();
        let index: Value = serde_json::from_str(&fs::read_to_string(dir.join("guilds.json")).unwrap()).unwrap();
        assert!(index["guilds"][0]["nodes"].as_array().unwrap().is_empty());
        assert_eq!(index["guilds"][0]["name"], "Admin");
        let access = "22".repeat(32);
        prepare_assign_index(&dir, &access).unwrap();
        let index: Value = serde_json::from_str(&fs::read_to_string(dir.join("guilds.json")).unwrap()).unwrap();
        assert_eq!(index["guilds"][1]["name"], "access-node");
        assert!(index["guilds"][1]["nodes"].as_array().unwrap().is_empty());
        assert!(!dir.join("guild-ops.log").exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
