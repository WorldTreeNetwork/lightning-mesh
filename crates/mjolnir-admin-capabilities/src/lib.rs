//! Canonical mesh administrative request codec and holder-proof verification.
//!
//! Local profile v1, pinned by
//! `openspec/changes/add-mesh-admin-capabilities/codec-profile.md`. This is a
//! local pin, not upstream IdentiKey conformance.
//!
//! This crate establishes **possession only**. A [`VerifiedHolderProof`] proves
//! that the holder of a key signed those exact canonical bytes and that the
//! presented raw token hashes to the identity named inside them. It is not
//! authorization: issuer/rights binding, trusted time, expiry, revocation,
//! target-challenge matching and durable one-use replay reservation are separate
//! mandatory checks that this crate deliberately does not implement or expose.

#![forbid(unsafe_code)]

use ed25519_dalek::{Signature, VerifyingKey};
use minicbor::{Decoder, Encoder};
use thiserror::Error;

/// Domain string that opens every canonical request array.
pub const REQUEST_DOMAIN: &str = "lightning-admin/request";

/// Canonical request profile version.
pub const REQUEST_VERSION: u64 = 1;

/// Hard pre-parse ceiling on a canonical request, applied before any parsing.
pub const MAX_REQUEST_BYTES: usize = 1024;

/// Domain-separation prefix for the raw-token identity digest.
pub const TOKEN_IDENTITY_DOMAIN: &[u8] = b"lightning-admin/token/v1";

/// Minimum accepted raw token size, in bytes.
pub const MIN_RAW_TOKEN_BYTES: usize = 1;

/// Maximum accepted raw token size, in bytes.
pub const MAX_RAW_TOKEN_BYTES: usize = 65536;

/// Inclusive upper bound on `authority_epoch`, for Biscuit Datalog compatibility.
pub const MAX_AUTHORITY_EPOCH: u64 = i64::MAX as u64;

/// Errors produced by this codec. Every failure is typed; no boolean results.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CodecError {
    #[error("request is {actual} bytes, exceeding the {max} byte ceiling")]
    RequestTooLarge { actual: usize, max: usize },
    #[error("malformed canonical request: {0}")]
    Malformed(&'static str),
    #[error("non-canonical encoding: decoded form does not re-encode to the input")]
    NonCanonical,
    #[error("trailing bytes after the canonical request")]
    TrailingBytes,
    #[error("unknown request domain")]
    UnknownDomain,
    #[error("unsupported request version {0}")]
    UnsupportedVersion(u64),
    #[error("unknown operation code {0}")]
    UnknownOperation(u64),
    #[error("operation {operation} requires exactly {expected} arguments, got {actual}")]
    OperationArity {
        operation: u64,
        expected: u64,
        actual: u64,
    },
    #[error("authority epoch out of range (0..=i64::MAX)")]
    EpochOutOfRange,
    #[error("byte string has length {actual}, expected {expected}")]
    ByteLength { actual: usize, expected: usize },
    #[error("unsupported holder algorithm")]
    UnsupportedAlgorithm,
    #[error("raw token size {actual} outside 1..=65536")]
    TokenSizeOutOfRange { actual: usize },
    #[error("presented token does not match the token identity inside the signed request")]
    TokenIdentityMismatch,
    #[error("malformed holder public key")]
    MalformedHolderKey,
    #[error("weak (small-order) holder public key")]
    WeakHolderKey,
    #[error("malformed signature")]
    MalformedSignature,
    #[error("signature does not verify over the canonical request bytes")]
    SignatureInvalid,
    #[error("request could not be canonically encoded")]
    EncodeFailed,
}

/// Closed set of administrative operations in local profile v1.
///
/// Presence here describes a wire operation. It is neither permission to run it
/// nor a claim that a consumer for it exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// Code 0. Arguments are exactly `[]`.
    DiagnosticsStatus,
    /// Code 1. Arguments are exactly `[plan_id, expected_revision, plan_digest]`.
    ApplyNetworkPlan {
        plan_id: [u8; 32],
        expected_revision: [u8; 32],
        plan_digest: [u8; 32],
    },
}

impl Operation {
    /// Wire code for this operation.
    pub fn code(&self) -> u64 {
        match self {
            Operation::DiagnosticsStatus => 0,
            Operation::ApplyNetworkPlan { .. } => 1,
        }
    }

    /// Number of canonical arguments this operation carries.
    pub fn argument_count(&self) -> u64 {
        match self {
            Operation::DiagnosticsStatus => 0,
            Operation::ApplyNetworkPlan { .. } => 3,
        }
    }
}

/// BLAKE3 identity of an exact raw token, in its own domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenIdentity([u8; 32]);

