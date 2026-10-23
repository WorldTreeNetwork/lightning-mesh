// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors
// Lightning Mesh is dual-licensed (AGPL-3.0-or-later or commercial); see LICENSE
// and COMMERCIAL-LICENSE.md at the repository root.

//! Owner-claim relay (openspec `add-household-owner-claim`, bead
//! `mjolnir-mesh-bf7.1`).
//!
//! hello only relays a claim request or a claim proof to meshd and hands back
//! meshd's node-signed reply. It never sees the card secret or `k`: the wire
//! types have no field for either, a body that carries one is rejected before
//! anything is forwarded, and meshd does the MAC check and the
//! compare-and-swap. The relay waits at most [`MAX_RELAY_WAIT`] and does not
//! use the async name spool.
//!
//! Lightning Admin is the claim client; the hello.mesh page has no secret
//! field. Route wiring and the meshd transport land with the meshd side.

#![allow(dead_code)]

use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;

use data_encoding::HEXLOWER;
use mjolnir_mesh::owner_claim::{CardSecret, ProofSubmission, PublicKey};
use serde::Deserialize;
use serde_json::Value;

/// Upper bound on how long hello waits for meshd.
pub const MAX_RELAY_WAIT: Duration = Duration::from_secs(5);

/// What hello forwards to meshd. No secret, no `k`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelayRequest {
    /// Ask meshd for a node-signed claim challenge.
    Request { owner: PublicKey },
    /// Present a claim proof inside an armed window.
    Prove(ProofSubmission),
}

/// The meshd side of the relay. Returns meshd's node-signed reply verbatim.
pub trait ClaimForwarder: Send + Sync + 'static {
    fn forward(&self, req: RelayRequest) -> Result<Vec<u8>, String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayResponse {
    pub status: u16,
    pub content_type: &'static str,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reject {
    /// The body names a secret-shaped field (`secret`, `k`, `verifier`, ...).
    SecretField(String),
    /// A string value has the shape of a raw card secret.
    RawSecret,
    Malformed(String),
}

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "lowercase", deny_unknown_fields)]
enum Wire {
    Request {
        owner: String,
    },
    Prove {
        request_id: u64,
        owner: String,
        nonce: String,
        mac: String,
    },
}

/// Field names a claim body must never carry, at any depth.
fn secret_key_name(key: &str) -> bool {
    let k = key.to_ascii_lowercase();
    k == "k"
        || [
            "secret", "verifier", "private", "password", "bearer", "card",
        ]
        .iter()
        .any(|w| k.contains(w))
}

fn scan(v: &Value) -> Result<(), Reject> {
    match v {
        Value::Object(map) => {
            for (key, val) in map {
                if secret_key_name(key) {
                    return Err(Reject::SecretField(key.clone()));
                }
                scan(val)?;
            }
            Ok(())
        }
        Value::Array(items) => items.iter().try_for_each(scan),
        // Every legitimate string here is 64 lowercase hex characters. A
        // value that reads as a card secret and is not that is the secret.
        Value::String(s) if hex32(s).is_none() && CardSecret::parse(s).is_ok() => {
            Err(Reject::RawSecret)
        }
        _ => Ok(()),
    }
}

fn hex32(s: &str) -> Option<[u8; 32]> {
    HEXLOWER.decode(s.as_bytes()).ok()?.try_into().ok()
}

fn field32(name: &str, s: &str) -> Result<[u8; 32], Reject> {
    hex32(s).ok_or_else(|| Reject::Malformed(format!("{name}: want 64 lowercase hex chars")))
}

/// Validate a relay body. Nothing is forwarded unless this succeeds.
pub fn parse_body(body: &[u8]) -> Result<RelayRequest, Reject> {
    let value: Value =
        serde_json::from_slice(body).map_err(|e| Reject::Malformed(e.to_string()))?;
    scan(&value)?;
    let wire: Wire = serde_json::from_value(value).map_err(|e| Reject::Malformed(e.to_string()))?;
    Ok(match wire {
        Wire::Request { owner } => RelayRequest::Request {
            owner: field32("owner", &owner)?,
        },
        Wire::Prove {
            request_id,
            owner,
            nonce,
            mac,
        } => RelayRequest::Prove(ProofSubmission {
            request_id,
            owner: field32("owner", &owner)?,
            nonce: field32("nonce", &nonce)?,
            mac: field32("mac", &mac)?,
        }),
    })
}

