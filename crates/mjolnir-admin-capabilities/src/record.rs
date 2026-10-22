//! Owner-signed admin records and the consumer-side verifier.
//!
//! Pinned by `openspec/changes/add-control-record-integrity/design.md` and the
//! record section of `add-mesh-admin-capabilities/codec-profile.md`.
//!
//! Records use the same canonical-CBOR profile and the same decode/re-encode
//! gate as admin requests ([`crate::decode_canonical`]). Each record type is a
//! closed schema with its own domain string `lightning-admin/record/<type>`,
//! version 1. There is no second canonicalizer.
//!
//! [`AuthorityBook`] is the consumer verifier. It checks signature, domain,
//! house, epoch, sequence and canonical bytes itself; it takes no "already
//! verified" flag from a peer. It is the only producer of
//! [`VerifiedAuthority`] and of [`TimeEvidence`]'s `authority_verified_at`,
//! through [`OwnerVerified`].

use std::collections::{HashMap, HashSet};

use biscuit_auth::{Algorithm, PublicKey};
use ed25519_dalek::{Signature, VerifyingKey};
use minicbor::{Decoder, Encoder};
use thiserror::Error;

use crate::engine::{AuthzError, GrantClass, SecurityProfile, TimeEvidence, VerifiedAuthority};
use crate::{CodecError, MAX_AUTHORITY_EPOCH, decode_bytes32, decode_canonical};

/// Prefix of every record domain string. The full domain is
/// `lightning-admin/record/<type>`.
pub const RECORD_DOMAIN_PREFIX: &str = "lightning-admin/record/";

/// Record profile version for every record type.
pub const RECORD_VERSION: u64 = 1;

/// Hard pre-parse ceiling on one canonical record.
pub const MAX_RECORD_BYTES: usize = 8192;

/// Most issuers one authority-state record may name.
pub const MAX_RECORD_ISSUERS: usize = 64;

/// Most revoked token lineage ids one authority-state record may name.
pub const MAX_RECORD_LINEAGE: usize = 32;

/// Longest accepted revoked lineage id, in bytes.
pub const MAX_LINEAGE_BYTES: usize = 128;

/// Domain and version of the persisted freshness state. Local state, not a
/// gossiped record.
const BOOK_DOMAIN: &str = "lightning-admin/state/record-book";
const BOOK_VERSION: u64 = 1;
const MAX_BOOK_RECORDS: u64 = 4096;
const MAX_BOOK_BYTES: usize = MAX_BOOK_RECORDS as usize * (MAX_RECORD_BYTES + 96) + 128;

/// Domain-separation prefix for a record digest (what a head names as `prev`).
const RECORD_DIGEST_DOMAIN: &[u8] = b"lightning-admin/record-digest/v1";

const ZERO32: [u8; 32] = [0u8; 32];

/// Closed set of reasons a record or a privileged use is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum RecordDeny {
    /// Bytes, domain, house, signature or signer do not verify at this
    /// consumer. Also covers records this consumer cannot verify yet (unknown
    /// previous head or epoch) and signed times ahead of authenticated now.
    #[error("record is forged or unverifiable at this consumer")]
    Forged,
    /// Older sequence or signed time for its scope, or a superseded record.
    #[error("record is replayed or superseded")]
    Replayed,
    #[error("issuer has been withdrawn by the owner")]
    WithdrawnIssuer,
    #[error("house authority head is forked")]
    ForkedHead,
    /// Permissive record too old, expired, or authority state older than the
    /// household stale-authority maximum.
    #[error("authority is stale")]
    Stale,
    #[error("freshness state is missing, corrupt or unreadable")]
    FreshnessUnreadable,
    #[error("node-vouched fact cannot satisfy an owner-signed check")]
    NodeVouched,
}

/// Which class a fact belongs to. A node-vouched fact never satisfies an
/// owner-signed check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactClass {
    /// Self-report the ingesting node sanity-checked (address, radio
    /// observation, leased-name ingest).
    NodeVouched,
    /// Canonical bytes that verify under an owner or issuer key.
    OwnerSigned,
}

/// A node-vouched observation. It stays marked as such for its whole life.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeVouched<T> {
    value: T,
    vouched_by: Option<[u8; 32]>,
}

impl<T> NodeVouched<T> {
    /// `vouched_by` names the delivering peer, or nothing. It is not an
    /// originator authentication.
    pub fn new(value: T, vouched_by: Option<[u8; 32]>) -> Self {
        NodeVouched { value, vouched_by }
    }
    pub fn value(&self) -> &T {
        &self.value
    }
    pub fn vouched_by(&self) -> Option<&[u8; 32]> {
        self.vouched_by.as_ref()
    }
    pub fn class(&self) -> FactClass {
        FactClass::NodeVouched
    }
}

/// Evidence offered for a decision that requires an owner-signed fact.
#[derive(Debug, Clone, Copy)]
pub enum Evidence<'a> {
    OwnerSigned(&'a SignedRecord),
    NodeVouched { vouched_by: Option<[u8; 32]> },
}

