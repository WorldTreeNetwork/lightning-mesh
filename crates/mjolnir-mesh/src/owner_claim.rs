// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors
// Lightning Mesh is dual-licensed (AGPL-3.0-or-later or commercial); see LICENSE
// and COMMERCIAL-LICENSE.md at the repository root.

//! Household owner claim: the one-time ceremony that binds the first owner to
//! an unclaimed node, plus non-destructive recovery and destructive reset
//! (openspec change `add-household-owner-claim`, bead `mjolnir-mesh-bf7.1`).
//!
//! Not to be confused with [`crate::claim_cooldown`], which is the first-boot
//! *subnet* claim cooldown.
//!
//! Ceremony, as accepted in `reviews/2026-10-22-advise-r3.md`:
//!
//! - The off-device card carries the full node id and a secret of at least
//!   80 bits ([`MIN_SECRET_CHARS`] lowercase RFC 4648 base32 characters).
//! - `k = HMAC-SHA256(salt, secret)`. The node stores `k` and the salt, never
//!   the card string. The salt travels in the node-signed challenge.
//! - The client (Lightning Admin, see [`client_prove`]) checks the node's
//!   signature on the challenge against the card's node id *before* it derives
//!   anything from the secret, then sends
//!   `HMAC-SHA256(k, owner pk ‖ node id ‖ epoch ‖ nonce)`. A
//!   [`ProofSubmission`] has no field for the secret or `k`.
//! - Order is request, then press. A press arms a window (default
//!   [`DEFAULT_WINDOW`]) that confirms only the requests pending at press time.
//!   The window closes on success, reboot (it is never persisted), loss of
//!   trustworthy timing, expiry, or after [`MAX_FAILED_PROOFS`] failed proofs.
//! - The first complete claim is a compare-and-swap from unclaimed (epoch
//!   [`UNCLAIMED_EPOCH`]) to epoch 1 on the [`OwnershipStore`]. A successful
//!   claim retires `k`.
//!
//! meshd owns the window and the compare-and-swap; the rc.wps hook only
//! signals a press ([`take_press`]) and reads the published
//! [`ButtonEffect`] ([`publish_button_effect`]); hello only relays.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Default physical window: 120 seconds from the press.
pub const DEFAULT_WINDOW: Duration = Duration::from_secs(120);
/// Failed proofs one window tolerates before it closes.
pub const MAX_FAILED_PROOFS: u32 = 5;
/// Minimum card secret length: 16 base32 characters = 80 bits.
pub const MIN_SECRET_CHARS: usize = 16;
/// Ownership epoch of an unclaimed node, as encoded in the proof input.
pub const UNCLAIMED_EPOCH: u64 = 0;
/// Salt width for `k = HMAC-SHA256(salt, secret)`.
pub const SALT_LEN: usize = 16;
/// Bound on concurrently pending requests of each kind; the oldest is evicted.
const MAX_PENDING: usize = 8;

/// Durable ownership record (written by meshd only).
pub const DEFAULT_STATE_PATH: &str = "/etc/mjolnir/owner-claim.json";
/// Volatile hand-off directory shared with `/etc/rc.wps/00-mjolnir-wan-admin`.
/// It is tmpfs, so a reboot clears both files below.
pub const DEFAULT_RUNTIME_DIR: &str = "/var/run/mjolnir/owner-claim";
/// meshd writes the current [`ButtonEffect::as_str`] here.
pub const BUTTON_EFFECT_FILE: &str = "button-effect";
/// The rc.wps hook creates this file to signal a ceremony press.
pub const PRESS_FILE: &str = "press";

const CHALLENGE_TAG: &[u8] = b"mjolnir-owner-claim/challenge/v1";
const OUTCOME_TAG: &[u8] = b"mjolnir-owner-claim/outcome/v1";

pub type NodeId = [u8; 32];
pub type PublicKey = [u8; 32];
pub type Nonce = [u8; 32];
pub type ProofMac = [u8; 32];
pub type Salt = [u8; SALT_LEN];

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ClaimError {
    #[error("card secret must be at least {MIN_SECRET_CHARS} lowercase base32 characters")]
    MalformedSecret,
    #[error("node is not claimable (claimed, or no provisioning verifier)")]
    NotClaimable,
    #[error("node already has a verifier or an owner")]
    AlreadyProvisioned,
    #[error("node is not owned")]
    NotOwned,
    #[error("no armed physical window covers this request")]
    NoWindow,
    #[error("proof did not verify")]
    BadProof { retry: Option<Box<SignedChallenge>> },
    #[error("window closed after {MAX_FAILED_PROOFS} failed proofs")]
    AttemptsExhausted,
    #[error("another claim already holds this node")]
    AlreadyClaimed,
    #[error("unknown or expired request")]
    UnknownRequest,
    #[error("recovery credential is not enrolled for the current epoch")]
    NotEnrolled,
    #[error("recovery credential signature did not verify")]
    BadCredential,
    #[error("node signature does not match the card's node id")]
    BadNodeSignature,
    #[error("challenge does not match this card or owner")]
    ChallengeMismatch,
    #[error("timing is not trustworthy; window closed")]
    TimingUntrusted,
    #[error("ownership record belongs to a different node id")]
    WrongNode,
    #[error("ownership store: {0}")]
    Store(String),
    #[error("nonce source: {0}")]
    Nonce(String),
}

/// The card secret, normalized. Never sent to the node or to hello.
#[derive(Clone, PartialEq, Eq)]
pub struct CardSecret(String);

impl CardSecret {
    /// Accepts the printed form: spaces and dashes are dropped and ASCII is
    /// lowercased. What remains must be ≥ 16 RFC 4648 base32 characters.
    pub fn parse(input: &str) -> Result<Self, ClaimError> {
        let s: String = input
            .chars()
            .filter(|c| !matches!(c, '-' | ' '))
            .map(|c| c.to_ascii_lowercase())
            .collect();
        let base32 = |b: u8| b.is_ascii_lowercase() || (b'2'..=b'7').contains(&b);
        if s.len() < MIN_SECRET_CHARS || !s.bytes().all(base32) {
            return Err(ClaimError::MalformedSecret);
        }
        Ok(Self(s))
    }

    /// `k = HMAC-SHA256(salt, secret)`.
    pub fn derive_key(&self, salt: &Salt) -> VerifierKey {
        VerifierKey(hmac(salt, &[self.0.as_bytes()]))
    }
}

impl std::fmt::Debug for CardSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CardSecret(<redacted>)")
    }
}

/// `k`. Secret-equivalent for forging proofs; reading it off flash is
/// physical compromise.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifierKey(pub [u8; 32]);

impl std::fmt::Debug for VerifierKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("VerifierKey(<redacted>)")
    }
}

/// What the provisioning image writes: the salt and `k`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verifier {
    pub salt: Salt,
    pub k: VerifierKey,
}

impl Verifier {
    pub fn from_card(salt: Salt, secret: &CardSecret) -> Self {
        Self {
            salt,
            k: secret.derive_key(&salt),
        }
    }
}

/// An enrolled recovery credential, scoped to the epoch it was enrolled in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryCredential {
    pub key: PublicKey,
    pub epoch: u64,
}

/// The durable ownership record. Every write is a compare-and-swap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnershipRecord {
    pub node_id: NodeId,
    /// Present only while unclaimed and provisioned; retired by a claim.
    pub verifier: Option<Verifier>,
    pub owner: Option<PublicKey>,
    /// Ownership epoch. [`UNCLAIMED_EPOCH`] while unclaimed.
    pub epoch: u64,
    /// Authority epoch; only ever advances (claim, recovery, reset).
    pub authority_epoch: u64,
    pub recovery_credentials: Vec<RecoveryCredential>,
}

