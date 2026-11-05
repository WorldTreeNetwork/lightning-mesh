//! Bounded authorization decision for local mesh admin profile v1.
//!
//! Success is **not** permission to execute. Downstream (`b6j.2`) must durably
//! reserve [`ReplayObligation`] before running the operation. This module never
//! calls [`biscuit_auth::AuthorizerBuilder::time`].

use std::time::Duration;

use biscuit_auth::builder::Fact;
use biscuit_auth::datalog::RunLimits;
use biscuit_auth::{AuthorizerBuilder, Biscuit, PublicKey};
use thiserror::Error;

use crate::{
    CodecError, HolderKey, Operation, TokenIdentity, VerifiedHolderProof, verify_holder_proof,
};

const MAX_BLOCKS: usize = 16;
const MAX_TOKEN_BYTES: usize = 65536;

/// Fact names the token must not assert. Checks that *read* verifier
/// predicates are allowed; `block_symbols` would false-positive those.
const RESERVED_FACTS: &[&str] = &[
    "time",
    "unix_seconds",
    "authority_verified_at",
    "current_authority",
    "request",
];

/// Seconds for DECIDED v2 profiles (token default / ceiling / max stale).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityProfile {
    Strict,
    Standard,
    Relaxed,
}

impl SecurityProfile {
    pub fn token_default_secs(self) -> u64 {
        match self {
            SecurityProfile::Strict => 15 * 60,
            SecurityProfile::Standard => 24 * 3600,
            SecurityProfile::Relaxed => 7 * 24 * 3600,
        }
    }
    pub fn token_ceiling_secs(self) -> u64 {
        match self {
            SecurityProfile::Strict => 3600,
            SecurityProfile::Standard => 7 * 24 * 3600,
            SecurityProfile::Relaxed => 30 * 24 * 3600,
        }
    }
    pub fn max_stale_secs(self) -> u64 {
        match self {
            SecurityProfile::Strict => 15 * 60,
            SecurityProfile::Standard => 24 * 3600,
            SecurityProfile::Relaxed => 7 * 24 * 3600,
        }
    }
}

/// Grant-class caps. No profile loosens these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantClass {
    /// Owner-bound read-only diagnostics. May waive wall-clock expiry.
    DiagnosticsRead,
    NetworkRadio,
    Firmware,
    Ownership,
    GuestWifi,
}

impl GrantClass {
    pub fn for_operation(op: &Operation) -> Self {
        match op {
            Operation::DiagnosticsStatus => GrantClass::DiagnosticsRead,
            Operation::ApplyNetworkPlan { .. } => GrantClass::NetworkRadio,
        }
    }

    /// Token lifetime cap in seconds. `None` means wall-clock expiry is waived.
    pub fn token_cap_secs(self, profile: SecurityProfile) -> Option<u64> {
        match self {
            GrantClass::DiagnosticsRead => None,
            GrantClass::NetworkRadio => Some(match profile {
                SecurityProfile::Strict => 3600,
                _ => 24 * 3600,
            }),
            GrantClass::Firmware => Some(15 * 60),
            GrantClass::Ownership => Some(5 * 60),
            GrantClass::GuestWifi => Some(profile.token_ceiling_secs()),
        }
    }
}

/// Destination-owned authenticated time. No wall-clock constructor.
///
/// The only public way to obtain one is [`crate::OwnerVerified::time_evidence`]:
/// `authority_verified_at` is the signed `issued_at` of the newest owner-signed
/// authority-state record, never a caller-supplied or node-vouched value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeEvidence {
    unix_seconds: u64,
    authority_verified_at: u64,
}

impl TimeEvidence {
    /// Frozen authenticated evidence. Production must not pass `SystemTime::now()`.
    pub(crate) fn authenticated(
        unix_seconds: u64,
        authority_verified_at: u64,
    ) -> Result<Self, AuthzError> {
        if unix_seconds > i64::MAX as u64 || authority_verified_at > i64::MAX as u64 {
            return Err(AuthzError::TimeOutOfRange);
        }
        if authority_verified_at > unix_seconds {
            return Err(AuthzError::VerifiedAfterNow);
        }
        Ok(TimeEvidence {
            unix_seconds,
            authority_verified_at,
        })
    }

    pub fn unix_seconds(&self) -> u64 {
        self.unix_seconds
    }
    pub fn authority_verified_at(&self) -> u64 {
        self.authority_verified_at
    }
    pub fn staleness_secs(&self) -> u64 {
        self.unix_seconds - self.authority_verified_at
    }
}