impl<'a> From<&'a SignedRecord> for Evidence<'a> {
    fn from(record: &'a SignedRecord) -> Self {
        Evidence::OwnerSigned(record)
    }
}

impl<'a, T> From<&'a NodeVouched<T>> for Evidence<'a> {
    fn from(fact: &'a NodeVouched<T>) -> Self {
        Evidence::NodeVouched {
            vouched_by: fact.vouched_by,
        }
    }
}

/// Closed set of record types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecordType {
    /// Authority transition. Names the previous head and states the complete
    /// owner, profile, issuer set and revoked lineage for its epoch.
    Head,
    /// Owner-signed issuer and revocation snapshot chained to a head.
    Snapshot,
    /// Owner-signed withdrawal of one issuer. Accepted at any age.
    Withdrawal,
    /// Issuer- or owner-signed control record: enables one grant class on one
    /// node for one biscuit root key. Permissive; refused when stale.
    Control,
}

impl RecordType {
    /// The `<type>` segment of `lightning-admin/record/<type>`.
    pub fn name(self) -> &'static str {
        match self {
            RecordType::Head => "head",
            RecordType::Snapshot => "snapshot",
            RecordType::Withdrawal => "withdrawal",
            RecordType::Control => "control",
        }
    }

    /// Full domain string that opens the record array.
    pub fn domain(self) -> &'static str {
        match self {
            RecordType::Head => "lightning-admin/record/head",
            RecordType::Snapshot => "lightning-admin/record/snapshot",
            RecordType::Withdrawal => "lightning-admin/record/withdrawal",
            RecordType::Control => "lightning-admin/record/control",
        }
    }

    fn from_domain(domain: &str) -> Result<Self, CodecError> {
        match domain {
            "lightning-admin/record/head" => Ok(RecordType::Head),
            "lightning-admin/record/snapshot" => Ok(RecordType::Snapshot),
            "lightning-admin/record/withdrawal" => Ok(RecordType::Withdrawal),
            "lightning-admin/record/control" => Ok(RecordType::Control),
            _ => Err(CodecError::UnknownDomain),
        }
    }

    fn arity(self) -> u64 {
        match self {
            RecordType::Head => 12,
            RecordType::Snapshot => 10,
            RecordType::Withdrawal => 8,
            RecordType::Control => 11,
        }
    }

    /// Owner-signed authority-state records: the only ones that advance
    /// freshness.
    pub fn is_authority_state(self) -> bool {
        matches!(self, RecordType::Head | RecordType::Snapshot)
    }
}

/// Fields every record signs, in wire order after domain and version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordHeader {
    pub house_id: [u8; 32],
    pub authority_epoch: u64,
    /// Ed25519 key that signs these bytes.
    pub signer: [u8; 32],
    /// Per-scope counter. Scope is `(house, epoch, signer, type, subject)`.
    pub sequence: u64,
    /// Issuer-signed issuance time, unix seconds.
    pub issued_at: u64,
    /// Head: previous head digest (all zero for genesis). Snapshot: the head
    /// digest it chains to. Withdrawal: the withdrawn issuer key. Control: the
    /// node id.
    pub subject: [u8; 32],
}

/// Type-specific closed body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordBody {
    Head {
        /// Key that signs snapshots, withdrawals and the next head at this
        /// epoch.
        owner: [u8; 32],
        profile: SecurityProfile,
        /// Complete approved issuer set, strictly ascending.
        issuers: Vec<[u8; 32]>,
        /// Complete revoked token lineage, strictly ascending.
        revoked_lineage: Vec<Vec<u8>>,
    },
    Snapshot {
        issuers: Vec<[u8; 32]>,
        revoked_lineage: Vec<Vec<u8>>,
    },
    Withdrawal,
    Control {
        grant_class: GrantClass,
        /// Biscuit root public key (Ed25519) the grant's tokens verify under.
        token_root: [u8; 32],
        expires_at: u64,
    },
}

impl RecordBody {
    pub fn record_type(&self) -> RecordType {
        match self {
            RecordBody::Head { .. } => RecordType::Head,
            RecordBody::Snapshot { .. } => RecordType::Snapshot,
            RecordBody::Withdrawal => RecordType::Withdrawal,
            RecordBody::Control { .. } => RecordType::Control,
        }
    }
}

/// A typed record. Construction validates the closed schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    header: RecordHeader,
    body: RecordBody,
}

impl Record {
    pub fn new(header: RecordHeader, body: RecordBody) -> Result<Self, CodecError> {
        if header.authority_epoch > MAX_AUTHORITY_EPOCH {
            return Err(CodecError::EpochOutOfRange);
        }
        if header.issued_at > i64::MAX as u64 {
            return Err(CodecError::Malformed("issued_at out of range"));
        }
        match &body {
            RecordBody::Head {
                issuers,
                revoked_lineage,
                ..
            }
            | RecordBody::Snapshot {
                issuers,
                revoked_lineage,
            } => {
                check_issuers(issuers)?;
                check_lineage(revoked_lineage)?;
            }
            RecordBody::Withdrawal => {}
            RecordBody::Control { expires_at, .. } => {
                if *expires_at > i64::MAX as u64 {
                    return Err(CodecError::Malformed("expires_at out of range"));
                }
                if *expires_at <= header.issued_at {
                    return Err(CodecError::Malformed("expires_at must follow issued_at"));
                }
            }
        }
        Ok(Record { header, body })
    }

