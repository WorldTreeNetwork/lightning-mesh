//! Canonical codec / holder-proof conformance tests for local profile v1.
//!
//! Golden vectors are LOCAL, derived by hand from
//! `openspec/changes/add-mesh-admin-capabilities/codec-profile.md`. They are
//! valid until upstream reconciliation (`ikp-6yz.2`) supplies real vectors.
//!
//! These tests distinguish altered-byte rejection from authorization or replay
//! resistance. This crate implements neither, and nothing here asserts either.

use ed25519_dalek::{Signer, SigningKey};
use mjolnir_admin_capabilities::{
    AdminRequest, CodecError, HolderAlgorithm, HolderKey, MAX_RAW_TOKEN_BYTES, MAX_REQUEST_BYTES,
    Operation, TokenIdentity, decode_request, verify_holder_proof,
};

// ---------------------------------------------------------------- helpers

fn hx(s: &str) -> Vec<u8> {
    assert!(
        s.len().is_multiple_of(2),
        "hex literal must be byte aligned"
    );
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex literal"))
        .collect()
}

fn b32(v: u8) -> [u8; 32] {
    [v; 32]
}

/// `token_identity` in the canonical fixtures is an opaque 32-byte field; the
/// codec does not recompute it at decode time, so a literal pattern is used
/// here to keep the golden hex derivable from the profile alone.
fn diagnostics_request() -> AdminRequest {
    AdminRequest::new(
        b32(0x11),
        b32(0x22),
        0,
        Operation::DiagnosticsStatus,
        TokenIdentity::from_digest(b32(0x33)),
        b32(0x44),
        b32(0x55),
    )
    .expect("valid diagnostics request")
}

fn apply_plan_request() -> AdminRequest {
    AdminRequest::new(
        b32(0x11),
        b32(0x22),
        i64::MAX as u64,
        Operation::ApplyNetworkPlan {
            plan_id: b32(0x66),
            expected_revision: b32(0x77),
            plan_digest: b32(0x88),
        },
        TokenIdentity::from_digest(b32(0x33)),
        b32(0x44),
        b32(0x55),
    )
    .expect("valid apply request")
}

// ------------------------------------------------- golden canonical vectors

const GOLDEN_DIAGNOSTICS: &str = "8a776c696768746e696e672d61646d696e2f72657175657374015820111111111111111111111111111111111111111111111111111111111111111158202222222222222222222222222222222222222222222222222222222222222222000080582033333333333333333333333333333333333333333333333333333333333333335820444444444444444444444444444444444444444444444444444444444444444458205555555555555555555555555555555555555555555555555555555555555555";

const GOLDEN_APPLY_PLAN: &str = "8a776c696768746e696e672d61646d696e2f726571756573740158201111111111111111111111111111111111111111111111111111111111111111582022222222222222222222222222222222222222222222222222222222222222221b7fffffffffffffff0183582066666666666666666666666666666666666666666666666666666666666666665820777777777777777777777777777777777777777777777777777777777777777758208888888888888888888888888888888888888888888888888888888888888888582033333333333333333333333333333333333333333333333333333333333333335820444444444444444444444444444444444444444444444444444444444444444458205555555555555555555555555555555555555555555555555555555555555555";

#[test]
fn golden_diagnostics_encodes_to_literal_bytes() {
    assert_eq!(
        diagnostics_request().encode().unwrap(),
        hx(GOLDEN_DIAGNOSTICS)
    );
}

#[test]
fn golden_apply_plan_encodes_to_literal_bytes() {
    assert_eq!(
        apply_plan_request().encode().unwrap(),
        hx(GOLDEN_APPLY_PLAN)
    );
}