/// Greatest `unix_seconds` previously accepted. Never decreases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HighWaterFloor(u64);

impl HighWaterFloor {
    pub fn new(value: u64) -> Self {
        HighWaterFloor(value)
    }
    pub fn get(&self) -> u64 {
        self.0
    }
}

/// Verifier-side authority snapshot. Not token-asserted.
///
/// There is no public constructor and no public field. The only producer is
/// [`crate::AuthorityBook::verified_authority`], which returns it inside
/// [`crate::OwnerVerified`] after verifying an owner-signed control record at
/// this consumer. A node-vouched fact cannot become one.
///
/// Callers cannot assemble one from fields:
///
/// ```compile_fail,E0451
/// use mjolnir_admin_capabilities::{GrantClass, SecurityProfile, VerifiedAuthority};
/// let issuer = biscuit_auth::KeyPair::new().public();
/// let _ = VerifiedAuthority {
///     issuer,
///     house_id: [0; 32],
///     node_id: [0; 32],
///     authority_epoch: 1,
///     profile: SecurityProfile::Standard,
///     grant_class: GrantClass::Ownership,
///     revoked_lineage: Vec::new(),
/// };
/// ```
///
/// nor the [`crate::OwnerVerified`] wrapper:
///
/// ```compile_fail,E0451
/// use mjolnir_admin_capabilities::{OwnerVerified, VerifiedAuthority};
/// fn forge(authority: VerifiedAuthority) -> OwnerVerified {
///     OwnerVerified { authority, authority_verified_at: 0 }
/// }
/// ```
///
/// nor [`TimeEvidence`] with a chosen `authority_verified_at`:
///
/// ```compile_fail,E0624
/// let _ = mjolnir_admin_capabilities::TimeEvidence::authenticated(10, 10);
/// ```
#[derive(Debug, Clone)]
pub struct VerifiedAuthority {
    pub(crate) issuer: PublicKey,
    pub(crate) house_id: [u8; 32],
    pub(crate) node_id: [u8; 32],
    pub(crate) authority_epoch: u64,
    pub(crate) profile: SecurityProfile,
    pub(crate) grant_class: GrantClass,
    /// Token lineage ids that must not authorize.
    pub(crate) revoked_lineage: Vec<Vec<u8>>,
}

impl VerifiedAuthority {
    /// Biscuit root key named by the verified control record.
    pub fn issuer(&self) -> &PublicKey {
        &self.issuer
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
    pub fn profile(&self) -> SecurityProfile {
        self.profile
    }
    pub fn grant_class(&self) -> GrantClass {
        self.grant_class
    }
    pub fn revoked_lineage(&self) -> &[Vec<u8>] {
        &self.revoked_lineage
    }
}

/// Bound identifiers that `b6j.2` must reserve before execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayObligation {
    pub target_challenge: [u8; 32],
    pub request_id: [u8; 32],
    pub authority_epoch: u64,
    pub token_identity: TokenIdentity,
}