    pub fn header(&self) -> &RecordHeader {
        &self.header
    }
    pub fn body(&self) -> &RecordBody {
        &self.body
    }
    pub fn record_type(&self) -> RecordType {
        self.body.record_type()
    }

    /// Encode with the one canonical encoder. These bytes are what gets signed.
    pub fn encode(&self) -> Result<Vec<u8>, CodecError> {
        let mut e = Encoder::new(Vec::with_capacity(256));
        let w = |r: Result<&mut Encoder<Vec<u8>>, _>| {
            r.map(|_| ()).map_err(|_| CodecError::EncodeFailed)
        };
        let ty = self.record_type();
        let h = &self.header;
        w(e.array(ty.arity()))?;
        w(e.str(ty.domain()))?;
        w(e.u64(RECORD_VERSION))?;
        w(e.bytes(&h.house_id))?;
        w(e.u64(h.authority_epoch))?;
        w(e.bytes(&h.signer))?;
        w(e.u64(h.sequence))?;
        w(e.u64(h.issued_at))?;
        w(e.bytes(&h.subject))?;
        match &self.body {
            RecordBody::Head {
                owner,
                profile,
                issuers,
                revoked_lineage,
            } => {
                w(e.bytes(owner))?;
                w(e.u64(profile_code(*profile)))?;
                encode_state(&mut e, issuers, revoked_lineage)?;
            }
            RecordBody::Snapshot {
                issuers,
                revoked_lineage,
            } => encode_state(&mut e, issuers, revoked_lineage)?,
            RecordBody::Withdrawal => {}
            RecordBody::Control {
                grant_class,
                token_root,
                expires_at,
            } => {
                w(e.u64(class_code(*grant_class)))?;
                w(e.bytes(token_root))?;
                w(e.u64(*expires_at))?;
            }
        }
        let out = e.into_writer();
        if out.len() > MAX_RECORD_BYTES {
            return Err(CodecError::RecordTooLarge {
                actual: out.len(),
                max: MAX_RECORD_BYTES,
            });
        }
        Ok(out)
    }

    /// Domain-separated BLAKE3 of the canonical bytes. A head names its
    /// predecessor by this digest.
    pub fn digest(&self) -> Result<[u8; 32], CodecError> {
        Ok(digest_bytes(&self.encode()?))
    }
}

/// Strictly decode canonical record bytes through the shared canonical gate.
pub fn decode_record(input: &[u8]) -> Result<Record, CodecError> {
    decode_canonical(
        input,
        MAX_RECORD_BYTES,
        |actual, max| CodecError::RecordTooLarge { actual, max },
        decode_record_fields,
        Record::encode,
    )
}

fn decode_record_fields(d: &mut Decoder<'_>) -> Result<Record, CodecError> {
    let arity = d
        .array()
        .map_err(|_| CodecError::Malformed("expected the outer record array"))?
        .ok_or(CodecError::NonCanonical)?;
    let domain = d
        .str()
        .map_err(|_| CodecError::Malformed("expected a definite-length domain string"))?;
    let ty = RecordType::from_domain(domain)?;
    if arity != ty.arity() {
        return Err(CodecError::Malformed("wrong arity for this record type"));
    }
    let version = d
        .u64()
        .map_err(|_| CodecError::Malformed("expected an unsigned version"))?;
    if version != RECORD_VERSION {
        return Err(CodecError::UnsupportedVersion(version));
    }
    let house_id = decode_bytes32(d, "house_id")?;
    let authority_epoch = decode_u64(d)?;
    let signer = decode_bytes32(d, "signer")?;
    let sequence = decode_u64(d)?;
    let issued_at = decode_u64(d)?;
    let subject = decode_bytes32(d, "subject")?;
    let header = RecordHeader {
        house_id,
        authority_epoch,
        signer,
        sequence,
        issued_at,
        subject,
    };
    let body = match ty {
        RecordType::Head => {
            let owner = decode_bytes32(d, "owner")?;
            let profile = profile_from_code(decode_u64(d)?)?;
            let (issuers, revoked_lineage) = decode_state(d)?;
            RecordBody::Head {
                owner,
                profile,
                issuers,
                revoked_lineage,
            }
        }
        RecordType::Snapshot => {
            let (issuers, revoked_lineage) = decode_state(d)?;
            RecordBody::Snapshot {
                issuers,
                revoked_lineage,
            }
        }
        RecordType::Withdrawal => RecordBody::Withdrawal,
        RecordType::Control => RecordBody::Control {
            grant_class: class_from_code(decode_u64(d)?)?,
            token_root: decode_bytes32(d, "token_root")?,
            expires_at: decode_u64(d)?,
        },
    };
    Record::new(header, body)
}

