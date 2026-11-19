//! Signed, short-lived presence of a client iroh endpoint on the household mesh.
//!
//! A client that already speaks iroh publishes the UDP socket it can be dialed
//! on. The record is keyed by that endpoint id (the Ed25519 public key), not by
//! a `.mesh` name, and it is not a [`ServiceEntryV2`](crate::crdt::service::ServiceEntryV2).
//! The signed preimage is
//! `mjolnir-iroh-dial:v1\n<challenge>\n<port>\n<ip>\n<signed_at_ms>`.
//! The gateway accepts the claim only when `ip` is the TCP peer address, so a
//! client on the open client SSID cannot publish somebody else's socket. Gossip
//! re-checks the signature. iroh's handshake still authenticates the peer; this
//! record is a dial hint.
//!
//! The book is memory-only. A missing renewal fades the socket out of lookup
//! after [`RESOLVE_STALE_MS`]. No relay URL is stored: the client's own iroh
//! endpoint keeps relay mode on, and address lookup supplies the home relay
//! when this socket does not answer.

use std::collections::BTreeMap;
use std::fs;
use std::net::Ipv4Addr;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};

/// Domain-separated prefix of the bytes a dial claim signs.
pub const DIAL_DOMAIN: &str = "mjolnir-iroh-dial:v1";

/// How long after `signed_at_ms` a record still answers lookup.
pub const RESOLVE_STALE_MS: u64 = 90_000;

/// How far `signed_at_ms` may sit ahead of the checker's clock.
pub const MAX_SKEW_MS: u64 = 120_000;

/// Live dial records one source address may hold, besides a renewal of the
/// same endpoint id.
pub const MAX_PER_SOURCE: usize = 8;

/// JSON projection `mjolnir-meshd` writes beside `directory.json`.
pub const PROJECTION_FILE: &str = "iroh-dial.json";

const PROJECTION_VERSION: u32 = 1;

/// One client's dial hint. `endpoint_id` is the lowercase hex Ed25519 public
/// key that signed [`signing_message`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrohDialEntry {
    pub endpoint_id: String,
    pub ip: Ipv4Addr,
    pub port: u16,
    pub signed_at_ms: u64,
    pub sig: String,
    pub challenge: String,
}

/// Endpoint id → newest accepted claim.
pub type IrohDialBook = BTreeMap<String, IrohDialEntry>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialError {
    Encoding,
    Signature,
    Address,
    Port,
    Clock,
    Cap,
}

