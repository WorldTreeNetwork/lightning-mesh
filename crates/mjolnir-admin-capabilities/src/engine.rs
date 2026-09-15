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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeEvidence {
    unix_seconds: u64,
    authority_verified_at: u64,
}

impl TimeEvidence {
    /// Frozen authenticated evidence. Production must not pass `SystemTime::now()`.
    pub fn authenticated(
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
#[derive(Debug, Clone)]
pub struct VerifiedAuthority {
    pub issuer: PublicKey,
    pub house_id: [u8; 32],
    pub node_id: [u8; 32],
    pub authority_epoch: u64,
    pub profile: SecurityProfile,
    pub grant_class: GrantClass,
    /// Token lineage ids that must not authorize.
    pub revoked_lineage: Vec<Vec<u8>>,
    /// Issuer revoked on a replica the destination has not yet verified.
    pub issuer_revoked: bool,
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

    let class = GrantClass::for_operation(request.operation());
    if class != authority.grant_class {
        return Err(AuthzError::ClassDenied);
    }

    if time.staleness_secs() > authority.profile.max_stale_secs() {
        return Err(AuthzError::StaleAuthority);
    }

    let token = Biscuit::from(raw_token, authority.issuer).map_err(token_err)?;
    if token.block_count() > MAX_BLOCKS {
        return Err(AuthzError::TooManyBlocks(token.block_count()));
    }
    if token.external_public_keys().iter().any(|k| k.is_some()) {
        return Err(AuthzError::ThirdPartyBlock);
    }
    reject_reserved_facts(&token)?;

    for id in token.revocation_identifiers() {
        if authority.revoked_lineage.iter().any(|r| r == &id) {
            return Err(AuthzError::Revoked);
        }
    }

    let (issued_at, expires_at) = issuance_times(&token)?;
    if issued_at > time.unix_seconds() {
        return Err(AuthzError::IssuedInTheFuture);
    }

    let effective_expiry = effective_expiry(issued_at, expires_at, authority.profile, class, None)?;
    if effective_expiry.is_some_and(|exp| time.unix_seconds() >= exp) {
        return Err(AuthzError::Expired);
    }

    // Issuer revoked: still ok inside the stale window (F-D positive control).
    // Stale already refused above.
    let _ = authority.issuer_revoked;

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
            check if holder("{}");
            allow if right($op), operation($op);
            deny if true;
            "#,
            hex32(holder.key_bytes())
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
        let hits: Vec<(i64,)> = authorizer.query(rule.as_str()).unwrap_or_default();
        if !hits.is_empty() {
            return Err(AuthzError::ReservedSymbol(reserved));
        }
    }
    Ok(())
}

fn issuance_times(token: &Biscuit) -> Result<(u64, u64), AuthzError> {
    let mut authorizer = token.authorizer().map_err(token_err)?;
    let issued: Vec<(i64,)> = authorizer
        .query("data($t) <- issued_at($t)")
        .map_err(token_err)?;
    let expires: Vec<(i64,)> = authorizer
        .query("data($t) <- expires_at($t)")
        .map_err(token_err)?;
    match (issued.as_slice(), expires.as_slice()) {
        ([(issued,)], [(expires,)]) if *issued >= 0 && *expires >= 0 => {
            Ok((*issued as u64, *expires as u64))
        }
        _ => Err(AuthzError::MissingIssuanceTimes),
    }
}

fn effective_expiry(
    issued_at: u64,
    expires_at: u64,
    profile: SecurityProfile,
    class: GrantClass,
    parent_ceiling: Option<u64>,
) -> Result<Option<u64>, AuthzError> {
    if class == GrantClass::DiagnosticsRead {
        return Ok(None);
    }
    let mut cap = expires_at;
    let class_cap = class
        .token_cap_secs(profile)
        .ok_or(AuthzError::ClassDenied)?;
    cap = min_expiry(cap, add_secs(issued_at, class_cap)?)?;
    cap = min_expiry(cap, add_secs(issued_at, profile.token_ceiling_secs())?)?;
    if let Some(parent) = parent_ceiling {
        cap = min_expiry(cap, add_secs(issued_at, parent)?)?;
    }
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