fn decode_u64(d: &mut Decoder<'_>) -> Result<u64, CodecError> {
    d.u64()
        .map_err(|_| CodecError::Malformed("expected an unsigned integer"))
}

fn decode_state(d: &mut Decoder<'_>) -> Result<(Vec<[u8; 32]>, Vec<Vec<u8>>), CodecError> {
    let n = d
        .array()
        .map_err(|_| CodecError::Malformed("expected the issuer array"))?
        .ok_or(CodecError::NonCanonical)?;
    if n > MAX_RECORD_ISSUERS as u64 {
        return Err(CodecError::Malformed("too many issuers"));
    }
    let mut issuers = Vec::with_capacity(n as usize);
    for _ in 0..n {
        issuers.push(decode_bytes32(d, "issuer")?);
    }
    let n = d
        .array()
        .map_err(|_| CodecError::Malformed("expected the revoked lineage array"))?
        .ok_or(CodecError::NonCanonical)?;
    if n > MAX_RECORD_LINEAGE as u64 {
        return Err(CodecError::Malformed("too many revoked lineage ids"));
    }
    let mut lineage = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let raw = d
            .bytes()
            .map_err(|_| CodecError::Malformed("expected a definite-length byte string"))?;
        lineage.push(raw.to_vec());
    }
    Ok((issuers, lineage))
}

fn encode_state(
    e: &mut Encoder<Vec<u8>>,
    issuers: &[[u8; 32]],
    lineage: &[Vec<u8>],
) -> Result<(), CodecError> {
    let w = |r: Result<&mut Encoder<Vec<u8>>, _>| {
        r.map(|_| ()).map_err(|_| CodecError::EncodeFailed)
    };
    w(e.array(issuers.len() as u64))?;
    for issuer in issuers {
        w(e.bytes(issuer))?;
    }
    w(e.array(lineage.len() as u64))?;
    for id in lineage {
        w(e.bytes(id))?;
    }
    Ok(())
}

fn check_issuers(issuers: &[[u8; 32]]) -> Result<(), CodecError> {
    if issuers.len() > MAX_RECORD_ISSUERS {
        return Err(CodecError::Malformed("too many issuers"));
    }
    if !issuers.windows(2).all(|p| p[0] < p[1]) {
        return Err(CodecError::Malformed("issuers must be strictly ascending"));
    }
    Ok(())
}

fn check_lineage(lineage: &[Vec<u8>]) -> Result<(), CodecError> {
    if lineage.len() > MAX_RECORD_LINEAGE {
        return Err(CodecError::Malformed("too many revoked lineage ids"));
    }
    if lineage
        .iter()
        .any(|id| id.is_empty() || id.len() > MAX_LINEAGE_BYTES)
    {
        return Err(CodecError::Malformed("revoked lineage id length out of range"));
    }
    if !lineage.windows(2).all(|p| p[0] < p[1]) {
        return Err(CodecError::Malformed(
            "revoked lineage must be strictly ascending",
        ));
    }
    Ok(())
}

fn profile_code(p: SecurityProfile) -> u64 {
    match p {
        SecurityProfile::Strict => 0,
        SecurityProfile::Standard => 1,
        SecurityProfile::Relaxed => 2,
    }
}

fn profile_from_code(code: u64) -> Result<SecurityProfile, CodecError> {
    match code {
        0 => Ok(SecurityProfile::Strict),
        1 => Ok(SecurityProfile::Standard),
        2 => Ok(SecurityProfile::Relaxed),
        _ => Err(CodecError::Malformed("unknown security profile")),
    }
}

fn class_code(c: GrantClass) -> u64 {
    match c {
        GrantClass::DiagnosticsRead => 0,
        GrantClass::NetworkRadio => 1,
        GrantClass::Firmware => 2,
        GrantClass::Ownership => 3,
        GrantClass::GuestWifi => 4,
    }
}

fn class_from_code(code: u64) -> Result<GrantClass, CodecError> {
    match code {
        0 => Ok(GrantClass::DiagnosticsRead),
        1 => Ok(GrantClass::NetworkRadio),
        2 => Ok(GrantClass::Firmware),
        3 => Ok(GrantClass::Ownership),
        4 => Ok(GrantClass::GuestWifi),
        _ => Err(CodecError::Malformed("unknown grant class")),
    }
}

fn digest_bytes(canonical: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(RECORD_DIGEST_DOMAIN);
    hasher.update(&[0x00]);
    hasher.update(canonical);
    *hasher.finalize().as_bytes()
}

/// Canonical record bytes plus the Ed25519 signature of `header.signer` over
/// exactly those bytes. This is the gossip unit. It has no "verified" flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedRecord {
    bytes: Vec<u8>,
    signature: [u8; 64],
}