impl OwnershipRecord {
    /// A developer-owned node: no verifier, no owner.
    pub fn unprovisioned(node_id: NodeId) -> Self {
        Self {
            node_id,
            verifier: None,
            owner: None,
            epoch: UNCLAIMED_EPOCH,
            authority_epoch: 0,
            recovery_credentials: Vec::new(),
        }
    }

    pub fn is_claimable(&self) -> bool {
        self.owner.is_none() && self.verifier.is_some()
    }
}

/// What a setup-button press does, decided in meshd.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonEffect {
    /// Arm the claim or recovery window. WAN SSH is not touched.
    ArmCeremony,
    /// The existing `mjolnir-wan-admin toggle`. No ceremony arms.
    ToggleWanSsh,
}

impl ButtonEffect {
    /// The token `/etc/rc.wps/00-mjolnir-wan-admin` matches on.
    pub fn as_str(self) -> &'static str {
        match self {
            ButtonEffect::ArmCeremony => "ceremony",
            ButtonEffect::ToggleWanSsh => "wan-ssh",
        }
    }
}

/// The three-state dispatch rule (design.md §Ceremony pin):
/// verifier present and unclaimed, or owned with a pending recovery whose
/// credential already verified → arm; no verifier → keep the WAN-SSH toggle;
/// owned with no verified pending recovery → WAN-SSH toggle.
pub fn button_effect(record: &OwnershipRecord, verified_recovery_pending: bool) -> ButtonEffect {
    match (&record.owner, &record.verifier) {
        (None, Some(_)) => ButtonEffect::ArmCeremony,
        (None, None) => ButtonEffect::ToggleWanSsh,
        (Some(_), _) if verified_recovery_pending => ButtonEffect::ArmCeremony,
        (Some(_), _) => ButtonEffect::ToggleWanSsh,
    }
}

/// Write the current effect for the rc.wps hook (tmp + rename).
pub fn publish_button_effect(dir: &Path, effect: ButtonEffect) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let tmp = dir.join(format!(".{BUTTON_EFFECT_FILE}.tmp"));
    fs::write(&tmp, format!("{}\n", effect.as_str()))?;
    fs::rename(&tmp, dir.join(BUTTON_EFFECT_FILE))
}

/// Consume a press signalled by the rc.wps hook. `true` once per press.
pub fn take_press(dir: &Path) -> io::Result<bool> {
    match fs::remove_file(dir.join(PRESS_FILE)) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

/// The fixed-width proof input: owner pk (32) ‖ node id (32) ‖ epoch (u64
/// big-endian) ‖ nonce (32).
pub fn proof_message(owner: &PublicKey, node_id: &NodeId, epoch: u64, nonce: &Nonce) -> [u8; 104] {
    let mut m = [0u8; 104];
    m[..32].copy_from_slice(owner);
    m[32..64].copy_from_slice(node_id);
    m[64..72].copy_from_slice(&epoch.to_be_bytes());
    m[72..].copy_from_slice(nonce);
    m
}

/// `HMAC-SHA256(k, owner pk ‖ node id ‖ epoch ‖ nonce)`.
pub fn compute_proof(
    k: &VerifierKey,
    owner: &PublicKey,
    node_id: &NodeId,
    epoch: u64,
    nonce: &Nonce,
) -> ProofMac {
    hmac(&k.0, &[&proof_message(owner, node_id, epoch, nonce)])
}

/// Constant-time check of a proof MAC.
fn proof_verifies(
    k: &VerifierKey,
    owner: &PublicKey,
    node_id: &NodeId,
    epoch: u64,
    nonce: &Nonce,
    mac: &ProofMac,
) -> bool {
    let mut h = HmacSha256::new_from_slice(&k.0).expect("HMAC accepts any key length");
    h.update(&proof_message(owner, node_id, epoch, nonce));
    h.verify_slice(mac).is_ok()
}

fn hmac(key: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    let mut h = HmacSha256::new_from_slice(key).expect("HMAC accepts any key length");
    for p in parts {
        h.update(p);
    }
    h.finalize().into_bytes().into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CeremonyKind {
    Claim = 1,
    Recovery = 2,
}

/// The node-signed challenge. It carries a request expiry, not a window
/// expiry: the window does not exist until the press (advise r3 C3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Challenge {
    pub kind: CeremonyKind,
    pub request_id: u64,
    pub node_id: NodeId,
    /// Claim: the claiming owner. Recovery: the new owner.
    pub owner: PublicKey,
    pub epoch: u64,
    pub nonce: Nonce,
    /// Claim: the verifier salt. Recovery: zero.
    pub salt: Salt,
    /// Recovery: the enrolled credential. Claim: zero.
    pub credential: PublicKey,
    /// Milliseconds until an unpressed request lapses.
    pub request_ttl_ms: u64,
}

impl Challenge {
    /// The bytes the node signs (and a recovery credential signs).
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut m = Vec::with_capacity(CHALLENGE_TAG.len() + 1 + 8 + 32 * 4 + 16 + 8 + 8);
        m.extend_from_slice(CHALLENGE_TAG);
        m.push(self.kind as u8);
        m.extend_from_slice(&self.request_id.to_be_bytes());
        m.extend_from_slice(&self.node_id);
        m.extend_from_slice(&self.owner);
        m.extend_from_slice(&self.epoch.to_be_bytes());
        m.extend_from_slice(&self.nonce);
        m.extend_from_slice(&self.salt);
        m.extend_from_slice(&self.credential);
        m.extend_from_slice(&self.request_ttl_ms.to_be_bytes());
        m
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedChallenge {
    pub challenge: Challenge,
    pub signature: [u8; 64],
}

/// What the client sends through hello. There is no field for the card
/// secret or for `k`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofSubmission {
    pub request_id: u64,
    pub owner: PublicKey,
    pub nonce: Nonce,
    pub mac: ProofMac,
}

/// A node-signed result the client accepts instead of a relay's word (C6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub kind: CeremonyKind,
    pub request_id: u64,
    pub node_id: NodeId,
    pub owner: PublicKey,
    pub epoch: u64,
    pub authority_epoch: u64,
    pub nonce: Nonce,
}

