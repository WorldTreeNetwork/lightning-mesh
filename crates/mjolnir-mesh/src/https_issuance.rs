// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors
// Lightning Mesh is dual-licensed (AGPL-3.0-or-later or commercial); see LICENSE
// and COMMERCIAL-LICENSE.md at the repository root.

//! Owner authorization for HTTPS certificate issuance. No DNS or persistence.

use std::collections::BTreeSet;

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::https_label;

pub const ISSUANCE_DOMAIN_LINE: &[u8] = b"mjolnir-https-issuance:v1\n";
pub const MAX_ISSUANCE_LIFETIME_SECS: u64 = 3600;

/// Field declaration order is the protocol's compact JSON key order.
/// The owner is bound through `fqdn`, not an extra payload field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IssuanceAuthorization {
    /// Normalized ASCII `a-<owner-label>.<mesh>.<zone>` or `n-...` name.
    pub fqdn: String,
    /// Unpadded base64url SHA-256 DNS-01 digest (43 characters).
    pub txt_digest: String,
    pub acme_account: String,
    /// Lowercase hexadecimal SHA-256 of the host-local CSR's SPKI.
    pub csr_spki_sha256: String,
    /// Lowercase hexadecimal 16-byte nonce.
    pub nonce: String,
    /// Unix seconds; strictly after the verifier's `now`, at most an hour out.
    pub expires_at: u64,
}

impl IssuanceAuthorization {
    /// Domain line plus canonical payload, with no trailing newline.
    pub fn signed_bytes(&self) -> Result<Vec<u8>, IssuanceError> {
        Ok(signed_bytes(&self.compact_json()?))
    }