/// The wait actually used: never more than [`MAX_RELAY_WAIT`].
pub fn effective_wait(wait: Duration) -> Duration {
    wait.min(MAX_RELAY_WAIT)
}

/// meshd's localhost control API. Same fixed port as `CONTROL_API_PORT`.
const MESHD_CONTROL: &str = "127.0.0.1:5380";

/// Validate `body`, then hand those same bytes to meshd. The card secret
/// never has a field here. A meshd that is down or has no claim service
/// comes back as 502 / the daemon's own JSON, not as a successful claim.
pub fn forward_to_meshd(body: &[u8]) -> RelayResponse {
    if let Err(reject) = parse_body(body) {
        return match reject {
            Reject::SecretField(name) => error(400, &format!("refusing field {name}")),
            Reject::RawSecret => error(400, "refusing a raw card secret"),
            Reject::Malformed(msg) => error(400, &msg),
        };
    }
    match post_control(body) {
        Ok((status, resp)) => json(status, resp),
        Err(msg) => error(502, &msg),
    }
}

fn post_control(body: &[u8]) -> Result<(u16, Vec<u8>), String> {
    use std::io::{Read, Write};
    use std::net::TcpStream;

    let mut stream = TcpStream::connect(MESHD_CONTROL)
        .map_err(|e| format!("meshd control API unreachable at {MESHD_CONTROL} ({e})"))?;
    stream
        .set_read_timeout(Some(MAX_RELAY_WAIT))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(MAX_RELAY_WAIT))
        .map_err(|e| e.to_string())?;
    let head = format!(
        "POST /v0/owner-claim HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(head.as_bytes())
        .and_then(|_| stream.write_all(body))
        .and_then(|_| stream.flush())
        .map_err(|e| e.to_string())?;
    let mut resp = Vec::new();
    stream.read_to_end(&mut resp).map_err(|e| e.to_string())?;
    let split = resp
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| "meshd response had no header".to_string())?;
    let head = String::from_utf8_lossy(&resp[..split]);
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse().ok())
        .ok_or_else(|| "meshd response had no status".to_string())?;
    Ok((status, resp[split + 4..].to_vec()))
}

fn json(status: u16, body: Vec<u8>) -> RelayResponse {
    RelayResponse {
        status,
        content_type: "application/json",
        body,
    }
}

fn error(status: u16, msg: &str) -> RelayResponse {
    json(
        status,
        serde_json::json!({ "error": msg }).to_string().into_bytes(),
    )
}

