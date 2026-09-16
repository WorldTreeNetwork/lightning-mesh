// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors
// Lightning Mesh is dual-licensed (AGPL-3.0-or-later or commercial); see LICENSE
// and COMMERCIAL-LICENSE.md at the repository root.

//! Owner-signed HTTPS aliases and their local-DNS projection.

use std::collections::{BTreeMap, BTreeSet};
use std::net::Ipv4Addr;
use std::sync::{Arc, RwLock};
#[cfg(feature = "daemon")]
use std::time::{SystemTime, UNIX_EPOCH};

use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::https_label;

pub const ALIAS_DOMAIN_LINE: &[u8] = b"mjolnir-https-alias:v1\n";
pub const DEFAULT_HTTPS_ZONE: &str = "mesh.worldtree.network";

/// The byte-exact compact JSON carried by a `mjolnir-https-alias:v1`
/// envelope. Field declaration order is protocol order: serde's struct
/// serializer preserves it when producing the canonical compact form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AliasRecord {
    pub owner_pubkey: String,
    pub fqdn: String,
    pub addr: String,
    pub seq: u64,
    pub valid_from: u64,
    pub valid_until: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedAliasRecord {
    pub record: AliasRecord,
    pub owner_pubkey: [u8; 32],
    pub addr: Ipv4Addr,
    /// The exact, canonical payload bytes covered by the signature.
    pub payload: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AliasError {
    #[error("alias envelope must be exactly three LF-separated lines")]
    Envelope,
    #[error("alias envelope field names or ordering are invalid")]
    EnvelopeFields,
    #[error("owner public key must be 64 lowercase hexadecimal characters")]
    OwnerPubkey,
    #[error("signature must be 128 lowercase hexadecimal characters")]
    Signature,
    #[error("alias payload is not valid protocol JSON")]
    PayloadJson,
    #[error("alias payload is not canonical compact JSON")]
    NonCanonicalPayload,
    #[error("envelope owner does not equal payload owner")]
    OwnerMismatch,
    #[error("owner public key is not a valid Ed25519 verification key")]
    InvalidOwnerKey,
    #[error("alias signature did not verify")]
    ForgedSignature,
    #[error("alias address is not canonical dotted IPv4")]
    Address,
    #[error("alias FQDN is not normalized ASCII DNS form")]
    Fqdn,
    #[error("alias FQDN label is not derived from its full owner public key")]
    WrongOwnerLabel,
    #[error("HTTPS mesh label or zone is not normalized DNS form")]
    DnsmasqName,
}

/// Parse and verify a complete three-line transport envelope.
pub fn verify_alias_envelope(envelope: &str) -> Result<VerifiedAliasRecord, AliasError> {
    verify_alias_envelope_with(envelope, https_label)
}

fn verify_alias_envelope_with(
    envelope: &str,
    labeler: fn(&[u8]) -> String,
) -> Result<VerifiedAliasRecord, AliasError> {
    if envelope.contains('\r') {
        return Err(AliasError::Envelope);
    }
    // A conventional final file newline is allowed, but it is not a fourth
    // line and is never part of the signed payload.
    let envelope = envelope.strip_suffix('\n').unwrap_or(envelope);
    if envelope.ends_with('\n') {
        return Err(AliasError::Envelope);
    }
    let mut lines = envelope.split('\n');
    let owner_line = lines.next().ok_or(AliasError::Envelope)?;
    let payload_line = lines.next().ok_or(AliasError::Envelope)?;
    let sig_line = lines.next().ok_or(AliasError::Envelope)?;
    if lines.next().is_some() {
        return Err(AliasError::Envelope);
    }
    let owner_hex = owner_line
        .strip_prefix("owner_pubkey=")
        .ok_or(AliasError::EnvelopeFields)?;
    let payload = payload_line
        .strip_prefix("payload=")
        .ok_or(AliasError::EnvelopeFields)?;
    let sig_hex = sig_line
        .strip_prefix("sig=")
        .ok_or(AliasError::EnvelopeFields)?;

    let owner_pubkey = decode_lower_hex::<32>(owner_hex).ok_or(AliasError::OwnerPubkey)?;
    let signature_bytes = decode_lower_hex::<64>(sig_hex).ok_or(AliasError::Signature)?;
    let record: AliasRecord = serde_json::from_str(payload).map_err(|_| AliasError::PayloadJson)?;
    let canonical = serde_json::to_string(&record).map_err(|_| AliasError::PayloadJson)?;
    if canonical != payload {
        return Err(AliasError::NonCanonicalPayload);
    }
    if record.owner_pubkey != owner_hex {
        return Err(AliasError::OwnerMismatch);
    }

    let verifying_key =
        VerifyingKey::from_bytes(&owner_pubkey).map_err(|_| AliasError::InvalidOwnerKey)?;
    if verifying_key.is_weak() {
        return Err(AliasError::InvalidOwnerKey);
    }
    let signature = Signature::from_bytes(&signature_bytes);
    let mut signed = Vec::with_capacity(ALIAS_DOMAIN_LINE.len() + payload.len());
    signed.extend_from_slice(ALIAS_DOMAIN_LINE);
    signed.extend_from_slice(payload.as_bytes());
    verifying_key
        .verify_strict(&signed, &signature)
        .map_err(|_| AliasError::ForgedSignature)?;

    validate_fqdn(&record.fqdn)?;
    let owner_label = fqdn_owner_label(&record.fqdn).ok_or(AliasError::Fqdn)?;
    if owner_label != labeler(&owner_pubkey) {
        return Err(AliasError::WrongOwnerLabel);
    }
    let addr: Ipv4Addr = record.addr.parse().map_err(|_| AliasError::Address)?;
    if addr.to_string() != record.addr {
        return Err(AliasError::Address);
    }

    Ok(VerifiedAliasRecord {
        record,
        owner_pubkey,
        addr,
        payload: payload.to_owned(),
    })
}

fn decode_lower_hex<const N: usize>(value: &str) -> Option<[u8; N]> {
    if value.len() != N * 2
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return None;
    }
    let mut out = [0_u8; N];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let text = std::str::from_utf8(pair).ok()?;
        out[index] = u8::from_str_radix(text, 16).ok()?;
    }
    Some(out)
}

