// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors
// Lightning Mesh is dual-licensed (AGPL-3.0-or-later or commercial); see LICENSE
// and COMMERCIAL-LICENSE.md at the repository root.

//! Replaceable, keyless DNS-01 challenge gate. No DNS network I/O or ACME client.
//!
//! History and the append-only decision log are in memory. Operators must persist
//! the nonce and full owner-key histories (including AliasTable tombstones) and
//! supply them on restart. Serialize submissions through one gate per zone.

use std::{collections::BTreeSet, convert::Infallible, fmt::Display};

use thiserror::Error;

use crate::https_issuance::{
    IssuanceError, VerifiedIssuanceAuthorization, verify_issuance_envelope,
};

/// The only record type the gate can publish. `name` has no trailing dot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChallengeTxtRecord {
    pub name: String,
    pub value: String,
}

/// Publication seam for a DNS provider. Implementations receive only verified
/// challenge records, never mesh authority or private keys.
pub trait ChallengeTxtWriter {
    type Error: Display;

    fn write_txt(&mut self, record: &ChallengeTxtRecord) -> Result<(), Self::Error>;
}

/// A publication journal for tests and embedding; performs no network I/O.
#[derive(Debug, Default)]
pub struct InMemoryTxtWriter {
    records: Vec<ChallengeTxtRecord>,
}

impl InMemoryTxtWriter {
    pub fn records(&self) -> &[ChallengeTxtRecord] {
        &self.records
    }
}

impl ChallengeTxtWriter for InMemoryTxtWriter {
    type Error = Infallible;