impl DialError {
    pub fn as_str(self) -> &'static str {
        match self {
            DialError::Encoding => "invalid encoding",
            DialError::Signature => "invalid signature",
            DialError::Address => "address not on the client mesh",
            DialError::Port => "invalid port",
            DialError::Clock => "signed_at outside the freshness window",
            DialError::Cap => "too many dial records from this address",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialOutcome {
    Inserted,
    Updated,
    Unchanged,
    Rejected(DialError),
}

/// The exact bytes the endpoint key signs. `ip` is dotted decimal, `port` and
/// `signed_at_ms` are unpadded decimal.
pub fn signing_message(challenge_hex: &str, port: u16, ip: Ipv4Addr, signed_at_ms: u64) -> Vec<u8> {
    format!("{DIAL_DOMAIN}\n{challenge_hex}\n{port}\n{ip}\n{signed_at_ms}").into_bytes()
}

pub fn unix_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

/// A client-LAN host address. Gateway `.1`, the network `.0`, and the subnet
/// broadcast `.255` are not dial targets. `10.254.0.0/16` is outside this range.
pub fn acceptable_dial_ipv4(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    o[0] == 10 && o[1] == 42 && o[3] != 0 && o[3] != 1 && o[3] != 255
}

/// `signed_at_ms` is still inside the resolve window, allowing limited future skew.
pub fn resolves(entry: &IrohDialEntry, now_ms: u64) -> bool {
    let ahead = entry.signed_at_ms.saturating_sub(now_ms);
    let behind = now_ms.saturating_sub(entry.signed_at_ms);
    ahead <= MAX_SKEW_MS && behind <= RESOLVE_STALE_MS
}

pub fn verify_signature(entry: &IrohDialEntry) -> Result<(), DialError> {
    if entry.port == 0 {
        return Err(DialError::Port);
    }
    if !acceptable_dial_ipv4(entry.ip) {
        return Err(DialError::Address);
    }
    let pubkey = decode_fixed::<32>(&entry.endpoint_id).ok_or(DialError::Encoding)?;
    let sig = decode_fixed::<64>(&entry.sig).ok_or(DialError::Encoding)?;
    if !valid_challenge(&entry.challenge) {
        return Err(DialError::Encoding);
    }
    let key = VerifyingKey::from_bytes(&pubkey).map_err(|_| DialError::Encoding)?;
    let signature = Signature::from_bytes(&sig);
    let message = signing_message(&entry.challenge, entry.port, entry.ip, entry.signed_at_ms);
    key.verify_strict(&message, &signature)
        .map_err(|_| DialError::Signature)
}

/// Signature, address, port, and freshness. Does not consult the book.
pub fn validate_submission(entry: &IrohDialEntry, now_ms: u64) -> Result<(), DialError> {
    verify_signature(entry)?;
    if !resolves(entry, now_ms) {
        return Err(DialError::Clock);
    }
    Ok(())
}

fn source_at_cap(book: &IrohDialBook, entry: &IrohDialEntry, now_ms: u64) -> bool {
    let others = book.values().filter(|existing| {
        existing.ip == entry.ip
            && existing.endpoint_id != entry.endpoint_id
            && resolves(existing, now_ms)
    });
    others.count() >= MAX_PER_SOURCE
}

/// Last-writer-wins on `signed_at_ms`. An equal timestamp keeps the record
/// already in the book. A failed signature or a stale claim is rejected and
/// does not evict the current row.
pub fn apply(book: &mut IrohDialBook, entry: IrohDialEntry, now_ms: u64) -> DialOutcome {
    if let Err(err) = validate_submission(&entry, now_ms) {
        return DialOutcome::Rejected(err);
    }
    if let Some(existing) = book.get(&entry.endpoint_id) {
        if existing.signed_at_ms > entry.signed_at_ms {
            return DialOutcome::Unchanged;
        }
        if existing.signed_at_ms == entry.signed_at_ms {
            return DialOutcome::Unchanged;
        }
    } else if source_at_cap(book, &entry, now_ms) {
        return DialOutcome::Rejected(DialError::Cap);
    }
    let id = entry.endpoint_id.clone();
    let inserted = !book.contains_key(&id);
    book.insert(id, entry);
    if inserted {
        DialOutcome::Inserted
    } else {
        DialOutcome::Updated
    }
}

pub fn reap(book: &mut IrohDialBook, now_ms: u64) {
    book.retain(|_, entry| resolves(entry, now_ms));
}

#[derive(Serialize)]
struct Projection<'a> {
    version: u32,
    records: &'a BTreeMap<String, IrohDialEntry>,
}

#[derive(Deserialize)]
struct ProjectionOwned {
    records: BTreeMap<String, IrohDialEntry>,
}

/// Live records only. The HTTP lookup reads this file and returns one id.
pub fn projection_json(book: &IrohDialBook, now_ms: u64) -> String {
    let records: BTreeMap<_, _> = book
        .iter()
        .filter(|(_, entry)| resolves(entry, now_ms))
        .map(|(id, entry)| (id.clone(), entry.clone()))
        .collect();
    serde_json::to_string(&Projection {
        version: PROJECTION_VERSION,
        records: &records,
    })
    .unwrap_or_else(|_| r#"{"version":1,"records":{}}"#.to_string())
}

pub fn write_projection(directory_file: &Path, book: &IrohDialBook, now_ms: u64) {
    let path = directory_file.with_file_name(PROJECTION_FILE);
    if let Some(parent) = path.parent()
        && let Err(err) = fs::create_dir_all(parent)
    {
        tracing::warn!(path = %parent.display(), "iroh dial projection dir: {err}");
        return;
    }
    let tmp = path.with_extension("json.tmp");
    let body = projection_json(book, now_ms);
    if let Err(err) = fs::write(&tmp, body) {
        tracing::warn!(path = %tmp.display(), "iroh dial projection write: {err}");
        return;
    }
    if let Err(err) = fs::rename(&tmp, &path) {
        tracing::warn!(path = %path.display(), "iroh dial projection rename: {err}");
    }
}

/// One record for `GET /api/iroh/<endpoint-id>`, or `None` when the id is
/// absent, stale, or the projection cannot be read.
pub fn lookup_json(projection: &[u8], endpoint_id: &str, now_ms: u64) -> Option<String> {
    decode_fixed::<32>(endpoint_id)?;
    let parsed: ProjectionOwned = serde_json::from_slice(projection).ok()?;
    let entry = parsed.records.get(endpoint_id)?;
    if entry.endpoint_id != endpoint_id || !resolves(entry, now_ms) {
        return None;
    }
    serde_json::to_string(entry).ok()
}

pub fn projection_source_at_cap(
    projection: &[u8],
    ip: Ipv4Addr,
    endpoint_id: &str,
    now_ms: u64,
) -> bool {
    let Ok(parsed) = serde_json::from_slice::<ProjectionOwned>(projection) else {
        return false;
    };
    let others = parsed.records.values().filter(|entry| {
        entry.ip == ip && entry.endpoint_id != endpoint_id && resolves(entry, now_ms)
    });
    others.count() >= MAX_PER_SOURCE
}

/// Read `<spool>/iroh-dial/*.json`, apply each claim, and delete the file.
/// Returns the claims that changed the book so the caller can gossip them.
pub fn ingest_spool(spool_dir: &Path, book: &mut IrohDialBook, now_ms: u64) -> Vec<IrohDialEntry> {
    let dir = spool_dir.join("iroh-dial");
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
        Err(err) => {
            tracing::warn!(path = %dir.display(), "iroh dial spool: {err}");
            return Vec::new();
        }
    };
    let mut applied = Vec::new();
    for dir_entry in entries.flatten() {
        let path = dir_entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(err) => {
                tracing::warn!(path = %path.display(), "iroh dial spool read: {err}");
                continue;
            }
        };
        let entry: IrohDialEntry = match serde_json::from_slice(&bytes) {
            Ok(entry) => entry,
            Err(err) => {
                tracing::warn!(path = %path.display(), "iroh dial spool malformed: {err}");
                let _ = fs::rename(&path, path.with_extension("json.bad"));
                continue;
            }
        };
        if entry.endpoint_id != stem {
            tracing::warn!(path = %path.display(), "iroh dial spool name does not match endpoint id");
            let _ = fs::remove_file(&path);
            continue;
        }
        let changed = matches!(
            apply(book, entry.clone(), now_ms),
            DialOutcome::Inserted | DialOutcome::Updated
        );
        if changed {
            applied.push(entry);
        }
        if let Err(err) = fs::remove_file(&path) {
            tracing::warn!(path = %path.display(), "iroh dial spool remove: {err}");
        }
    }
    applied
}

