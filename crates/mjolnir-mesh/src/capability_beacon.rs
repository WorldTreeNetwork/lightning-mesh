// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors
// Lightning Mesh is dual-licensed (AGPL-3.0-or-later or commercial); see LICENSE
// and COMMERCIAL-LICENSE.md at the repository root.

//! Signed discovery locators and scanner-challenged proof of key possession.
//!
//! Neither result grants membership or lifts quarantine. This module performs
//! no I/O. Coordinates are signed hints, not permission to connect or merge.

use std::net::SocketAddr;

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;

pub const BEACON_DOMAIN_LINE: &[u8] = b"mjolnir-capability-beacon:v1\n";
pub const HANDSHAKE_DOMAIN_LINE: &[u8] = b"mjolnir-capability-handshake:v1\n";

/// Declaration order, including nested structs, fixes compact JSON key order.
/// Integers use shortest decimal form; slashes are not escaped. No final LF.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityDescriptor {
    /// Advertised generation, for the scanner to classify compatibility.
    pub protocol_generation: u32,
    /// Full Ed25519 public key, encoded as 64 lowercase hexadecimal characters.
    pub node_id: String,
    pub channel_plan: Vec<ChannelPlan>,
    pub bootstrap: BootstrapCoordinates,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelPlan {
    pub frequency_mhz: u32,
    pub width_mhz: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootstrapCoordinates {
    pub direct_addrs: Vec<SocketAddr>,
    pub relay_url: Option<String>,
}

impl CapabilityDescriptor {
    /// Exact bytes covered by the beacon signature.
    pub fn signed_bytes(&self) -> Result<Vec<u8>, BeaconError> {
        Ok(signed_bytes(BEACON_DOMAIN_LINE, &compact_json(self)?))
    }
}

/// A verified signature on discovery hints. Replaying it yields this same
/// locator; it says nothing about freshness, compatibility, or membership.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeaconLocator {
    descriptor: CapabilityDescriptor,
}

impl BeaconLocator {
    pub fn descriptor(&self) -> &CapabilityDescriptor {
        &self.descriptor
    }
}

/// Separate wire payload: the scanner nonce followed by the full beacon payload.
/// Its signature uses HANDSHAKE_DOMAIN_LINE, never BEACON_DOMAIN_LINE.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityHandshake {
    /// 32 scanner-generated random bytes in lowercase hexadecimal.
    pub nonce: String,
    pub beacon: CapabilityDescriptor,
}

impl CapabilityHandshake {
    pub fn signed_bytes(&self) -> Result<Vec<u8>, BeaconError> {
        Ok(signed_bytes(HANDSHAKE_DOMAIN_LINE, &compact_json(self)?))
    }
}

/// Scanner-owned, single-use challenge, bound to the exact discovered locator.
///
/// The scanner MUST supply a fresh, unpredictable 32-byte nonce for every new
/// challenge, including after restart. This type deliberately cannot be cloned
/// or deserialized. A successful verification consumes its nonce. Creating a
/// second challenge with an old nonce defeats freshness; nonce generation and
/// transport/session lifetime belong to the caller, not this pure library.
#[derive(Debug)]
pub struct HandshakeChallenge {
    locator: BeaconLocator,
    nonce: Option<[u8; 32]>,
}

impl HandshakeChallenge {
    pub fn new(locator: BeaconLocator, fresh_nonce: [u8; 32]) -> Self {
        Self {
            locator,
            nonce: Some(fresh_nonce),
        }
    }
}

/// Proof that the locator's key answered this scanner's challenge. This is
/// not an enrollment, authorization, capability grant, or membership decision.
#[derive(Debug, PartialEq, Eq)]
pub struct VerifiedHandshake {
    locator: BeaconLocator,
    nonce: [u8; 32],
}

impl VerifiedHandshake {
    pub fn locator(&self) -> &BeaconLocator {
        &self.locator
    }