impl SignedRecord {
    pub fn new(bytes: Vec<u8>, signature: [u8; 64]) -> Self {
        SignedRecord { bytes, signature }
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn signature(&self) -> &[u8; 64] {
        &self.signature
    }
}

/// The pinned trust anchor: house id and the owner key that signs the genesis
/// head. Comes from the owner claim, not from gossip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthorityAnchor {
    house_id: [u8; 32],
    owner: [u8; 32],
}

impl AuthorityAnchor {
    pub fn new(house_id: [u8; 32], owner: [u8; 32]) -> Self {
        AuthorityAnchor { house_id, owner }
    }
    pub fn house_id(&self) -> &[u8; 32] {
        &self.house_id
    }
    pub fn owner(&self) -> &[u8; 32] {
        &self.owner
    }
}

/// Outcome of a successful ingest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ingested {
    /// Newly verified and applied.
    Applied,
    /// Byte-identical to a record already applied. No state changed and
    /// freshness did not move.
    AlreadyApplied,
}

/// Result of verifying an owner-signed control record at this consumer.
///
/// The only production constructor of [`VerifiedAuthority`] and of
/// [`TimeEvidence`]'s `authority_verified_at`.
#[derive(Debug, Clone)]
pub struct OwnerVerified {
    authority: VerifiedAuthority,
    authority_verified_at: u64,
}

impl OwnerVerified {
    pub fn authority(&self) -> &VerifiedAuthority {
        &self.authority
    }

    /// Signed `issued_at` of the newest owner-signed authority-state record.
    pub fn authority_verified_at(&self) -> u64 {
        self.authority_verified_at
    }

    /// Bind authenticated `unix_seconds` (destination-owned, never the wall
    /// clock) to this verified freshness.
    pub fn time_evidence(&self, unix_seconds: u64) -> Result<TimeEvidence, AuthzError> {
        TimeEvidence::authenticated(unix_seconds, self.authority_verified_at)
    }
}

/// Sequence scope: `(house, epoch, signer, type, subject)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Scope {
    house_id: [u8; 32],
    epoch: u64,
    signer: [u8; 32],
    ty: RecordType,
    subject: [u8; 32],
}

impl Scope {
    fn of(h: &RecordHeader, ty: RecordType) -> Self {
        Scope {
            house_id: h.house_id,
            epoch: h.authority_epoch,
            signer: h.signer,
            ty,
            subject: h.subject,
        }
    }
}

/// Issuer and revocation state as of `issued_at`, from a head or snapshot.
#[derive(Debug, Clone)]
struct AuthState {
    issued_at: u64,
    issuers: Vec<[u8; 32]>,
    revoked_lineage: Vec<Vec<u8>>,
}

#[derive(Debug, Clone)]
struct HeadInfo {
    epoch: u64,
    owner: [u8; 32],
    profile: SecurityProfile,
    state: AuthState,
}

/// Verified, readable freshness state.
#[derive(Debug, Clone, Default)]
struct Book {
    /// Applied records in apply order, compacted per scope. This is what
    /// persists; restore re-verifies every entry.
    applied: Vec<SignedRecord>,
    digests: HashSet<[u8; 32]>,
    high_water: HashMap<Scope, (u64, [u8; 32])>,
    heads: HashMap<[u8; 32], HeadInfo>,
    /// Newest snapshot per head digest.
    snapshots: HashMap<[u8; 32], AuthState>,
    /// Issuer key → greatest signed `issued_at` of a verified withdrawal.
    withdrawn: HashMap<[u8; 32], u64>,
}

#[derive(Debug, Clone)]
enum BookState {
    Readable(Book),
    Unreadable,
}

/// Consumer-side verifier and freshness state for one house.
///
/// Missing or corrupt stored state yields an unreadable book that denies every
/// privileged use and every ingest. It is never silently replaced with an
/// empty one.
#[derive(Debug, Clone)]
pub struct AuthorityBook {
    anchor: AuthorityAnchor,
    state: BookState,
}

struct Current<'a> {
    digest: [u8; 32],
    head: &'a HeadInfo,
    state: &'a AuthState,
}

impl AuthorityBook {
    /// First provisioning at owner claim, before any freshness state exists.
    /// Not a recovery path for unreadable state.
    pub fn provision(anchor: AuthorityAnchor) -> Self {
        AuthorityBook {
            anchor,
            state: BookState::Readable(Book::default()),
        }
    }

    /// Restore persisted freshness state. `None` (missing) or any decode,
    /// anchor or re-verification failure yields an unreadable book.
    pub fn restore(anchor: AuthorityAnchor, stored: Option<&[u8]>) -> Self {
        let state = match stored.and_then(|bytes| restore_book(&anchor, bytes)) {
            Some(book) => BookState::Readable(book),
            None => BookState::Unreadable,
        };
        AuthorityBook { anchor, state }
    }

    pub fn anchor(&self) -> &AuthorityAnchor {
        &self.anchor
    }

    pub fn is_readable(&self) -> bool {
        matches!(self.state, BookState::Readable(_))
    }