impl TokenIdentity {
    /// Digest the exact complete received raw token bytes.
    ///
    /// The caller must base64-decode transport encoding first and must not
    /// parse or re-serialize the token. This is not a revocation-lineage id.
    pub fn from_raw_token(raw_token: &[u8]) -> Result<Self, CodecError> {
        if raw_token.len() < MIN_RAW_TOKEN_BYTES || raw_token.len() > MAX_RAW_TOKEN_BYTES {
            return Err(CodecError::TokenSizeOutOfRange {
                actual: raw_token.len(),
            });
        }
        let mut hasher = blake3::Hasher::new();
        hasher.update(TOKEN_IDENTITY_DOMAIN);
        hasher.update(&[0x00]);
        hasher.update(raw_token);
        Ok(TokenIdentity(*hasher.finalize().as_bytes()))
    }

    /// Adopt an identity that appeared inside a request being decoded.
    pub fn from_digest(digest: [u8; 32]) -> Self {
        TokenIdentity(digest)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Closed holder algorithm allowlist. Extending it requires a reviewed pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolderAlgorithm {
    Ed25519,
}

impl HolderAlgorithm {
    /// Canonical ASCII tag committed to by the fingerprint preimage.
    pub fn tag(&self) -> &'static str {
        match self {
            HolderAlgorithm::Ed25519 => "ed25519",
        }
    }

    /// Resolve an exact tag. Aliases and case variants are rejected.
    pub fn from_tag(tag: &str) -> Result<Self, CodecError> {
        // Exact bytes only. No case folding, no trimming, no alias table.
        match tag {
            "ed25519" => Ok(HolderAlgorithm::Ed25519),
            _ => Err(CodecError::UnsupportedAlgorithm),
        }
    }
}

/// A self-describing holder public key: algorithm plus 32 raw key bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HolderKey {
    algorithm: HolderAlgorithm,
    key: [u8; 32],
}

impl HolderKey {
    pub fn new(algorithm: HolderAlgorithm, key: [u8; 32]) -> Self {
        HolderKey { algorithm, key }
    }

    pub fn algorithm(&self) -> HolderAlgorithm {
        self.algorithm
    }

    pub fn key_bytes(&self) -> &[u8; 32] {
        &self.key
    }

    /// Canonical CBOR preimage `{"alg": <tag>, "key": bstr32}`, in that order.
    pub fn fingerprint_preimage(&self) -> Result<Vec<u8>, CodecError> {
        let mut e = Encoder::new(Vec::with_capacity(48));
        e.map(2).map_err(|_| CodecError::EncodeFailed)?;
        e.str("alg").map_err(|_| CodecError::EncodeFailed)?;
        e.str(self.algorithm.tag())
            .map_err(|_| CodecError::EncodeFailed)?;
        e.str("key").map_err(|_| CodecError::EncodeFailed)?;
        e.bytes(&self.key).map_err(|_| CodecError::EncodeFailed)?;
        Ok(e.into_writer())
    }

    /// Algorithm-committing BLAKE3 fingerprint. Never a raw-key digest.
    pub fn fingerprint(&self) -> Result<HolderFingerprint, CodecError> {
        let preimage = self.fingerprint_preimage()?;
        Ok(HolderFingerprint(*blake3::hash(&preimage).as_bytes()))
    }
}

/// BLAKE3 over the algorithm-committing key preimage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HolderFingerprint([u8; 32]);

impl HolderFingerprint {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// A typed administrative request. Construction validates field ranges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminRequest {
    house_id: [u8; 32],
    node_id: [u8; 32],
    authority_epoch: u64,
    operation: Operation,
    token_identity: TokenIdentity,
    target_challenge: [u8; 32],
    request_id: [u8; 32],
}