fn valid_challenge(challenge: &str) -> bool {
    let len = challenge.len();
    (2..=128).contains(&len)
        && len.is_multiple_of(2)
        && challenge
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn decode_fixed<const N: usize>(hex: &str) -> Option<[u8; N]> {
    if hex.len() != N * 2 {
        return None;
    }
    let mut out = [0u8; N];
    for (i, byte) in out.iter_mut().enumerate() {
        let hi = from_hex(hex.as_bytes()[i * 2])?;
        let lo = from_hex(hex.as_bytes()[i * 2 + 1])?;
        *byte = (hi << 4) | lo;
    }
    Some(out)
}

fn from_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn signed(port: u16, ip: Ipv4Addr, at: u64) -> (SigningKey, IrohDialEntry) {
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let endpoint_id = hex_encode(key.verifying_key().as_bytes());
        let challenge = "ab".repeat(32);
        let message = signing_message(&challenge, port, ip, at);
        let sig = hex_encode(&key.sign(&message).to_bytes());
        (
            key,
            IrohDialEntry {
                endpoint_id,
                ip,
                port,
                signed_at_ms: at,
                sig,
                challenge,
            },
        )
    }

    fn hex_encode(bytes: &[u8]) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut out = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            out.push(HEX[(byte >> 4) as usize] as char);
            out.push(HEX[(byte & 0xf) as usize] as char);
        }
        out
    }

    #[test]
    fn signature_binds_the_socket_and_the_time() {
        let now = 1_700_000_000_000;
        let (_key, entry) = signed(4000, Ipv4Addr::new(10, 42, 7, 20), now);
        assert_eq!(validate_submission(&entry, now), Ok(()));

        let mut moved = entry.clone();
        moved.ip = Ipv4Addr::new(10, 42, 7, 21);
        assert_eq!(verify_signature(&moved), Err(DialError::Signature));
    }

    #[test]
    fn refuses_gateway_overlay_and_port_zero() {
        let now = 1_700_000_000_000;
        for ip in [
            Ipv4Addr::new(10, 42, 7, 1),
            Ipv4Addr::new(10, 254, 1, 20),
            Ipv4Addr::new(192, 168, 1, 20),
            Ipv4Addr::new(10, 42, 7, 0),
            Ipv4Addr::new(10, 42, 7, 255),
        ] {
            let (_key, entry) = signed(4000, ip, now);
            assert_eq!(
                validate_submission(&entry, now),
                Err(DialError::Address),
                "{ip}"
            );
        }
        let (_key, entry) = signed(0, Ipv4Addr::new(10, 42, 7, 20), now);
        assert_eq!(validate_submission(&entry, now), Err(DialError::Port));
    }

    #[test]
    fn newer_signature_replaces_and_stale_does_not() {
        let now = 1_700_000_000_000;
        let ip = Ipv4Addr::new(10, 42, 7, 20);
        let (_key, first) = signed(4000, ip, now);
        let mut book = IrohDialBook::new();
        assert_eq!(apply(&mut book, first.clone(), now), DialOutcome::Inserted);

        let (_key, older) = signed(4000, ip, now - 1_000);
        assert_eq!(apply(&mut book, older, now), DialOutcome::Unchanged);
        assert_eq!(book[&first.endpoint_id].port, 4000);

        let (_key, newer) = signed(4001, ip, now + 1_000);
        assert_eq!(apply(&mut book, newer, now + 1_000), DialOutcome::Updated);
        assert_eq!(book[&first.endpoint_id].port, 4001);

        let signed_at = book[&first.endpoint_id].signed_at_ms;
        let stale_at = signed_at + RESOLVE_STALE_MS + 1;
        assert!(!resolves(book.get(&first.endpoint_id).unwrap(), stale_at));
        reap(&mut book, stale_at);
        assert!(book.is_empty());
    }

    #[test]
    fn eighth_other_endpoint_from_one_address_is_refused() {
        let now = 1_700_000_000_000;
        let ip = Ipv4Addr::new(10, 42, 7, 20);
        let mut book = IrohDialBook::new();
        for i in 0..MAX_PER_SOURCE {
            let key = SigningKey::from_bytes(&[(i as u8) + 1; 32]);
            let endpoint_id = hex_encode(key.verifying_key().as_bytes());
            let challenge = "cd".repeat(32);
            let message = signing_message(&challenge, 4000, ip, now);
            let entry = IrohDialEntry {
                endpoint_id,
                ip,
                port: 4000,
                signed_at_ms: now,
                sig: hex_encode(&key.sign(&message).to_bytes()),
                challenge,
            };
            assert_eq!(apply(&mut book, entry, now), DialOutcome::Inserted);
        }
        let ninth = SigningKey::from_bytes(&[9u8; 32]);
        let endpoint_id = hex_encode(ninth.verifying_key().as_bytes());
        let challenge = "cd".repeat(32);
        let message = signing_message(&challenge, 4000, ip, now);
        let extra = IrohDialEntry {
            endpoint_id,
            ip,
            port: 4000,
            signed_at_ms: now,
            sig: hex_encode(&ninth.sign(&message).to_bytes()),
            challenge,
        };
        assert_eq!(
            apply(&mut book, extra, now),
            DialOutcome::Rejected(DialError::Cap)
        );

        let renew_key = SigningKey::from_bytes(&[1u8; 32]);
        let renew_id = hex_encode(renew_key.verifying_key().as_bytes());
        let mut renewal = book[&renew_id].clone();
        renewal.signed_at_ms = now + 1_000;
        renewal.port = 4001;
        renewal.sig = hex_encode(
            &renew_key
                .sign(&signing_message(
                    &renewal.challenge,
                    renewal.port,
                    renewal.ip,
                    renewal.signed_at_ms,
                ))
                .to_bytes(),
        );
        assert_eq!(apply(&mut book, renewal, now + 1_000), DialOutcome::Updated);
    }

    #[test]
    fn lookup_returns_one_live_record() {
        let now = 1_700_000_000_000;
        let (_key, entry) = signed(4000, Ipv4Addr::new(10, 42, 7, 20), now);
        let mut book = IrohDialBook::new();
        apply(&mut book, entry.clone(), now);
        let body = projection_json(&book, now);
        let got = lookup_json(body.as_bytes(), &entry.endpoint_id, now).unwrap();
        assert!(got.contains("\"port\":4000"));
        assert!(
            lookup_json(
                body.as_bytes(),
                &entry.endpoint_id,
                now + RESOLVE_STALE_MS + 1
            )
            .is_none()
        );
        assert!(lookup_json(body.as_bytes(), &"ab".repeat(32), now).is_none());
    }
}