#[test]
fn golden_diagnostics_decodes_to_typed_fields() {
    let r = decode_request(&hx(GOLDEN_DIAGNOSTICS)).unwrap();
    assert_eq!(r, diagnostics_request());
    assert_eq!(r.house_id(), &b32(0x11));
    assert_eq!(r.node_id(), &b32(0x22));
    assert_eq!(r.authority_epoch(), 0);
    assert_eq!(r.operation(), &Operation::DiagnosticsStatus);
    assert_eq!(r.token_identity().as_bytes(), &b32(0x33));
    assert_eq!(r.target_challenge(), &b32(0x44));
    assert_eq!(r.request_id(), &b32(0x55));
}

#[test]
fn golden_apply_plan_decodes_to_typed_fields() {
    let r = decode_request(&hx(GOLDEN_APPLY_PLAN)).unwrap();
    assert_eq!(r, apply_plan_request());
    assert_eq!(r.authority_epoch(), i64::MAX as u64);
    assert_eq!(
        r.operation(),
        &Operation::ApplyNetworkPlan {
            plan_id: b32(0x66),
            expected_revision: b32(0x77),
            plan_digest: b32(0x88),
        }
    );
}

// ------------------------------------------------------- integer boundaries

#[test]
fn authority_epoch_boundaries_encode_minimally_and_round_trip() {
    // (epoch, expected minimal CBOR head for that unsigned integer)
    let cases: &[(u64, &str)] = &[
        (0, "00"),
        (23, "17"),
        (24, "1818"),
        (255, "18ff"),
        (256, "190100"),
        (65535, "19ffff"),
        (65536, "1a00010000"),
        (u32::MAX as u64, "1affffffff"),
        (u32::MAX as u64 + 1, "1b0000000100000000"),
        (i64::MAX as u64, "1b7fffffffffffffff"),
    ];
    for (epoch, head) in cases {
        let req = AdminRequest::new(
            b32(0x11),
            b32(0x22),
            *epoch,
            Operation::DiagnosticsStatus,
            TokenIdentity::from_digest(b32(0x33)),
            b32(0x44),
            b32(0x55),
        )
        .unwrap_or_else(|e| panic!("epoch {epoch} must be constructible: {e}"));
        let bytes = req.encode().unwrap();
        // 1 array head + 24 domain + 1 version + 34 house + 34 node = 94
        let expect = hx(head);
        assert_eq!(&bytes[94..94 + expect.len()], &expect[..], "epoch {epoch}");
        assert_eq!(decode_request(&bytes).unwrap().authority_epoch(), *epoch);
    }
}

#[test]
fn authority_epoch_above_i64_max_is_rejected_at_construction() {
    let err = AdminRequest::new(
        b32(0x11),
        b32(0x22),
        i64::MAX as u64 + 1,
        Operation::DiagnosticsStatus,
        TokenIdentity::from_digest(b32(0x33)),
        b32(0x44),
        b32(0x55),
    )
    .unwrap_err();
    assert_eq!(err, CodecError::EpochOutOfRange);
}

#[test]
fn authority_epoch_above_i64_max_is_rejected_at_decode() {
    let err = decode_request(&hx("8a776c696768746e696e672d61646d696e2f726571756573740158201111111111111111111111111111111111111111111111111111111111111111582022222222222222222222222222222222222222222222222222222222222222221b80000000000000000080582033333333333333333333333333333333333333333333333333333333333333335820444444444444444444444444444444444444444444444444444444444444444458205555555555555555555555555555555555555555555555555555555555555555")).unwrap_err();
    assert_eq!(err, CodecError::EpochOutOfRange);
}

// ------------------------------------------------------- malformed encodings