impl Outcome {
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut m = Vec::with_capacity(OUTCOME_TAG.len() + 1 + 8 + 32 * 3 + 16);
        m.extend_from_slice(OUTCOME_TAG);
        m.push(self.kind as u8);
        m.extend_from_slice(&self.request_id.to_be_bytes());
        m.extend_from_slice(&self.node_id);
        m.extend_from_slice(&self.owner);
        m.extend_from_slice(&self.epoch.to_be_bytes());
        m.extend_from_slice(&self.authority_epoch.to_be_bytes());
        m.extend_from_slice(&self.nonce);
        m
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedOutcome {
    pub outcome: Outcome,
    pub signature: [u8; 64],
}

fn verify_sig(key: &PublicKey, msg: &[u8], sig: &[u8; 64]) -> bool {
    VerifyingKey::from_bytes(key)
        .and_then(|vk| vk.verify_strict(msg, &Signature::from_bytes(sig)))
        .is_ok()
}

// ── client side (Lightning Admin) ───────────────────────────────────────────

/// What the purchaser types in: the card.
#[derive(Debug, Clone)]
pub struct ClaimCard {
    pub node_id: NodeId,
    pub secret: CardSecret,
}

/// Client step: check the node signature against the card's node id, then
/// compute the proof. Nothing is derived from the secret until the signature
/// and the challenge fields check out.
pub fn client_prove(
    card: &ClaimCard,
    owner: &PublicKey,
    signed: &SignedChallenge,
) -> Result<ProofSubmission, ClaimError> {
    let c = &signed.challenge;
    if !verify_sig(&card.node_id, &c.signing_bytes(), &signed.signature) {
        return Err(ClaimError::BadNodeSignature);
    }
    if c.kind != CeremonyKind::Claim || c.node_id != card.node_id || &c.owner != owner {
        return Err(ClaimError::ChallengeMismatch);
    }
    let k = card.secret.derive_key(&c.salt);
    Ok(ProofSubmission {
        request_id: c.request_id,
        owner: *owner,
        nonce: c.nonce,
        mac: compute_proof(&k, owner, &card.node_id, c.epoch, &c.nonce),
    })
}

/// Client step: accept only a node-signed outcome naming this owner.
pub fn client_verify_outcome(
    node_id: &NodeId,
    owner: &PublicKey,
    signed: &SignedOutcome,
) -> Result<Outcome, ClaimError> {
    let o = &signed.outcome;
    if !verify_sig(node_id, &o.signing_bytes(), &signed.signature) {
        return Err(ClaimError::BadNodeSignature);
    }
    if &o.node_id != node_id || &o.owner != owner {
        return Err(ClaimError::ChallengeMismatch);
    }
    Ok(o.clone())
}

// ── persistence ─────────────────────────────────────────────────────────────

/// Durable ownership storage. `compare_and_swap` must be atomic with
/// respect to every other writer of the same record.
pub trait OwnershipStore: Send + Sync {
    fn load(&self) -> Result<OwnershipRecord, ClaimError>;
    /// Replace `expected` with `new`. On mismatch, returns the current record.
    fn compare_and_swap(
        &self,
        expected: &OwnershipRecord,
        new: &OwnershipRecord,
    ) -> Result<(), CasConflict>;
}

#[derive(Debug)]
pub enum CasConflict {
    Changed(Box<OwnershipRecord>),
    Store(ClaimError),
}

/// In-memory store (tests, and callers that persist elsewhere).
pub struct MemoryOwnershipStore(Mutex<OwnershipRecord>);

impl MemoryOwnershipStore {
    pub fn new(record: OwnershipRecord) -> Self {
        Self(Mutex::new(record))
    }
}

impl OwnershipStore for MemoryOwnershipStore {
    fn load(&self) -> Result<OwnershipRecord, ClaimError> {
        Ok(self.0.lock().expect("store lock").clone())
    }

    fn compare_and_swap(
        &self,
        expected: &OwnershipRecord,
        new: &OwnershipRecord,
    ) -> Result<(), CasConflict> {
        let mut cur = self.0.lock().expect("store lock");
        if *cur != *expected {
            return Err(CasConflict::Changed(Box::new(cur.clone())));
        }
        *cur = new.clone();
        Ok(())
    }
}

/// JSON file store. Compare-and-swap holds an in-process lock across read,
/// compare, and an fsynced tmp + rename; meshd is the record's only writer.
/// A missing file reads as [`OwnershipRecord::unprovisioned`].
pub struct FileOwnershipStore {
    path: PathBuf,
    node_id: NodeId,
    lock: Mutex<()>,
}

impl FileOwnershipStore {
    pub fn new(path: impl Into<PathBuf>, node_id: NodeId) -> Self {
        Self {
            path: path.into(),
            node_id,
            lock: Mutex::new(()),
        }
    }

    fn read(&self) -> Result<OwnershipRecord, ClaimError> {
        match fs::read(&self.path) {
            Ok(bytes) => {
                serde_json::from_slice(&bytes).map_err(|e| ClaimError::Store(e.to_string()))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                Ok(OwnershipRecord::unprovisioned(self.node_id))
            }
            Err(e) => Err(ClaimError::Store(e.to_string())),
        }
    }

    fn write(&self, record: &OwnershipRecord) -> Result<(), ClaimError> {
        let err = |e: io::Error| ClaimError::Store(e.to_string());
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir).map_err(err)?;
        }
        let mut tmp = self.path.clone().into_os_string();
        tmp.push(".tmp");
        let tmp = PathBuf::from(tmp);
        // Compact JSON: the rc.wps hook greps for `"verifier":null`.
        let bytes = serde_json::to_vec(record).map_err(|e| ClaimError::Store(e.to_string()))?;
        let mut f = fs::File::create(&tmp).map_err(err)?;
        f.write_all(&bytes).map_err(err)?;
        f.sync_all().map_err(err)?;
        fs::rename(&tmp, &self.path).map_err(err)?;
        if let Some(Ok(d)) = self.path.parent().map(fs::File::open) {
            let _ = d.sync_all();
        }
        Ok(())
    }
}

impl OwnershipStore for FileOwnershipStore {
    fn load(&self) -> Result<OwnershipRecord, ClaimError> {
        let _g = self.lock.lock().expect("store lock");
        self.read()
    }

    fn compare_and_swap(
        &self,
        expected: &OwnershipRecord,
        new: &OwnershipRecord,
    ) -> Result<(), CasConflict> {
        let _g = self.lock.lock().expect("store lock");
        let cur = self.read().map_err(CasConflict::Store)?;
        if cur != *expected {
            return Err(CasConflict::Changed(Box::new(cur)));
        }
        self.write(new).map_err(CasConflict::Store)
    }
}

// ── nonces ──────────────────────────────────────────────────────────────────

pub trait NonceSource: Send {
    fn next_nonce(&mut self) -> Result<Nonce, ClaimError>;
}

/// Reads the kernel CSPRNG.
pub struct OsNonceSource;

impl NonceSource for OsNonceSource {
    fn next_nonce(&mut self) -> Result<Nonce, ClaimError> {
        let mut n = [0u8; 32];
        fs::File::open("/dev/urandom")
            .and_then(|mut f| f.read_exact(&mut n))
            .map_err(|e| ClaimError::Nonce(e.to_string()))?;
        Ok(n)
    }
}

// ── node side (meshd) ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct PendingClaim {
    request_id: u64,
    owner: PublicKey,
    epoch: u64,
    nonce: Nonce,
    expires_at_ms: u64,
}

#[derive(Debug, Clone)]
struct PendingRecovery {
    request_id: u64,
    credential: PublicKey,
    new_owner: PublicKey,
    epoch: u64,
    nonce: Nonce,
    expires_at_ms: u64,
    /// The signed challenge bytes the credential must sign.
    challenge_bytes: Vec<u8>,
    verified: bool,
}

#[derive(Debug, Clone)]
enum Covers {
    Claims(Vec<u64>),
    Recovery(u64),
}

#[derive(Debug, Clone)]
struct Window {
    closes_at_ms: u64,
    failed: u32,
    covers: Covers,
}

/// Result of a press.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PressOutcome {
    pub effect: ButtonEffect,
    /// Requests the armed window confirms (0 when the effect is the toggle).
    pub covered: usize,
}

/// The ceremony state machine. The window and pending requests are volatile:
/// constructing a fresh `OwnerClaim` (what a reboot does) closes them.
///
/// Times are monotonic milliseconds supplied by the caller. A clock that
/// moves backwards is loss of trustworthy timing.
pub struct OwnerClaim {
    signing: SigningKey,
    node_id: NodeId,
    store: Arc<dyn OwnershipStore>,
    nonces: Box<dyn NonceSource>,
    window_ms: u64,
    next_request_id: u64,
    claims: Vec<PendingClaim>,
    recoveries: Vec<PendingRecovery>,
    window: Option<Window>,
    last_now_ms: Option<u64>,
}