/// Validate `body`, forward it, and wait up to `wait` (capped at 5 s) for
/// meshd's reply.
pub fn relay(body: &[u8], forwarder: Arc<dyn ClaimForwarder>, wait: Duration) -> RelayResponse {
    let req = match parse_body(body) {
        Ok(req) => req,
        Err(Reject::SecretField(_)) | Err(Reject::RawSecret) => {
            return error(400, "claim bodies must not carry the card secret or k");
        }
        Err(Reject::Malformed(msg)) => return error(400, &msg),
    };
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(forwarder.forward(req));
    });
    match rx.recv_timeout(effective_wait(wait)) {
        Ok(Ok(reply)) => json(200, reply),
        Ok(Err(msg)) => error(502, &msg),
        Err(_) => error(504, "meshd did not answer in time"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::time::Instant;

    const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HEX_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const HEX_C: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[derive(Default)]
    struct Recorder {
        seen: Mutex<Vec<RelayRequest>>,
        delay: Option<Duration>,
    }

    impl ClaimForwarder for Recorder {
        fn forward(&self, req: RelayRequest) -> Result<Vec<u8>, String> {
            if let Some(d) = self.delay {
                std::thread::sleep(d);
            }
            self.seen.lock().unwrap().push(req);
            Ok(br#"{"signed":"outcome"}"#.to_vec())
        }
    }

    fn prove_body(extra: &str) -> Vec<u8> {
        format!(
            r#"{{"op":"prove","request_id":7,"owner":"{HEX_A}","nonce":"{HEX_B}","mac":"{HEX_C}"{extra}}}"#
        )
        .into_bytes()
    }

    #[test]
    fn claim_relay_forwards_a_proof_verbatim() {
        let rec = Arc::new(Recorder::default());
        let resp = relay(&prove_body(""), rec.clone(), MAX_RELAY_WAIT);
        assert_eq!(resp.status, 200);
        assert_eq!(resp.body, br#"{"signed":"outcome"}"#);
        let seen = rec.seen.lock().unwrap();
        let RelayRequest::Prove(p) = &seen[0] else {
            panic!("expected a proof");
        };
        assert_eq!(p.request_id, 7);
        assert_eq!(p.owner, [0xaa; 32]);
        assert_eq!(p.nonce, [0xbb; 32]);
    }

    #[test]
    fn claim_relay_forwards_a_challenge_request() {
        let rec = Arc::new(Recorder::default());
        let body = format!(r#"{{"op":"request","owner":"{HEX_A}"}}"#);
        assert_eq!(
            relay(body.as_bytes(), rec.clone(), MAX_RELAY_WAIT).status,
            200
        );
        assert_eq!(
            rec.seen.lock().unwrap()[0],
            RelayRequest::Request { owner: [0xaa; 32] }
        );
    }

    #[test]
    fn forward_rejects_a_secret_field_without_calling_meshd() {
        let resp = forward_to_meshd(br#"{"op":"request","owner":"aa","secret":"no"}"#);
        assert_eq!(resp.status, 400);
        let body = String::from_utf8(resp.body).unwrap();
        assert!(body.contains("secret"));
    }

    fn claim_relay_rejects_the_raw_secret_and_k() {
        let bodies = [
            prove_body(r#","secret":"abcdefghijk23456""#),
            prove_body(r#","card_secret":"abcdefghijk23456""#),
            prove_body(&format!(r#","k":"{HEX_A}""#)),
            prove_body(&format!(r#","meta":{{"verifier":"{HEX_A}"}}"#)),
            prove_body(r#","extra":["abcd-efgh-ijk2-3456"]"#),
            br#"{"op":"request","owner":"abcdefghijk23456abcdefghijk23456"}"#.to_vec(),
        ];
        for body in bodies {
            let rec = Arc::new(Recorder::default());
            let resp = relay(&body, rec.clone(), MAX_RELAY_WAIT);
            assert_eq!(resp.status, 400, "{}", String::from_utf8_lossy(&body));
            assert!(
                rec.seen.lock().unwrap().is_empty(),
                "nothing reaches meshd: {}",
                String::from_utf8_lossy(&body)
            );
        }
        assert!(matches!(
            parse_body(&prove_body(r#","secret":"x""#)),
            Err(Reject::SecretField(_))
        ));
        assert_eq!(
            parse_body(br#"{"op":"request","owner":"abcdefghijk23456"}"#),
            Err(Reject::RawSecret)
        );
    }

    #[test]
    fn claim_relay_rejects_unknown_fields_and_bad_widths() {
        assert!(matches!(
            parse_body(&prove_body(r#","note":1"#)),
            Err(Reject::Malformed(_))
        ));
        assert!(matches!(
            parse_body(br#"{"op":"request","owner":"abcd"}"#),
            Err(Reject::Malformed(_))
        ));
    }

    #[test]
    fn claim_relay_waits_at_most_five_seconds() {
        assert_eq!(effective_wait(Duration::from_secs(30)), MAX_RELAY_WAIT);
        assert_eq!(MAX_RELAY_WAIT, Duration::from_secs(5));
        let slow = Arc::new(Recorder {
            delay: Some(Duration::from_millis(500)),
            ..Default::default()
        });
        let start = Instant::now();
        let resp = relay(&prove_body(""), slow, Duration::from_millis(50));
        assert_eq!(resp.status, 504);
        assert!(start.elapsed() < Duration::from_millis(450));
    }
}