/// Every one of these must be rejected. Each is a distinct malformed shape:
/// non-minimal integers, indefinite lengths, wrong arity, wrong types, wrong
/// byte-string lengths, unknown domain/version/operation, operation/argument
/// mismatch and trailing bytes.
#[test]
fn malformed_encodings_are_rejected() {
    let cases: &[(&str, &str)] = &[
        (
            "non-minimal epoch",
            "8a776c696768746e696e672d61646d696e2f7265717565737401582011111111111111111111111111111111111111111111111111111111111111115820222222222222222222222222222222222222222222222222222222222222222218000080582033333333333333333333333333333333333333333333333333333333333333335820444444444444444444444444444444444444444444444444444444444444444458205555555555555555555555555555555555555555555555555555555555555555",
        ),
        (
            "indefinite arguments array",
            "8a776c696768746e696e672d61646d696e2f7265717565737401582011111111111111111111111111111111111111111111111111111111111111115820222222222222222222222222222222222222222222222222222222222222222200009fff582033333333333333333333333333333333333333333333333333333333333333335820444444444444444444444444444444444444444444444444444444444444444458205555555555555555555555555555555555555555555555555555555555555555",
        ),
        (
            "indefinite outer array",
            "9f776c696768746e696e672d61646d696e2f72657175657374015820111111111111111111111111111111111111111111111111111111111111111158202222222222222222222222222222222222222222222222222222222222222222000080582033333333333333333333333333333333333333333333333333333333333333335820444444444444444444444444444444444444444444444444444444444444444458205555555555555555555555555555555555555555555555555555555555555555ff",
        ),
        (
            "indefinite byte string",
            "8a776c696768746e696e672d61646d696e2f72657175657374015f58201111111111111111111111111111111111111111111111111111111111111111ff58202222222222222222222222222222222222222222222222222222222222222222000080582033333333333333333333333333333333333333333333333333333333333333335820444444444444444444444444444444444444444444444444444444444444444458205555555555555555555555555555555555555555555555555555555555555555",
        ),
        (
            "outer arity 9",
            "89776c696768746e696e672d61646d696e2f726571756573740158201111111111111111111111111111111111111111111111111111111111111111582022222222222222222222222222222222222222222222222222222222222222220000805820333333333333333333333333333333333333333333333333333333333333333358204444444444444444444444444444444444444444444444444444444444444444",
        ),
        (
            "epoch encoded as text",
            "8a776c696768746e696e672d61646d696e2f72657175657374015820111111111111111111111111111111111111111111111111111111111111111158202222222222222222222222222222222222222222222222222222222222222222647a65726f0080582033333333333333333333333333333333333333333333333333333333333333335820444444444444444444444444444444444444444444444444444444444444444458205555555555555555555555555555555555555555555555555555555555555555",
        ),
        (
            "31-byte house id",
            "8a776c696768746e696e672d61646d696e2f7265717565737401581f1111111111111111111111111111111111111111111111111111111111111158202222222222222222222222222222222222222222222222222222222222222222000080582033333333333333333333333333333333333333333333333333333333333333335820444444444444444444444444444444444444444444444444444444444444444458205555555555555555555555555555555555555555555555555555555555555555",
        ),
        (
            "unknown operation 2",
            "8a776c696768746e696e672d61646d696e2f72657175657374015820111111111111111111111111111111111111111111111111111111111111111158202222222222222222222222222222222222222222222222222222222222222222000280582033333333333333333333333333333333333333333333333333333333333333335820444444444444444444444444444444444444444444444444444444444444444458205555555555555555555555555555555555555555555555555555555555555555",
        ),
        (
            "operation 0 with arguments",
            "8a776c696768746e696e672d61646d696e2f7265717565737401582011111111111111111111111111111111111111111111111111111111111111115820222222222222222222222222222222222222222222222222222222222222222200008158206666666666666666666666666666666666666666666666666666666666666666582033333333333333333333333333333333333333333333333333333333333333335820444444444444444444444444444444444444444444444444444444444444444458205555555555555555555555555555555555555555555555555555555555555555",
        ),
        (
            "operation 1 with two arguments",
            "8a776c696768746e696e672d61646d696e2f726571756573740158201111111111111111111111111111111111111111111111111111111111111111582022222222222222222222222222222222222222222222222222222222222222220001825820666666666666666666666666666666666666666666666666666666666666666658207777777777777777777777777777777777777777777777777777777777777777582033333333333333333333333333333333333333333333333333333333333333335820444444444444444444444444444444444444444444444444444444444444444458205555555555555555555555555555555555555555555555555555555555555555",
        ),
        (
            "operation 1 with 31-byte plan digest",
            "8a776c696768746e696e672d61646d696e2f726571756573740158201111111111111111111111111111111111111111111111111111111111111111582022222222222222222222222222222222222222222222222222222222222222220001835820666666666666666666666666666666666666666666666666666666666666666658207777777777777777777777777777777777777777777777777777777777777777581f88888888888888888888888888888888888888888888888888888888888888582033333333333333333333333333333333333333333333333333333333333333335820444444444444444444444444444444444444444444444444444444444444444458205555555555555555555555555555555555555555555555555555555555555555",
        ),
        (
            "unknown domain",
            "8a766c696768746e696e672d61646d696e2f726571756573015820111111111111111111111111111111111111111111111111111111111111111158202222222222222222222222222222222222222222222222222222222222222222000080582033333333333333333333333333333333333333333333333333333333333333335820444444444444444444444444444444444444444444444444444444444444444458205555555555555555555555555555555555555555555555555555555555555555",
        ),
        (
            "unsupported version 2",
            "8a776c696768746e696e672d61646d696e2f72657175657374025820111111111111111111111111111111111111111111111111111111111111111158202222222222222222222222222222222222222222222222222222222222222222000080582033333333333333333333333333333333333333333333333333333333333333335820444444444444444444444444444444444444444444444444444444444444444458205555555555555555555555555555555555555555555555555555555555555555",
        ),
        (
            "trailing byte",
            "8a776c696768746e696e672d61646d696e2f7265717565737401582011111111111111111111111111111111111111111111111111111111111111115820222222222222222222222222222222222222222222222222222222222222222200008058203333333333333333333333333333333333333333333333333333333333333333582044444444444444444444444444444444444444444444444444444444444444445820555555555555555555555555555555555555555555555555555555555555555500",
        ),
        ("empty input", ""),
        ("array head only", "8a"),
        (
            "truncated mid byte string",
            "8a776c696768746e696e672d61646d696e2f7265717565737401582011111111",
        ),
    ];
    for (name, hex) in cases {
        let r = decode_request(&hx(hex));
        assert!(r.is_err(), "{name} must be rejected, got {r:?}");
    }
}