    fn compact_json(&self) -> Result<String, IssuanceError> {
        serde_json::to_string(self).map_err(|_| IssuanceError::PayloadJson)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedIssuanceAuthorization {
    pub authorization: IssuanceAuthorization,
    pub owner_pubkey: [u8; 32],
    /// The exact canonical payload covered by the owner signature.
    pub payload: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IssuanceError {
    #[error("issuance envelope must be exactly three LF-separated lines")]
    Envelope,
    #[error("issuance envelope field names or ordering are invalid")]
    EnvelopeFields,
    #[error("owner public key must be 64 lowercase hexadecimal characters")]
    OwnerPubkey,
    #[error("signature must be 128 lowercase hexadecimal characters")]
    Signature,
    #[error("issuance payload is not valid protocol JSON")]
    PayloadJson,
    #[error("issuance payload is not canonical compact JSON")]
    NonCanonicalPayload,
    #[error("owner public key is not a valid Ed25519 verification key")]
    InvalidOwnerKey,
    #[error("issuance signature did not verify")]
    ForgedSignature,
    #[error("issuance FQDN is not normalized ASCII DNS form")]
    Fqdn,
    #[error("issuance FQDN label is not derived from its full owner public key")]
    WrongOwnerLabel,
    #[error("TXT digest must be an unpadded base64url SHA-256 digest")]
    TxtDigest,
    #[error("ACME account must be an ASCII HTTP(S) URL without whitespace")]
    AcmeAccount,
    #[error("CSR SPKI SHA-256 must be 64 lowercase hexadecimal characters")]
    CsrSpkiHash,
    #[error("CSR SPKI SHA-256 does not match the authorization")]
    CsrMismatch,
    #[error("nonce must be 32 lowercase hexadecimal characters")]
    Nonce,
    #[error("issuance authorization has expired")]
    Expired,
    #[error("issuance authorization expires more than one hour from now")]
    Lifetime,
    #[error("issuance nonce has already been used")]
    ReplayedNonce,
    #[error("owner label collides with a different retained owner key")]
    LabelCollision,
}

/// Sign the canonical three-line envelope. Time and retained history are
/// evaluated by the verifier, using its caller-supplied clock and sets.
pub fn sign_issuance_envelope(
    authorization: &IssuanceAuthorization,
    owner_key: &SigningKey,
) -> Result<String, IssuanceError> {
    let owner_pubkey = owner_key.verifying_key().to_bytes();
    validate_authorization(authorization, &owner_pubkey, https_label)?;
    let payload = authorization.compact_json()?;
    let signature = owner_key.sign(&signed_bytes(&payload));
    Ok(format!(
        "owner_pubkey={}\npayload={}\nsig={}",
        lower_hex(&owner_pubkey),
        payload,
        lower_hex(&signature.to_bytes())
    ))
}

/// Verify without changing history. `csr_spki_sha256` must be computed from
/// the actual CSR by the caller, not copied from the authorization.
///
/// `retained_owner_pubkeys` is AliasTable's append-only full-key history,
/// including expired and tombstoned owners. Callers must atomically consume
/// the returned nonce with their challenge write and retain the owner key;
/// concurrent calls using the same snapshot do not provide replay protection.
pub fn verify_issuance_envelope(
    envelope: &str,
    csr_spki_sha256: &[u8; 32],
    now: u64,
    seen_nonces: &BTreeSet<String>,
    retained_owner_pubkeys: &BTreeSet<[u8; 32]>,
) -> Result<VerifiedIssuanceAuthorization, IssuanceError> {
    verify_issuance_envelope_with(
        envelope,
        csr_spki_sha256,
        now,
        seen_nonces,
        retained_owner_pubkeys,
        https_label,
    )
}

fn verify_issuance_envelope_with(
    envelope: &str,
    csr_spki_sha256: &[u8; 32],
    now: u64,
    seen_nonces: &BTreeSet<String>,
    retained_owner_pubkeys: &BTreeSet<[u8; 32]>,
    labeler: fn(&[u8]) -> String,
) -> Result<VerifiedIssuanceAuthorization, IssuanceError> {
    // Match verify_alias_envelope's transport and reserialization discipline,
    // including its allowance for one conventional final file newline.
    if envelope.contains('\r') {
        return Err(IssuanceError::Envelope);
    }
    let envelope = envelope.strip_suffix('\n').unwrap_or(envelope);
    let mut lines = envelope.split('\n');
    let owner_line = lines.next().ok_or(IssuanceError::Envelope)?;
    let payload_line = lines.next().ok_or(IssuanceError::Envelope)?;
    let sig_line = lines.next().ok_or(IssuanceError::Envelope)?;
    if lines.next().is_some() {
        return Err(IssuanceError::Envelope);
    }
    let owner_hex = owner_line
        .strip_prefix("owner_pubkey=")
        .ok_or(IssuanceError::EnvelopeFields)?;
    let payload = payload_line
        .strip_prefix("payload=")
        .ok_or(IssuanceError::EnvelopeFields)?;
    let sig_hex = sig_line
        .strip_prefix("sig=")
        .ok_or(IssuanceError::EnvelopeFields)?;
    let owner_pubkey = decode_lower_hex::<32>(owner_hex).ok_or(IssuanceError::OwnerPubkey)?;
    let signature_bytes = decode_lower_hex::<64>(sig_hex).ok_or(IssuanceError::Signature)?;
    let authorization: IssuanceAuthorization =
        serde_json::from_str(payload).map_err(|_| IssuanceError::PayloadJson)?;
    if authorization.compact_json()? != payload {
        return Err(IssuanceError::NonCanonicalPayload);
    }
    let verifying_key =
        VerifyingKey::from_bytes(&owner_pubkey).map_err(|_| IssuanceError::InvalidOwnerKey)?;
    if verifying_key.is_weak() {
        return Err(IssuanceError::InvalidOwnerKey);
    }
    verifying_key
        .verify_strict(
            &signed_bytes(payload),
            &Signature::from_bytes(&signature_bytes),
        )
        .map_err(|_| IssuanceError::ForgedSignature)?;
    validate_authorization(&authorization, &owner_pubkey, labeler)?;
    if decode_lower_hex::<32>(&authorization.csr_spki_sha256).as_ref() != Some(csr_spki_sha256) {
        return Err(IssuanceError::CsrMismatch);
    }
    let remaining = authorization
        .expires_at
        .checked_sub(now)
        .filter(|&seconds| seconds > 0)
        .ok_or(IssuanceError::Expired)?;
    if remaining > MAX_ISSUANCE_LIFETIME_SECS {
        return Err(IssuanceError::Lifetime);
    }
    if seen_nonces.contains(&authorization.nonce) {
        return Err(IssuanceError::ReplayedNonce);
    }
    let label = labeler(&owner_pubkey);
    if retained_owner_pubkeys
        .iter()
        .any(|retained| retained != &owner_pubkey && labeler(retained) == label)
    {
        return Err(IssuanceError::LabelCollision);
    }
    Ok(VerifiedIssuanceAuthorization {
        authorization,
        owner_pubkey,
        payload: payload.to_owned(),
    })
}

fn signed_bytes(payload: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(ISSUANCE_DOMAIN_LINE.len() + payload.len());
    bytes.extend_from_slice(ISSUANCE_DOMAIN_LINE);
    bytes.extend_from_slice(payload.as_bytes());
    bytes
}

fn validate_authorization(
    authorization: &IssuanceAuthorization,
    owner_pubkey: &[u8; 32],
    labeler: fn(&[u8]) -> String,
) -> Result<(), IssuanceError> {
    let fqdn = &authorization.fqdn;
    if fqdn.len() > 253 || !fqdn.is_ascii() {
        return Err(IssuanceError::Fqdn);
    }
    let labels: Vec<_> = fqdn.split('.').collect();
    if labels.len() < 3
        || labels.iter().any(|label| {
            label.is_empty()
                || label.len() > 63
                || label.starts_with('-')
                || label.ends_with('-')
                || !label
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
    {
        return Err(IssuanceError::Fqdn);
    }
    let owner_label = labels[0]
        .strip_prefix("a-")
        .or_else(|| labels[0].strip_prefix("n-"))
        .ok_or(IssuanceError::Fqdn)?;
    if owner_label != labeler(owner_pubkey) {
        return Err(IssuanceError::WrongOwnerLabel);
    }
    // A SHA-256 base64url digest has 43 sextets, with two zero pad bits.
    let digest = authorization.txt_digest.as_bytes();
    const BASE64URL: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    if digest.len() != 43
        || !digest.iter().all(|b| BASE64URL.contains(b))
        || BASE64URL.iter().position(|b| b == &digest[42]).unwrap() % 4 != 0
    {
        return Err(IssuanceError::TxtDigest);
    }
    let account = &authorization.acme_account;
    let account_path = account
        .strip_prefix("https://")
        .or_else(|| account.strip_prefix("http://"))
        .ok_or(IssuanceError::AcmeAccount)?;
    if account_path.split('/').next().unwrap_or("").is_empty()
        || !account.bytes().all(|b| b.is_ascii_graphic())
    {
        return Err(IssuanceError::AcmeAccount);
    }
    decode_lower_hex::<32>(&authorization.csr_spki_sha256).ok_or(IssuanceError::CsrSpkiHash)?;
    decode_lower_hex::<16>(&authorization.nonce).ok_or(IssuanceError::Nonce)?;
    Ok(())
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

    const NOW: u64 = 1_800_000_000;
    const CSR: [u8; 32] = [0x42; 32];
    const GOLDEN_PAYLOAD: &str = concat!(
        r#"{"fqdn":"a-nqyqieti6rywbhdz.house.mesh.worldtree.network","txt_digest":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","acme_account":"https://acme.example/acct/1","csr_spki_sha256":""#,
        r#"4242424242424242424242424242424242424242424242424242424242424242","nonce":"000102030405060708090a0b0c0d0e0f","expires_at":1800003600}"#,
    );

    fn owner() -> SigningKey {
        // RFC 8032 vector 1, also used by the shared label fixtures.
        SigningKey::from_bytes(
            &decode_lower_hex("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
                .unwrap(),
        )
    }

    fn authorization() -> IssuanceAuthorization {
        serde_json::from_str(GOLDEN_PAYLOAD).unwrap()
    }

    fn raw_envelope(payload: &str, key: &SigningKey) -> String {
        raw_domain_envelope(payload, key, ISSUANCE_DOMAIN_LINE)
    }

    fn raw_domain_envelope(payload: &str, key: &SigningKey, domain: &[u8]) -> String {
        let bytes = [domain, payload.as_bytes()].concat();
        format!(
            "owner_pubkey={}\npayload={}\nsig={}",
            lower_hex(&key.verifying_key().to_bytes()),
            payload,
            lower_hex(&key.sign(&bytes).to_bytes())
        )
    }

    fn verify(envelope: &str) -> Result<VerifiedIssuanceAuthorization, IssuanceError> {
        verify_issuance_envelope(envelope, &CSR, NOW, &BTreeSet::new(), &BTreeSet::new())
    }

    #[test]
    fn https_issuance_golden_signed_bytes() {
        let auth = authorization();
        let expected = [
            b"mjolnir-https-issuance:v1\n".as_slice(),
            GOLDEN_PAYLOAD.as_bytes(),
        ]
        .concat();
        assert_eq!(auth.signed_bytes().unwrap(), expected);
        assert_eq!(
            lower_hex(&owner().verifying_key().to_bytes()),
            "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
        );
        let envelope = sign_issuance_envelope(&auth, &owner()).unwrap();
        assert_eq!(envelope, raw_envelope(GOLDEN_PAYLOAD, &owner()));
        // Independently generated with OpenSSL Ed25519 pkeyutl -sign -rawin
        // from the RFC 8032 seed and the literal domain + payload above.
        assert_eq!(
            envelope.split('\n').nth(2).unwrap(),
            concat!(
                "sig=3fd98593b3e215b5ff2502f6d270b0e61ca988d62f26c6949f9965979ecc548f",
                "862723c9e323e42c2c0a86751c86e697c7088a0a220b985c56a286017f653a02"
            )
        );
        let verified = verify(&envelope).unwrap();
        assert_eq!(verified.authorization, auth);
        assert_eq!(verified.payload, GOLDEN_PAYLOAD);
        assert_eq!(verified.owner_pubkey, owner().verifying_key().to_bytes());
        assert!(verify(&(envelope + "\n")).is_ok());
    }

    #[test]
    fn https_issuance_replay_and_csr_binding() {
        let auth = authorization();
        let envelope = sign_issuance_envelope(&auth, &owner()).unwrap();
        let nonces = BTreeSet::from([auth.nonce.clone()]);
        assert_eq!(
            verify_issuance_envelope(&envelope, &CSR, NOW, &nonces, &BTreeSet::new()),
            Err(IssuanceError::ReplayedNonce)
        );
        assert_eq!(
            verify_issuance_envelope(
                &envelope,
                &[0x43; 32],
                NOW,
                &BTreeSet::new(),
                &BTreeSet::new()
            ),
            Err(IssuanceError::CsrMismatch)
        );
        assert_eq!(nonces, BTreeSet::from([auth.nonce]));
    }

    #[test]
    fn https_issuance_expiry_boundaries_and_overflow() {
        for (expires_at, now, expected) in [
            (NOW - 1, NOW, Some(IssuanceError::Expired)),
            (NOW, NOW, Some(IssuanceError::Expired)),
            (NOW + 1, NOW, None),
            (NOW + 3600, NOW, None),
            (NOW + 3601, NOW, Some(IssuanceError::Lifetime)),
            (u64::MAX, NOW, Some(IssuanceError::Lifetime)),
            (u64::MAX, u64::MAX - 1, None),
            (u64::MAX, u64::MAX, Some(IssuanceError::Expired)),
        ] {
            let auth = IssuanceAuthorization {
                expires_at,
                ..authorization()
            };
            let envelope = sign_issuance_envelope(&auth, &owner()).unwrap();
            let result =
                verify_issuance_envelope(&envelope, &CSR, now, &BTreeSet::new(), &BTreeSet::new());
            assert_eq!(result.err(), expected, "expiry={expires_at}, now={now}");
        }
    }

    #[test]
    fn https_issuance_owner_substitution_and_domain_separation() {
        let auth = authorization();
        let other = SigningKey::from_bytes(&[7; 32]);
        let envelope = sign_issuance_envelope(&auth, &owner()).unwrap();
        let substituted = envelope.replacen(
            &lower_hex(&owner().verifying_key().to_bytes()),
            &lower_hex(&other.verifying_key().to_bytes()),
            1,
        );
        assert_eq!(verify(&substituted), Err(IssuanceError::ForgedSignature));
        // Correct signature by an envelope owner that does not own payload.fqdn.
        assert_eq!(
            verify(&raw_envelope(GOLDEN_PAYLOAD, &other)),
            Err(IssuanceError::WrongOwnerLabel)
        );
        assert_eq!(
            sign_issuance_envelope(&auth, &other),
            Err(IssuanceError::WrongOwnerLabel)
        );
        assert_eq!(
            verify(&raw_domain_envelope(
                GOLDEN_PAYLOAD,
                &owner(),
                crate::https_alias::ALIAS_DOMAIN_LINE
            )),
            Err(IssuanceError::ForgedSignature)
        );
        assert_eq!(
            verify(&envelope.replace("/acct/1", "/acct/2")),
            Err(IssuanceError::ForgedSignature)
        );
    }

    #[test]
    fn https_issuance_rejects_noncanonical_payloads_even_when_signed() {
        let reordered = GOLDEN_PAYLOAD.replacen(
            r#""nonce":"000102030405060708090a0b0c0d0e0f","expires_at":1800003600"#,
            r#""expires_at":1800003600,"nonce":"000102030405060708090a0b0c0d0e0f"#,
            1,
        );
        for payload in [
            GOLDEN_PAYLOAD.replacen(':', ": ", 1),
            GOLDEN_PAYLOAD.replace('/', r"\/"),
            GOLDEN_PAYLOAD.replace("1800003600", "01800003600"),
            GOLDEN_PAYLOAD.replace("1800003600", "1800003600.0"),
            GOLDEN_PAYLOAD.replace("1800003600", "18000036e2"),
            GOLDEN_PAYLOAD.replace("1800003600", "\"1800003600\""),
            GOLDEN_PAYLOAD.replace("1800003600", "-1"),
            GOLDEN_PAYLOAD.replace("1800003600", "18446744073709551616"),
            GOLDEN_PAYLOAD.replacen("house", r"\u0068ouse", 1),
            GOLDEN_PAYLOAD.replacen('{', r#"{"owner_pubkey":"ignored","#, 1),
            GOLDEN_PAYLOAD.replacen('{', r#"{"expires_at":1800003600,"#, 1),
            GOLDEN_PAYLOAD.replace(r#""nonce":"000102030405060708090a0b0c0d0e0f","#, ""),
            format!("{GOLDEN_PAYLOAD} "),
            reordered,
        ] {
            assert!(
                verify(&raw_envelope(&payload, &owner())).is_err(),
                "accepted {payload}"
            );
        }
    }

    #[test]
    fn https_issuance_rejects_malformed_envelopes_and_keys() {
        let envelope = sign_issuance_envelope(&authorization(), &owner()).unwrap();
        let lines: Vec<_> = envelope.split('\n').collect();
        for malformed in [
            envelope.replace('\n', "\r\n"),
            format!("{envelope}\n\n"),
            format!("{envelope}\nextra=value"),
            lines[..2].join("\n"),
            [lines[1], lines[0], lines[2]].join("\n"),
            envelope.replacen("owner_pubkey=", "owner_pubkey= ", 1),
            envelope.replacen("d75a", "D75A", 1),
            envelope.replacen("sig=", "sig=A", 1),
            envelope.replacen("sig=", "signature=", 1),
            envelope.replacen(
                &lower_hex(&owner().verifying_key().to_bytes()),
                &"00".repeat(32),
                1,
            ),
        ] {
            assert!(verify(&malformed).is_err(), "accepted {malformed}");
        }
    }

    #[test]
    fn https_issuance_validates_identifiers_and_supports_router_names() {
        for (field, value) in [
            ("fqdn", "a-nqyqieti6rywbhdz.house.mesh.worldtree.network."),
            ("fqdn", "a-nqyqieti6rywbhdz.HOUSE.mesh.worldtree.network"),
            ("fqdn", "a-nqyqieti6rywbhdz.housé.mesh.worldtree.network"),
            ("fqdn", "*.house.mesh.worldtree.network"),
            ("fqdn", "a-nqyqieti6rywbhdz..mesh.worldtree.network"),
            ("fqdn", "a-nqyqieti6rywbhdz.-house.mesh.worldtree.network"),
            ("txt_digest", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAB"),
            ("txt_digest", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="),
            ("acme_account", "https:///acct/1"),
            ("acme_account", "https://acme.example/ acct/1"),
            ("csr_spki_sha256", "4242"),
            ("csr_spki_sha256", &"AB".repeat(32)),
            ("nonce", "00"),
            ("nonce", "000102030405060708090A0B0C0D0E0F"),
        ] {
            let mut json = serde_json::to_value(authorization()).unwrap();
            json[field] = value.into();
            let auth: IssuanceAuthorization = serde_json::from_value(json).unwrap();
            assert!(
                sign_issuance_envelope(&auth, &owner()).is_err(),
                "accepted {field}={value}"
            );
            assert!(verify(&raw_envelope(&auth.compact_json().unwrap(), &owner())).is_err());
        }
        let mut auth = authorization();
        auth.fqdn = auth.fqdn.replacen("a-", "n-", 1);
        assert!(verify(&sign_issuance_envelope(&auth, &owner()).unwrap()).is_ok());
    }

    #[test]
    fn https_issuance_retained_collision_fails_closed_for_both_owners() {
        // As in AliasTable tests, inject a collision rather than finding an
        // 80-bit preimage. Production always uses the full-key https_label.
        fn colliding_label(_: &[u8]) -> String {
            "nqyqieti6rywbhdz".to_owned()
        }
        let owners = [owner(), SigningKey::from_bytes(&[7; 32])];
        let history: BTreeSet<_> = owners
            .iter()
            .map(|key| key.verifying_key().to_bytes())
            .collect();
        for key in &owners {
            let envelope = raw_envelope(GOLDEN_PAYLOAD, key);
            let verify_with_history = |history| {
                verify_issuance_envelope_with(
                    &envelope,
                    &CSR,
                    NOW,
                    &BTreeSet::new(),
                    history,
                    colliding_label,
                )
            };
            assert_eq!(
                verify_with_history(&history),
                Err(IssuanceError::LabelCollision)
            );
            let own_history = BTreeSet::from([key.verifying_key().to_bytes()]);
            assert!(verify_with_history(&own_history).is_ok());
        }
        let envelope = sign_issuance_envelope(&authorization(), &owner()).unwrap();
        // An unrelated retained owner is not a collision with the real labeler.
        assert!(verify_issuance_envelope(&envelope, &CSR, NOW, &BTreeSet::new(), &history).is_ok());
        assert_eq!(history.len(), 2);
    }
}