impl AdminRequest {
    /// Build a request. `authority_epoch` must be within `0..=i64::MAX`.
    ///
    /// `house_id`/`node_id` are opaque identifiers supplied by verified
    /// authority state; this crate neither derives nor authenticates them.
    /// 32 challenge/request-id bytes do not by themselves prove freshness.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        house_id: [u8; 32],
        node_id: [u8; 32],
        authority_epoch: u64,
        operation: Operation,
        token_identity: TokenIdentity,
        target_challenge: [u8; 32],
        request_id: [u8; 32],
    ) -> Result<Self, CodecError> {
        if authority_epoch > MAX_AUTHORITY_EPOCH {
            return Err(CodecError::EpochOutOfRange);
        }
        Ok(AdminRequest {
            house_id,
            node_id,
            authority_epoch,
            operation,
            token_identity,
            target_challenge,
            request_id,
        })
    }

    pub fn house_id(&self) -> &[u8; 32] {
        &self.house_id
    }
    pub fn node_id(&self) -> &[u8; 32] {
        &self.node_id
    }
    pub fn authority_epoch(&self) -> u64 {
        self.authority_epoch
    }
    pub fn operation(&self) -> &Operation {
        &self.operation
    }
    pub fn token_identity(&self) -> &TokenIdentity {
        &self.token_identity
    }
    pub fn target_challenge(&self) -> &[u8; 32] {
        &self.target_challenge
    }
    pub fn request_id(&self) -> &[u8; 32] {
        &self.request_id
    }

    /// Encode to the one canonical byte form. These bytes are what gets signed.
    pub fn encode(&self) -> Result<Vec<u8>, CodecError> {
        let mut e = Encoder::new(Vec::with_capacity(MAX_REQUEST_BYTES));
        let w = |r: Result<&mut Encoder<Vec<u8>>, _>| {
            r.map(|_| ()).map_err(|_| CodecError::EncodeFailed)
        };
        w(e.array(10))?;
        w(e.str(REQUEST_DOMAIN))?;
        w(e.u64(REQUEST_VERSION))?;
        w(e.bytes(&self.house_id))?;
        w(e.bytes(&self.node_id))?;
        w(e.u64(self.authority_epoch))?;
        w(e.u64(self.operation.code()))?;
        w(e.array(self.operation.argument_count()))?;
        match &self.operation {
            Operation::DiagnosticsStatus => {}
            Operation::ApplyNetworkPlan {
                plan_id,
                expected_revision,
                plan_digest,
            } => {
                w(e.bytes(plan_id))?;
                w(e.bytes(expected_revision))?;
                w(e.bytes(plan_digest))?;
            }
        }
        w(e.bytes(self.token_identity.as_bytes()))?;
        w(e.bytes(&self.target_challenge))?;
        w(e.bytes(&self.request_id))?;
        let out = e.into_writer();
        if out.len() > MAX_REQUEST_BYTES {
            return Err(CodecError::RequestTooLarge {
                actual: out.len(),
                max: MAX_REQUEST_BYTES,
            });
        }
        Ok(out)
    }
}

/// Strictly decode canonical request bytes.
///
/// Applies the pre-parse size ceiling, a closed schema, and an exact
/// decode/re-encode equality gate. Hostile bytes are never normalised and then
/// accepted.
pub fn decode_request(input: &[u8]) -> Result<AdminRequest, CodecError> {
    // Size ceiling first: applied before any parsing or allocation.
    if input.len() > MAX_REQUEST_BYTES {
        return Err(CodecError::RequestTooLarge {
            actual: input.len(),
            max: MAX_REQUEST_BYTES,
        });
    }

    let mut d = Decoder::new(input);

    // Definite 10-element array. `None` means an indefinite-length array.
    let arity = d
        .array()
        .map_err(|_| CodecError::Malformed("expected the outer request array"))?
        .ok_or(CodecError::NonCanonical)?;
    if arity != 10 {
        return Err(CodecError::Malformed(
            "the request array must have exactly 10 elements",
        ));
    }

    let domain = d
        .str()
        .map_err(|_| CodecError::Malformed("expected a definite-length domain string"))?;
    if domain != REQUEST_DOMAIN {
        return Err(CodecError::UnknownDomain);
    }

    let version = d
        .u64()
        .map_err(|_| CodecError::Malformed("expected an unsigned version"))?;
    if version != REQUEST_VERSION {
        return Err(CodecError::UnsupportedVersion(version));
    }

    let house_id = decode_bytes32(&mut d, "house_id")?;
    let node_id = decode_bytes32(&mut d, "node_id")?;

    let authority_epoch = d
        .u64()
        .map_err(|_| CodecError::Malformed("expected an unsigned authority_epoch"))?;
    if authority_epoch > MAX_AUTHORITY_EPOCH {
        return Err(CodecError::EpochOutOfRange);
    }

    let operation_code = d
        .u64()
        .map_err(|_| CodecError::Malformed("expected an unsigned operation code"))?;
    let argument_count = d
        .array()
        .map_err(|_| CodecError::Malformed("expected the arguments array"))?
        .ok_or(CodecError::NonCanonical)?;

    let operation = match operation_code {
        0 => {
            expect_arity(0, 0, argument_count)?;
            Operation::DiagnosticsStatus
        }
        1 => {
            expect_arity(1, 3, argument_count)?;
            Operation::ApplyNetworkPlan {
                plan_id: decode_bytes32(&mut d, "plan_id")?,
                expected_revision: decode_bytes32(&mut d, "expected_revision")?,
                plan_digest: decode_bytes32(&mut d, "plan_digest")?,
            }
        }
        other => return Err(CodecError::UnknownOperation(other)),
    };

    let token_identity = TokenIdentity::from_digest(decode_bytes32(&mut d, "token_identity")?);
    let target_challenge = decode_bytes32(&mut d, "target_challenge")?;
    let request_id = decode_bytes32(&mut d, "request_id")?;

    if d.position() != input.len() {
        return Err(CodecError::TrailingBytes);
    }

    let request = AdminRequest::new(
        house_id,
        node_id,
        authority_epoch,
        operation,
        token_identity,
        target_challenge,
        request_id,
    )?;

    // Canonical gate: the typed form must re-encode to exactly the input. This
    // is what rejects non-minimal integers and any other encoding variant the
    // closed schema above did not already catch. Hostile bytes are never
    // normalised and then accepted.
    if request.encode()? != input {
        return Err(CodecError::NonCanonical);
    }

    Ok(request)
}