#[test]
fn oversize_input_is_rejected_by_the_pre_parse_ceiling() {
    let mut buf = hx(GOLDEN_DIAGNOSTICS);
    buf.resize(MAX_REQUEST_BYTES + 1, 0x00);
    match decode_request(&buf).unwrap_err() {
        CodecError::RequestTooLarge { actual, max } => {
            assert_eq!(actual, MAX_REQUEST_BYTES + 1);
            assert_eq!(max, MAX_REQUEST_BYTES);
        }
        other => panic!("size ceiling must precede parsing, got {other:?}"),
    }
}

#[test]
fn declared_byte_string_length_cannot_drive_an_allocation() {
    // Byte-string head claims ~4 GiB with no payload behind it.
    let evil = hx("8a776c696768746e696e672d61646d696e2f72657175657374015affffffff");
    assert!(decode_request(&evil).is_err());
    let evil64 = hx("8a776c696768746e696e672d61646d696e2f72657175657374015bffffffffffffffff");
    assert!(decode_request(&evil64).is_err());
}

// ----------------------------------------------------------- token identity

const GOLDEN_TOKEN_RAW: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f";
const GOLDEN_TOKEN_IDENTITY: &str =
    "83bd34b9bdcb268e622c28d62befa03ababd0a1da7c0feb4ee40bfca70bd2e8e";
/// BLAKE3 of the raw token with no domain prefix — must NOT be the identity.
const RAW_TOKEN_NAMESPACE_DIGEST: &str =
    "4eed7141ea4a5cd4b788606bd23f46e212af9cacebacdc7d1f4c6dc7f2511b98";