impl OwnerClaim {
    /// Start (or restart) with no window armed.
    pub fn boot(
        signing: SigningKey,
        store: Arc<dyn OwnershipStore>,
        nonces: Box<dyn NonceSource>,
    ) -> Result<Self, ClaimError> {
        let node_id = signing.verifying_key().to_bytes();
        if store.load()?.node_id != node_id {
            return Err(ClaimError::WrongNode);
        }
        Ok(Self {
            signing,
            node_id,
            store,
            nonces,
            window_ms: DEFAULT_WINDOW.as_millis() as u64,
            next_request_id: 1,
            claims: Vec::new(),
            recoveries: Vec::new(),
            window: None,
            last_now_ms: None,
        })
    }

    /// Configure the physical window length.
    pub fn with_window(mut self, window: Duration) -> Self {
        self.window_ms = window.as_millis() as u64;
        self
    }

    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub fn record(&self) -> Result<OwnershipRecord, ClaimError> {
        self.store.load()
    }

    /// Whether an armed, unexpired window exists at `now_ms`.
    pub fn window_open(&self, now_ms: u64) -> bool {
        self.window
            .as_ref()
            .is_some_and(|w| now_ms < w.closes_at_ms)
    }

    /// The press decision for the current state, for publishing to rc.wps.
    pub fn button_effect(&self, now_ms: u64) -> Result<ButtonEffect, ClaimError> {
        let verified = self
            .recoveries
            .iter()
            .any(|r| r.verified && now_ms < r.expires_at_ms);
        Ok(button_effect(&self.store.load()?, verified))
    }

    /// Loss of trustworthy timing: close the window and drop pending requests.
    pub fn timing_lost(&mut self) {
        self.window = None;
        self.claims.clear();
        self.recoveries.clear();
    }

    fn observe(&mut self, now_ms: u64) -> Result<(), ClaimError> {
        if self.last_now_ms.is_some_and(|last| now_ms < last) {
            self.timing_lost();
            self.last_now_ms = Some(now_ms);
            return Err(ClaimError::TimingUntrusted);
        }
        self.last_now_ms = Some(now_ms);
        if self
            .window
            .as_ref()
            .is_some_and(|w| now_ms >= w.closes_at_ms)
        {
            self.window = None;
        }
        self.claims.retain(|c| now_ms < c.expires_at_ms);
        self.recoveries.retain(|r| now_ms < r.expires_at_ms);
        Ok(())
    }

    fn cas(&self, expected: &OwnershipRecord, new: &OwnershipRecord) -> Result<(), ClaimError> {
        self.store
            .compare_and_swap(expected, new)
            .map_err(|e| match e {
                CasConflict::Changed(_) => ClaimError::AlreadyClaimed,
                CasConflict::Store(e) => e,
            })
    }

    fn sign_challenge(&self, challenge: Challenge) -> SignedChallenge {
        let signature = self.signing.sign(&challenge.signing_bytes()).to_bytes();
        SignedChallenge {
            challenge,
            signature,
        }
    }

    fn sign_outcome(&self, outcome: Outcome) -> SignedOutcome {
        let signature = self.signing.sign(&outcome.signing_bytes()).to_bytes();
        SignedOutcome { outcome, signature }
    }

    fn claim_challenge(&self, p: &PendingClaim, salt: Salt, now_ms: u64) -> SignedChallenge {
        self.sign_challenge(Challenge {
            kind: CeremonyKind::Claim,
            request_id: p.request_id,
            node_id: self.node_id,
            owner: p.owner,
            epoch: p.epoch,
            nonce: p.nonce,
            salt,
            credential: [0; 32],
            request_ttl_ms: p.expires_at_ms.saturating_sub(now_ms),
        })
    }

    /// Provisioning image: write a verifier onto an unclaimed, unprovisioned
    /// node (first install, or after a reset).
    pub fn provision_verifier(&mut self, verifier: Verifier) -> Result<(), ClaimError> {
        let cur = self.store.load()?;
        if cur.owner.is_some() || cur.verifier.is_some() {
            return Err(ClaimError::AlreadyProvisioned);
        }
        let new = OwnershipRecord {
            verifier: Some(verifier),
            ..cur.clone()
        };
        self.cas(&cur, &new)
    }

    /// Step 1 of a claim: issue a node-signed challenge for `owner`.
    pub fn request_claim(
        &mut self,
        owner: PublicKey,
        now_ms: u64,
    ) -> Result<SignedChallenge, ClaimError> {
        self.observe(now_ms)?;
        let rec = self.store.load()?;
        let Some(verifier) = rec.verifier.as_ref().filter(|_| rec.owner.is_none()) else {
            return Err(ClaimError::NotClaimable);
        };
        let pending = PendingClaim {
            request_id: self.next_request_id,
            owner,
            epoch: rec.epoch,
            nonce: self.nonces.next_nonce()?,
            expires_at_ms: now_ms.saturating_add(self.window_ms),
        };
        self.next_request_id += 1;
        if self.claims.len() >= MAX_PENDING {
            self.claims.remove(0);
        }
        let signed = self.claim_challenge(&pending, verifier.salt, now_ms);
        self.claims.push(pending);
        Ok(signed)
    }

    /// Step 2: a setup-button press. Arms a fresh window that confirms only
    /// the requests pending now, or returns [`ButtonEffect::ToggleWanSsh`]
    /// and arms nothing.
    pub fn press(&mut self, now_ms: u64) -> Result<PressOutcome, ClaimError> {
        self.observe(now_ms)?;
        let effect = self.button_effect(now_ms)?;
        if effect == ButtonEffect::ToggleWanSsh {
            return Ok(PressOutcome { effect, covered: 0 });
        }
        let rec = self.store.load()?;
        let covers = if rec.owner.is_none() {
            Covers::Claims(self.claims.iter().map(|c| c.request_id).collect())
        } else {
            let id = self
                .recoveries
                .iter()
                .rev()
                .find(|r| r.verified)
                .map(|r| r.request_id)
                .expect("effect ArmCeremony on an owned node implies a verified recovery");
            Covers::Recovery(id)
        };
        let covered = match &covers {
            Covers::Claims(ids) => ids.len(),
            Covers::Recovery(_) => 1,
        };
        self.window = Some(Window {
            closes_at_ms: now_ms.saturating_add(self.window_ms),
            failed: 0,
            covers,
        });
        Ok(PressOutcome { effect, covered })
    }

    /// Count a failed proof; close the window at the cap.
    fn fail(&mut self) -> bool {
        let w = self.window.as_mut().expect("fail() only with a window");
        w.failed += 1;
        if w.failed >= MAX_FAILED_PROOFS {
            self.window = None;
            self.claims.clear();
            return true;
        }
        false
    }