    /// Persisted form of the freshness state. Refused when unreadable, so an
    /// unreadable book can never be written back as an empty one.
    pub fn to_bytes(&self) -> Result<Vec<u8>, RecordDeny> {
        let book = self.readable()?;
        encode_book(&self.anchor, &book.applied).map_err(|_| RecordDeny::FreshnessUnreadable)
    }

    /// Signed `issued_at` of the current authority-state record, if any.
    pub fn freshness(&self) -> Result<Option<u64>, RecordDeny> {
        let book = self.readable()?;
        Ok(book.current()?.map(|c| c.state.issued_at))
    }

    /// Current authority epoch, if a single head exists.
    pub fn current_epoch(&self) -> Result<Option<u64>, RecordDeny> {
        let book = self.readable()?;
        Ok(book.current()?.map(|c| c.head.epoch))
    }

    /// Digest of the current head, if a single head exists.
    pub fn current_head(&self) -> Result<Option<[u8; 32]>, RecordDeny> {
        let book = self.readable()?;
        Ok(book.current()?.map(|c| c.digest))
    }

    /// Verify and apply one gossiped record at this consumer.
    ///
    /// `now` is authenticated unix seconds (the same provenance as
    /// `TimeEvidence::unix_seconds`), never the wall clock.
    pub fn ingest(&mut self, record: &SignedRecord, now: u64) -> Result<Ingested, RecordDeny> {
        let anchor = self.anchor;
        let book = self.readable_mut()?;
        book.apply(&anchor, record, Some(now))
    }

    /// Decide whether `evidence` authorizes privileged use now.
    ///
    /// Only an owner-signed control record that verifies here, from a signer
    /// that is the owner or a non-withdrawn issuer of the single current head,
    /// with authority state no older than the profile's stale maximum, yields
    /// [`OwnerVerified`]. A new valid record is applied first; a re-gossiped
    /// one changes nothing.
    pub fn verified_authority<'a>(
        &mut self,
        evidence: impl Into<Evidence<'a>>,
        now: u64,
    ) -> Result<OwnerVerified, RecordDeny> {
        let anchor = self.anchor;
        let book = self.readable_mut()?;
        let record = match evidence.into() {
            Evidence::NodeVouched { .. } => return Err(RecordDeny::NodeVouched),
            Evidence::OwnerSigned(record) => record,
        };
        book.apply(&anchor, record, Some(now))?;

        // Point-of-use checks, independent of how ingest went.
        let decoded = verify_signed(&anchor, record)?;
        let digest = digest_bytes(record.bytes());
        let RecordBody::Control {
            grant_class,
            token_root,
            expires_at,
        } = *decoded.body()
        else {
            return Err(RecordDeny::Forged);
        };
        let h = decoded.header();
        let current = book.current()?.ok_or(RecordDeny::Stale)?;
        if h.authority_epoch != current.head.epoch {
            return Err(RecordDeny::Replayed);
        }
        book.signer_may_control(&current, &h.signer)?;
        match book.high_water.get(&Scope::of(h, RecordType::Control)) {
            Some((_, d)) if *d == digest => {}
            _ => return Err(RecordDeny::Replayed),
        }
        if h.issued_at > now {
            return Err(RecordDeny::Forged);
        }
        if now >= expires_at {
            return Err(RecordDeny::Stale);
        }
        let verified_at = current.state.issued_at;
        if now < verified_at || now - verified_at > current.head.profile.max_stale_secs() {
            return Err(RecordDeny::Stale);
        }
        let issuer =
            PublicKey::from_bytes(&token_root, Algorithm::Ed25519).map_err(|_| RecordDeny::Forged)?;
        Ok(OwnerVerified {
            authority: VerifiedAuthority {
                issuer,
                house_id: anchor.house_id,
                node_id: h.subject,
                authority_epoch: current.head.epoch,
                profile: current.head.profile,
                grant_class,
                revoked_lineage: current.state.revoked_lineage.clone(),
            },
            authority_verified_at: verified_at,
        })
    }

    fn readable(&self) -> Result<&Book, RecordDeny> {
        match &self.state {
            BookState::Readable(book) => Ok(book),
            BookState::Unreadable => Err(RecordDeny::FreshnessUnreadable),
        }
    }

    fn readable_mut(&mut self) -> Result<&mut Book, RecordDeny> {
        match &mut self.state {
            BookState::Readable(book) => Ok(book),
            BookState::Unreadable => Err(RecordDeny::FreshnessUnreadable),
        }
    }
}

/// Decode through the canonical gate, check house, and verify the Ed25519
/// signature of the embedded signer over exactly the presented bytes. Whether
/// that signer is *allowed* is decided per record type afterwards.
fn verify_signed(anchor: &AuthorityAnchor, record: &SignedRecord) -> Result<Record, RecordDeny> {
    let decoded = decode_record(record.bytes()).map_err(|_| RecordDeny::Forged)?;
    if decoded.header().house_id != anchor.house_id {
        return Err(RecordDeny::Forged);
    }
    let key =
        VerifyingKey::from_bytes(&decoded.header().signer).map_err(|_| RecordDeny::Forged)?;
    if key.is_weak() {
        return Err(RecordDeny::Forged);
    }
    let signature = Signature::from_bytes(record.signature());
    key.verify_strict(record.bytes(), &signature)
        .map_err(|_| RecordDeny::Forged)?;
    Ok(decoded)
}