#[test]
fn golden_token_identity_digest() {
    let id = TokenIdentity::from_raw_token(&hx(GOLDEN_TOKEN_RAW)).unwrap();
    assert_eq!(id.as_bytes()[..], hx(GOLDEN_TOKEN_IDENTITY)[..]);
}

#[test]
fn token_identity_is_not_the_undomained_raw_namespace() {
    let id = TokenIdentity::from_raw_token(&hx(GOLDEN_TOKEN_RAW)).unwrap();
    assert_ne!(id.as_bytes()[..], hx(RAW_TOKEN_NAMESPACE_DIGEST)[..]);
}

#[test]
fn token_identity_covers_every_raw_byte() {
    let base = hx(GOLDEN_TOKEN_RAW);
    let id = TokenIdentity::from_raw_token(&base).unwrap();
    for i in 0..base.len() {
        let mut m = base.clone();
        m[i] ^= 0x01;
        assert_ne!(
            TokenIdentity::from_raw_token(&m).unwrap(),
            id,
            "raw token byte {i} must change the identity"
        );
    }
    // Appending an attenuation-sized suffix also changes identity.
    let mut longer = base.clone();
    longer.push(0xAA);
    assert_ne!(TokenIdentity::from_raw_token(&longer).unwrap(), id);
}

#[test]
fn raw_token_size_bounds() {
    assert!(TokenIdentity::from_raw_token(&[]).is_err());
    assert!(TokenIdentity::from_raw_token(&[0x00]).is_ok());
    assert!(TokenIdentity::from_raw_token(&vec![0x00; MAX_RAW_TOKEN_BYTES]).is_ok());
    assert_eq!(
        TokenIdentity::from_raw_token(&vec![0x00; MAX_RAW_TOKEN_BYTES + 1]).unwrap_err(),
        CodecError::TokenSizeOutOfRange {
            actual: MAX_RAW_TOKEN_BYTES + 1
        }
    );
}

// ------------------------------------------------------ holder fingerprint

const GOLDEN_FP_PREIMAGE: &str = "a263616c676765643235353139636b65795820aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const GOLDEN_FINGERPRINT: &str = "e76aa2e090dd06670f155706bff0b2ddb2844fa0c12d72684958af22df9bac27";
/// BLAKE3 of the bare 32-byte key — the raw-key namespace, which must NOT be used.
const RAW_KEY_NAMESPACE_DIGEST: &str =
    "0360ad5e4f24b851262c924bd9f7cd852c5a48116c9e96fe736910e26067626f";

#[test]
fn golden_fingerprint_preimage_is_algorithm_committing_cbor() {
    let key = HolderKey::new(HolderAlgorithm::Ed25519, b32(0xAA));
    assert_eq!(key.fingerprint_preimage().unwrap(), hx(GOLDEN_FP_PREIMAGE));
}

#[test]
fn golden_fingerprint_digest() {
    let key = HolderKey::new(HolderAlgorithm::Ed25519, b32(0xAA));
    assert_eq!(
        key.fingerprint().unwrap().as_bytes()[..],
        hx(GOLDEN_FINGERPRINT)[..]
    );
}

#[test]
fn fingerprint_is_not_the_raw_key_namespace() {
    let key = HolderKey::new(HolderAlgorithm::Ed25519, b32(0xAA));
    assert_ne!(
        key.fingerprint().unwrap().as_bytes()[..],
        hx(RAW_KEY_NAMESPACE_DIGEST)[..]
    );
}

#[test]
fn only_the_exact_ed25519_tag_resolves() {
    assert_eq!(
        HolderAlgorithm::from_tag("ed25519").unwrap(),
        HolderAlgorithm::Ed25519
    );
    assert_eq!(HolderAlgorithm::Ed25519.tag(), "ed25519");
    for bad in [
        "Ed25519",
        "ED25519",
        "ed25519ph",
        "ed25519-dalek",
        "eddsa",
        "ed448",
        "p256",
        "ml-dsa-44",
        "ed25519 ",
        " ed25519",
        "",
    ] {
        assert_eq!(
            HolderAlgorithm::from_tag(bad).unwrap_err(),
            CodecError::UnsupportedAlgorithm,
            "algorithm tag {bad:?} must be rejected"
        );
    }
}

