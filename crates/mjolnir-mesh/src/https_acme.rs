// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors
// Lightning Mesh is dual-licensed (AGPL-3.0-or-later or commercial); see LICENSE
// and COMMERCIAL-LICENSE.md at the repository root.

//! Transport-injected DNS-01 issuance for app hosts and routers. No sockets.
//!
//! First the gateway prepares an order. Its public challenge details go to the
//! host, which signs an issuance envelope and sends back a [`CourierPayload`].
//! The gateway completes that same order, publishing only through the DNS gate.
//! Account JWS, directory discovery, HTTP and Retry-After pacing belong to the
//! injected transport. Host TLS key generation/storage and certificate
//! validation/installation belong to the host; neither private key is relayed.

use std::{collections::BTreeSet, fmt::Display};

use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    https_dns::{ChallengeTxtWriter, DnsChallengeError, DnsChallengeGate},
    https_issuance::{
        IssuanceAuthorization, IssuanceError, sign_issuance_envelope, verify_issuance_envelope,
    },
};

pub const ACME_PROFILE: &str = "classic";
pub const ACME_CHALLENGE: &str = "dns-01";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcmeAccount {
    pub url: String,
    /// Unpadded base64url SHA-256 JWK thumbprint of the transport's account key.
    pub key_thumbprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identifier {
    #[serde(rename = "type")]
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NewOrderRequest {
    pub identifiers: Vec<Identifier>,
    pub profile: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderStatus {
    Pending,
    Ready,
    Processing,
    Valid,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthorizationStatus {
    Pending,
    Valid,
    Invalid,
    Deactivated,
    Expired,
    Revoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChallengeStatus {
    Pending,
    Processing,
    Valid,
    Invalid,
}

/// URLs are supplied by the transport from the directory's order response and
/// Location header. Only a single DNS identifier is supported per order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcmeOrder {
    pub url: String,
    pub finalize_url: String,
    pub authorization_urls: Vec<String>,
    pub identifiers: Vec<Identifier>,
    pub status: OrderStatus,
    pub certificate_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcmeChallenge {
    pub kind: String,
    pub url: String,
    pub token: String,
    pub status: ChallengeStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcmeAuthorization {
    pub identifier: Identifier,
    pub wildcard: bool,
    pub status: AuthorizationStatus,
    pub challenges: Vec<AcmeChallenge>,
}

/// Semantic ACME directory transport. Implementations authenticate operations
/// with the account returned by `account`, preserve server statuses, and honor
/// Retry-After before subsequent polls. `finalize` encodes the supplied DER CSR
/// as the ACME base64url `csr` field. There is no default network implementation.
pub trait DirectoryTransport {
    type Error: Display;

    fn account(&self) -> &AcmeAccount;
    fn new_order(&mut self, request: &NewOrderRequest) -> Result<AcmeOrder, Self::Error>;
    fn authorization(&mut self, url: &str) -> Result<AcmeAuthorization, Self::Error>;
    /// POST an empty JSON object to the selected dns-01 challenge URL.
    fn challenge_ready(&mut self, url: &str) -> Result<(), Self::Error>;
    fn order(&mut self, url: &str) -> Result<AcmeOrder, Self::Error>;
    fn finalize(&mut self, url: &str, csr_der: &[u8]) -> Result<(), Self::Error>;
    fn certificate(&mut self, url: &str) -> Result<Vec<u8>, Self::Error>;
}

/// The complete host-to-gateway wire payload: public CSR and signed authority.
/// Unknown fields are refused, including attempts to attach private keys.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CourierPayload {
    pub issuance_envelope: String,
    pub csr_der: Vec<u8>,
}

impl CourierPayload {
    /// The owner key is used locally to sign; the host's TLS key is never an
    /// argument. Verify the SPKI binding before constructing the wire payload.
    pub fn sign(
        authorization: &IssuanceAuthorization,
        csr_der: Vec<u8>,
        owner_key: &SigningKey,
    ) -> Result<Self, AcmeError> {
        if authorization.csr_spki_sha256 != hex(&csr_spki_sha256(&csr_der)?) {
            return Err(IssuanceError::CsrMismatch.into());
        }
        Ok(Self {
            issuance_envelope: sign_issuance_envelope(authorization, owner_key)?,
            csr_der,
        })
    }
}

/// Gateway-local order state. It must survive the trip to the host and back;
/// completion never creates a new order with a different challenge token.
#[derive(Debug)]
pub struct PreparedOrder {
    order: AcmeOrder,
    account: AcmeAccount,
    challenge: AcmeChallenge,
    txt_digest: String,
}

impl PreparedOrder {
    /// Public details for the host to authorize, using its actual DER CSR.
    /// Expiry and nonce policy remain the issuance verifier's responsibility.
    pub fn authorization(
        &self,
        csr_der: &[u8],
        nonce: String,
        expires_at: u64,
    ) -> Result<IssuanceAuthorization, AcmeError> {
        Ok(IssuanceAuthorization {
            fqdn: self.order.identifiers[0].value.clone(),
            txt_digest: self.txt_digest.clone(),
            acme_account: self.account.url.clone(),
            csr_spki_sha256: hex(&csr_spki_sha256(csr_der)?),
            nonce,
            expires_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AcmeError {
    #[error(transparent)]
    Issuance(#[from] IssuanceError),
    #[error(transparent)]
    Dns(#[from] DnsChallengeError),
    #[error("directory transport failed: {0}")]
    Transport(String),
    #[error("unexpected ACME directory response: {0}")]
    Protocol(&'static str),
    #[error("signed authorization does not match this order, account or DNS-01 challenge")]
    OrderBinding,
    #[error("ACME resource entered a terminal failure state")]
    Rejected,
    #[error("ACME polling limit reached")]
    PollLimit,
    #[error("CSR must be a DER PKCS#10 request containing a SubjectPublicKeyInfo")]
    Csr,
    #[error("invalid ARI renewal-info body or suggested window")]
    RenewalInfo,
}

/// A bounded synchronous driver. It can run on a connected host or its gateway.
pub struct AcmeClient<T> {
    transport: T,
    max_polls: usize,
}

impl<T: DirectoryTransport> AcmeClient<T> {
    pub fn new(transport: T, max_polls: usize) -> Self {
        Self {
            transport,
            max_polls,
        }
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn into_transport(self) -> T {
        self.transport
    }

    /// Start a fresh classic-profile order. Reused/already-valid authorizations
    /// are deliberately refused: this flow requires an owner-approved TXT write.
    pub fn prepare(&mut self, fqdn: &str) -> Result<PreparedOrder, AcmeError> {
        let account = self.transport.account().clone();
        if !is_digest(&account.key_thumbprint) {
            return Err(AcmeError::Protocol("invalid account thumbprint"));
        }
        let request = NewOrderRequest {
            identifiers: vec![Identifier {
                kind: "dns".into(),
                value: fqdn.into(),
            }],
            profile: ACME_PROFILE.into(),
        };
        let order = self
            .transport
            .new_order(&request)
            .map_err(transport_error)?;
        if order.identifiers != request.identifiers
            || order.authorization_urls.len() != 1
            || order.authorization_urls[0].is_empty()
            || order.url.is_empty()
            || order.finalize_url.is_empty()
            || order.status != OrderStatus::Pending
            || order.certificate_url.is_some()
        {
            return Err(AcmeError::Protocol("expected a fresh single-name order"));
        }
        let authorization = self
            .transport
            .authorization(&order.authorization_urls[0])
            .map_err(transport_error)?;
        if authorization.identifier != order.identifiers[0]
            || authorization.wildcard
            || authorization.status != AuthorizationStatus::Pending
        {
            return Err(AcmeError::Protocol(
                "expected a pending non-wildcard authorization",
            ));
        }
        let mut challenges = authorization.challenges.into_iter().filter(|challenge| {
            challenge.kind == ACME_CHALLENGE && challenge.status == ChallengeStatus::Pending
        });
        let challenge = challenges
            .next()
            .ok_or(AcmeError::Protocol("missing pending dns-01"))?;
        if challenges.next().is_some()
            || challenge.url.is_empty()
            || challenge.token.len() < 22
            || !is_base64url(&challenge.token)
        {
            return Err(AcmeError::Protocol("invalid or ambiguous dns-01 challenge"));
        }
        let txt_digest = base64url(&Sha256::digest(format!(
            "{}.{}",
            challenge.token, account.key_thumbprint
        )));
        Ok(PreparedOrder {
            order,
            account,
            challenge,
            txt_digest,
        })
    }

    /// Publish, signal ready, poll, finalize with the original CSR, and return
    /// certificate bytes to the host. Refresh AliasTable's append-only owner
    /// history before calling. `now` is Unix seconds at submission. Persist the
    /// gate's histories/decisions even on failure; a failed submission may have
    /// consumed its nonce. Poll exhaustion requires a fresh order/authorization.
    pub fn complete<W: ChallengeTxtWriter>(
        &mut self,
        prepared: PreparedOrder,
        payload: &CourierPayload,
        gate: &mut DnsChallengeGate<W>,
        now: u64,
        seen_nonces: &BTreeSet<String>,
        retained_owner_pubkeys: &BTreeSet<[u8; 32]>,
    ) -> Result<Vec<u8>, AcmeError> {
        let spki_hash = csr_spki_sha256(&payload.csr_der)?;
        // Use the existing verifier, including gate-local replay/collision
        // history, before binding directory state or invoking any side effects.
        let nonces = seen_nonces.union(gate.seen_nonces()).cloned().collect();
        let owners = retained_owner_pubkeys
            .union(gate.retained_owner_pubkeys())
            .copied()
            .collect();
        let verified = verify_issuance_envelope(
            &payload.issuance_envelope,
            &spki_hash,
            now,
            &nonces,
            &owners,
        )?;
        let auth = verified.authorization;
        if self.transport.account() != &prepared.account
            || auth.acme_account != prepared.account.url
            || auth.fqdn != prepared.order.identifiers[0].value
            || auth.txt_digest != prepared.txt_digest
        {
            return Err(AcmeError::OrderBinding);
        }
        if self.max_polls == 0 {
            return Err(AcmeError::PollLimit);
        }
        // This is the sole TXT publication path. Do not notify ACME on refusal
        // or writer failure, and never publish directly through the writer.
        gate.submit(
            &payload.issuance_envelope,
            &spki_hash,
            now,
            seen_nonces,
            retained_owner_pubkeys,
        )?;
        self.transport
            .challenge_ready(&prepared.challenge.url)
            .map_err(transport_error)?;
        self.await_authorization(&prepared)?;
        self.await_order(&prepared.order, false)?;
        self.transport
            .finalize(&prepared.order.finalize_url, &payload.csr_der)
            .map_err(transport_error)?;
        let order = self.await_order(&prepared.order, true)?;
        let url = order
            .certificate_url
            .filter(|url| !url.is_empty())
            .ok_or(AcmeError::Protocol("valid order has no certificate URL"))?;
        let certificate = self.transport.certificate(&url).map_err(transport_error)?;
        if certificate.is_empty() {
            return Err(AcmeError::Protocol("empty certificate response"));
        }
        Ok(certificate)
    }

    fn await_authorization(&mut self, prepared: &PreparedOrder) -> Result<(), AcmeError> {
        for _ in 0..self.max_polls {
            let auth = self
                .transport
                .authorization(&prepared.order.authorization_urls[0])
                .map_err(transport_error)?;
            if auth.identifier != prepared.order.identifiers[0] || auth.wildcard {
                return Err(AcmeError::OrderBinding);
            }
            let challenge = auth
                .challenges
                .iter()
                .find(|c| c.url == prepared.challenge.url)
                .ok_or(AcmeError::OrderBinding)?;
            if challenge.kind != ACME_CHALLENGE || challenge.token != prepared.challenge.token {
                return Err(AcmeError::OrderBinding);
            }
            if challenge.status == ChallengeStatus::Invalid {
                return Err(AcmeError::Rejected);
            }
            match auth.status {
                AuthorizationStatus::Valid if challenge.status == ChallengeStatus::Valid => {
                    return Ok(());
                }
                AuthorizationStatus::Pending => {}
                AuthorizationStatus::Valid => {
                    return Err(AcmeError::Protocol(
                        "valid authorization with unvalidated challenge",
                    ));
                }
                _ => return Err(AcmeError::Rejected),
            }
        }
        Err(AcmeError::PollLimit)
    }

    fn await_order(
        &mut self,
        original: &AcmeOrder,
        finalized: bool,
    ) -> Result<AcmeOrder, AcmeError> {
        for _ in 0..self.max_polls {
            let order = self
                .transport
                .order(&original.url)
                .map_err(transport_error)?;
            if order.url != original.url
                || order.finalize_url != original.finalize_url
                || order.identifiers != original.identifiers
                || order.authorization_urls != original.authorization_urls
            {
                return Err(AcmeError::OrderBinding);
            }
            match (finalized, order.status) {
                (_, OrderStatus::Invalid) => return Err(AcmeError::Rejected),
                (false, OrderStatus::Ready) | (true, OrderStatus::Valid) => return Ok(order),
                (false, OrderStatus::Pending) | (true, OrderStatus::Processing) => {}
                _ => return Err(AcmeError::Protocol("unexpected order transition")),
            }
        }
        Err(AcmeError::PollLimit)
    }
}

fn transport_error(error: impl Display) -> AcmeError {
    AcmeError::Transport(error.to_string())
}

/// Hash the complete DER SubjectPublicKeyInfo, including its tag and length,
/// extracted from the actual PKCS#10 CSR. This only parses the CSR structure;
/// the CA verifies the CSR signature and requested identifiers on finalization.
pub fn csr_spki_sha256(csr_der: &[u8]) -> Result<[u8; 32], AcmeError> {
    let mut input = csr_der;
    let mut request = der_take(&mut input, 0x30)?.1;
    if !input.is_empty() {
        return Err(AcmeError::Csr);
    }
    let mut info = der_take(&mut request, 0x30)?.1;
    der_take(&mut request, 0x30)?; // Signature algorithm.
    let signature = der_take(&mut request, 0x03)?.1;
    if !request.is_empty() || signature.len() < 2 || signature[0] != 0 {
        return Err(AcmeError::Csr);
    }
    if der_take(&mut info, 0x02)?.1 != [0] {
        return Err(AcmeError::Csr);
    }
    der_take(&mut info, 0x30)?; // Subject.
    let (spki_der, mut spki) = der_take(&mut info, 0x30)?;
    der_take(&mut spki, 0x30)?; // Public-key algorithm.
    let public_key = der_take(&mut spki, 0x03)?.1;
    if !spki.is_empty() || public_key.len() < 2 || public_key[0] != 0 {
        return Err(AcmeError::Csr);
    }
    der_take(&mut info, 0xa0)?; // Attributes (possibly empty).
    if !info.is_empty() {
        return Err(AcmeError::Csr);
    }
    Ok(Sha256::digest(spki_der).into())
}

/// Take a DER TLV with definite, minimally encoded length, without allocation.
fn der_take<'a>(input: &mut &'a [u8], tag: u8) -> Result<(&'a [u8], &'a [u8]), AcmeError> {
    let bytes = *input;
    if bytes.len() < 2 || bytes[0] != tag {
        return Err(AcmeError::Csr);
    }
    let (header, length) = if bytes[1] < 128 {
        (2, usize::from(bytes[1]))
    } else {
        let count = usize::from(bytes[1] & 0x7f);
        if count == 0
            || count > std::mem::size_of::<usize>()
            || bytes.len() < 2 + count
            || bytes[2] == 0
        {
            return Err(AcmeError::Csr);
        }
        let mut length = 0usize;
        for byte in &bytes[2..2 + count] {
            length = length
                .checked_mul(256)
                .and_then(|n| n.checked_add(usize::from(*byte)))
                .ok_or(AcmeError::Csr)?;
        }
        if length < 128 {
            return Err(AcmeError::Csr);
        }
        (2 + count, length)
    };
    let end = header
        .checked_add(length)
        .filter(|end| *end <= bytes.len())
        .ok_or(AcmeError::Csr)?;
    *input = &bytes[end..];
    Ok((&bytes[..end], &bytes[header..end]))
}

fn is_base64url(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn is_digest(value: &str) -> bool {
    value.len() == 43 && is_base64url(value) && b"AEIMQUYcgkosw048".contains(&value.as_bytes()[42])
}

fn base64url(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut output = String::new();
    for chunk in bytes.chunks(3) {
        let bits = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for shift in [18, 12, 6, 0].into_iter().take(chunk.len() + 1) {
            output.push(ALPHABET[((bits >> shift) & 63) as usize] as char);
        }
    }
    output
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(output, "{byte:02x}").expect("String write");
    }
    output
}

/// Validated RFC 9773 suggested window. Unknown renewal-info fields are allowed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenewalInfo {
    start: Timestamp,
    end: Timestamp,
    pub explanation_url: Option<String>,
}

impl RenewalInfo {
    pub fn parse(body: &[u8]) -> Result<Self, AcmeError> {
        #[derive(Deserialize)]
        struct Window {
            start: String,
            end: String,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Body {
            suggested_window: Window,
            #[serde(rename = "explanationURL")]
            explanation_url: Option<String>,
        }
        let body: Body = serde_json::from_slice(body).map_err(|_| AcmeError::RenewalInfo)?;
        let start = Timestamp::parse(&body.suggested_window.start)?;
        let end = Timestamp::parse(&body.suggested_window.end)?;
        if start >= end {
            return Err(AcmeError::RenewalInfo);
        }
        Ok(Self {
            start,
            end,
            explanation_url: body.explanation_url,
        })
    }

    /// True for Unix seconds in [start, end). This reports window membership;
    /// choosing a randomized renewal instant and scheduling belong to the host.
    pub fn contains(&self, now: u64) -> bool {
        let Ok(seconds) = i64::try_from(now) else {
            return false;
        };
        let now = Timestamp {
            seconds,
            fraction: String::new(),
        };
        self.start <= now && now < self.end
    }
}

// Fraction strings have trailing zeros removed, so lexical comparison is exact
// even for RFC3339 fractions longer than nanoseconds. Leap seconds are refused.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Timestamp {
    seconds: i64,
    fraction: String,
}

impl Timestamp {
    fn parse(value: &str) -> Result<Self, AcmeError> {
        let invalid = AcmeError::RenewalInfo;
        let b = value.as_bytes();
        if !value.is_ascii()
            || b.len() < 20
            || b[4] != b'-'
            || b[7] != b'-'
            || !matches!(b[10], b'T' | b't')
            || b[13] != b':'
            || b[16] != b':'
        {
            return Err(invalid);
        }
        let number = |range: std::ops::Range<usize>| -> Result<i64, AcmeError> {
            let s = &value[range];
            if !s.bytes().all(|b| b.is_ascii_digit()) {
                return Err(AcmeError::RenewalInfo);
            }
            s.parse().map_err(|_| AcmeError::RenewalInfo)
        };
        let (year, month, day) = (number(0..4)?, number(5..7)?, number(8..10)?);
        let (hour, minute, second) = (number(11..13)?, number(14..16)?, number(17..19)?);
        let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        let days_in_month = match month {
            2 => {
                if leap {
                    29
                } else {
                    28
                }
            }
            4 | 6 | 9 | 11 => 30,
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            _ => return Err(invalid),
        };
        if day < 1 || day > days_in_month || hour > 23 || minute > 59 || second > 59 {
            return Err(invalid);
        }
        let mut index = 19;
        let fraction = if b[index] == b'.' {
            index += 1;
            let start = index;
            while index < b.len() && b[index].is_ascii_digit() {
                index += 1;
            }
            if index == start {
                return Err(invalid);
            }
            value[start..index].trim_end_matches('0').to_owned()
        } else {
            String::new()
        };
        let offset = match b.get(index..) {
            Some(b"Z" | b"z") => 0,
            Some(tail) if tail.len() == 6 && matches!(tail[0], b'+' | b'-') && tail[3] == b':' => {
                let hours = number(index + 1..index + 3)?;
                let minutes = number(index + 4..index + 6)?;
                if hours > 23 || minutes > 59 {
                    return Err(invalid);
                }
                (hours * 3600 + minutes * 60) * if tail[0] == b'+' { 1 } else { -1 }
            }
            _ => return Err(invalid),
        };
        // Gregorian date to days since 1970-01-01, using March-based eras.
        let y = year - i64::from(month <= 2);
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let m = month + if month > 2 { -3 } else { 9 };
        let doy = (153 * m + 2) / 5 + day - 1;
        let days = era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468;
        Ok(Self {
            seconds: days * 86400 + hour * 3600 + minute * 60 + second - offset,
            fraction,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, collections::VecDeque, rc::Rc};

    use ed25519_dalek::Signer;

    use super::*;
    use crate::https_dns::{ChallengeTxtRecord, InMemoryTxtWriter};

    const NOW: u64 = 1_800_000_000;
    const HOST_SEED: [u8; 32] = [0x53; 32];
    const CERTIFICATE: &[u8] = b"scripted certificate chain";
    type Events = Rc<RefCell<Vec<&'static str>>>;

    #[derive(Debug)]
    enum Step {
        NewOrder(NewOrderRequest, AcmeOrder),
        Authorization(String, AcmeAuthorization),
        Ready(String),
        Order(String, AcmeOrder),
        Finalize(String, Vec<u8>),
        Certificate(String, Vec<u8>),
        Failure,
    }

    struct ScriptedDirectory {
        account: AcmeAccount,
        steps: VecDeque<Step>,
        events: Events,
    }

    impl ScriptedDirectory {
        fn next(&mut self, event: &'static str) -> Result<Step, &'static str> {
            self.events.borrow_mut().push(event);
            match self.steps.pop_front().expect("unexpected directory call") {
                Step::Failure => Err("scripted directory failure"),
                step => Ok(step),
            }
        }
    }

    impl DirectoryTransport for ScriptedDirectory {
        type Error = &'static str;

        fn account(&self) -> &AcmeAccount {
            &self.account
        }

        fn new_order(&mut self, request: &NewOrderRequest) -> Result<AcmeOrder, Self::Error> {
            let Step::NewOrder(expected, order) = self.next("new-order")? else {
                panic!("expected new-order");
            };
            assert_eq!(request, &expected);
            assert_eq!(
                serde_json::to_value(request).unwrap(),
                serde_json::json!({
                    "profile": "classic",
                    "identifiers": [{"type": "dns", "value": expected.identifiers[0].value}],
                })
            );
            Ok(order)
        }

        fn authorization(&mut self, url: &str) -> Result<AcmeAuthorization, Self::Error> {
            let Step::Authorization(expected, auth) = self.next("authorization")? else {
                panic!("expected authorization");
            };
            assert_eq!(url, expected);
            Ok(auth)
        }

        fn challenge_ready(&mut self, url: &str) -> Result<(), Self::Error> {
            let Step::Ready(expected) = self.next("ready")? else {
                panic!("expected ready");
            };
            assert_eq!(url, expected);
            Ok(())
        }

        fn order(&mut self, url: &str) -> Result<AcmeOrder, Self::Error> {
            let Step::Order(expected, order) = self.next("order")? else {
                panic!("expected order");
            };
            assert_eq!(url, expected);
            Ok(order)
        }

        fn finalize(&mut self, url: &str, csr_der: &[u8]) -> Result<(), Self::Error> {
            let Step::Finalize(expected, csr) = self.next("finalize")? else {
                panic!("expected finalize");
            };
            assert_eq!(url, expected);
            assert_eq!(csr_der, csr);
            Ok(())
        }

        fn certificate(&mut self, url: &str) -> Result<Vec<u8>, Self::Error> {
            let Step::Certificate(expected, certificate) = self.next("certificate")? else {
                panic!("expected certificate");
            };
            assert_eq!(url, expected);
            Ok(certificate)
        }
    }

    struct JournalWriter {
        events: Events,
        fail: bool,
        records: Vec<ChallengeTxtRecord>,
    }

    impl ChallengeTxtWriter for JournalWriter {
        type Error = &'static str;

        fn write_txt(&mut self, record: &ChallengeTxtRecord) -> Result<(), Self::Error> {
            self.events.borrow_mut().push("txt");
            if self.fail {
                return Err("scripted TXT failure");
            }
            self.records.push(record.clone());
            Ok(())
        }
    }

    fn owner() -> SigningKey {
        SigningKey::from_bytes(&[7; 32])
    }

    fn fqdn(kind: char) -> String {
        format!(
            "{kind}-{}.house.mesh.worldtree.network",
            crate::https_label(&owner().verifying_key().to_bytes())
        )
    }

    fn account() -> AcmeAccount {
        AcmeAccount {
            url: "https://acme.example/acct/1".into(),
            key_thumbprint: "A".repeat(43),
        }
    }

    fn order(kind: char, status: OrderStatus) -> AcmeOrder {
        AcmeOrder {
            url: "https://acme.example/order/1".into(),
            finalize_url: "https://acme.example/finalize/1".into(),
            authorization_urls: vec!["https://acme.example/auth/1".into()],
            identifiers: vec![Identifier {
                kind: "dns".into(),
                value: fqdn(kind),
            }],
            status,
            certificate_url: (status == OrderStatus::Valid)
                .then(|| "https://acme.example/cert/1".into()),
        }
    }

    fn authorization(kind: char, status: AuthorizationStatus) -> AcmeAuthorization {
        AcmeAuthorization {
            identifier: order(kind, OrderStatus::Pending).identifiers.remove(0),
            wildcard: false,
            status,
            challenges: vec![
                AcmeChallenge {
                    kind: "http-01".into(),
                    url: "https://acme.example/http/1".into(),
                    token: "B".repeat(32),
                    status: ChallengeStatus::Pending,
                },
                AcmeChallenge {
                    kind: "dns-01".into(),
                    url: "https://acme.example/dns/1".into(),
                    token: "C".repeat(32),
                    status: if status == AuthorizationStatus::Valid {
                        ChallengeStatus::Valid
                    } else {
                        ChallengeStatus::Pending
                    },
                },
            ],
        }
    }

    // A signed PKCS#10 request with a host-local Ed25519 key, distinct from the
    // owner key. The script does not act as a CA or validate requested SANs.
    fn csr(seed: [u8; 32]) -> (Vec<u8>, Vec<u8>) {
        fn tlv(tag: u8, value: &[u8]) -> Vec<u8> {
            let mut bytes = vec![tag];
            if value.len() < 128 {
                bytes.push(value.len() as u8);
            } else {
                bytes.extend_from_slice(&[0x81, value.len() as u8]);
            }
            bytes.extend_from_slice(value);
            bytes
        }
        let key = SigningKey::from_bytes(&seed);
        let algorithm = [0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70];
        let public_key = tlv(0x03, &[&[0][..], &key.verifying_key().to_bytes()].concat());
        let spki = tlv(0x30, &[algorithm.as_slice(), &public_key].concat());
        let info = tlv(
            0x30,
            &[&[0x02, 0x01, 0, 0x30, 0][..], &spki, &[0xa0, 0]].concat(),
        );
        let signature = tlv(0x03, &[&[0][..], &key.sign(&info).to_bytes()].concat());
        (
            tlv(0x30, &[info.as_slice(), &algorithm, &signature].concat()),
            spki,
        )
    }

    fn preparation_steps(kind: char) -> VecDeque<Step> {
        let order = order(kind, OrderStatus::Pending);
        VecDeque::from([
            Step::NewOrder(
                NewOrderRequest {
                    identifiers: order.identifiers.clone(),
                    profile: "classic".into(),
                },
                order.clone(),
            ),
            Step::Authorization(
                order.authorization_urls[0].clone(),
                authorization(kind, AuthorizationStatus::Pending),
            ),
        ])
    }

    fn prepared(kind: char) -> (AcmeClient<ScriptedDirectory>, PreparedOrder, CourierPayload) {
        let mut client = AcmeClient::new(
            ScriptedDirectory {
                account: account(),
                steps: preparation_steps(kind),
                events: Events::default(),
            },
            3,
        );
        let prepared = client.prepare(&fqdn(kind)).unwrap();
        let auth = prepared
            .authorization(&csr(HOST_SEED).0, "01".repeat(16), NOW + 3600)
            .unwrap();
        let payload = CourierPayload::sign(&auth, csr(HOST_SEED).0, &owner()).unwrap();
        (client, prepared, payload)
    }

    fn completion_steps(kind: char, csr_der: &[u8]) -> VecDeque<Step> {
        let order = order(kind, OrderStatus::Valid);
        let mut pending = authorization(kind, AuthorizationStatus::Pending);
        pending.challenges[1].status = ChallengeStatus::Processing;
        VecDeque::from([
            Step::Ready("https://acme.example/dns/1".into()),
            Step::Authorization(order.authorization_urls[0].clone(), pending),
            Step::Authorization(
                order.authorization_urls[0].clone(),
                authorization(kind, AuthorizationStatus::Valid),
            ),
            Step::Order(order.url.clone(), self::order(kind, OrderStatus::Pending)),
            Step::Order(order.url.clone(), self::order(kind, OrderStatus::Ready)),
            Step::Finalize(order.finalize_url.clone(), csr_der.into()),
            Step::Order(
                order.url.clone(),
                self::order(kind, OrderStatus::Processing),
            ),
            Step::Order(order.url.clone(), order.clone()),
            Step::Certificate(order.certificate_url.unwrap(), CERTIFICATE.into()),
        ])
    }

    fn run<W: ChallengeTxtWriter>(
        client: &mut AcmeClient<ScriptedDirectory>,
        prepared: PreparedOrder,
        payload: &CourierPayload,
        gate: &mut DnsChallengeGate<W>,
    ) -> Result<Vec<u8>, AcmeError> {
        client.complete(
            prepared,
            payload,
            gate,
            NOW,
            &BTreeSet::new(),
            &BTreeSet::new(),
        )
    }

    #[test]
    fn https_acme_courier_round_trip_classic_dns01_for_apps_and_routers() {
        for kind in ['a', 'n'] {
            let (mut client, prepared, payload) = prepared(kind);
            client.transport.steps = completion_steps(kind, &payload.csr_der);
            let writer = JournalWriter {
                events: client.transport.events.clone(),
                fail: false,
                records: vec![],
            };
            let mut gate = DnsChallengeGate::new("mesh.worldtree.network", writer).unwrap();
            // Serialize through the host/gateway boundary, then return only cert bytes.
            let received: CourierPayload =
                serde_json::from_slice(&serde_json::to_vec(&payload).unwrap()).unwrap();
            assert_eq!(
                run(&mut client, prepared, &received, &mut gate).unwrap(),
                CERTIFICATE
            );
            assert!(client.transport.steps.is_empty());
            let events = client.transport.events.borrow();
            assert_eq!(
                &events[..4],
                &["new-order", "authorization", "txt", "ready"]
            );
            assert!(
                events.iter().position(|e| *e == "finalize").unwrap()
                    > events.iter().position(|e| *e == "ready").unwrap()
            );
            assert_eq!(gate.writer().records.len(), 1);
            assert_eq!(
                gate.writer().records[0].name,
                format!("_acme-challenge.{}", fqdn(kind))
            );
            assert_eq!(
                gate.writer().records[0].value,
                // Independently calculated with Python hashlib + urlsafe_b64encode.
                "PFEQIhYbCxM0TRymYlzljFM5hgjYcxINGOeY1PHX-fQ"
            );
            assert_eq!(gate.decisions().len(), 1);
            assert_eq!(gate.seen_nonces(), &BTreeSet::from(["01".repeat(16)]));
        }
    }

    #[test]
    fn https_acme_payload_has_only_envelope_and_csr_and_never_host_key_seed() {
        let (_, _, payload) = prepared('a');
        let wire = serde_json::to_vec(&payload).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&wire).unwrap();
        let object = value.as_object().unwrap();
        assert_eq!(object.len(), 2);
        assert!(object.contains_key("issuance_envelope") && object.contains_key("csr_der"));
        for secret in [HOST_SEED, owner().to_bytes()] {
            assert!(!wire.windows(32).any(|window| window == secret));
            assert!(!payload.csr_der.windows(32).any(|window| window == secret));
            let text = std::str::from_utf8(&wire).unwrap();
            assert!(!text.contains(&hex(&secret)));
            assert!(!text.contains(&base64url(&secret)));
            assert!(!text.contains(&serde_json::to_string(&secret).unwrap()));
        }
        let mut value = value;
        value["private_key"] = serde_json::json!(HOST_SEED);
        assert!(serde_json::from_value::<CourierPayload>(value).is_err());
    }

    #[test]
    fn https_acme_rejects_forgery_csr_substitution_expiry_and_replay_before_ready() {
        for case in ["forged", "csr", "expired", "replayed"] {
            let (mut client, prepared, mut payload) = prepared('a');
            let mut gate =
                DnsChallengeGate::new("mesh.worldtree.network", InMemoryTxtWriter::default())
                    .unwrap();
            let mut nonces = BTreeSet::new();
            let expected = match case {
                "forged" => {
                    payload.issuance_envelope = payload.issuance_envelope.replace("house", "other");
                    IssuanceError::ForgedSignature
                }
                "csr" => {
                    payload.csr_der = csr([0x54; 32]).0;
                    IssuanceError::CsrMismatch
                }
                "expired" => {
                    let auth = prepared
                        .authorization(&payload.csr_der, "01".repeat(16), NOW)
                        .unwrap();
                    payload = CourierPayload::sign(&auth, payload.csr_der, &owner()).unwrap();
                    IssuanceError::Expired
                }
                _ => {
                    nonces.insert("01".repeat(16));
                    IssuanceError::ReplayedNonce
                }
            };
            assert_eq!(
                client.complete(
                    prepared,
                    &payload,
                    &mut gate,
                    NOW,
                    &nonces,
                    &BTreeSet::new()
                ),
                Err(AcmeError::Issuance(expected)),
                "{case}"
            );
            assert_eq!(
                &*client.transport.events.borrow(),
                &["new-order", "authorization"]
            );
            assert!(gate.writer().records().is_empty());
        }
    }

    #[test]
    fn https_acme_binds_signed_name_account_and_digest_to_prepared_order() {
        for case in ["name", "account", "digest", "transport-account"] {
            let (mut client, prepared, mut payload) = prepared('a');
            let mut auth = prepared
                .authorization(&payload.csr_der, "01".repeat(16), NOW + 3600)
                .unwrap();
            match case {
                "name" => auth.fqdn = auth.fqdn.replace("house", "work"),
                "account" => auth.acme_account.push('2'),
                "digest" => auth.txt_digest = "A".repeat(43),
                _ => client.transport.account.url.push('2'),
            }
            payload = CourierPayload::sign(&auth, payload.csr_der, &owner()).unwrap();
            let mut gate =
                DnsChallengeGate::new("mesh.worldtree.network", InMemoryTxtWriter::default())
                    .unwrap();
            assert_eq!(
                run(&mut client, prepared, &payload, &mut gate),
                Err(AcmeError::OrderBinding),
                "{case}"
            );
            assert!(gate.writer().records().is_empty());
        }
    }

    #[test]
    fn https_acme_gate_refusal_and_writer_failure_never_signal_ready() {
        for case in ["zone", "writer", "gate-replay"] {
            let (mut client, prepared, payload) = prepared('a');
            let writer = JournalWriter {
                events: client.transport.events.clone(),
                fail: case == "writer",
                records: vec![],
            };
            let zone = if case == "zone" {
                "different.example"
            } else {
                "mesh.worldtree.network"
            };
            let mut gate = DnsChallengeGate::new(zone, writer).unwrap();
            if case == "gate-replay" {
                gate.submit(
                    &payload.issuance_envelope,
                    &csr_spki_sha256(&payload.csr_der).unwrap(),
                    NOW,
                    &BTreeSet::new(),
                    &BTreeSet::new(),
                )
                .unwrap();
            }
            let result = run(&mut client, prepared, &payload, &mut gate);
            match case {
                "zone" => assert_eq!(result, Err(AcmeError::Dns(DnsChallengeError::OutsideZone))),
                "writer" => assert_eq!(
                    result,
                    Err(AcmeError::Dns(DnsChallengeError::Writer(
                        "scripted TXT failure".into()
                    )))
                ),
                _ => assert_eq!(
                    result,
                    Err(AcmeError::Issuance(IssuanceError::ReplayedNonce))
                ),
            }
            assert!(!client.transport.events.borrow().contains(&"ready"));
            assert_eq!(gate.decisions().len(), 1);
        }
    }

    #[test]
    fn https_acme_directory_failures_and_bounded_authorization_polling() {
        for case in ["transport", "invalid", "limit", "changed-challenge"] {
            let (mut client, prepared, payload) = prepared('a');
            client.max_polls = 2;
            client
                .transport
                .steps
                .push_back(Step::Ready(prepared.challenge.url.clone()));
            let expected = match case {
                "transport" => {
                    client.transport.steps.push_back(Step::Failure);
                    AcmeError::Transport("scripted directory failure".into())
                }
                "invalid" => {
                    client.transport.steps.push_back(Step::Authorization(
                        prepared.order.authorization_urls[0].clone(),
                        authorization('a', AuthorizationStatus::Invalid),
                    ));
                    AcmeError::Rejected
                }
                "changed-challenge" => {
                    let mut auth = authorization('a', AuthorizationStatus::Valid);
                    auth.challenges[1].token = "D".repeat(32);
                    client.transport.steps.push_back(Step::Authorization(
                        prepared.order.authorization_urls[0].clone(),
                        auth,
                    ));
                    AcmeError::OrderBinding
                }
                _ => {
                    for _ in 0..2 {
                        client.transport.steps.push_back(Step::Authorization(
                            prepared.order.authorization_urls[0].clone(),
                            authorization('a', AuthorizationStatus::Pending),
                        ));
                    }
                    AcmeError::PollLimit
                }
            };
            let mut gate =
                DnsChallengeGate::new("mesh.worldtree.network", InMemoryTxtWriter::default())
                    .unwrap();
            assert_eq!(
                run(&mut client, prepared, &payload, &mut gate),
                Err(expected)
            );
            assert!(client.transport.steps.is_empty());
            assert_eq!(gate.writer().records().len(), 1);
            assert!(!client.transport.events.borrow().contains(&"finalize"));
        }
    }

    #[test]
    fn https_acme_refuses_non_dns01_and_wrong_directory_identifier() {
        for case in ["http-only", "identifier", "wildcard", "token"] {
            let mut steps = preparation_steps('a');
            let Step::Authorization(_, auth) = steps.back_mut().unwrap() else {
                unreachable!()
            };
            match case {
                "http-only" => {
                    auth.challenges.pop();
                }
                "identifier" => auth.identifier.value = fqdn('n'),
                "wildcard" => auth.wildcard = true,
                _ => auth.challenges[1].token.push('='),
            }
            let mut client = AcmeClient::new(
                ScriptedDirectory {
                    account: account(),
                    steps,
                    events: Events::default(),
                },
                2,
            );
            assert!(matches!(
                client.prepare(&fqdn('a')),
                Err(AcmeError::Protocol(_))
            ));
        }
    }

    #[test]
    fn https_acme_order_poll_failures_stop_before_certificate_download() {
        for case in ["invalid", "limit", "binding", "empty-cert", "missing-cert"] {
            let (mut client, prepared, payload) = prepared('a');
            client.max_polls = 1;
            let order = prepared.order.clone();
            client.transport.steps = VecDeque::from([
                Step::Ready(prepared.challenge.url.clone()),
                Step::Authorization(
                    order.authorization_urls[0].clone(),
                    authorization('a', AuthorizationStatus::Valid),
                ),
                Step::Order(order.url.clone(), self::order('a', OrderStatus::Ready)),
                Step::Finalize(order.finalize_url.clone(), payload.csr_der.clone()),
            ]);
            let mut response = self::order('a', OrderStatus::Valid);
            let expected = match case {
                "invalid" => {
                    response.status = OrderStatus::Invalid;
                    AcmeError::Rejected
                }
                "limit" => {
                    response.status = OrderStatus::Processing;
                    AcmeError::PollLimit
                }
                "binding" => {
                    response.identifiers[0].value = fqdn('n');
                    AcmeError::OrderBinding
                }
                "missing-cert" => {
                    response.certificate_url = None;
                    AcmeError::Protocol("valid order has no certificate URL")
                }
                _ => AcmeError::Protocol("empty certificate response"),
            };
            client
                .transport
                .steps
                .push_back(Step::Order(order.url, response));
            if case == "empty-cert" {
                client.transport.steps.push_back(Step::Certificate(
                    "https://acme.example/cert/1".into(),
                    vec![],
                ));
            }
            let mut gate =
                DnsChallengeGate::new("mesh.worldtree.network", InMemoryTxtWriter::default())
                    .unwrap();
            assert_eq!(
                run(&mut client, prepared, &payload, &mut gate),
                Err(expected)
            );
            assert!(client.transport.steps.is_empty());
        }
    }

    #[test]
    fn https_acme_csr_hash_uses_actual_complete_spki_and_rejects_malformed_der() {
        let (csr, spki) = csr(HOST_SEED);
        assert_eq!(
            csr_spki_sha256(&csr).unwrap(),
            <[u8; 32]>::from(Sha256::digest(&spki))
        );
        for end in 0..csr.len() {
            assert_eq!(csr_spki_sha256(&csr[..end]), Err(AcmeError::Csr));
        }
        let mut trailing = csr.clone();
        trailing.push(0);
        assert_eq!(csr_spki_sha256(&trailing), Err(AcmeError::Csr));
        let mut indefinite = csr.clone();
        indefinite[1] = 0x80;
        assert_eq!(csr_spki_sha256(&indefinite), Err(AcmeError::Csr));
        assert_eq!(csr_spki_sha256(b"not a CSR"), Err(AcmeError::Csr));
        assert_eq!(csr_spki_sha256(&spki), Err(AcmeError::Csr));
        let (_, prepared, payload) = prepared('a');
        let mut auth = prepared
            .authorization(&payload.csr_der, "01".repeat(16), NOW + 3600)
            .unwrap();
        auth.csr_spki_sha256 = "00".repeat(32);
        assert_eq!(
            CourierPayload::sign(&auth, payload.csr_der, &owner()),
            Err(AcmeError::Issuance(IssuanceError::CsrMismatch))
        );
    }

    fn renewal(start: &str, end: &str) -> Result<RenewalInfo, AcmeError> {
        RenewalInfo::parse(
            &serde_json::to_vec(&serde_json::json!({
                "suggestedWindow": {"start": start, "end": end},
                "explanationURL": "https://acme.example/docs/ari",
                "futureExtension": true,
            }))
            .unwrap(),
        )
    }

    #[test]
    fn https_acme_ari_parses_window_and_reports_membership_without_lifetime_guessing() {
        // RFC 9773 example, with epoch boundaries independently calculated.
        let info = renewal("2025-01-02T04:00:00Z", "2025-01-03T04:00:00Z").unwrap();
        assert_eq!(
            info.explanation_url.as_deref(),
            Some("https://acme.example/docs/ari")
        );
        assert!(!info.contains(1_735_790_399));
        assert!(info.contains(1_735_790_400));
        assert!(info.contains(1_735_876_799));
        assert!(!info.contains(1_735_876_800));
        assert!(!info.contains(u64::MAX));
        assert_eq!(
            renewal("2025-01-02T05:00:00+01:00", "2025-01-02T23:00:00-05:00").unwrap(),
            info
        );
        let fractional = renewal(
            "1970-01-01t00:00:00.0001z",
            "1970-01-01T00:00:01.00000000001Z",
        )
        .unwrap();
        assert!(!fractional.contains(0));
        assert!(fractional.contains(1));
        assert!(!fractional.contains(2));
        assert!(renewal("2000-02-29T00:00:00Z", "2000-03-01T00:00:00Z").is_ok());
    }

    #[test]
    fn https_acme_ari_rejects_malformed_empty_or_reversed_windows() {
        for start in [
            "",
            "2025-02-29T00:00:00Z",
            "1900-02-29T00:00:00Z",
            "2025-13-01T00:00:00Z",
            "2025-01-01T24:00:00Z",
            "2025-01-01T00:60:00Z",
            "2025-01-01T00:00:60Z",
            "2025-01-01T00:00:00.Z",
            "2025-01-01T00:00:00",
            "2025-01-01T00:00:00+24:00",
            "2025-01-01T00:00:00Zjunk",
            "2025-01-01T00:00:00.éZ",
        ] {
            assert_eq!(
                renewal(start, "2026-01-01T00:00:00Z"),
                Err(AcmeError::RenewalInfo),
                "{start}"
            );
        }
        for (start, end) in [
            ("2025-01-01T00:00:00Z", "2025-01-01T00:00:00Z"),
            ("2026-01-01T00:00:00Z", "2025-01-01T00:00:00Z"),
            ("2025-01-01T00:00:00.1Z", "2025-01-01T00:00:00.10Z"),
        ] {
            assert_eq!(renewal(start, end), Err(AcmeError::RenewalInfo));
        }
        for body in [
            b"{}".as_slice(),
            br#"{"suggestedWindow":{"start":42,"end":false}}"#,
            b"not json",
        ] {
            assert_eq!(RenewalInfo::parse(body), Err(AcmeError::RenewalInfo));
        }
    }
}
