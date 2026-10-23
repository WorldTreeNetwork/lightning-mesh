// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

//! First-owner claim client. Lightning Admin holds the card secret and
//! the owner seed. hello.mesh only sees `op`, the owner public key, and
//! a proof MAC. A grant is not an SSH key; this module does not write
//! `authorized_keys`.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use ed25519_dalek::SigningKey;
use mjolnir_mesh::owner_claim::{
    CardSecret, CeremonyKind, Challenge, ClaimCard, PublicKey, SignedChallenge, client_prove,
};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnerKey {
    pub public_hex: String,
    pub seed_hex: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimRequestInput {
    pub gateway: String,
    pub owner_public_hex: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimHttp {
    pub status: u16,
    pub body: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimProveInput {
    pub gateway: String,
    pub owner_seed_hex: String,
    pub node_id_hex: String,
    pub card_secret: String,
    pub challenge_json: String,
}

fn hex32(label: &str, s: &str) -> Result<[u8; 32], String> {
    let bytes = hex_decode(s.trim())?;
    bytes
        .try_into()
        .map_err(|_| format!("{label} must be 64 hex characters"))
}

fn hex16(label: &str, s: &str) -> Result<[u8; 16], String> {
    let bytes = hex_decode(s.trim())?;
    bytes
        .try_into()
        .map_err(|_| format!("{label} must be 32 hex characters"))
}

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    if s.len() % 2 != 0 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("expected hex".into());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// JSON hello will relay. No secret field, by construction.
pub fn request_json(owner_public_hex: &str) -> Result<String, String> {
    let owner = hex32("owner", owner_public_hex)?;
    Ok(serde_json::json!({
        "op": "request",
        "owner": hex_encode(&owner),
    })
    .to_string())
}

fn gateway_ok(gateway: &str) -> Result<String, String> {
    let gateway = gateway.trim();
    let ip: std::net::Ipv4Addr = gateway
        .parse()
        .map_err(|_| "gateway must be the node's LAN address, like 10.42.1.1")?;
    if ip.is_unspecified() || ip.is_multicast() {
        return Err("that address is not a node".into());
    }
    Ok(ip.to_string())
}

fn post_hello(gateway: &str, body: &str) -> Result<ClaimHttp, String> {
    let gateway = gateway_ok(gateway)?;
    let addr = format!("{gateway}:80");
    let mut stream = TcpStream::connect(addr.as_str()).map_err(|e| {
        format!("cannot reach hello.mesh on {gateway}:80 ({e}). Join the node's Wi-Fi or plug into its LAN.")
    })?;
    stream
        .set_read_timeout(Some(Duration::from_secs(6)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(6)))
        .map_err(|e| e.to_string())?;
    let head = format!(
        "POST /api/owner-claim HTTP/1.1\r\nHost: {gateway}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(head.as_bytes())
        .and_then(|_| stream.write_all(body.as_bytes()))
        .and_then(|_| stream.flush())
        .map_err(|e| e.to_string())?;
    let mut resp = Vec::new();
    stream.read_to_end(&mut resp).map_err(|e| e.to_string())?;
    let split = resp
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| "the node returned no HTTP response".to_string())?;
    let head = String::from_utf8_lossy(&resp[..split]);
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse().ok())
        .ok_or_else(|| "the node returned no status".to_string())?;
    let body = String::from_utf8_lossy(&resp[split + 4..]).into_owned();
    Ok(ClaimHttp { status, body })
}

#[derive(Deserialize)]
struct ChallengeWire {
    request_id: u64,
    node_id: String,
    owner: String,
    epoch: u64,
    nonce: String,
    salt: String,
    #[serde(default)]
    credential: String,
    request_ttl_ms: u64,
    signature: String,
}

fn signed_from_json(raw: &str, owner: PublicKey) -> Result<(SignedChallenge, [u8; 32]), String> {
    let wire: ChallengeWire = serde_json::from_str(raw)
        .map_err(|_| "the node did not return a claim challenge".to_string())?;
    let node_id = hex32("node_id", &wire.node_id)?;
    let challenge_owner = hex32("owner", &wire.owner)?;
    if challenge_owner != owner {
        return Err("the challenge names a different owner key".into());
    }
    let credential = if wire.credential.is_empty() {
        [0u8; 32]
    } else {
        hex32("credential", &wire.credential)?
    };
    let signed = SignedChallenge {
        challenge: Challenge {
            kind: CeremonyKind::Claim,
            request_id: wire.request_id,
            node_id,
            owner: challenge_owner,
            epoch: wire.epoch,
            nonce: hex32("nonce", &wire.nonce)?,
            salt: hex16("salt", &wire.salt)?,
            credential,
            request_ttl_ms: wire.request_ttl_ms,
        },
        signature: hex_decode(wire.signature.trim())?
            .try_into()
            .map_err(|_| "signature must be 128 hex characters".to_string())?,
    };
    Ok((signed, node_id))
}

/// A new owner key. The seed stays in the window until the claim finishes.
/// It is not written to disk.
#[tauri::command]
pub fn claim_owner_key() -> OwnerKey {
    let signing = SigningKey::generate(&mut OsRng);
    OwnerKey {
        public_hex: hex_encode(signing.verifying_key().as_bytes()),
        seed_hex: hex_encode(&signing.to_bytes()),
    }
}

/// Ask the node for a challenge. The card secret is not an argument.
#[tauri::command]
pub fn claim_request(input: ClaimRequestInput) -> Result<ClaimHttp, String> {
    let body = request_json(&input.owner_public_hex)?;
    post_hello(&input.gateway, &body)
}

/// Check the node's signature, MAC the challenge under the card, and
/// post the proof. The secret is not in the body.
#[tauri::command]
pub fn claim_prove(input: ClaimProveInput) -> Result<ClaimHttp, String> {
    let seed = hex32("owner seed", &input.owner_seed_hex)?;
    let signing = SigningKey::from_bytes(&seed);
    let owner = signing.verifying_key().to_bytes();
    let (signed, node_id) = signed_from_json(&input.challenge_json, owner)?;
    let card_node = hex32("node id", &input.node_id_hex)?;
    if card_node != node_id {
        return Err("the card's node id does not match the challenge".into());
    }
    let secret = CardSecret::parse(&input.card_secret).map_err(|e| e.to_string())?;
    let card = ClaimCard {
        node_id,
        secret,
    };
    let proof = client_prove(&card, &owner, &signed).map_err(|e| e.to_string())?;
    let body = serde_json::json!({
        "op": "prove",
        "request_id": proof.request_id,
        "owner": hex_encode(&proof.owner),
        "nonce": hex_encode(&proof.nonce),
        "mac": hex_encode(&proof.mac),
    })
    .to_string();
    if body.contains("secret") {
        return Err("refusing to send a body that names a secret".into());
    }
    post_hello(&input.gateway, &body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_json_has_no_secret_field() {
        let body = request_json(&"ab".repeat(32)).unwrap();
        assert!(body.contains("\"op\":\"request\""));
        assert!(!body.to_ascii_lowercase().contains("secret"));
        assert!(request_json("zz").is_err());
    }

    #[test]
    fn gateway_rejects_a_name() {
        assert!(gateway_ok("hello.mesh").is_err());
        assert_eq!(gateway_ok("10.42.1.1").unwrap(), "10.42.1.1");
    }
}