    pub fn nonce(&self) -> &[u8; 32] {
        &self.nonce
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BeaconError {
    #[error("envelope must be exactly three LF-separated lines")]
    Envelope,
    #[error("envelope field names or ordering are invalid")]
    EnvelopeFields,
    #[error("node public key must be 64 lowercase hexadecimal characters")]
    NodePubkey,
    #[error("signature must be 128 lowercase hexadecimal characters")]
    Signature,
    #[error("payload is not valid protocol JSON")]
    PayloadJson,
    #[error("payload is not canonical compact JSON")]
    NonCanonicalPayload,
    #[error("node public key is not a valid non-weak Ed25519 verification key")]
    InvalidNodeKey,
    #[error("signature did not verify")]
    ForgedSignature,
    #[error("descriptor node id does not match the signing public key")]
    NodeIdMismatch,
    #[error("handshake nonce does not match the scanner challenge")]
    NonceMismatch,
    #[error("handshake challenge has already been consumed")]
    ReplayedNonce,
    #[error("handshake beacon does not match the scanner locator")]
    LocatorMismatch,
}

/// Same three-line envelope discipline as https_issuance.rs, with a node key:
/// `node_pubkey=<lower hex>\npayload=<compact JSON>\nsig=<lower hex>`.
pub fn sign_beacon(
    descriptor: &CapabilityDescriptor,
    node_key: &SigningKey,
) -> Result<String, BeaconError> {
    validate_node_id(descriptor, &node_key.verifying_key().to_bytes())?;
    sign_envelope(descriptor, BEACON_DOMAIN_LINE, node_key)
}

pub fn verify_beacon(envelope: &str) -> Result<BeaconLocator, BeaconError> {
    let (descriptor, node_pubkey) = verify_envelope(envelope, BEACON_DOMAIN_LINE)?;
    validate_node_id(&descriptor, &node_pubkey)?;
    Ok(BeaconLocator { descriptor })
}

/// Node's response to a scanner-supplied nonce. This does not open an endpoint.
pub fn sign_handshake(
    descriptor: &CapabilityDescriptor,
    scanner_nonce: &[u8; 32],
    node_key: &SigningKey,
) -> Result<String, BeaconError> {
    validate_node_id(descriptor, &node_key.verifying_key().to_bytes())?;
    let handshake = CapabilityHandshake {
        nonce: lower_hex(scanner_nonce),
        beacon: descriptor.clone(),
    };
    sign_envelope(&handshake, HANDSHAKE_DOMAIN_LINE, node_key)
}

/// Verify against the scanner's own challenge, never a nonce read from the
/// response. Success consumes the challenge; failures do not consume it.
pub fn verify_handshake(
    envelope: &str,
    challenge: &mut HandshakeChallenge,
) -> Result<VerifiedHandshake, BeaconError> {
    let nonce = challenge.nonce.ok_or(BeaconError::ReplayedNonce)?;
    let (handshake, node_pubkey): (CapabilityHandshake, _) =
        verify_envelope(envelope, HANDSHAKE_DOMAIN_LINE)?;
    validate_node_id(&handshake.beacon, &node_pubkey)?;
    if handshake.nonce != lower_hex(&nonce) {
        return Err(BeaconError::NonceMismatch);
    }
    if handshake.beacon != challenge.locator.descriptor {
        return Err(BeaconError::LocatorMismatch);
    }
    challenge.nonce = None;
    Ok(VerifiedHandshake {
        locator: challenge.locator.clone(),
        nonce,
    })
}

fn validate_node_id(
    descriptor: &CapabilityDescriptor,
    node_pubkey: &[u8; 32],
) -> Result<(), BeaconError> {
    if descriptor.node_id != lower_hex(node_pubkey) {
        return Err(BeaconError::NodeIdMismatch);
    }
    Ok(())
}

fn compact_json<T: Serialize>(payload: &T) -> Result<String, BeaconError> {
    serde_json::to_string(payload).map_err(|_| BeaconError::PayloadJson)
}

fn signed_bytes(domain: &[u8], payload: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(domain.len() + payload.len());
    bytes.extend_from_slice(domain);
    bytes.extend_from_slice(payload.as_bytes());
    bytes
}

fn sign_envelope<T: Serialize>(
    payload: &T,
    domain: &[u8],
    key: &SigningKey,
) -> Result<String, BeaconError> {
    let payload = compact_json(payload)?;
    let signature = key.sign(&signed_bytes(domain, &payload));
    Ok(format!(
        "node_pubkey={}\npayload={}\nsig={}",
        lower_hex(&key.verifying_key().to_bytes()),
        payload,
        lower_hex(&signature.to_bytes())
    ))
}

fn verify_envelope<T: Serialize + DeserializeOwned>(
    envelope: &str,
    domain: &[u8],
) -> Result<(T, [u8; 32]), BeaconError> {
    // As in https_issuance.rs, allow one conventional final file newline.
    if envelope.contains('\r') {
        return Err(BeaconError::Envelope);
    }
    let envelope = envelope.strip_suffix('\n').unwrap_or(envelope);
    let mut lines = envelope.split('\n');
    let node_line = lines.next().ok_or(BeaconError::Envelope)?;
    let payload_line = lines.next().ok_or(BeaconError::Envelope)?;
    let sig_line = lines.next().ok_or(BeaconError::Envelope)?;
    if lines.next().is_some() {
        return Err(BeaconError::Envelope);
    }
    let node_hex = node_line
        .strip_prefix("node_pubkey=")
        .ok_or(BeaconError::EnvelopeFields)?;
    let payload = payload_line
        .strip_prefix("payload=")
        .ok_or(BeaconError::EnvelopeFields)?;
    let sig_hex = sig_line
        .strip_prefix("sig=")
        .ok_or(BeaconError::EnvelopeFields)?;
    let node_pubkey = decode_lower_hex::<32>(node_hex).ok_or(BeaconError::NodePubkey)?;
    let signature = decode_lower_hex::<64>(sig_hex).ok_or(BeaconError::Signature)?;
    let parsed: T = serde_json::from_str(payload).map_err(|_| BeaconError::PayloadJson)?;
    if compact_json(&parsed)? != payload {
        return Err(BeaconError::NonCanonicalPayload);
    }
    let key = VerifyingKey::from_bytes(&node_pubkey).map_err(|_| BeaconError::InvalidNodeKey)?;
    if key.is_weak() {
        return Err(BeaconError::InvalidNodeKey);
    }
    key.verify_strict(
        &signed_bytes(domain, payload),
        &Signature::from_bytes(&signature),
    )
    .map_err(|_| BeaconError::ForgedSignature)?;
    Ok((parsed, node_pubkey))
}

fn decode_lower_hex<const N: usize>(value: &str) -> Option<[u8; N]> {
    if value.len() != N * 2
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return None;
    }
    let mut bytes = [0; N];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        bytes[index] = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
    }
    Some(bytes)
}