/// Bounded authorization. Not an execute capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedGrant {
    pub operation: Operation,
    pub replay: ReplayObligation,
    pub effective_expiry: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AuthzError {
    #[error("codec: {0}")]
    Codec(#[from] CodecError),
    #[error("token rejected: {0}")]
    Token(String),
    #[error("third-party block is not allowed in v1")]
    ThirdPartyBlock,
    #[error("token has {0} blocks, exceeding 16")]
    TooManyBlocks(usize),
    #[error("token asserts reserved verifier fact or rule {0}")]
    ReservedSymbol(&'static str),
    #[error("missing issued_at or expires_at in the authority block")]
    MissingIssuanceTimes,
    #[error("issued_at is after authenticated now")]
    IssuedInTheFuture,
    #[error("grant is expired under authenticated time")]
    Expired,
    #[error("authority view is stale")]
    StaleAuthority,
    #[error("time evidence is below the high-water floor")]
    RegressedTime,
    #[error("time fields out of range")]
    TimeOutOfRange,
    #[error("authority_verified_at is after unix_seconds")]
    VerifiedAfterNow,
    #[error("token house/node/epoch does not match verified authority")]
    TargetMismatch,
    #[error("holder does not match the grant")]
    HolderMismatch,
    #[error("token asserts a holder fact; the verifier injects holder(fingerprint)")]
    HolderFact,
    #[error("token right {0} is not in the v1 profile")]
    UnknownRight(String),
    #[error("operation is not allowed by this grant class")]
    ClassDenied,
    #[error("grant lineage is revoked")]
    Revoked,
    #[error("biscuit policy denied the request")]
    PolicyDenied,
    #[error("arithmetic overflow computing expiry")]
    Overflow,
}

/// Authorize a possessed request against verified authority and time evidence.
///
/// `raw_token` must be the same buffer hashed into the proof. Do not pass
/// `Biscuit::to_vec()` as a substitute.
pub fn authorize(
    proof: &VerifiedHolderProof,
    raw_token: &[u8],
    holder: &HolderKey,
    authority: &VerifiedAuthority,
    time: TimeEvidence,
    floor: &mut HighWaterFloor,
) -> Result<AuthorizedGrant, AuthzError> {
    let presented = TokenIdentity::from_raw_token(raw_token)?;
    if presented != *proof.token_identity() {
        return Err(AuthzError::Codec(CodecError::TokenIdentityMismatch));
    }
    if raw_token.len() > MAX_TOKEN_BYTES {
        return Err(AuthzError::Codec(CodecError::TokenSizeOutOfRange {
            actual: raw_token.len(),
        }));
    }

    if time.unix_seconds() < floor.get() {
        return Err(AuthzError::RegressedTime);
    }

    let request = proof.request();
    if request.house_id() != &authority.house_id
        || request.node_id() != &authority.node_id
        || request.authority_epoch() != authority.authority_epoch
    {
        return Err(AuthzError::TargetMismatch);
    }

    // Diagnostics is the only cross-class narrowing. Any other operation must
    // match the control record, so a read issuer cannot authorize a radio change.
    let op_class = GrantClass::for_operation(request.operation());
    if op_class != authority.grant_class && op_class != GrantClass::DiagnosticsRead {
        return Err(AuthzError::ClassDenied);
    }

    let token = Biscuit::from(raw_token, authority.issuer).map_err(token_err)?;
    if token.block_count() > MAX_BLOCKS {
        return Err(AuthzError::TooManyBlocks(token.block_count()));
    }
    if token.external_public_keys().iter().any(|k| k.is_some()) {
        return Err(AuthzError::ThirdPartyBlock);
    }
    reject_reserved_facts(&token)?;
    reject_asserted_holder(&token)?;
    // The authority block pins the holder. The verifier injects the fingerprint
    // of the key that signed this request. identikey-capability-v1 §7.1.
    authority_pins_holder(&token)?;
    let fp = holder_fingerprint_hex(holder)?;

    let rights = token_rights(&token)?;
    let classes = classes_of(&rights)?;
    let op_right = match request.operation() {
        Operation::DiagnosticsStatus => "diagnostics_status",
        Operation::ApplyNetworkPlan { .. } => "apply_network_plan",
    };
    if !rights.iter().any(|right| right == op_right) {
        return Err(AuthzError::PolicyDenied);
    }
    // A network right keeps the stale window at 24h even under Relaxed.
    // Mixed tokens take that tighter window too.
    if time.staleness_secs() > staleness_limit(authority.profile, &classes) {
        return Err(AuthzError::StaleAuthority);
    }

    for id in token.revocation_identifiers() {
        if authority.revoked_lineage.iter().any(|r| r == &id) {
            return Err(AuthzError::Revoked);
        }
    }

    let class_cap = tightest_token_cap(authority.profile, &classes);
    let (issued_at, expires_at) = issuance_times(&token, class_cap.is_none())?;
    if issued_at > time.unix_seconds() {
        return Err(AuthzError::IssuedInTheFuture);
    }

    let effective_expiry = effective_expiry(issued_at, expires_at, authority.profile, class_cap)?;
    if effective_expiry.is_some_and(|exp| time.unix_seconds() >= exp) {
        return Err(AuthzError::Expired);
    }

    // An issuer withdrawal this destination has not verified cannot be seen
    // here; exposure is bounded by the stale check above (F-D). A verified
    // withdrawal never reaches this point: `AuthorityBook` refuses to build
    // `VerifiedAuthority` for a withdrawn issuer.

    let op_name = match request.operation() {
        Operation::DiagnosticsStatus => "diagnostics_status",
        Operation::ApplyNetworkPlan { .. } => "apply_network_plan",
    };

    let limits = RunLimits {
        max_facts: 1024,
        max_iterations: 64,
        max_time: Duration::from_millis(20),
    };

    let mut builder = AuthorizerBuilder::new().set_limits(limits);
    builder = builder
        .fact(int_fact("time", time.unix_seconds())?)
        .map_err(token_err)?;
    builder = builder
        .fact(str_fact("operation", op_name)?)
        .map_err(token_err)?;
    builder = builder
        .fact(str_fact("house", &hex32(request.house_id()))?)
        .map_err(token_err)?;
    builder = builder
        .fact(str_fact("node", &hex32(request.node_id()))?)
        .map_err(token_err)?;
    builder = builder
        .fact(int_fact("epoch", request.authority_epoch())?)
        .map_err(token_err)?;
    builder = builder
        .code(format!(
            r#"
            holder("{fp}");
            allow if right($op), operation($op);
            deny if true;
            "#,
        ))
        .map_err(token_err)?;

    let mut authorizer = builder.build(&token).map_err(token_err)?;
    authorizer
        .authorize()
        .map_err(|_| AuthzError::PolicyDenied)?;

    floor.0 = time.unix_seconds();

    Ok(AuthorizedGrant {
        operation: *request.operation(),
        replay: ReplayObligation {
            target_challenge: *request.target_challenge(),
            request_id: *request.request_id(),
            authority_epoch: request.authority_epoch(),
            token_identity: *proof.token_identity(),
        },
        effective_expiry,
    })
}

/// Possession then authorization. Same `raw_token` buffer throughout.
pub fn verify_and_authorize(
    canonical_request: &[u8],
    raw_token: &[u8],
    holder: &HolderKey,
    signature: &[u8],
    authority: &VerifiedAuthority,
    time: TimeEvidence,
    floor: &mut HighWaterFloor,
) -> Result<AuthorizedGrant, AuthzError> {
    let proof = verify_holder_proof(canonical_request, raw_token, holder, signature)?;
    authorize(&proof, raw_token, holder, authority, time, floor)
}

fn reject_reserved_facts(token: &Biscuit) -> Result<(), AuthzError> {
    let mut authorizer = token.authorizer().map_err(token_err)?;
    for reserved in RESERVED_FACTS {
        let rule = format!("data($x) <- {reserved}($x)");
        let hits: Vec<(i64,)> = authorizer
            .query_all(rule.as_str())
            .unwrap_or_default();
        if !hits.is_empty() {
            return Err(AuthzError::ReservedSymbol(reserved));
        }
    }
    Ok(())
}

/// Lowercase hex of the auth-challenge §5 fingerprint. This is the datalog
/// atom. Base58 remains the display form.
fn holder_fingerprint_hex(holder: &HolderKey) -> Result<String, AuthzError> {
    Ok(hex32(holder.fingerprint()?.as_bytes()))
}

/// `query_all` sees every block and does not treat a check as a fact.
fn reject_asserted_holder(token: &Biscuit) -> Result<(), AuthzError> {
    let mut authorizer = token.authorizer().map_err(token_err)?;
    let hits: Vec<(String,)> = authorizer
        .query_all("data($h) <- holder($h)")
        .map_err(token_err)?;
    if !hits.is_empty() {
        return Err(AuthzError::HolderFact);
    }
    // A rule headed `holder` derives the fact once the verifier injects its own.
    let (_facts, rules, _checks, _policies) = authorizer.dump();
    if rules.iter().any(|rule| rule.head.name == "holder") {
        return Err(AuthzError::HolderFact);
    }
    Ok(())
}

/// The issuer pins the holder in the authority block. A later block cannot
/// add that pin: checks accumulate, and only block 0 is the ceiling.
fn authority_pins_holder(token: &Biscuit) -> Result<(), AuthzError> {
    let src = token.print_block_source(0).map_err(token_err)?;
    if src
        .lines()
        .any(|line| line.trim().starts_with("check if holder("))
    {
        return Ok(());
    }
    Err(AuthzError::HolderMismatch)
}

/// v1 right atoms. A later wire operation is not required for a right to
/// tighten the cap: a mixed token takes the shortest class present.
fn class_for_right(right: &str) -> Result<GrantClass, AuthzError> {
    match right {
        "diagnostics_status" => Ok(GrantClass::DiagnosticsRead),
        "apply_network_plan" => Ok(GrantClass::NetworkRadio),
        "guest_wifi" => Ok(GrantClass::GuestWifi),
        "firmware_install" => Ok(GrantClass::Firmware),
        "ownership_change" => Ok(GrantClass::Ownership),
        _ => Err(AuthzError::UnknownRight(right.to_string())),
    }
}

fn token_rights(token: &Biscuit) -> Result<Vec<String>, AuthzError> {
    let mut authorizer = token.authorizer().map_err(token_err)?;
    // Later blocks may only add checks. A fact there is not a right.
    let hits: Vec<(String,)> = authorizer
        .query("data($r) <- right($r) trusting authority")
        .map_err(token_err)?;
    if hits.is_empty() {
        return Err(AuthzError::PolicyDenied);
    }
    Ok(hits.into_iter().map(|(right,)| right).collect())
}

fn classes_of(rights: &[String]) -> Result<Vec<GrantClass>, AuthzError> {
    let mut classes = Vec::new();
    for right in rights {
        let class = class_for_right(right)?;
        if !classes.contains(&class) {
            classes.push(class);
        }
    }
    Ok(classes)
}

/// Shortest grant-class lifetime, in seconds. `None` means every right is
/// the diagnostics waiver.
fn tightest_token_cap(profile: SecurityProfile, classes: &[GrantClass]) -> Option<u64> {
    let mut tightest: Option<u64> = None;
    for class in classes {
        if let Some(cap) = class.token_cap_secs(profile) {
            tightest = Some(match tightest {
                Some(so_far) => so_far.min(cap),
                None => cap,
            });
        }
    }
    tightest
}

/// Profile stale window, tightened to 24h when the token carries a network right.
fn staleness_limit(profile: SecurityProfile, classes: &[GrantClass]) -> u64 {
    let mut limit = profile.max_stale_secs();
    if classes.contains(&GrantClass::NetworkRadio) {
        limit = limit.min(24 * 3600);
    }
    limit
}

fn issuance_times(token: &Biscuit, waive_expiry: bool) -> Result<(u64, Option<u64>), AuthzError> {
    let mut authorizer = token.authorizer().map_err(token_err)?;
    let issued: Vec<(i64,)> = authorizer
        .query("data($t) <- issued_at($t) trusting authority")
        .map_err(token_err)?;
    let expires: Vec<(i64,)> = authorizer
        .query("data($t) <- expires_at($t) trusting authority")
        .map_err(token_err)?;
    let issued_at = match issued.as_slice() {
        [(issued,)] if *issued >= 0 => *issued as u64,
        _ => return Err(AuthzError::MissingIssuanceTimes),
    };
    let expires_at = match expires.as_slice() {
        [] if waive_expiry => None,
        [(expires,)] if *expires >= 0 => Some(*expires as u64),
        _ => return Err(AuthzError::MissingIssuanceTimes),
    };
    Ok((issued_at, expires_at))
}

fn effective_expiry(
    issued_at: u64,
    expires_at: Option<u64>,
    profile: SecurityProfile,
    class_cap: Option<u64>,
) -> Result<Option<u64>, AuthzError> {
    let Some(class_cap) = class_cap else {
        return Ok(None);
    };
    let expires_at = expires_at.ok_or(AuthzError::MissingIssuanceTimes)?;
    let mut cap = expires_at;
    cap = min_expiry(cap, add_secs(issued_at, class_cap)?)?;
    cap = min_expiry(cap, add_secs(issued_at, profile.token_ceiling_secs())?)?;
    Ok(Some(cap))
}

fn add_secs(base: u64, extra: u64) -> Result<u64, AuthzError> {
    base.checked_add(extra).ok_or(AuthzError::Overflow)
}

fn min_expiry(a: u64, b: u64) -> Result<u64, AuthzError> {
    Ok(a.min(b))
}

fn int_fact(name: &str, value: u64) -> Result<Fact, AuthzError> {
    if value > i64::MAX as u64 {
        return Err(AuthzError::TimeOutOfRange);
    }
    let src = format!("{name}({value})");
    Fact::try_from(src.as_str()).map_err(token_err)
}

fn str_fact(name: &str, value: &str) -> Result<Fact, AuthzError> {
    let src = format!("{name}(\"{value}\")");
    Fact::try_from(src.as_str()).map_err(token_err)
}

fn hex32(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn token_err(e: biscuit_auth::error::Token) -> AuthzError {
    AuthzError::Token(e.to_string())
}