fn validate_fqdn(fqdn: &str) -> Result<(), AliasError> {
    if fqdn.is_empty()
        || fqdn.len() > 253
        || !fqdn.is_ascii()
        || fqdn.ends_with('.')
        || fqdn != fqdn.to_ascii_lowercase()
    {
        return Err(AliasError::Fqdn);
    }
    let labels: Vec<&str> = fqdn.split('.').collect();
    if labels.len() < 3 || labels.iter().any(|label| !valid_dns_label(label)) {
        return Err(AliasError::Fqdn);
    }
    Ok(())
}

fn valid_dns_label(label: &str) -> bool {
    !label.is_empty()
        && label.len() <= 63
        && label
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !label.starts_with('-')
        && !label.ends_with('-')
}

fn fqdn_owner_label(fqdn: &str) -> Option<&str> {
    let first = fqdn.split('.').next()?;
    let label = first
        .strip_prefix("a-")
        .or_else(|| first.strip_prefix("n-"))?;
    if label.len() == 16
        && label
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || (b'2'..=b'7').contains(&byte))
    {
        Some(label)
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyAlias {
    Inserted,
    Replaced,
    Unchanged,
    IgnoredOlder,
    Conflicted,
    LabelCollision,
}

#[derive(Debug, Clone)]
enum AliasHead {
    Record(VerifiedAliasRecord),
    Conflict { seq: u64 },
}

#[derive(Debug, Default)]
struct AliasState {
    heads: BTreeMap<([u8; 32], String), AliasHead>,
    /// Append-only by contract. Tombstones and expiry never remove an owner.
    owners_by_label: BTreeMap<String, BTreeSet<[u8; 32]>>,
}

/// Thread-safe projection of verified alias envelopes into A answers.
///
/// `retained_owner_pubkeys` is the persistence seam: callers must persist and
/// restore this append-only set along with record heads so truncated labels
/// are never recycled after restart or compaction.
#[derive(Clone)]
pub struct AliasTable {
    state: Arc<RwLock<AliasState>>,
    labeler: fn(&[u8]) -> String,
}

impl Default for AliasTable {
    fn default() -> Self {
        Self {
            state: Arc::new(RwLock::new(AliasState::default())),
            labeler: https_label,
        }
    }
}

impl AliasTable {
    /// Restore the append-only collision history before replaying retained
    /// signed record heads. This is the restart/compaction seam: even owners
    /// whose last record was a tombstone remain represented.
    pub fn from_retained_owner_pubkeys(
        owners: impl IntoIterator<Item = [u8; 32]>,
    ) -> Result<Self, AliasError> {
        let table = Self::default();
        {
            let mut state = table.state.write().unwrap_or_else(|e| e.into_inner());
            for owner in owners {
                let key =
                    VerifyingKey::from_bytes(&owner).map_err(|_| AliasError::InvalidOwnerKey)?;
                if key.is_weak() {
                    return Err(AliasError::InvalidOwnerKey);
                }
                state
                    .owners_by_label
                    .entry((table.labeler)(&owner))
                    .or_default()
                    .insert(owner);
            }
        }
        Ok(table)
    }

    /// Verify and merge one transport envelope. No unverified record enters
    /// the table or its retained collision history.
    pub fn apply_envelope(&self, envelope: &str) -> Result<ApplyAlias, AliasError> {
        let verified = verify_alias_envelope_with(envelope, self.labeler)?;
        let label = (self.labeler)(&verified.owner_pubkey);
        let key = (verified.owner_pubkey, verified.record.fqdn.clone());
        let mut state = self.state.write().unwrap_or_else(|e| e.into_inner());
        let owners = state.owners_by_label.entry(label).or_default();
        owners.insert(verified.owner_pubkey);
        let collided = owners.len() > 1;

        let outcome = match state.heads.get(&key) {
            None => {
                state.heads.insert(key, AliasHead::Record(verified));
                ApplyAlias::Inserted
            }
            Some(AliasHead::Record(old)) if verified.record.seq < old.record.seq => {
                ApplyAlias::IgnoredOlder
            }
            Some(AliasHead::Conflict { seq }) if verified.record.seq < *seq => {
                ApplyAlias::IgnoredOlder
            }
            Some(AliasHead::Record(old))
                if verified.record.seq == old.record.seq && verified.payload == old.payload =>
            {
                ApplyAlias::Unchanged
            }
            Some(AliasHead::Conflict { seq }) if verified.record.seq == *seq => {
                ApplyAlias::Conflicted
            }
            Some(AliasHead::Record(old)) if verified.record.seq == old.record.seq => {
                state.heads.insert(
                    key,
                    AliasHead::Conflict {
                        seq: verified.record.seq,
                    },
                );
                ApplyAlias::Conflicted
            }
            Some(_) => {
                state.heads.insert(key, AliasHead::Record(verified));
                ApplyAlias::Replaced
            }
        };
        Ok(if collided {
            ApplyAlias::LabelCollision
        } else {
            outcome
        })
    }

    /// Resolve at an explicit Unix timestamp, used both by the DNS adapter
    /// and deterministic expiry/tombstone tests.
    pub fn lookup_a_at(&self, name: &str, now_unix: u64) -> Option<Vec<Ipv4Addr>> {
        let fqdn = name.trim_end_matches('.').to_ascii_lowercase();
        let label = fqdn_owner_label(&fqdn)?;
        let state = self.state.read().unwrap_or_else(|e| e.into_inner());
        let owners = state.owners_by_label.get(label)?;
        if owners.len() != 1 {
            return None;
        }
        let owner = *owners.iter().next()?;
        let AliasHead::Record(head) = state.heads.get(&(owner, fqdn))? else {
            return None;
        };
        if now_unix < head.record.valid_from || now_unix >= head.record.valid_until {
            return None;
        }
        Some(vec![head.addr])
    }

    pub fn retained_owner_pubkeys(&self) -> Vec<[u8; 32]> {
        let state = self.state.read().unwrap_or_else(|e| e.into_inner());
        state
            .owners_by_label
            .values()
            .flat_map(|owners| owners.iter().copied())
            .collect()
    }

    #[cfg(test)]
    fn with_labeler(labeler: fn(&[u8]) -> String) -> Self {
        Self {
            state: Arc::new(RwLock::new(AliasState::default())),
            labeler,
        }
    }
}

#[cfg(feature = "daemon")]
impl crate::dns_responder::NameTable for AliasTable {
    fn lookup_a(&self, name: &str) -> Option<Vec<Ipv4Addr>> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        self.lookup_a_at(name, now)
    }
}