    fn write_txt(&mut self, record: &ChallengeTxtRecord) -> Result<(), Self::Error> {
        self.records.push(record.clone());
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DnsChallengeError {
    #[error("zone must be a normalized lowercase ASCII DNS name without a trailing dot")]
    InvalidZone,
    #[error(transparent)]
    Authorization(#[from] IssuanceError),
    #[error("issuance FQDN is outside the configured zone")]
    OutsideZone,
    #[error("challenge record name exceeds the DNS name length limit")]
    ChallengeNameTooLong,
    #[error("TXT publication failed: {0}")]
    Writer(String),
}

/// One entry per submission, including refusals and publication failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsChallengeDecision {
    pub now: u64,
    /// BLAKE3 of the exact submitted envelope, for audit correlation.
    pub envelope_hash: String,
    pub outcome: Result<ChallengeTxtRecord, DnsChallengeError>,
}

/// Owns a configured zone, a writer, and grow-only submission history.
/// There is deliberately no mutable accessor to the writer or audit log.
pub struct DnsChallengeGate<W> {
    zone: String,
    writer: W,
    seen_nonces: BTreeSet<String>,
    retained_owner_pubkeys: BTreeSet<[u8; 32]>,
    decisions: Vec<DnsChallengeDecision>,
}

impl<W: ChallengeTxtWriter> DnsChallengeGate<W> {
    pub fn new(zone: impl Into<String>, writer: W) -> Result<Self, DnsChallengeError> {
        let zone = zone.into();
        if zone.len() > 253
            || zone.split('.').any(|label| {
                label.is_empty()
                    || label.len() > 63
                    || label.starts_with('-')
                    || label.ends_with('-')
                    || !label
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            })
        {
            return Err(DnsChallengeError::InvalidZone);
        }
        Ok(Self {
            zone,
            writer,
            seen_nonces: BTreeSet::new(),
            retained_owner_pubkeys: BTreeSet::new(),
            decisions: Vec::new(),
        })
    }

    /// Submit using the hash of the actual CSR's SPKI and Unix seconds.
    /// Supplied histories are merged with local history, never substituted for
    /// it. Refresh retained keys from AliasTable before every submission.
    ///
    /// A verified, in-zone nonce is consumed before invoking the writer. Even a
    /// failed writer may have partially published: retry with a fresh signed
    /// nonce. Refusals never invoke the writer or consume the submitted nonce.
    /// Persist the exposed histories and decisions before dropping the gate;
    /// crash-safe storage and cross-process coordination belong to the operator.
    pub fn submit(
        &mut self,
        envelope: &str,
        csr_spki_sha256: &[u8; 32],
        now: u64,
        seen_nonces: &BTreeSet<String>,
        retained_owner_pubkeys: &BTreeSet<[u8; 32]>,
    ) -> Result<ChallengeTxtRecord, DnsChallengeError> {
        self.seen_nonces.extend(seen_nonces.iter().cloned());
        self.retained_owner_pubkeys
            .extend(retained_owner_pubkeys.iter().copied());
        let verified = verify_issuance_envelope(
            envelope,
            csr_spki_sha256,
            now,
            &self.seen_nonces,
            &self.retained_owner_pubkeys,
        );
        self.publish_and_log(envelope, now, verified)
    }

    fn publish_and_log(
        &mut self,
        envelope: &str,
        now: u64,
        verified: Result<VerifiedIssuanceAuthorization, IssuanceError>,
    ) -> Result<ChallengeTxtRecord, DnsChallengeError> {
        let outcome = (|| {
            let verified = verified?;
            let auth = verified.authorization;
            // Require a whole-label suffix: `evilmesh.example` is not within
            // `mesh.example`, and the zone apex is not a challenge host.
            if !auth
                .fqdn
                .strip_suffix(&self.zone)
                .is_some_and(|prefix| prefix.ends_with('.'))
            {
                return Err(DnsChallengeError::OutsideZone);
            }
            let name = format!("_acme-challenge.{}", auth.fqdn);
            if name.len() > 253 {
                return Err(DnsChallengeError::ChallengeNameTooLong);
            }
            let record = ChallengeTxtRecord {
                name,
                value: auth.txt_digest,
            };
            self.seen_nonces.insert(auth.nonce);
            self.retained_owner_pubkeys.insert(verified.owner_pubkey);
            self.writer
                .write_txt(&record)
                .map_err(|error| DnsChallengeError::Writer(error.to_string()))?;
            Ok(record)
        })();
        self.decisions.push(DnsChallengeDecision {
            now,
            envelope_hash: blake3::hash(envelope.as_bytes()).to_hex().to_string(),
            outcome: outcome.clone(),
        });
        outcome
    }

    pub fn zone(&self) -> &str {
        &self.zone
    }

    pub fn writer(&self) -> &W {
        &self.writer
    }

    pub fn decisions(&self) -> &[DnsChallengeDecision] {
        &self.decisions
    }

    pub fn seen_nonces(&self) -> &BTreeSet<String> {
        &self.seen_nonces
    }

    pub fn retained_owner_pubkeys(&self) -> &BTreeSet<[u8; 32]> {
        &self.retained_owner_pubkeys
    }
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::SigningKey;

    use super::*;
    use crate::{https_issuance::IssuanceAuthorization, https_issuance::sign_issuance_envelope};

    const NOW: u64 = 1_800_000_000;
    const CSR: [u8; 32] = [0x42; 32];

    fn owner() -> SigningKey {
        SigningKey::from_bytes(&[7; 32])
    }

    fn authorization(zone: &str, kind: char) -> IssuanceAuthorization {
        IssuanceAuthorization {
            fqdn: format!(
                "{kind}-{}.house.{zone}",
                crate::https_label(&owner().verifying_key().to_bytes())
            ),
            txt_digest: "A".repeat(43),
            acme_account: "https://acme.example/acct/1".to_owned(),
            csr_spki_sha256: "42".repeat(32),
            nonce: "01".repeat(16),
            expires_at: NOW + 3600,
        }
    }

    fn signed(auth: &IssuanceAuthorization) -> String {
        sign_issuance_envelope(auth, &owner()).unwrap()
    }

    fn gate(zone: &str) -> DnsChallengeGate<InMemoryTxtWriter> {
        DnsChallengeGate::new(zone, InMemoryTxtWriter::default()).unwrap()
    }

    #[test]
    fn https_dns_publishes_apps_and_routers_in_project_and_bring_your_own_zones() {
        for zone in ["mesh.worldtree.network", "my-home.example"] {
            for kind in ['a', 'n'] {
                let mut gate = gate(zone);
                let auth = authorization(zone, kind);
                let envelope = signed(&auth);
                let record = gate
                    .submit(&envelope, &CSR, NOW, &BTreeSet::new(), &BTreeSet::new())
                    .unwrap();
                assert_eq!(record.name, format!("_acme-challenge.{}", auth.fqdn));
                assert_eq!(record.value, auth.txt_digest);
                assert_eq!(gate.zone(), zone);
                assert_eq!(gate.writer().records(), &[record.clone()]);
                assert_eq!(gate.decisions()[0].outcome, Ok(record));
                assert_eq!(gate.decisions()[0].now, NOW);
                assert_eq!(
                    gate.decisions()[0].envelope_hash,
                    blake3::hash(envelope.as_bytes()).to_hex().to_string()
                );
                assert!(gate.seen_nonces().contains(&auth.nonce));
                assert!(
                    gate.retained_owner_pubkeys()
                        .contains(&owner().verifying_key().to_bytes())
                );
            }
        }
    }

    #[test]
    fn https_dns_replay_uses_local_and_supplied_history_and_appends_decisions() {
        let auth = authorization("mesh.worldtree.network", 'a');
        let envelope = signed(&auth);
        let mut gate = gate("mesh.worldtree.network");
        let empty = BTreeSet::new();
        let owners = BTreeSet::new();
        assert!(gate.submit(&envelope, &CSR, NOW, &empty, &owners).is_ok());
        let first = gate.decisions()[0].clone();
        assert_eq!(
            gate.submit(&envelope, &CSR, NOW, &empty, &owners),
            Err(DnsChallengeError::Authorization(
                IssuanceError::ReplayedNonce
            ))
        );
        assert_eq!(gate.writer().records().len(), 1);
        assert_eq!(gate.decisions().len(), 2);
        assert_eq!(gate.decisions()[0], first);

        let mut restarted =
            super::DnsChallengeGate::new("mesh.worldtree.network", InMemoryTxtWriter::default())
                .unwrap();
        assert_eq!(
            restarted.submit(
                &envelope,
                &CSR,
                NOW,
                gate.seen_nonces(),
                gate.retained_owner_pubkeys()
            ),
            Err(DnsChallengeError::Authorization(
                IssuanceError::ReplayedNonce
            ))
        );
        assert!(restarted.writer().records().is_empty());
        assert_eq!(restarted.decisions().len(), 1);
    }

    #[test]
    fn https_dns_refused_authorizations_never_reach_writer_or_consume_nonce() {
        let auth = authorization("mesh.worldtree.network", 'a');
        let envelope = signed(&auth);
        let cases = [
            (
                envelope.replacen(&auth.txt_digest, &"B".repeat(43), 1),
                CSR,
                NOW,
                IssuanceError::ForgedSignature,
            ),
            (envelope.clone(), [0; 32], NOW, IssuanceError::CsrMismatch),
            (
                envelope.clone(),
                CSR,
                auth.expires_at,
                IssuanceError::Expired,
            ),
            (envelope.clone(), CSR, NOW - 1, IssuanceError::Lifetime),
            ("malformed".to_owned(), CSR, NOW, IssuanceError::Envelope),
        ];
        for (envelope, csr, now, error) in cases {
            let mut gate = gate("mesh.worldtree.network");
            let result = gate.submit(&envelope, &csr, now, &BTreeSet::new(), &BTreeSet::new());
            assert_eq!(result, Err(DnsChallengeError::Authorization(error)));
            assert!(gate.writer().records().is_empty());
            assert!(gate.seen_nonces().is_empty());
            assert!(gate.retained_owner_pubkeys().is_empty());
            assert_eq!(gate.decisions().len(), 1);
            assert_eq!(gate.decisions()[0].outcome, result);
            // A refusal has not burned this authorization's nonce.
            assert!(
                gate.submit(
                    &signed(&auth),
                    &CSR,
                    NOW,
                    &BTreeSet::new(),
                    &BTreeSet::new()
                )
                .is_ok()
            );
        }
    }

    #[test]
    fn https_dns_zone_checks_whole_labels_and_rejects_apex() {
        let auth = authorization("mesh.worldtree.network", 'a');
        for zone in ["other.example", "esh.worldtree.network", &auth.fqdn] {
            let mut gate = gate(zone);
            let result = gate.submit(
                &signed(&auth),
                &CSR,
                NOW,
                &BTreeSet::new(),
                &BTreeSet::new(),
            );
            assert_eq!(result, Err(DnsChallengeError::OutsideZone));
            assert!(gate.writer().records().is_empty());
            assert!(gate.seen_nonces().is_empty());
            assert!(gate.retained_owner_pubkeys().is_empty());
            assert_eq!(gate.decisions()[0].outcome, result);
        }
    }

    #[test]
    fn https_dns_rejects_invalid_zone_configuration() {
        for zone in [
            "",
            ".",
            "mesh.example.",
            "Mesh.example",
            "mêsh.example",
            "a..example",
            "-a.example",
            "a-.example",
            "*.example",
            "a/b",
            &"a".repeat(64),
        ] {
            assert!(matches!(
                DnsChallengeGate::new(zone, InMemoryTxtWriter::default()),
                Err(DnsChallengeError::InvalidZone)
            ));
        }
    }

    #[test]
    fn https_dns_rejects_oversized_challenge_name() {
        let zone = format!(
            "{}.{}.{}.{}.example",
            "a".repeat(63),
            "b".repeat(63),
            "c".repeat(63),
            "d".repeat(20)
        );
        let auth = authorization(&zone, 'a');
        assert!(auth.fqdn.len() <= 253);
        let mut gate = gate(&zone);
        assert_eq!(
            gate.submit(
                &signed(&auth),
                &CSR,
                NOW,
                &BTreeSet::new(),
                &BTreeSet::new()
            ),
            Err(DnsChallengeError::ChallengeNameTooLong)
        );
        assert!(gate.writer().records().is_empty());
        assert!(gate.seen_nonces().is_empty());
        assert_eq!(gate.decisions().len(), 1);
    }

    #[test]
    fn https_dns_retains_full_owner_history_without_pruning() {
        let auth = authorization("mesh.worldtree.network", 'a');
        let historical_owner = SigningKey::from_bytes(&[8; 32]).verifying_key().to_bytes();
        let history = BTreeSet::from([historical_owner]);
        let mut gate = gate("mesh.worldtree.network");
        assert!(
            gate.submit(&signed(&auth), &CSR, NOW, &BTreeSet::new(), &history)
                .is_ok()
        );
        // Empty later snapshots must not erase tombstoned or expired owners.
        assert!(
            gate.submit("malformed", &CSR, NOW, &BTreeSet::new(), &BTreeSet::new())
                .is_err()
        );
        assert_eq!(
            gate.retained_owner_pubkeys(),
            &BTreeSet::from([historical_owner, owner().verifying_key().to_bytes()])
        );
    }

    #[test]
    fn https_dns_collision_refusal_never_reaches_writer() {
        // Real 80-bit collisions are infeasible fixtures. The verifier tests
        // collision detection with an injected labeler; here inject its verdict
        // at the private publication boundary, never through a public bypass.
        let mut gate = gate("mesh.worldtree.network");
        let result =
            gate.publish_and_log("collision fixture", NOW, Err(IssuanceError::LabelCollision));
        assert_eq!(
            result,
            Err(DnsChallengeError::Authorization(
                IssuanceError::LabelCollision
            ))
        );
        assert!(gate.writer().records().is_empty());
        assert!(gate.seen_nonces().is_empty());
        assert_eq!(gate.decisions()[0].outcome, result);
    }

    #[test]
    fn https_dns_writer_failure_is_logged_and_nonce_cannot_be_republished() {
        #[derive(Default)]
        struct FailingWriter(usize);
        impl ChallengeTxtWriter for FailingWriter {
            type Error = &'static str;

            fn write_txt(&mut self, _: &ChallengeTxtRecord) -> Result<(), Self::Error> {
                self.0 += 1;
                Err("publication uncertain")
            }
        }
        let mut gate =
            DnsChallengeGate::new("mesh.worldtree.network", FailingWriter::default()).unwrap();
        let auth = authorization("mesh.worldtree.network", 'a');
        let envelope = signed(&auth);
        let result = gate.submit(&envelope, &CSR, NOW, &BTreeSet::new(), &BTreeSet::new());
        assert_eq!(
            result,
            Err(DnsChallengeError::Writer(
                "publication uncertain".to_owned()
            ))
        );
        assert_eq!(gate.decisions()[0].outcome, result);
        assert!(
            gate.retained_owner_pubkeys()
                .contains(&owner().verifying_key().to_bytes())
        );
        assert_eq!(
            gate.submit(&envelope, &CSR, NOW, &BTreeSet::new(), &BTreeSet::new()),
            Err(DnsChallengeError::Authorization(
                IssuanceError::ReplayedNonce
            ))
        );
        assert_eq!(gate.writer().0, 1);
        assert_eq!(gate.decisions().len(), 2);
    }
}