impl Book {
    /// Unique head at the greatest epoch. Two or more heads at that epoch is a
    /// fork; there is no fallback to a lower epoch or a common ancestor.
    fn current(&self) -> Result<Option<Current<'_>>, RecordDeny> {
        let Some(max) = self.heads.values().map(|h| h.epoch).max() else {
            return Ok(None);
        };
        let mut at_max = self.heads.iter().filter(|(_, h)| h.epoch == max);
        let (digest, head) = at_max.next().expect("max came from this map");
        if at_max.next().is_some() {
            return Err(RecordDeny::ForkedHead);
        }
        let state = self.snapshots.get(digest).unwrap_or(&head.state);
        Ok(Some(Current {
            digest: *digest,
            head,
            state,
        }))
    }

    /// The owner of the current head, or an issuer named by the current
    /// authority state and not withdrawn since that state was signed.
    fn signer_may_control(&self, current: &Current<'_>, signer: &[u8; 32]) -> Result<(), RecordDeny> {
        if *signer == current.head.owner {
            return Ok(());
        }
        let listed = current.state.issuers.binary_search(signer).is_ok();
        // A withdrawal is lifted only by a strictly later owner-signed
        // authority-state record that still lists the issuer (N3).
        if let Some(withdrawn_at) = self.withdrawn.get(signer)
            && (!listed || *withdrawn_at >= current.state.issued_at)
        {
            return Err(RecordDeny::WithdrawnIssuer);
        }
        if listed {
            Ok(())
        } else {
            Err(RecordDeny::Forged)
        }
    }

    /// `now = None` only while restoring already-applied state; every
    /// time-based check is then deferred to the point of use.
    fn apply(
        &mut self,
        anchor: &AuthorityAnchor,
        record: &SignedRecord,
        now: Option<u64>,
    ) -> Result<Ingested, RecordDeny> {
        let decoded = verify_signed(anchor, record)?;
        let digest = digest_bytes(record.bytes());
        if self.digests.contains(&digest) {
            return Ok(Ingested::AlreadyApplied);
        }
        let h = *decoded.header();
        if now.is_some_and(|now| h.issued_at > now) {
            return Err(RecordDeny::Forged);
        }
        let ty = decoded.record_type();
        match decoded.body() {
            RecordBody::Head {
                owner,
                profile,
                issuers,
                revoked_lineage,
            } => {
                // Accepted at any age: a head never advances freshness past
                // its own signed time, which the use check judges.
                if h.subject == ZERO32 {
                    if h.signer != anchor.owner {
                        return Err(RecordDeny::Forged);
                    }
                } else {
                    let prev = self.heads.get(&h.subject).ok_or(RecordDeny::Forged)?;
                    if prev.epoch >= h.authority_epoch || prev.owner != h.signer {
                        return Err(RecordDeny::Forged);
                    }
                }
                self.heads.insert(
                    digest,
                    HeadInfo {
                        epoch: h.authority_epoch,
                        owner: *owner,
                        profile: *profile,
                        state: AuthState {
                            issued_at: h.issued_at,
                            issuers: issuers.clone(),
                            revoked_lineage: revoked_lineage.clone(),
                        },
                    },
                );
                self.applied.push(record.clone());
                self.digests.insert(digest);
                Ok(Ingested::Applied)
            }
            RecordBody::Snapshot {
                issuers,
                revoked_lineage,
            } => {
                let head = self.heads.get(&h.subject).ok_or(RecordDeny::Forged)?;
                if head.owner != h.signer || head.epoch != h.authority_epoch {
                    return Err(RecordDeny::Forged);
                }
                let current = self.current()?.ok_or(RecordDeny::Forged)?;
                if current.digest != h.subject {
                    return Err(RecordDeny::Replayed);
                }
                // Freshness advances only to a strictly newer signed time.
                if h.issued_at <= current.state.issued_at {
                    return Err(RecordDeny::Replayed);
                }
                self.check_sequence(&h, ty)?;
                self.snapshots.insert(
                    h.subject,
                    AuthState {
                        issued_at: h.issued_at,
                        issuers: issuers.clone(),
                        revoked_lineage: revoked_lineage.clone(),
                    },
                );
                self.commit(&h, ty, digest, record);
                Ok(Ingested::Applied)
            }
            RecordBody::Withdrawal => {
                // Only an owner signs a withdrawal, and it is accepted at any
                // age: it only narrows authority.
                if !self
                    .heads
                    .values()
                    .any(|head| head.epoch == h.authority_epoch && head.owner == h.signer)
                {
                    return Err(RecordDeny::Forged);
                }
                self.check_sequence(&h, ty)?;
                let at = self.withdrawn.entry(h.subject).or_insert(h.issued_at);
                *at = (*at).max(h.issued_at);
                self.commit(&h, ty, digest, record);
                Ok(Ingested::Applied)
            }
            RecordBody::Control { expires_at, .. } => {
                let current = self.current()?.ok_or(RecordDeny::Stale)?;
                if h.authority_epoch < current.head.epoch {
                    return Err(RecordDeny::Replayed);
                }
                if h.authority_epoch > current.head.epoch {
                    return Err(RecordDeny::Forged);
                }
                // On restore the authority state that admitted this signer
                // may have been compacted away; the point of use re-checks.
                if now.is_some() {
                    self.signer_may_control(&current, &h.signer)?;
                }
                // Permissive: refused when expired or older than the stale
                // maximum by its own signed time.
                if let Some(now) = now
                    && (now >= *expires_at
                        || now - h.issued_at > current.head.profile.max_stale_secs())
                {
                    return Err(RecordDeny::Stale);
                }
                self.check_sequence(&h, ty)?;
                self.commit(&h, ty, digest, record);
                Ok(Ingested::Applied)
            }
        }
    }

    fn check_sequence(&self, h: &RecordHeader, ty: RecordType) -> Result<(), RecordDeny> {
        match self.high_water.get(&Scope::of(h, ty)) {
            Some((seq, _)) if h.sequence <= *seq => Err(RecordDeny::Replayed),
            _ => Ok(()),
        }
    }

    /// Record a sequence-scoped apply and drop the record it supersedes from
    /// the persisted list.
    fn commit(&mut self, h: &RecordHeader, ty: RecordType, digest: [u8; 32], record: &SignedRecord) {
        if let Some((_, old)) = self
            .high_water
            .insert(Scope::of(h, ty), (h.sequence, digest))
        {
            self.applied.retain(|r| digest_bytes(r.bytes()) != old);
        }
        self.applied.push(record.clone());
        self.digests.insert(digest);
    }
}