/// Exact child-suffix entries owned by the HTTPS alias feature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpsDnsmasqConfig {
    pub suffix: String,
    pub server_line: String,
    pub rebind_domain: String,
    pub parent_server_line: String,
    pub parent_rebind_domain: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct HttpsDnsmasqPlan {
    pub delete_servers: Vec<String>,
    pub add_servers: Vec<String>,
    pub delete_rebind_domains: Vec<String>,
    pub add_rebind_domains: Vec<String>,
}

impl HttpsDnsmasqPlan {
    pub fn is_empty(&self) -> bool {
        self.delete_servers.is_empty()
            && self.add_servers.is_empty()
            && self.delete_rebind_domains.is_empty()
            && self.add_rebind_domains.is_empty()
    }
}

/// Build the exact child suffix dnsmasq may forward and rebind-exempt.
/// `None`/empty `mesh_label` disables the feature. The parent-zone forms are
/// returned only so reconciliation can remove them; they are never desired.
pub fn https_dnsmasq_config(
    mesh_label: Option<&str>,
    zone: Option<&str>,
) -> Result<Option<HttpsDnsmasqConfig>, AliasError> {
    let Some(mesh_label) = mesh_label.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let zone = zone
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_HTTPS_ZONE);
    if !valid_dns_label(mesh_label)
        || zone.split('.').count() < 2
        || zone.split('.').any(|label| !valid_dns_label(label))
        || zone.len() > 253
    {
        return Err(AliasError::DnsmasqName);
    }
    let suffix = format!("{mesh_label}.{zone}");
    Ok(Some(HttpsDnsmasqConfig {
        server_line: format!("/{suffix}/127.0.0.1#5335"),
        rebind_domain: format!("/{suffix}/"),
        parent_server_line: format!("/{zone}/127.0.0.1#5335"),
        parent_rebind_domain: format!("/{zone}/"),
        suffix,
    }))
}