/// Decode one definite-length 32-byte string. Borrows from the input, so a
/// declared length never drives an allocation.
fn decode_bytes32(d: &mut Decoder<'_>, field: &'static str) -> Result<[u8; 32], CodecError> {
    let _ = field;
    let raw = d
        .bytes()
        .map_err(|_| CodecError::Malformed("expected a definite-length byte string"))?;
    <[u8; 32]>::try_from(raw).map_err(|_| CodecError::ByteLength {
        actual: raw.len(),
        expected: 32,
    })
}

fn expect_arity(operation: u64, expected: u64, actual: u64) -> Result<(), CodecError> {
    if actual == expected {
        Ok(())
    } else {
        Err(CodecError::OperationArity {
            operation,
            expected,
            actual,
        })
    }
}

/// Proof that some holder of `holder` signed exactly `canonical_request`, and
/// that the raw token presented alongside it hashes to the identity named
/// inside those signed bytes.
///
/// This type is **not** `AuthorizedRequest`. It can only be produced by
/// [`verify_holder_proof`]; there is no public constructor, no public field and
/// no `Default`. It carries no authority, time, revocation or replay meaning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedHolderProof {
    request: AdminRequest,
    canonical_bytes: Vec<u8>,
    holder: HolderFingerprint,
    token_identity: TokenIdentity,
}

impl VerifiedHolderProof {
    pub fn request(&self) -> &AdminRequest {
        &self.request
    }
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
    pub fn holder_fingerprint(&self) -> &HolderFingerprint {
        &self.holder
    }
    pub fn token_identity(&self) -> &TokenIdentity {
        &self.token_identity
    }
}

/// Verify possession: canonical gate, exact raw-token rehash, strict Ed25519.
///
/// Success means only that the key holder signed those bytes for that token.
/// It is not an authorization decision and must not be treated as one.
pub fn verify_holder_proof(
    canonical_request: &[u8],
    raw_token: &[u8],
    holder: &HolderKey,
    signature: &[u8],
) -> Result<VerifiedHolderProof, CodecError> {
    // 1. Canonical gate. The bytes that will be verified are the bytes that
    //    were presented; a non-canonical encoding is rejected here even if a
    //    signature is genuinely valid over it.
    let request = decode_request(canonical_request)?;

    // 2. Rehash the exact raw token presented and require it to equal the
    //    identity inside the signed bytes. Attenuation or sealing changes the
    //    identity and demands a fresh holder signature.
    let presented = TokenIdentity::from_raw_token(raw_token)?;
    if presented != *request.token_identity() {
        return Err(CodecError::TokenIdentityMismatch);
    }

    // 3. Closed algorithm allowlist, with the declared algorithm committed to
    //    by the fingerprint below. No downgrade and no alias.
    let HolderAlgorithm::Ed25519 = holder.algorithm();

    let verifying_key =
        VerifyingKey::from_bytes(holder.key_bytes()).map_err(|_| CodecError::MalformedHolderKey)?;
    if verifying_key.is_weak() {
        return Err(CodecError::WeakHolderKey);
    }

    let signature_bytes =
        <[u8; 64]>::try_from(signature).map_err(|_| CodecError::MalformedSignature)?;
    let signature = Signature::from_bytes(&signature_bytes);

    // `verify_strict` rejects small-order and non-canonical R components.
    verifying_key
        .verify_strict(canonical_request, &signature)
        .map_err(|_| CodecError::SignatureInvalid)?;

    let holder_fingerprint = holder.fingerprint()?;

    // Possession only. Nothing here is an authorization decision.
    Ok(VerifiedHolderProof {
        request,
        canonical_bytes: canonical_request.to_vec(),
        holder: holder_fingerprint,
        token_identity: presented,
    })
}