fn encode_book(anchor: &AuthorityAnchor, applied: &[SignedRecord]) -> Result<Vec<u8>, CodecError> {
    let mut e = Encoder::new(Vec::new());
    let w = |r: Result<&mut Encoder<Vec<u8>>, _>| {
        r.map(|_| ()).map_err(|_| CodecError::EncodeFailed)
    };
    w(e.array(5))?;
    w(e.str(BOOK_DOMAIN))?;
    w(e.u64(BOOK_VERSION))?;
    w(e.bytes(&anchor.house_id))?;
    w(e.bytes(&anchor.owner))?;
    w(e.array(applied.len() as u64))?;
    for r in applied {
        w(e.array(2))?;
        w(e.bytes(r.bytes()))?;
        w(e.bytes(r.signature()))?;
    }
    Ok(e.into_writer())
}

fn decode_book_fields(d: &mut Decoder<'_>) -> Result<(AuthorityAnchor, Vec<SignedRecord>), CodecError> {
    let malformed = |_| CodecError::Malformed("record book");
    if d.array().map_err(malformed)? != Some(5) {
        return Err(CodecError::Malformed("record book"));
    }
    if d.str().map_err(malformed)? != BOOK_DOMAIN {
        return Err(CodecError::UnknownDomain);
    }
    let version = d.u64().map_err(malformed)?;
    if version != BOOK_VERSION {
        return Err(CodecError::UnsupportedVersion(version));
    }
    let anchor = AuthorityAnchor::new(decode_bytes32(d, "house_id")?, decode_bytes32(d, "owner")?);
    let n = d
        .array()
        .map_err(malformed)?
        .ok_or(CodecError::NonCanonical)?;
    if n > MAX_BOOK_RECORDS {
        return Err(CodecError::Malformed("record book too long"));
    }
    let mut applied = Vec::with_capacity(n as usize);
    for _ in 0..n {
        if d.array().map_err(malformed)? != Some(2) {
            return Err(CodecError::Malformed("record book entry"));
        }
        let bytes = d.bytes().map_err(malformed)?.to_vec();
        let signature =
            <[u8; 64]>::try_from(d.bytes().map_err(malformed)?).map_err(|_| CodecError::MalformedSignature)?;
        applied.push(SignedRecord::new(bytes, signature));
    }
    Ok((anchor, applied))
}

fn restore_book(anchor: &AuthorityAnchor, bytes: &[u8]) -> Option<Book> {
    let (stored_anchor, applied) = decode_canonical(
        bytes,
        MAX_BOOK_BYTES,
        |actual, max| CodecError::RecordTooLarge { actual, max },
        decode_book_fields,
        |(a, r)| encode_book(a, r),
    )
    .ok()?;
    if stored_anchor != *anchor {
        return None;
    }
    let mut book = Book::default();
    for record in &applied {
        match book.apply(anchor, record, None) {
            Ok(Ingested::Applied) => {}
            _ => return None,
        }
    }
    Some(book)
}