// ---------------------------------------------------------- holder proof

fn signing_key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn holder_key_of(sk: &SigningKey) -> HolderKey {
    HolderKey::new(HolderAlgorithm::Ed25519, sk.verifying_key().to_bytes())
}

/// Request whose `token_identity` really is the digest of `raw_token`.
fn bound_request(raw_token: &[u8], operation: Operation) -> AdminRequest {
    AdminRequest::new(
        b32(0x11),
        b32(0x22),
        7,
        operation,
        TokenIdentity::from_raw_token(raw_token).unwrap(),
        b32(0x44),
        b32(0x55),
    )
    .unwrap()
}

#[test]
fn valid_proof_positive_control() {
    let sk = signing_key(7);
    let raw = hx(GOLDEN_TOKEN_RAW);
    let req = bound_request(&raw, Operation::DiagnosticsStatus);
    let bytes = req.encode().unwrap();
    let sig = sk.sign(&bytes);

    let proof = verify_holder_proof(&bytes, &raw, &holder_key_of(&sk), &sig.to_bytes()).unwrap();
    assert_eq!(proof.request(), &req);
    assert_eq!(proof.canonical_bytes(), &bytes[..]);
    assert_eq!(proof.token_identity(), req.token_identity());
    assert_eq!(
        proof.holder_fingerprint(),
        &holder_key_of(&sk).fingerprint().unwrap()
    );
}

#[test]
fn valid_proof_positive_control_for_apply_plan() {
    let sk = signing_key(9);
    let raw = vec![0xEE; 300];
    let req = bound_request(
        &raw,
        Operation::ApplyNetworkPlan {
            plan_id: b32(0x66),
            expected_revision: b32(0x77),
            plan_digest: b32(0x88),
        },
    );
    let bytes = req.encode().unwrap();
    let sig = sk.sign(&bytes);
    assert!(verify_holder_proof(&bytes, &raw, &holder_key_of(&sk), &sig.to_bytes()).is_ok());
}

#[test]
fn mutating_any_signed_byte_breaks_the_proof() {
    let sk = signing_key(7);
    let raw = hx(GOLDEN_TOKEN_RAW);
    let holder = holder_key_of(&sk);
    for operation in [
        Operation::DiagnosticsStatus,
        Operation::ApplyNetworkPlan {
            plan_id: b32(0x66),
            expected_revision: b32(0x77),
            plan_digest: b32(0x88),
        },
    ] {
        let bytes = bound_request(&raw, operation).encode().unwrap();
        let sig = sk.sign(&bytes).to_bytes();
        assert!(verify_holder_proof(&bytes, &raw, &holder, &sig).is_ok());
        for i in 0..bytes.len() {
            let mut m = bytes.clone();
            m[i] ^= 0x01;
            assert!(
                verify_holder_proof(&m, &raw, &holder, &sig).is_err(),
                "mutating signed byte {i} of {operation:?} must break the proof"
            );
        }
    }
}