/// Plan an idempotent exact-suffix reconciliation. A mistakenly configured
/// parent-zone entry is removed. Duplicate child entries are collapsed to
/// exactly one by deleting the value and adding it once.
pub fn https_dnsmasq_plan(
    current_server_list: &str,
    current_rebind_domains: &str,
    config: Option<&HttpsDnsmasqConfig>,
) -> HttpsDnsmasqPlan {
    let Some(config) = config else {
        return HttpsDnsmasqPlan::default();
    };
    let servers: Vec<&str> = current_server_list.split_whitespace().collect();
    let rebinds: Vec<&str> = current_rebind_domains.split_whitespace().collect();
    let mut plan = HttpsDnsmasqPlan::default();

    if servers.contains(&config.parent_server_line.as_str()) {
        plan.delete_servers.push(config.parent_server_line.clone());
    }
    let server_count = servers
        .iter()
        .filter(|entry| **entry == config.server_line.as_str())
        .count();
    if server_count != 1 {
        if server_count > 0 {
            plan.delete_servers.push(config.server_line.clone());
        }
        plan.add_servers.push(config.server_line.clone());
    }

    if rebinds.contains(&config.parent_rebind_domain.as_str()) {
        plan.delete_rebind_domains
            .push(config.parent_rebind_domain.clone());
    }
    let rebind_count = rebinds
        .iter()
        .filter(|entry| **entry == config.rebind_domain.as_str())
        .count();
    if rebind_count != 1 {
        if rebind_count > 0 {
            plan.delete_rebind_domains
                .push(config.rebind_domain.clone());
        }
        plan.add_rebind_domains.push(config.rebind_domain.clone());
    }
    plan
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::{Signer, SigningKey};

    use super::*;

    fn lower_hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    fn envelope(
        key: &SigningKey,
        fqdn: &str,
        addr: &str,
        seq: u64,
        valid_from: u64,
        valid_until: u64,
    ) -> String {
        let owner_pubkey = lower_hex(&key.verifying_key().to_bytes());
        let record = AliasRecord {
            owner_pubkey: owner_pubkey.clone(),
            fqdn: fqdn.to_owned(),
            addr: addr.to_owned(),
            seq,
            valid_from,
            valid_until,
        };
        let payload = serde_json::to_string(&record).unwrap();
        let mut signed = ALIAS_DOMAIN_LINE.to_vec();
        signed.extend_from_slice(payload.as_bytes());
        let sig = lower_hex(&key.sign(&signed).to_bytes());
        format!("owner_pubkey={owner_pubkey}\npayload={payload}\nsig={sig}")
    }

    fn fqdn(key: &SigningKey) -> String {
        format!(
            "a-{}.house.mesh.worldtree.network",
            https_label(&key.verifying_key().to_bytes())
        )
    }

    #[test]
    fn https_alias_golden_compact_json_and_signed_bytes() {
        let key = SigningKey::from_bytes(&[7; 32]);
        let owner = lower_hex(&key.verifying_key().to_bytes());
        let name = fqdn(&key);
        let wire = envelope(&key, &name, "10.42.7.9", 12, 1_700_000_000, 1_700_003_600);
        let verified = verify_alias_envelope(&wire).unwrap();
        let expected = format!(
            "{{\"owner_pubkey\":\"{owner}\",\"fqdn\":\"{name}\",\"addr\":\"10.42.7.9\",\"seq\":12,\"valid_from\":1700000000,\"valid_until\":1700003600}}"
        );
        assert_eq!(verified.payload, expected);
        assert_eq!(
            [ALIAS_DOMAIN_LINE, expected.as_bytes()].concat(),
            [b"mjolnir-https-alias:v1\n".as_slice(), expected.as_bytes()].concat()
        );
    }

    #[test]
    fn https_alias_forged_signature_is_rejected() {
        let key = SigningKey::from_bytes(&[1; 32]);
        let mut wire = envelope(&key, &fqdn(&key), "10.42.1.2", 1, 0, 100);
        let last = wire.pop().unwrap();
        wire.push(if last == '0' { '1' } else { '0' });
        assert_eq!(
            verify_alias_envelope(&wire),
            Err(AliasError::ForgedSignature)
        );
    }

    #[test]
    fn https_alias_wrong_owner_for_label_is_rejected() {
        let named = SigningKey::from_bytes(&[2; 32]);
        let signer = SigningKey::from_bytes(&[3; 32]);
        let wire = envelope(&signer, &fqdn(&named), "10.42.1.3", 1, 0, 100);
        assert_eq!(
            verify_alias_envelope(&wire),
            Err(AliasError::WrongOwnerLabel)
        );
    }

    fn collision_label(_key: &[u8]) -> String {
        "aaaaaaaaaaaaaaaa".to_owned()
    }

    #[test]
    fn https_alias_collision_of_full_owner_keys_fails_closed() {
        let one = SigningKey::from_bytes(&[4; 32]);
        let two = SigningKey::from_bytes(&[5; 32]);
        let name = "a-aaaaaaaaaaaaaaaa.house.mesh.worldtree.network";
        let table = AliasTable::with_labeler(collision_label);
        table
            .apply_envelope(&envelope(&one, name, "10.42.1.4", 1, 0, 100))
            .unwrap();
        assert_eq!(
            table.lookup_a_at(name, 50),
            Some(vec!["10.42.1.4".parse().unwrap()])
        );
        assert_eq!(
            table
                .apply_envelope(&envelope(&two, name, "10.42.1.5", 1, 0, 100))
                .unwrap(),
            ApplyAlias::LabelCollision
        );
        assert_eq!(table.lookup_a_at(name, 50), None);
        assert_eq!(table.retained_owner_pubkeys().len(), 2);
    }

    #[test]
    fn https_alias_tombstone_stops_answers_but_retains_owner() {
        let key = SigningKey::from_bytes(&[6; 32]);
        let name = fqdn(&key);
        let table = AliasTable::default();
        table
            .apply_envelope(&envelope(&key, &name, "10.42.1.6", 1, 0, 100))
            .unwrap();
        assert!(table.lookup_a_at(&name, 50).is_some());
        table
            .apply_envelope(&envelope(&key, &name, "10.42.1.6", 2, 0, 40))
            .unwrap();
        assert_eq!(table.lookup_a_at(&name, 50), None);
        assert_eq!(
            table.retained_owner_pubkeys(),
            vec![key.verifying_key().to_bytes()]
        );
    }

    #[test]
    fn https_alias_equal_seq_different_payload_fails_closed() {
        let key = SigningKey::from_bytes(&[8; 32]);
        let name = fqdn(&key);
        let table = AliasTable::default();
        table
            .apply_envelope(&envelope(&key, &name, "10.42.1.8", 9, 0, 100))
            .unwrap();
        assert_eq!(
            table
                .apply_envelope(&envelope(&key, &name, "10.42.1.9", 9, 0, 100))
                .unwrap(),
            ApplyAlias::Conflicted
        );
        assert_eq!(table.lookup_a_at(&name, 50), None);
    }

    #[test]
    fn dnsmasq_https_suffix_is_exact_and_parent_is_never_desired() {
        let config = https_dnsmasq_config(Some("house"), None).unwrap().unwrap();
        assert_eq!(config.suffix, "house.mesh.worldtree.network");
        assert_eq!(
            config.server_line,
            "/house.mesh.worldtree.network/127.0.0.1#5335"
        );
        assert_eq!(config.rebind_domain, "/house.mesh.worldtree.network/");
        assert_ne!(config.server_line, config.parent_server_line);
        assert_ne!(config.rebind_domain, config.parent_rebind_domain);
        assert_eq!(config.parent_rebind_domain, "/mesh.worldtree.network/");
    }

    #[test]
    fn dnsmasq_https_suffix_is_absent_without_mesh_label() {
        assert_eq!(https_dnsmasq_config(None, None).unwrap(), None);
        assert_eq!(
            https_dnsmasq_config(Some(""), Some("net.example")).unwrap(),
            None
        );
    }

    #[test]
    fn dnsmasq_parent_zone_is_removed_and_child_added_exactly_once() {
        let config = https_dnsmasq_config(Some("house"), None).unwrap().unwrap();
        let plan = https_dnsmasq_plan(
            "/mesh.worldtree.network/127.0.0.1#5335",
            "/mesh.worldtree.network/",
            Some(&config),
        );
        assert_eq!(plan.delete_servers, vec![config.parent_server_line.clone()]);
        assert_eq!(plan.add_servers, vec![config.server_line.clone()]);
        assert_eq!(
            plan.delete_rebind_domains,
            vec![config.parent_rebind_domain.clone()]
        );
        assert_eq!(plan.add_rebind_domains, vec![config.rebind_domain.clone()]);

        let converged =
            https_dnsmasq_plan(&config.server_line, &config.rebind_domain, Some(&config));
        assert!(converged.is_empty());
    }

    #[test]
    fn dnsmasq_duplicate_child_entries_are_collapsed() {
        let config = https_dnsmasq_config(Some("house"), Some("net.example"))
            .unwrap()
            .unwrap();
        let plan = https_dnsmasq_plan(
            &format!("{} {}", config.server_line, config.server_line),
            &format!("{} {}", config.rebind_domain, config.rebind_domain),
            Some(&config),
        );
        assert_eq!(plan.delete_servers, vec![config.server_line.clone()]);
        assert_eq!(plan.add_servers, vec![config.server_line.clone()]);
        assert_eq!(
            plan.delete_rebind_domains,
            vec![config.rebind_domain.clone()]
        );
        assert_eq!(plan.add_rebind_domains, vec![config.rebind_domain]);
    }
}