    /// Step 3: verify the proof inside the armed window and run the
    /// compare-and-swap from unclaimed to epoch 1.
    pub fn submit_proof(
        &mut self,
        proof: &ProofSubmission,
        now_ms: u64,
    ) -> Result<SignedOutcome, ClaimError> {
        self.observe(now_ms)?;
        let Some(window) = self.window.as_ref() else {
            return Err(ClaimError::NoWindow);
        };
        let Covers::Claims(covered) = &window.covers else {
            return Err(ClaimError::NoWindow);
        };
        let covered = covered.contains(&proof.request_id);
        let rec = self.store.load()?;
        if !rec.is_claimable() {
            // Claimed (or reset) behind this window's back: it confirms nothing.
            self.window = None;
            self.claims.clear();
            return Err(ClaimError::AlreadyClaimed);
        }
        let pending = self
            .claims
            .iter()
            .position(|c| c.request_id == proof.request_id);

        let ok = match (covered, pending, rec.verifier.as_ref()) {
            (true, Some(i), Some(v)) if rec.owner.is_none() => {
                let p = &self.claims[i];
                // Owner and nonce must be this request's; the MAC binds both.
                (p.owner == proof.owner)
                    & (p.nonce == proof.nonce)
                    & (p.epoch == rec.epoch)
                    & proof_verifies(
                        &v.k,
                        &proof.owner,
                        &self.node_id,
                        p.epoch,
                        &proof.nonce,
                        &proof.mac,
                    )
            }
            _ => false,
        };

        if !ok {
            if self.fail() {
                return Err(ClaimError::AttemptsExhausted);
            }
            // Fresh nonce per attempt: a sniffed MAC cannot be replayed.
            let retry = match (pending, rec.verifier.as_ref()) {
                (Some(i), Some(v)) if covered => {
                    let salt = v.salt;
                    let nonce = self.nonces.next_nonce()?;
                    self.claims[i].nonce = nonce;
                    let p = self.claims[i].clone();
                    Some(Box::new(self.claim_challenge(&p, salt, now_ms)))
                }
                _ => None,
            };
            return Err(ClaimError::BadProof { retry });
        }

        let p = self.claims[pending.expect("ok implies pending")].clone();
        let new = OwnershipRecord {
            verifier: None, // one-time: a claim retires k (advise r3 C5)
            owner: Some(p.owner),
            epoch: 1,
            authority_epoch: rec.authority_epoch + 1,
            recovery_credentials: Vec::new(),
            ..rec.clone()
        };
        // The window is consumed whether the swap wins or loses.
        self.window = None;
        self.claims.clear();
        self.cas(&rec, &new)?;
        Ok(self.sign_outcome(Outcome {
            kind: CeremonyKind::Claim,
            request_id: p.request_id,
            node_id: self.node_id,
            owner: p.owner,
            epoch: new.epoch,
            authority_epoch: new.authority_epoch,
            nonce: p.nonce,
        }))
    }

    /// Enroll a recovery credential under the current epoch. Who may enroll
    /// is the household-authority engine's decision, not this module's.
    pub fn enroll_recovery_credential(&mut self, key: PublicKey) -> Result<(), ClaimError> {
        let cur = self.store.load()?;
        if cur.owner.is_none() {
            return Err(ClaimError::NotOwned);
        }
        let mut new = cur.clone();
        new.recovery_credentials.retain(|c| c.key != key);
        new.recovery_credentials.push(RecoveryCredential {
            key,
            epoch: cur.epoch,
        });
        self.cas(&cur, &new)
    }

    /// Recovery step 1: a challenge for an enrolled credential.
    pub fn request_recovery(
        &mut self,
        credential: PublicKey,
        new_owner: PublicKey,
        now_ms: u64,
    ) -> Result<SignedChallenge, ClaimError> {
        self.observe(now_ms)?;
        let rec = self.store.load()?;
        if rec.owner.is_none() {
            return Err(ClaimError::NotOwned);
        }
        if !rec
            .recovery_credentials
            .iter()
            .any(|c| c.key == credential && c.epoch == rec.epoch)
        {
            return Err(ClaimError::NotEnrolled);
        }
        let challenge = Challenge {
            kind: CeremonyKind::Recovery,
            request_id: self.next_request_id,
            node_id: self.node_id,
            owner: new_owner,
            epoch: rec.epoch,
            nonce: self.nonces.next_nonce()?,
            salt: [0; SALT_LEN],
            credential,
            request_ttl_ms: self.window_ms,
        };
        self.next_request_id += 1;
        if self.recoveries.len() >= MAX_PENDING {
            self.recoveries.remove(0);
        }
        self.recoveries.push(PendingRecovery {
            request_id: challenge.request_id,
            credential,
            new_owner,
            epoch: challenge.epoch,
            nonce: challenge.nonce,
            expires_at_ms: now_ms.saturating_add(self.window_ms),
            challenge_bytes: challenge.signing_bytes(),
            verified: false,
        });
        Ok(self.sign_challenge(challenge))
    }

    /// Recovery step 2: the credential signs the challenge bytes. Success
    /// makes the recovery pending-and-verified, so the next press arms.
    pub fn verify_recovery_credential(
        &mut self,
        request_id: u64,
        signature: &[u8; 64],
        now_ms: u64,
    ) -> Result<(), ClaimError> {
        self.observe(now_ms)?;
        let Some(i) = self
            .recoveries
            .iter()
            .position(|r| r.request_id == request_id)
        else {
            return Err(ClaimError::UnknownRequest);
        };
        let p = &self.recoveries[i];
        if !verify_sig(&p.credential, &p.challenge_bytes, signature) {
            self.recoveries.remove(i);
            return Err(ClaimError::BadCredential);
        }
        self.recoveries[i].verified = true;
        Ok(())
    }

    /// Recovery step 4 (after the press): advance both epochs, install the
    /// new owner, and retire every prior recovery credential.
    pub fn complete_recovery(
        &mut self,
        request_id: u64,
        now_ms: u64,
    ) -> Result<SignedOutcome, ClaimError> {
        self.observe(now_ms)?;
        match self.window.as_ref().map(|w| &w.covers) {
            Some(Covers::Recovery(id)) if *id == request_id => {}
            _ => return Err(ClaimError::NoWindow),
        }
        let Some(p) = self
            .recoveries
            .iter()
            .find(|r| r.request_id == request_id && r.verified)
            .cloned()
        else {
            return Err(ClaimError::UnknownRequest);
        };
        self.window = None;
        self.recoveries.clear();
        let rec = self.store.load()?;
        if rec.owner.is_none() || rec.epoch != p.epoch {
            return Err(ClaimError::AlreadyClaimed);
        }
        let new = OwnershipRecord {
            owner: Some(p.new_owner),
            epoch: rec.epoch + 1,
            authority_epoch: rec.authority_epoch + 1,
            recovery_credentials: Vec::new(),
            ..rec.clone()
        };
        self.cas(&rec, &new)?;
        Ok(self.sign_outcome(Outcome {
            kind: CeremonyKind::Recovery,
            request_id,
            node_id: self.node_id,
            owner: p.new_owner,
            epoch: new.epoch,
            authority_epoch: new.authority_epoch,
            nonce: p.nonce,
        }))
    }