#[test]
fn mutating_any_signed_field_breaks_the_proof() {
    let sk = signing_key(7);
    let holder = holder_key_of(&sk);
    let raw = hx(GOLDEN_TOKEN_RAW);
    let other_raw = vec![0x5A; 64];
    let original = bound_request(&raw, Operation::DiagnosticsStatus);
    let bytes = original.encode().unwrap();
    let sig = sk.sign(&bytes).to_bytes();

    let variants: Vec<(&str, AdminRequest)> = vec![
        (
            "house_id",
            AdminRequest::new(
                b32(0x12),
                b32(0x22),
                7,
                Operation::DiagnosticsStatus,
                *original.token_identity(),
                b32(0x44),
                b32(0x55),
            )
            .unwrap(),
        ),
        (
            "node_id",
            AdminRequest::new(
                b32(0x11),
                b32(0x23),
                7,
                Operation::DiagnosticsStatus,
                *original.token_identity(),
                b32(0x44),
                b32(0x55),
            )
            .unwrap(),
        ),
        (
            "authority_epoch",
            AdminRequest::new(
                b32(0x11),
                b32(0x22),
                8,
                Operation::DiagnosticsStatus,
                *original.token_identity(),
                b32(0x44),
                b32(0x55),
            )
            .unwrap(),
        ),
        (
            "operation",
            AdminRequest::new(
                b32(0x11),
                b32(0x22),
                7,
                Operation::ApplyNetworkPlan {
                    plan_id: b32(0x66),
                    expected_revision: b32(0x77),
                    plan_digest: b32(0x88),
                },
                *original.token_identity(),
                b32(0x44),
                b32(0x55),
            )
            .unwrap(),
        ),
        (
            "token_identity",
            AdminRequest::new(
                b32(0x11),
                b32(0x22),
                7,
                Operation::DiagnosticsStatus,
                TokenIdentity::from_raw_token(&other_raw).unwrap(),
                b32(0x44),
                b32(0x55),
            )
            .unwrap(),
        ),
        (
            "target_challenge",
            AdminRequest::new(
                b32(0x11),
                b32(0x22),
                7,
                Operation::DiagnosticsStatus,
                *original.token_identity(),
                b32(0x45),
                b32(0x55),
            )
            .unwrap(),
        ),
        (
            "request_id",
            AdminRequest::new(
                b32(0x11),
                b32(0x22),
                7,
                Operation::DiagnosticsStatus,
                *original.token_identity(),
                b32(0x44),
                b32(0x56),
            )
            .unwrap(),
        ),
    ];

    for (field, variant) in variants {
        let vb = variant.encode().unwrap();
        assert_ne!(vb, bytes, "{field} variant must differ on the wire");
        assert!(
            verify_holder_proof(&vb, &raw, &holder, &sig).is_err(),
            "mutated {field} must break the proof"
        );
    }
}

#[test]
fn presenting_a_different_token_is_rejected() {
    let sk = signing_key(7);
    let raw = hx(GOLDEN_TOKEN_RAW);
    let bytes = bound_request(&raw, Operation::DiagnosticsStatus)
        .encode()
        .unwrap();
    let sig = sk.sign(&bytes).to_bytes();
    let holder = holder_key_of(&sk);

    // Attenuation-style suffix: same prefix, different exact bytes.
    let mut attenuated = raw.clone();
    attenuated.extend_from_slice(&[0xAB; 16]);
    assert_eq!(
        verify_holder_proof(&bytes, &attenuated, &holder, &sig).unwrap_err(),
        CodecError::TokenIdentityMismatch
    );

    // Single-bit change anywhere in the token.
    for i in 0..raw.len() {
        let mut m = raw.clone();
        m[i] ^= 0x01;
        assert_eq!(
            verify_holder_proof(&bytes, &m, &holder, &sig).unwrap_err(),
            CodecError::TokenIdentityMismatch,
            "token byte {i}"
        );
    }
}

#[test]
fn a_different_holder_key_does_not_verify() {
    let sk = signing_key(7);
    let raw = hx(GOLDEN_TOKEN_RAW);
    let bytes = bound_request(&raw, Operation::DiagnosticsStatus)
        .encode()
        .unwrap();
    let sig = sk.sign(&bytes).to_bytes();
    assert_eq!(
        verify_holder_proof(&bytes, &raw, &holder_key_of(&signing_key(8)), &sig).unwrap_err(),
        CodecError::SignatureInvalid
    );
}

#[test]
fn weak_holder_keys_are_rejected() {
    let sk = signing_key(7);
    let raw = hx(GOLDEN_TOKEN_RAW);
    let bytes = bound_request(&raw, Operation::DiagnosticsStatus)
        .encode()
        .unwrap();
    let sig = sk.sign(&bytes).to_bytes();
    // Canonical encoding of a small-order (order-4) point.
    let weak = HolderKey::new(HolderAlgorithm::Ed25519, [0u8; 32]);
    assert_eq!(
        verify_holder_proof(&bytes, &raw, &weak, &sig).unwrap_err(),
        CodecError::WeakHolderKey
    );
}

