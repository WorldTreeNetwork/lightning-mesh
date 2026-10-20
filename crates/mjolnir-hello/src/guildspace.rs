// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

//! Access-node wrap-on-login request.
//!
//! An identikey login may ask the holder of this node's access-node guild
//! to wrap that identikey's encapsulation key. Joining the client SSID does
//! not. This spool is not the guild, and it does not contain the guild PTK.
//! A holder applies it with identikey-keys `wrap_login`. The house guild is
//! never the target.

use std::fs;
use std::path::Path;

use data_encoding::HEXLOWER;
use serde::Serialize;

const CONFIG_NAME: &str = "access-node-keyspace";
const SPOOL_SUBDIR: &str = "guildspace";

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct WrapLoginRequest {
    pub kind: &'static str,
    pub profile: &'static str,
    pub keyspace_id: String,
    pub actor: String,
    pub recipient: String,
    /// The holder merges this recipient at the current epoch. Login does not rotate.
    pub epoch_policy: &'static str,
}

/// What a successful identikey login does to guild membership.
#[derive(Debug, PartialEq, Eq)]
pub enum LoginMembership {
    /// Authenticated, and not a guild event.
    None,
    /// Ask the access-node holder to wrap `recipient`.
    Wrap(WrapLoginRequest),
}

/// Read the node's access-node id. Missing file means this node has not been
/// nested into a guild. A `house` profile is refused.
pub fn decide(spool_dir: &Path, actor_hex: &str, recipient_hex: Option<&str>) -> Result<LoginMembership, String> {
    let Some(recipient_hex) = recipient_hex.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(LoginMembership::None);
    };
    let recipient = parse_hex32(recipient_hex, "wrap recipient")?;
    let actor = parse_hex32(actor_hex, "actor")?;
    if recipient == actor {
        return Err("the identikey Sign key cannot hold the guild secret".into());
    }
    let config_path = spool_dir.join(CONFIG_NAME);
    if !config_path.is_file() {
        return Ok(LoginMembership::None);
    }
    let text = fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
    let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty());
    let keyspace = lines
        .next()
        .ok_or("access-node keyspace file is empty")?;
    let _ = parse_hex32(keyspace, "access-node keyspace id")?;
    let profile = lines.next().unwrap_or("access-node");
    if profile != "access-node" {
        return Err("login wraps the access-node guild only".into());
    }
    Ok(LoginMembership::Wrap(WrapLoginRequest {
        kind: "keyspace.join",
        profile: "access-node",
        keyspace_id: keyspace.to_ascii_lowercase(),
        actor: hex32(&actor),
        recipient: hex32(&recipient),
        epoch_policy: "unchanged",
    }))
}

/// Write a wrap-login request beside the identity spool. Returns the
/// access-node keyspace id when a request was written.
pub fn note_login(
    spool_dir: &Path,
    actor_hex: &str,
    recipient_hex: Option<&str>,
) -> Result<Option<String>, String> {
    match decide(spool_dir, actor_hex, recipient_hex)? {
        LoginMembership::None => Ok(None),
        LoginMembership::Wrap(req) => {
            let dir = spool_dir.join(SPOOL_SUBDIR);
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let path = dir.join(format!("{}.json", req.actor));
            let body = serde_json::to_vec(&req).map_err(|e| e.to_string())?;
            fs::write(&path, body).map_err(|e| e.to_string())?;
            Ok(Some(req.keyspace_id))
        }
    }
}

fn parse_hex32(raw: &str, label: &str) -> Result<[u8; 32], String> {
    let bytes = HEXLOWER
        .decode(raw.trim().to_ascii_lowercase().as_bytes())
        .map_err(|_| format!("{label} must be 32 bytes as 64 hex characters"))?;
    bytes
        .try_into()
        .map_err(|_| format!("{label} must be 32 bytes as 64 hex characters"))
}

fn hex32(bytes: &[u8; 32]) -> String {
    HEXLOWER.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACTOR: &str = "1111111111111111111111111111111111111111111111111111111111111111";
    const RECIPIENT: &str = "2222222222222222222222222222222222222222222222222222222222222222";
    const KEYSPACE: &str = "3333333333333333333333333333333333333333333333333333333333333333";

    #[test]
    fn ssid_equivalent_login_is_not_membership() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            decide(dir.path(), ACTOR, None).unwrap(),
            LoginMembership::None
        );
        assert!(note_login(dir.path(), ACTOR, None).unwrap().is_none());
        assert!(!dir.path().join(SPOOL_SUBDIR).exists());
    }

    #[test]
    fn login_without_a_nested_node_does_not_invent_a_guild() {
        let dir = tempfile::tempdir().unwrap();
        assert!(
            note_login(dir.path(), ACTOR, Some(RECIPIENT))
                .unwrap()
                .is_none()
        );
        assert!(!dir.path().join(SPOOL_SUBDIR).exists());
    }

    #[test]
    fn login_spools_an_access_node_wrap_request_without_a_secret() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(CONFIG_NAME),
            format!("{KEYSPACE}\naccess-node\n"),
        )
        .unwrap();
        let id = note_login(dir.path(), ACTOR, Some(RECIPIENT))
            .unwrap()
            .unwrap();
        assert_eq!(id, KEYSPACE);
        let raw = fs::read(dir.path().join(SPOOL_SUBDIR).join(format!("{ACTOR}.json"))).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        assert_eq!(value["kind"], "keyspace.join");
        assert_eq!(value["profile"], "access-node");
        assert_eq!(value["recipient"], RECIPIENT);
        assert_eq!(value["epoch_policy"], "unchanged");
        let text = String::from_utf8(raw).unwrap();
        assert!(!text.contains("ciphertext"));
        assert!(!text.contains("ptk"));
    }

    #[test]
    fn house_guild_and_sign_key_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(CONFIG_NAME), format!("{KEYSPACE}\nhouse\n")).unwrap();
        let err = note_login(dir.path(), ACTOR, Some(RECIPIENT)).unwrap_err();
        assert!(err.contains("access-node"));
        assert!(!dir.path().join(SPOOL_SUBDIR).exists());

        fs::write(
            dir.path().join(CONFIG_NAME),
            format!("{KEYSPACE}\naccess-node\n"),
        )
        .unwrap();
        let err = note_login(dir.path(), ACTOR, Some(ACTOR)).unwrap_err();
        assert!(err.contains("Sign key"));
    }
}