    /// Destructive local reset: clear the binding, the verifier, and the
    /// recovery credentials. The original card is not re-armed; the node
    /// stays unclaimed until a provisioning image writes a new verifier.
    pub fn reset(&mut self) -> Result<(), ClaimError> {
        let cur = self.store.load()?;
        let new = OwnershipRecord {
            verifier: None,
            owner: None,
            epoch: UNCLAIMED_EPOCH,
            authority_epoch: cur.authority_epoch + 1,
            recovery_credentials: Vec::new(),
            ..cur.clone()
        };
        self.window = None;
        self.claims.clear();
        self.recoveries.clear();
        self.cas(&cur, &new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "abcdefghijk23456";
    const SALT: Salt = [7; SALT_LEN];
    const T0: u64 = 1_000_000;

    struct CountingNonces(u64);
    impl NonceSource for CountingNonces {
        fn next_nonce(&mut self) -> Result<Nonce, ClaimError> {
            self.0 += 1;
            Ok(*blake3::hash(&self.0.to_be_bytes()).as_bytes())
        }
    }

    fn node_key() -> SigningKey {
        SigningKey::from_bytes(&[1; 32])
    }

    fn owner(n: u8) -> PublicKey {
        SigningKey::from_bytes(&[n; 32]).verifying_key().to_bytes()
    }

    fn card() -> ClaimCard {
        ClaimCard {
            node_id: node_key().verifying_key().to_bytes(),
            secret: CardSecret::parse(SECRET).unwrap(),
        }
    }

    fn store_with_verifier() -> Arc<MemoryOwnershipStore> {
        let mut rec = OwnershipRecord::unprovisioned(node_key().verifying_key().to_bytes());
        rec.verifier = Some(Verifier::from_card(SALT, &card().secret));
        Arc::new(MemoryOwnershipStore::new(rec))
    }

    fn node(store: Arc<dyn OwnershipStore>, seed: u64) -> OwnerClaim {
        OwnerClaim::boot(node_key(), store, Box::new(CountingNonces(seed))).unwrap()
    }

    fn provisioned() -> OwnerClaim {
        node(store_with_verifier(), 0)
    }

    fn claimed() -> OwnerClaim {
        let mut n = provisioned();
        let ch = n.request_claim(owner(2), T0).unwrap();
        n.press(T0 + 1).unwrap();
        let proof = client_prove(&card(), &owner(2), &ch).unwrap();
        n.submit_proof(&proof, T0 + 2).unwrap();
        n
    }

    #[test]
    fn card_secret_is_at_least_80_bits_of_lowercase_base32() {
        assert!(CardSecret::parse(SECRET).is_ok());
        assert!(CardSecret::parse("ABCD-EFGH-IJK2-3456").is_ok());
        assert_eq!(
            CardSecret::parse("abcdefghijk2345"),
            Err(ClaimError::MalformedSecret)
        );
        assert_eq!(
            CardSecret::parse("abcdefghijk23450"),
            Err(ClaimError::MalformedSecret),
            "0 is not in the RFC 4648 alphabet"
        );
        assert!(!format!("{:?}", CardSecret::parse(SECRET).unwrap()).contains(SECRET));
    }

    #[test]
    fn k_and_proof_are_the_pinned_hmacs() {
        let secret = CardSecret::parse(SECRET).unwrap();
        let mut h = HmacSha256::new_from_slice(&SALT).unwrap();
        h.update(SECRET.as_bytes());
        let k: [u8; 32] = h.finalize().into_bytes().into();
        assert_eq!(secret.derive_key(&SALT).0, k);

        let (o, id, nonce) = (owner(2), [9u8; 32], [5u8; 32]);
        let mut msg = Vec::new();
        msg.extend_from_slice(&o);
        msg.extend_from_slice(&id);
        msg.extend_from_slice(&3u64.to_be_bytes());
        msg.extend_from_slice(&nonce);
        let mut h = HmacSha256::new_from_slice(&k).unwrap();
        h.update(&msg);
        let expect: [u8; 32] = h.finalize().into_bytes().into();
        assert_eq!(compute_proof(&VerifierKey(k), &o, &id, 3, &nonce), expect);
    }

    #[test]
    fn owner_claim_happy_path_retires_k_and_returns_a_signed_outcome() {
        let mut n = provisioned();
        let ch = n.request_claim(owner(2), T0).unwrap();
        assert_eq!(ch.challenge.epoch, UNCLAIMED_EPOCH);
        let press = n.press(T0 + 10).unwrap();
        assert_eq!(press.effect, ButtonEffect::ArmCeremony);
        assert_eq!(press.covered, 1);
        let proof = client_prove(&card(), &owner(2), &ch).unwrap();
        let out = n.submit_proof(&proof, T0 + 20).unwrap();
        let o = client_verify_outcome(&card().node_id, &owner(2), &out).unwrap();
        assert_eq!(o.epoch, 1);

        let rec = n.record().unwrap();
        assert_eq!(rec.owner, Some(owner(2)));
        assert_eq!(rec.epoch, 1);
        assert!(rec.verifier.is_none(), "a successful claim retires k");
        assert!(!n.window_open(T0 + 21), "success closes the window");
    }

    #[test]
    fn owner_claim_secret_only_fails() {
        let mut n = provisioned();
        let ch = n.request_claim(owner(2), T0).unwrap();
        let proof = client_prove(&card(), &owner(2), &ch).unwrap();
        assert_eq!(n.submit_proof(&proof, T0 + 1), Err(ClaimError::NoWindow));
        assert_eq!(n.record().unwrap().owner, None);
    }

    #[test]
    fn owner_claim_button_only_fails() {
        let mut n = provisioned();
        let ch = n.request_claim(owner(2), T0).unwrap();
        n.press(T0 + 1).unwrap();
        let wrong = ClaimCard {
            secret: CardSecret::parse("zzzzzzzzzzzzzzzz").unwrap(),
            ..card()
        };
        let proof = client_prove(&wrong, &owner(2), &ch).unwrap();
        assert!(matches!(
            n.submit_proof(&proof, T0 + 2),
            Err(ClaimError::BadProof { .. })
        ));
        assert_eq!(n.record().unwrap().owner, None);
    }

    #[test]
    fn owner_claim_press_before_request_confirms_nothing() {
        let mut n = provisioned();
        let press = n.press(T0).unwrap();
        assert_eq!(press.covered, 0);
        let ch = n.request_claim(owner(2), T0 + 1).unwrap();
        let proof = client_prove(&card(), &owner(2), &ch).unwrap();
        assert!(matches!(
            n.submit_proof(&proof, T0 + 2),
            Err(ClaimError::BadProof { retry: None })
        ));
        assert_eq!(n.record().unwrap().owner, None);
    }

    #[test]
    fn owner_claim_sniffed_hmac_with_substituted_owner_key_fails() {
        let mut n = provisioned();
        let purchaser = n.request_claim(owner(2), T0).unwrap();
        let attacker = n.request_claim(owner(3), T0).unwrap();
        n.press(T0 + 1).unwrap();
        let sniffed = client_prove(&card(), &owner(2), &purchaser).unwrap();

        // Same request, owner key swapped.
        let swapped = ProofSubmission {
            owner: owner(3),
            ..sniffed.clone()
        };
        assert!(matches!(
            n.submit_proof(&swapped, T0 + 2),
            Err(ClaimError::BadProof { .. })
        ));
        // Attacker's own request, purchaser's MAC.
        let replanted = ProofSubmission {
            request_id: attacker.challenge.request_id,
            owner: owner(3),
            nonce: attacker.challenge.nonce,
            mac: sniffed.mac,
        };
        assert!(matches!(
            n.submit_proof(&replanted, T0 + 3),
            Err(ClaimError::BadProof { .. })
        ));
        assert_eq!(n.record().unwrap().owner, None);
    }

    #[test]
    fn owner_claim_failed_proof_rotates_the_nonce() {
        let mut n = provisioned();
        let ch = n.request_claim(owner(2), T0).unwrap();
        n.press(T0 + 1).unwrap();
        let good = client_prove(&card(), &owner(2), &ch).unwrap();
        let bad = ProofSubmission {
            mac: [0; 32],
            ..good.clone()
        };
        let Err(ClaimError::BadProof { retry: Some(r1) }) = n.submit_proof(&bad, T0 + 2) else {
            panic!("expected a retry challenge");
        };
        assert_ne!(r1.challenge.nonce, ch.challenge.nonce);
        let Err(ClaimError::BadProof { retry: Some(r2) }) = n.submit_proof(&good, T0 + 3) else {
            panic!("a proof over the retired nonce must not replay");
        };
        let fresh = client_prove(&card(), &owner(2), &r2).unwrap();
        assert!(n.submit_proof(&fresh, T0 + 4).is_ok());
    }

    #[test]
    fn owner_claim_two_racing_claimants_produce_one_winner() {
        // Two household members holding the same card, one window.
        let n = Arc::new(Mutex::new(provisioned()));
        let a = n.lock().unwrap().request_claim(owner(2), T0).unwrap();
        let b = n.lock().unwrap().request_claim(owner(3), T0).unwrap();
        assert_eq!(n.lock().unwrap().press(T0 + 1).unwrap().covered, 2);
        let pa = client_prove(&card(), &owner(2), &a).unwrap();
        let pb = client_prove(&card(), &owner(3), &b).unwrap();

        let handles: Vec<_> = [pa, pb]
            .into_iter()
            .map(|p| {
                let n = Arc::clone(&n);
                std::thread::spawn(move || n.lock().unwrap().submit_proof(&p, T0 + 2))
            })
            .collect();
        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        let winners: Vec<_> = results.iter().filter_map(|r| r.as_ref().ok()).collect();
        assert_eq!(winners.len(), 1, "{results:?}");
        let rec = n.lock().unwrap().record().unwrap();
        assert_eq!(rec.owner, Some(winners[0].outcome.owner));
        assert_eq!(rec.epoch, 1);
    }

    #[test]
    fn owner_claim_store_cas_lets_only_the_first_claim_land() {
        // Two state machines over one store: both verify, one swap wins.
        let store = store_with_verifier();
        let mut n1 = node(store.clone(), 0);
        let mut n2 = node(store.clone(), 1000);
        let c1 = n1.request_claim(owner(2), T0).unwrap();
        let c2 = n2.request_claim(owner(3), T0).unwrap();
        n1.press(T0 + 1).unwrap();
        n2.press(T0 + 1).unwrap();
        let p1 = client_prove(&card(), &owner(2), &c1).unwrap();
        let p2 = client_prove(&card(), &owner(3), &c2).unwrap();
        assert!(n1.submit_proof(&p1, T0 + 2).is_ok());
        assert_eq!(
            n2.submit_proof(&p2, T0 + 2),
            Err(ClaimError::AlreadyClaimed)
        );
        assert_eq!(store.load().unwrap().owner, Some(owner(2)));
        // A retry cannot overwrite it either.
        assert_eq!(
            n2.request_claim(owner(3), T0 + 3),
            Err(ClaimError::NotClaimable)
        );
    }

    #[test]
    fn owner_claim_file_store_persists_and_refuses_stale_swaps() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("owner-claim.json");
        let id = node_key().verifying_key().to_bytes();
        let store = FileOwnershipStore::new(&path, id);
        let blank = store.load().unwrap();
        assert_eq!(blank, OwnershipRecord::unprovisioned(id));

        let mut provisioned = blank.clone();
        provisioned.verifier = Some(Verifier::from_card(SALT, &card().secret));
        store.compare_and_swap(&blank, &provisioned).unwrap();
        assert!(
            !fs::read_to_string(&path)
                .unwrap()
                .contains("\"verifier\":null")
        );
        assert!(matches!(
            store.compare_and_swap(&blank, &blank),
            Err(CasConflict::Changed(_))
        ));
        let reopened = FileOwnershipStore::new(&path, id);
        assert_eq!(reopened.load().unwrap(), provisioned);
        assert!(!fs::read_to_string(&path).unwrap().contains(SECRET));
        // The rc.wps hook reads a retired verifier as this exact token.
        let claimed = OwnershipRecord {
            verifier: None,
            owner: Some(owner(2)),
            epoch: 1,
            ..provisioned.clone()
        };
        reopened.compare_and_swap(&provisioned, &claimed).unwrap();
        assert!(
            fs::read_to_string(&path)
                .unwrap()
                .contains("\"verifier\":null")
        );
    }

    #[test]
    fn owner_claim_ceremony_press_does_not_open_wan_ssh() {
        let mut n = provisioned();
        n.request_claim(owner(2), T0).unwrap();
        assert_eq!(n.press(T0 + 1).unwrap().effect, ButtonEffect::ArmCeremony);
        assert_eq!(ButtonEffect::ArmCeremony.as_str(), "ceremony");
    }

    #[test]
    fn owner_claim_no_verifier_keeps_the_wan_toggle_and_arms_nothing() {
        let id = node_key().verifying_key().to_bytes();
        let store = Arc::new(MemoryOwnershipStore::new(OwnershipRecord::unprovisioned(
            id,
        )));
        let mut n = node(store, 0);
        assert_eq!(n.request_claim(owner(2), T0), Err(ClaimError::NotClaimable));
        let press = n.press(T0 + 1).unwrap();
        assert_eq!(press.effect, ButtonEffect::ToggleWanSsh);
        assert!(!n.window_open(T0 + 2));
        assert_eq!(ButtonEffect::ToggleWanSsh.as_str(), "wan-ssh");
    }

    #[test]
    fn owner_claim_owned_node_toggles_unless_a_verified_recovery_is_pending() {
        let mut n = claimed();
        assert_eq!(n.press(T0 + 10).unwrap().effect, ButtonEffect::ToggleWanSsh);
        assert!(!n.window_open(T0 + 11));

        let cred = SigningKey::from_bytes(&[4; 32]);
        n.enroll_recovery_credential(cred.verifying_key().to_bytes())
            .unwrap();
        let ch = n
            .request_recovery(cred.verifying_key().to_bytes(), owner(5), T0 + 20)
            .unwrap();
        assert_eq!(
            n.press(T0 + 21).unwrap().effect,
            ButtonEffect::ToggleWanSsh,
            "an unverified pending recovery does not arm"
        );
        let sig = cred.sign(&ch.challenge.signing_bytes()).to_bytes();
        n.verify_recovery_credential(ch.challenge.request_id, &sig, T0 + 21)
            .unwrap();
        assert_eq!(n.press(T0 + 22).unwrap().effect, ButtonEffect::ArmCeremony);
    }

    #[test]
    fn owner_claim_reboot_closes_the_window() {
        let store = store_with_verifier();
        let mut n = node(store.clone(), 0);
        let ch = n.request_claim(owner(2), T0).unwrap();
        n.press(T0 + 1).unwrap();
        assert!(n.window_open(T0 + 2));
        drop(n);
        let mut rebooted = node(store.clone(), 0);
        assert!(!rebooted.window_open(T0 + 2));
        let proof = client_prove(&card(), &owner(2), &ch).unwrap();
        assert_eq!(
            rebooted.submit_proof(&proof, T0 + 3),
            Err(ClaimError::NoWindow)
        );
        assert_eq!(store.load().unwrap().owner, None);
    }

    #[test]
    fn owner_claim_five_failed_proofs_close_the_window() {
        let mut n = provisioned();
        let ch = n.request_claim(owner(2), T0).unwrap();
        n.press(T0 + 1).unwrap();
        let mut current = ch.clone();
        for i in 0..MAX_FAILED_PROOFS - 1 {
            let bad = ProofSubmission {
                mac: [0; 32],
                ..client_prove(&card(), &owner(2), &current).unwrap()
            };
            match n.submit_proof(&bad, T0 + 2 + i as u64) {
                Err(ClaimError::BadProof { retry: Some(r) }) => current = *r,
                other => panic!("attempt {i}: {other:?}"),
            }
        }
        let bad = ProofSubmission {
            mac: [0; 32],
            ..client_prove(&card(), &owner(2), &current).unwrap()
        };
        assert_eq!(
            n.submit_proof(&bad, T0 + 10),
            Err(ClaimError::AttemptsExhausted)
        );
        assert!(!n.window_open(T0 + 11));
        let good = client_prove(&card(), &owner(2), &current).unwrap();
        assert_eq!(n.submit_proof(&good, T0 + 12), Err(ClaimError::NoWindow));

        // DoS only: the purchaser requests and presses again.
        let ch = n.request_claim(owner(2), T0 + 20).unwrap();
        n.press(T0 + 21).unwrap();
        let good = client_prove(&card(), &owner(2), &ch).unwrap();
        assert!(n.submit_proof(&good, T0 + 22).is_ok());
    }

    #[test]
    fn owner_claim_window_defaults_to_120_seconds() {
        let mut n = provisioned();
        let ch = n.request_claim(owner(2), T0).unwrap();
        n.press(T0 + 1).unwrap();
        assert!(n.window_open(T0 + 120_000));
        let proof = client_prove(&card(), &owner(2), &ch).unwrap();
        assert_eq!(
            n.submit_proof(&proof, T0 + 1 + 120_000),
            Err(ClaimError::NoWindow)
        );
    }

    #[test]
    fn owner_claim_loss_of_trustworthy_timing_closes_the_window() {
        let mut n = provisioned();
        let ch = n.request_claim(owner(2), T0).unwrap();
        n.press(T0 + 100).unwrap();
        let proof = client_prove(&card(), &owner(2), &ch).unwrap();
        assert_eq!(
            n.submit_proof(&proof, T0 + 50),
            Err(ClaimError::TimingUntrusted)
        );
        assert_eq!(n.submit_proof(&proof, T0 + 101), Err(ClaimError::NoWindow));
    }

    #[test]
    fn owner_claim_client_refuses_an_impostor_node_before_deriving_anything() {
        let impostor = SigningKey::from_bytes(&[9; 32]);
        let store = Arc::new(MemoryOwnershipStore::new(OwnershipRecord {
            verifier: Some(Verifier::from_card(SALT, &card().secret)),
            ..OwnershipRecord::unprovisioned(impostor.verifying_key().to_bytes())
        }));
        let mut fake = OwnerClaim::boot(impostor, store, Box::new(CountingNonces(0))).unwrap();
        let ch = fake.request_claim(owner(2), T0).unwrap();
        assert_eq!(
            client_prove(&card(), &owner(2), &ch),
            Err(ClaimError::BadNodeSignature)
        );
        // A relay rewriting the node id to the card's breaks the signature.
        let mut forged = ch.clone();
        forged.challenge.node_id = card().node_id;
        assert_eq!(
            client_prove(&card(), &owner(2), &forged),
            Err(ClaimError::BadNodeSignature)
        );
    }

    #[test]
    fn owner_claim_client_refuses_an_unsigned_outcome() {
        let mut n = provisioned();
        let ch = n.request_claim(owner(2), T0).unwrap();
        n.press(T0 + 1).unwrap();
        let proof = client_prove(&card(), &owner(2), &ch).unwrap();
        let mut out = n.submit_proof(&proof, T0 + 2).unwrap();
        out.outcome.owner = owner(3);
        assert_eq!(
            client_verify_outcome(&card().node_id, &owner(3), &out),
            Err(ClaimError::BadNodeSignature)
        );
    }

    #[test]
    fn owner_claim_reset_clears_binding_and_verifier_and_does_not_restore_the_card() {
        let mut n = claimed();
        let before = n.record().unwrap();
        n.reset().unwrap();
        let rec = n.record().unwrap();
        assert_eq!(rec.owner, None);
        assert!(rec.verifier.is_none());
        assert_eq!(rec.epoch, UNCLAIMED_EPOCH);
        assert!(rec.authority_epoch > before.authority_epoch);
        assert_eq!(
            n.request_claim(owner(2), T0 + 50),
            Err(ClaimError::NotClaimable)
        );
        assert_eq!(n.press(T0 + 51).unwrap().effect, ButtonEffect::ToggleWanSsh);

        // A new provisioning image arms a new card; the old one stays dead.
        let new_card = ClaimCard {
            secret: CardSecret::parse("mnopqrstuvwxyz77").unwrap(),
            ..card()
        };
        n.provision_verifier(Verifier::from_card([8; SALT_LEN], &new_card.secret))
            .unwrap();
        let ch = n.request_claim(owner(6), T0 + 60).unwrap();
        n.press(T0 + 61).unwrap();
        let old = client_prove(&card(), &owner(6), &ch).unwrap();
        let Err(ClaimError::BadProof { retry: Some(retry) }) = n.submit_proof(&old, T0 + 62) else {
            panic!("the original card must not verify after reset");
        };
        let fresh = client_prove(&new_card, &owner(6), &retry).unwrap();
        let out = n.submit_proof(&fresh, T0 + 63).unwrap();
        assert_eq!(out.outcome.epoch, 1);
        assert!(out.outcome.authority_epoch > rec.authority_epoch);
    }

    #[test]
    fn owner_claim_recovery_advances_both_epochs_and_retires_credentials() {
        let mut n = claimed();
        let before = n.record().unwrap();
        let cred = SigningKey::from_bytes(&[4; 32]);
        let cred_pk = cred.verifying_key().to_bytes();
        n.enroll_recovery_credential(cred_pk).unwrap();

        let ch = n.request_recovery(cred_pk, owner(5), T0 + 10).unwrap();
        let sig = cred.sign(&ch.challenge.signing_bytes()).to_bytes();
        n.verify_recovery_credential(ch.challenge.request_id, &sig, T0 + 10)
            .unwrap();
        assert_eq!(n.press(T0 + 11).unwrap().effect, ButtonEffect::ArmCeremony);
        let out = n
            .complete_recovery(ch.challenge.request_id, T0 + 12)
            .unwrap();
        assert!(client_verify_outcome(&card().node_id, &owner(5), &out).is_ok());

        let rec = n.record().unwrap();
        assert_eq!(rec.owner, Some(owner(5)));
        assert_eq!(rec.epoch, before.epoch + 1);
        assert_eq!(rec.authority_epoch, before.authority_epoch + 1);
        assert!(rec.recovery_credentials.is_empty());
        assert_eq!(
            n.request_recovery(cred_pk, owner(7), T0 + 20),
            Err(ClaimError::NotEnrolled),
            "prior recovery credentials are retired"
        );
    }

    #[test]
    fn owner_claim_recovery_credential_without_a_window_fails() {
        let mut n = claimed();
        let cred = SigningKey::from_bytes(&[4; 32]);
        let cred_pk = cred.verifying_key().to_bytes();
        n.enroll_recovery_credential(cred_pk).unwrap();
        let ch = n.request_recovery(cred_pk, owner(5), T0 + 10).unwrap();
        let sig = cred.sign(&ch.challenge.signing_bytes()).to_bytes();
        n.verify_recovery_credential(ch.challenge.request_id, &sig, T0 + 10)
            .unwrap();
        assert_eq!(
            n.complete_recovery(ch.challenge.request_id, T0 + 11),
            Err(ClaimError::NoWindow)
        );
        assert_eq!(n.record().unwrap().owner, Some(owner(2)));
    }

    #[test]
    fn owner_claim_button_without_a_credential_cannot_replace_the_owner() {
        let mut n = claimed();
        let stranger = SigningKey::from_bytes(&[6; 32]).verifying_key().to_bytes();
        assert_eq!(
            n.request_recovery(stranger, owner(6), T0 + 10),
            Err(ClaimError::NotEnrolled)
        );
        assert_eq!(n.press(T0 + 11).unwrap().effect, ButtonEffect::ToggleWanSsh);
        assert_eq!(n.complete_recovery(1, T0 + 12), Err(ClaimError::NoWindow));
        assert_eq!(n.record().unwrap().owner, Some(owner(2)));
    }

    #[test]
    fn owner_claim_button_effect_files_hand_off_to_rc_wps() {
        let dir = tempfile::tempdir().unwrap();
        publish_button_effect(dir.path(), ButtonEffect::ArmCeremony).unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join(BUTTON_EFFECT_FILE)).unwrap(),
            "ceremony\n"
        );
        assert!(!take_press(dir.path()).unwrap());
        fs::write(dir.path().join(PRESS_FILE), b"").unwrap();
        assert!(take_press(dir.path()).unwrap());
        assert!(!take_press(dir.path()).unwrap());
    }
}