#[test]
fn undecodable_holder_keys_are_rejected() {
    let sk = signing_key(7);
    let raw = hx(GOLDEN_TOKEN_RAW);
    let bytes = bound_request(&raw, Operation::DiagnosticsStatus)
        .encode()
        .unwrap();
    let sig = sk.sign(&bytes).to_bytes();
    // y = 2 is not the y-coordinate of any curve point: (y^2-1)/(d*y^2+1) is a
    // non-square, so decompression fails outright.
    let mut undecodable = [0u8; 32];
    undecodable[0] = 0x02;
    assert_eq!(
        verify_holder_proof(
            &bytes,
            &raw,
            &HolderKey::new(HolderAlgorithm::Ed25519, undecodable),
            &sig
        )
        .unwrap_err(),
        CodecError::MalformedHolderKey
    );

    // An all-ones encoding carries a non-canonical y. curve25519-dalek reduces
    // rather than rejecting it, so it decompresses to a normal-order point;
    // it is still rejected, by the signature check rather than by key parsing.
    assert_eq!(
        verify_holder_proof(
            &bytes,
            &raw,
            &HolderKey::new(HolderAlgorithm::Ed25519, [0xFF; 32]),
            &sig
        )
        .unwrap_err(),
        CodecError::SignatureInvalid
    );
}

#[test]
fn bad_signatures_are_rejected() {
    let sk = signing_key(7);
    let raw = hx(GOLDEN_TOKEN_RAW);
    let bytes = bound_request(&raw, Operation::DiagnosticsStatus)
        .encode()
        .unwrap();
    let holder = holder_key_of(&sk);
    let good = sk.sign(&bytes).to_bytes();

    assert_eq!(
        verify_holder_proof(&bytes, &raw, &holder, &[0u8; 64]).unwrap_err(),
        CodecError::SignatureInvalid
    );
    for len in [0usize, 32, 63, 65, 128] {
        assert_eq!(
            verify_holder_proof(&bytes, &raw, &holder, &vec![0u8; len]).unwrap_err(),
            CodecError::MalformedSignature,
            "signature length {len} must be rejected as malformed"
        );
    }
    for i in 0..good.len() {
        let mut m = good;
        m[i] ^= 0x01;
        assert!(
            verify_holder_proof(&bytes, &raw, &holder, &m).is_err(),
            "mutated signature byte {i} must not verify"
        );
    }
}

/// F1: the canonical gate must reject non-canonical bytes even when the
/// presented signature is genuinely valid over exactly those bytes. Hostile
/// bytes are never normalised and then accepted.
#[test]
fn canonical_gate_rejects_non_canonical_bytes_with_a_valid_signature() {
    let sk = signing_key(7);
    let raw = hx(GOLDEN_TOKEN_RAW);
    let canonical = bound_request(&raw, Operation::DiagnosticsStatus)
        .encode()
        .unwrap();

    // Re-encode epoch 7 non-minimally as 0x1807 instead of 0x07.
    let mut non_canonical = canonical.clone();
    assert_eq!(non_canonical[94], 0x07, "epoch head offset");
    non_canonical.splice(94..95, [0x18, 0x07]);

    let sig = sk.sign(&non_canonical).to_bytes();
    // The attacker's signature really is valid over these exact bytes:
    assert!(
        ed25519_dalek::VerifyingKey::from_bytes(&sk.verifying_key().to_bytes())
            .unwrap()
            .verify_strict(&non_canonical, &ed25519_dalek::Signature::from_bytes(&sig))
            .is_ok()
    );
    assert_eq!(
        verify_holder_proof(&non_canonical, &raw, &holder_key_of(&sk), &sig).unwrap_err(),
        CodecError::NonCanonical
    );
}