fn lower_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(hex, "{byte:02x}").expect("writing to a String cannot fail");
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::*;

    const NODE_ID: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
    const GOLDEN_PAYLOAD: &str = concat!(
        r#"{"protocol_generation":1,"node_id":"d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a","channel_plan":[{"frequency_mhz":5180,"width_mhz":80}],"bootstrap":{"direct_addrs":["10.254.0.1:7000","[fd00::1]:7000"],"relay_url":"https://relay.example/bootstrap"}}"#,
    );

    fn node() -> SigningKey {
        // RFC 8032 test vector 1, shared with https_issuance.rs fixtures.
        SigningKey::from_bytes(
            &decode_lower_hex("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
                .unwrap(),
        )
    }

    fn descriptor() -> CapabilityDescriptor {
        CapabilityDescriptor {
            protocol_generation: 1,
            node_id: NODE_ID.into(),
            channel_plan: vec![ChannelPlan {
                frequency_mhz: 5180,
                width_mhz: 80,
            }],
            bootstrap: BootstrapCoordinates {
                direct_addrs: vec![
                    "10.254.0.1:7000".parse().unwrap(),
                    "[fd00::1]:7000".parse().unwrap(),
                ],
                relay_url: Some("https://relay.example/bootstrap".into()),
            },
        }
    }

    fn locator() -> BeaconLocator {
        verify_beacon(&sign_beacon(&descriptor(), &node()).unwrap()).unwrap()
    }

    /// Sign raw alternate encodings so rejection cannot be attributed to a
    /// stale signature alone.
    fn raw_envelope(payload: &str, domain: &[u8], key: &SigningKey) -> String {
        format!(
            "node_pubkey={}\npayload={}\nsig={}",
            lower_hex(&key.verifying_key().to_bytes()),
            payload,
            lower_hex(&key.sign(&signed_bytes(domain, payload)).to_bytes())
        )
    }

    #[test]
    fn canonical_bytes_and_envelope_round_trip() {
        let descriptor = descriptor();
        assert_eq!(compact_json(&descriptor).unwrap(), GOLDEN_PAYLOAD);
        assert_eq!(
            descriptor.signed_bytes().unwrap(),
            [BEACON_DOMAIN_LINE, GOLDEN_PAYLOAD.as_bytes()].concat()
        );
        let envelope = sign_beacon(&descriptor, &node()).unwrap();
        assert_eq!(verify_beacon(&envelope).unwrap().descriptor(), &descriptor);
        assert_eq!(verify_beacon(&format!("{envelope}\n")).unwrap(), locator());
        assert!(!envelope.ends_with('\n'));
    }

    #[test]
    fn captured_beacon_remains_only_a_locator() {
        let captured = sign_beacon(&descriptor(), &node()).unwrap();
        let first: BeaconLocator = verify_beacon(&captured).unwrap();
        let replay: BeaconLocator = verify_beacon(&captured).unwrap();
        assert_eq!(first, replay);
        let mut challenge = HandshakeChallenge::new(replay, [1; 32]);
        assert!(verify_handshake(&captured, &mut challenge).is_err());
    }

    #[test]
    fn nonce_bound_handshake_is_single_use_and_not_membership() {
        let nonce = [0xa5; 32];
        let response = sign_handshake(&descriptor(), &nonce, &node()).unwrap();
        let handshake = CapabilityHandshake {
            nonce: lower_hex(&nonce),
            beacon: descriptor(),
        };
        assert_eq!(
            handshake.signed_bytes().unwrap(),
            [
                HANDSHAKE_DOMAIN_LINE,
                format!(
                    r#"{{"nonce":"{}","beacon":{GOLDEN_PAYLOAD}}}"#,
                    lower_hex(&nonce)
                )
                .as_bytes(),
            ]
            .concat()
        );
        let mut challenge = HandshakeChallenge::new(locator(), nonce);
        let proof: VerifiedHandshake = verify_handshake(&response, &mut challenge).unwrap();
        assert_eq!(proof.locator(), &locator());
        assert_eq!(proof.nonce(), &nonce);
        assert_eq!(
            verify_handshake(&response, &mut challenge),
            Err(BeaconError::ReplayedNonce)
        );
        let mut fresh = HandshakeChallenge::new(locator(), [0xb6; 32]);
        assert_eq!(
            verify_handshake(&response, &mut fresh),
            Err(BeaconError::NonceMismatch)
        );
        let fresh_response = sign_handshake(&descriptor(), &[0xb6; 32], &node()).unwrap();
        assert!(verify_handshake(&fresh_response, &mut fresh).is_ok());
    }

    #[test]
    fn handshake_binds_exact_locator_and_signing_identity() {
        let mut challenge = HandshakeChallenge::new(locator(), [2; 32]);
        let mut other = descriptor();
        other.bootstrap.relay_url = None;
        let response = sign_handshake(&other, &[2; 32], &node()).unwrap();
        assert_eq!(
            verify_handshake(&response, &mut challenge),
            Err(BeaconError::LocatorMismatch)
        );
        let other_key = SigningKey::from_bytes(&[7; 32]);
        other.node_id = lower_hex(&other_key.verifying_key().to_bytes());
        let response = sign_handshake(&other, &[2; 32], &other_key).unwrap();
        assert_eq!(
            verify_handshake(&response, &mut challenge),
            Err(BeaconError::LocatorMismatch)
        );
        let valid = sign_handshake(&descriptor(), &[2; 32], &node()).unwrap();
        assert!(verify_handshake(&valid, &mut challenge).is_ok());
    }

    #[test]
    fn payload_tampering_and_wrong_identity_fail() {
        let envelope = sign_beacon(&descriptor(), &node()).unwrap();
        for (old, new) in [
            ("\"protocol_generation\":1", "\"protocol_generation\":2"),
            ("5180", "5200"),
            ("80", "40"),
            ("10.254.0.1", "10.254.0.2"),
            ("relay.example", "evil.example"),
            (
                NODE_ID,
                "a75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
            ),
        ] {
            assert!(verify_beacon(&envelope.replacen(old, new, 1)).is_err());
        }
        let wrong_key = SigningKey::from_bytes(&[3; 32]);
        assert_eq!(
            sign_beacon(&descriptor(), &wrong_key),
            Err(BeaconError::NodeIdMismatch)
        );
        assert_eq!(
            sign_handshake(&descriptor(), &[1; 32], &wrong_key),
            Err(BeaconError::NodeIdMismatch)
        );
        let forged_id = raw_envelope(GOLDEN_PAYLOAD, BEACON_DOMAIN_LINE, &wrong_key);
        assert_eq!(verify_beacon(&forged_id), Err(BeaconError::NodeIdMismatch));
    }

    #[test]
    fn correctly_signed_noncanonical_and_unknown_json_is_rejected() {
        for payload in [
            GOLDEN_PAYLOAD.replacen(':', ": ", 1),
            GOLDEN_PAYLOAD.replace('/', "\\/"),
            GOLDEN_PAYLOAD.replace("5180", "5.18e3"),
            GOLDEN_PAYLOAD.replacen(
                "{\"protocol_generation\":1,",
                &format!("{{\"node_id\":\"{NODE_ID}\",\"protocol_generation\":1,"),
                1,
            ),
            GOLDEN_PAYLOAD.replace(
                "\"protocol_generation\":1",
                "\"protocol_generation\":1,\"unknown\":0",
            ),
            GOLDEN_PAYLOAD.replace("\"width_mhz\":80", "\"width_mhz\":80,\"unknown\":0"),
            GOLDEN_PAYLOAD.replace("\"direct_addrs\":", "\"unknown\":0,\"direct_addrs\":"),
            GOLDEN_PAYLOAD.replace("\"protocol_generation\":1", "\"protocol_generation\":01"),
            GOLDEN_PAYLOAD.replace("\"protocol_generation\":1", "\"protocol_generation\":-1"),
        ] {
            let envelope = raw_envelope(&payload, BEACON_DOMAIN_LINE, &node());
            assert!(matches!(
                verify_beacon(&envelope),
                Err(BeaconError::NonCanonicalPayload | BeaconError::PayloadJson)
            ));
        }
        let reordered = GOLDEN_PAYLOAD.replacen(
            &format!(r#""protocol_generation":1,"node_id":"{NODE_ID}""#),
            &format!(r#""node_id":"{NODE_ID}","protocol_generation":1"#),
            1,
        );
        assert_eq!(
            verify_beacon(&raw_envelope(&reordered, BEACON_DOMAIN_LINE, &node())),
            Err(BeaconError::NonCanonicalPayload)
        );
    }

    #[test]
    fn strict_transport_hex_and_signature_checks() {
        let envelope = sign_beacon(&descriptor(), &node()).unwrap();
        for malformed in [
            format!("{envelope}\n\n"),
            envelope.replace('\n', "\r\n"),
            envelope.replace("node_pubkey=", "owner_pubkey="),
            envelope.replace("payload=", "sig="),
            envelope.replace("sig=", "signature="),
            envelope.replace(NODE_ID, &NODE_ID.to_uppercase()),
            envelope.replace("sig=", "sig=A"),
            envelope.replace("sig=", "sig=0"),
            format!("{envelope}0"),
            envelope.lines().take(2).collect::<Vec<_>>().join("\n"),
        ] {
            assert!(verify_beacon(&malformed).is_err());
        }
        let mut lines: Vec<_> = envelope.lines().map(str::to_owned).collect();
        lines[2] = format!("sig={}", "00".repeat(64));
        assert_eq!(
            verify_beacon(&lines.join("\n")),
            Err(BeaconError::ForgedSignature)
        );
        lines[0] = format!("node_pubkey=01{}", "00".repeat(31));
        assert_eq!(
            verify_beacon(&lines.join("\n")),
            Err(BeaconError::InvalidNodeKey)
        );
    }

    #[test]
    fn domain_separation_and_handshake_tampering() {
        let wrong_domain = raw_envelope(GOLDEN_PAYLOAD, HANDSHAKE_DOMAIN_LINE, &node());
        assert_eq!(
            verify_beacon(&wrong_domain),
            Err(BeaconError::ForgedSignature)
        );
        let response = sign_handshake(&descriptor(), &[1; 32], &node()).unwrap();
        let mut challenge = HandshakeChallenge::new(locator(), [1; 32]);
        for tampered in [
            response.replace(&"01".repeat(32), &"02".repeat(32)),
            response.replace("5180", "5200"),
        ] {
            assert_eq!(
                verify_handshake(&tampered, &mut challenge),
                Err(BeaconError::ForgedSignature)
            );
        }
        let payload = compact_json(&CapabilityHandshake {
            nonce: "01".repeat(32),
            beacon: descriptor(),
        })
        .unwrap();
        let wrong_domain = raw_envelope(&payload, BEACON_DOMAIN_LINE, &node());
        assert_eq!(
            verify_handshake(&wrong_domain, &mut challenge),
            Err(BeaconError::ForgedSignature)
        );
        let noncanonical =
            raw_envelope(&payload.replace('/', "\\/"), HANDSHAKE_DOMAIN_LINE, &node());
        assert_eq!(
            verify_handshake(&noncanonical, &mut challenge),
            Err(BeaconError::NonCanonicalPayload)
        );
        assert!(verify_handshake(&response, &mut challenge).is_ok());
    }
}
