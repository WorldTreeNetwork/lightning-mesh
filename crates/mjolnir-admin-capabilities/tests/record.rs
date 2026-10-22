//! Consumer-verifier vectors for owner-signed admin records
//! (`add-control-record-integrity`). Fixtures are local deterministic keys.

use ed25519_dalek::{Signer, SigningKey};
use mjolnir_admin_capabilities::{
    AuthorityAnchor, AuthorityBook, CodecError, Evidence, GrantClass, Ingested, NodeVouched,
    Record, RecordBody, RecordDeny, RecordHeader, RecordType, SecurityProfile, SignedRecord,
    decode_record,
};

const HOUSE: [u8; 32] = [0x11; 32];
const NODE: [u8; 32] = [0x22; 32];
const T0: u64 = 1_800_000_000;
const DAY: u64 = 24 * 3600;
const STALE: u64 = DAY; // Standard profile stale-authority maximum.
const FAR: u64 = i64::MAX as u64;

fn key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn pk(sk: &SigningKey) -> [u8; 32] {
    sk.verifying_key().to_bytes()
}

fn owner() -> SigningKey {
    key(0xa1)
}

fn issuer() -> SigningKey {
    key(0xb2)
}

fn other_issuer() -> SigningKey {
    key(0xc3)
}

fn attacker() -> SigningKey {
    key(0xd4)
}

fn token_root() -> [u8; 32] {
    // Any valid Ed25519 point works as a biscuit root here.
    pk(&key(0xe5))
}

fn header(
    signer: &SigningKey,
    epoch: u64,
    sequence: u64,
    issued_at: u64,
    subject: [u8; 32],
) -> RecordHeader {
    RecordHeader {
        house_id: HOUSE,
        authority_epoch: epoch,
        signer: pk(signer),
        sequence,
        issued_at,
        subject,
    }
}

fn sign(sk: &SigningKey, header: RecordHeader, body: RecordBody) -> SignedRecord {
    let bytes = Record::new(header, body).unwrap().encode().unwrap();
    SignedRecord::new(bytes.clone(), sk.sign(&bytes).to_bytes())
}

fn digest(r: &SignedRecord) -> [u8; 32] {
    decode_record(r.bytes()).unwrap().digest().unwrap()
}

fn sorted(mut keys: Vec<[u8; 32]>) -> Vec<[u8; 32]> {
    keys.sort();
    keys
}

fn head(epoch: u64, issued_at: u64, prev: [u8; 32], issuers: Vec<[u8; 32]>) -> SignedRecord {
    let o = owner();
    sign(
        &o,
        header(&o, epoch, 0, issued_at, prev),
        RecordBody::Head {
            owner: pk(&o),
            profile: SecurityProfile::Standard,
            issuers: sorted(issuers),
            revoked_lineage: Vec::new(),
        },
    )
}

fn genesis(issued_at: u64) -> SignedRecord {
    head(1, issued_at, [0; 32], vec![pk(&issuer())])
}

fn snapshot(
    head: &SignedRecord,
    sequence: u64,
    issued_at: u64,
    issuers: Vec<[u8; 32]>,
) -> SignedRecord {
    let o = owner();
    let epoch = decode_record(head.bytes())
        .unwrap()
        .header()
        .authority_epoch;
    sign(
        &o,
        header(&o, epoch, sequence, issued_at, digest(head)),
        RecordBody::Snapshot {
            issuers: sorted(issuers),
            revoked_lineage: Vec::new(),
        },
    )
}

fn withdrawal(epoch: u64, sequence: u64, issued_at: u64, withdrawn: &SigningKey) -> SignedRecord {
    let o = owner();
    sign(
        &o,
        header(&o, epoch, sequence, issued_at, pk(withdrawn)),
        RecordBody::Withdrawal,
    )
}

fn control(signer: &SigningKey, epoch: u64, sequence: u64, issued_at: u64) -> SignedRecord {
    sign(
        signer,
        header(signer, epoch, sequence, issued_at, NODE),
        RecordBody::Control {
            grant_class: GrantClass::NetworkRadio,
            token_root: token_root(),
            expires_at: FAR,
        },
    )
}

fn book() -> AuthorityBook {
    AuthorityBook::provision(AuthorityAnchor::new(HOUSE, pk(&owner())))
}

/// A consumer that has the genesis head and nothing else.
fn book_with_genesis(at: u64) -> (AuthorityBook, SignedRecord) {
    let mut b = book();
    let h = genesis(at);
    assert_eq!(b.ingest(&h, at), Ok(Ingested::Applied));
    (b, h)
}

// --- Forged records at a consumer that did not ingest them -----------------

#[test]
fn non_ingesting_consumer_rejects_forged_records() {
    let genuine = control(&issuer(), 1, 1, T0);

    // Node A ingested and uses the genuine record.
    let (mut a, _) = book_with_genesis(T0);
    assert_eq!(a.ingest(&genuine, T0), Ok(Ingested::Applied));
    a.verified_authority(&genuine, T0).unwrap();

    // Node B never ingested it. Whatever A or the relay asserts, B verifies
    // the bytes itself.
    let (mut b, _) = book_with_genesis(T0);

    // Tampered body under the genuine signature.
    let mut tampered = genuine.bytes().to_vec();
    // Tail is grant_class (1 byte), token_root (2 + 32), expires_at (1 + 8).
    let at = tampered.len() - 9 - 34 - 1;
    assert_eq!(tampered[at], 1, "fixture: NetworkRadio grant class code");
    tampered[at] = 3; // Ownership
    let tampered = SignedRecord::new(tampered, *genuine.signature());
    assert_eq!(
        b.verified_authority(&tampered, T0).unwrap_err(),
        RecordDeny::Forged
    );

    // Same bytes, attacker's signature.
    let resigned = SignedRecord::new(
        genuine.bytes().to_vec(),
        attacker().sign(genuine.bytes()).to_bytes(),
    );
    assert_eq!(
        b.verified_authority(&resigned, T0).unwrap_err(),
        RecordDeny::Forged
    );

    // A validly self-signed record from a key the house never approved.
    let self_signed = control(&attacker(), 1, 1, T0);
    assert_eq!(
        b.verified_authority(&self_signed, T0).unwrap_err(),
        RecordDeny::Forged
    );
    assert_eq!(b.ingest(&self_signed, T0).unwrap_err(), RecordDeny::Forged);

    // A record for another house, correctly signed by the issuer.
    let mut h = header(&issuer(), 1, 1, T0, NODE);
    h.house_id = [0x99; 32];
    let other_house = sign(
        &issuer(),
        h,
        RecordBody::Control {
            grant_class: GrantClass::NetworkRadio,
            token_root: token_root(),
            expires_at: FAR,
        },
    );
    assert_eq!(
        b.verified_authority(&other_house, T0).unwrap_err(),
        RecordDeny::Forged
    );

    // A head the anchor owner did not sign.
    let a_key = attacker();
    let fake_head = sign(
        &a_key,
        header(&a_key, 2, 0, T0, [0; 32]),
        RecordBody::Head {
            owner: pk(&a_key),
            profile: SecurityProfile::Relaxed,
            issuers: vec![pk(&a_key)],
            revoked_lineage: Vec::new(),
        },
    );
    assert_eq!(b.ingest(&fake_head, T0).unwrap_err(), RecordDeny::Forged);
    assert_eq!(b.current_epoch(), Ok(Some(1)));

    // None of the rejections changed B's state; the genuine record still works.
    b.verified_authority(&genuine, T0).unwrap();
}

#[test]
fn divergent_encoding_fails_verification() {
    // Same fields, sequence written as a non-minimal 8-byte integer by a
    // second canonicalizer, and signed by the real issuer over those bytes.
    let canonical = Record::new(
        header(&issuer(), 1, 1, T0, NODE),
        RecordBody::Control {
            grant_class: GrantClass::NetworkRadio,
            token_root: token_root(),
            expires_at: FAR,
        },
    )
    .unwrap()
    .encode()
    .unwrap();
    let mut divergent = Vec::new();
    // Array header, domain (2-byte text header), version, house bstr32,
    // epoch 1, signer bstr32.
    let prefix = 1 + 2 + "lightning-admin/record/control".len() + 1 + 34 + 1 + 34;
    divergent.extend_from_slice(&canonical[..prefix]);
    assert_eq!(canonical[prefix], 0x01, "fixture: sequence 1 is one byte");
    divergent.extend_from_slice(&[0x1b, 0, 0, 0, 0, 0, 0, 0, 1]);
    divergent.extend_from_slice(&canonical[prefix + 1..]);
    assert_eq!(
        decode_record(&divergent).unwrap_err(),
        CodecError::NonCanonical
    );

    let signed = SignedRecord::new(divergent.clone(), issuer().sign(&divergent).to_bytes());
    let (mut b, _) = book_with_genesis(T0);
    assert_eq!(
        b.verified_authority(&signed, T0).unwrap_err(),
        RecordDeny::Forged
    );
}

#[test]
fn record_domains_are_per_type_version_one() {
    for (ty, name) in [
        (RecordType::Head, "head"),
        (RecordType::Snapshot, "snapshot"),
        (RecordType::Withdrawal, "withdrawal"),
        (RecordType::Control, "control"),
    ] {
        assert_eq!(ty.domain(), format!("lightning-admin/record/{name}"));
    }
    // A control record relabelled with another record type's domain does not
    // decode, so its signature can never be reused across types.
    let bytes = control(&issuer(), 1, 1, T0).bytes().to_vec();
    let from = RecordType::Control.domain();
    assert_eq!(&bytes[1..3], &[0x78, from.len() as u8]);
    for to in [
        RecordType::Head,
        RecordType::Snapshot,
        RecordType::Withdrawal,
    ] {
        let to = to.domain();
        let mut relabelled = vec![bytes[0], 0x78, to.len() as u8];
        relabelled.extend_from_slice(to.as_bytes());
        relabelled.extend_from_slice(&bytes[3 + from.len()..]);
        assert!(decode_record(&relabelled).is_err(), "{to}");
    }
}

// --- Re-gossip and replay ------------------------------------------------

#[test]
fn regossip_of_applied_record_is_a_no_op() {
    let (mut b, h) = book_with_genesis(T0);
    let s = snapshot(&h, 1, T0 + 100, vec![pk(&issuer())]);
    let c = control(&issuer(), 1, 1, T0 + 100);
    assert_eq!(b.ingest(&s, T0 + 100), Ok(Ingested::Applied));
    assert_eq!(b.ingest(&c, T0 + 100), Ok(Ingested::Applied));
    let stored = b.to_bytes().unwrap();
    assert_eq!(b.freshness(), Ok(Some(T0 + 100)));

    // Re-gossip hours later, of every applied record.
    for r in [&h, &s, &c] {
        assert_eq!(b.ingest(r, T0 + 7200), Ok(Ingested::AlreadyApplied));
    }
    assert_eq!(
        b.freshness(),
        Ok(Some(T0 + 100)),
        "re-gossip must not advance freshness"
    );
    assert_eq!(
        b.to_bytes().unwrap(),
        stored,
        "re-gossip must not change state"
    );

    // Applied state was not demoted.
    let v = b.verified_authority(&c, T0 + 7200).unwrap();
    assert_eq!(v.authority_verified_at(), T0 + 100);
}

#[test]
fn older_sequence_for_a_scope_is_replay() {
    let (mut b, h) = book_with_genesis(T0);
    let c1 = control(&issuer(), 1, 1, T0 + 10);
    let c2 = control(&issuer(), 1, 2, T0 + 20);
    let s2 = snapshot(&h, 2, T0 + 30, vec![pk(&issuer())]);
    let s1 = snapshot(&h, 1, T0 + 25, vec![pk(&issuer())]);
    b.ingest(&c2, T0 + 30).unwrap();
    b.ingest(&s2, T0 + 30).unwrap();

    // Never-seen older sequence: refused, not a new decision.
    assert_eq!(b.ingest(&c1, T0 + 40).unwrap_err(), RecordDeny::Replayed);
    assert_eq!(
        b.verified_authority(&c1, T0 + 40).unwrap_err(),
        RecordDeny::Replayed
    );
    // Older snapshot: refused, freshness does not move.
    assert_eq!(b.ingest(&s1, T0 + 40).unwrap_err(), RecordDeny::Replayed);
    assert_eq!(b.freshness(), Ok(Some(T0 + 30)));
    b.verified_authority(&c2, T0 + 40).unwrap();

    // Superseded after apply: the older record no longer authorizes.
    let c3 = control(&issuer(), 1, 3, T0 + 50);
    b.ingest(&c3, T0 + 50).unwrap();
    assert_eq!(
        b.verified_authority(&c2, T0 + 60).unwrap_err(),
        RecordDeny::Replayed
    );
    b.verified_authority(&c3, T0 + 60).unwrap();
}

#[test]
fn clock_rollback_does_not_revive_a_record() {
    let (mut b, h) = book_with_genesis(T0);
    let c = control(&issuer(), 1, 1, T0);
    b.ingest(&c, T0).unwrap();
    let s = snapshot(&h, 1, T0 + 2 * STALE, vec![pk(&issuer())]);
    b.ingest(&s, T0 + 2 * STALE).unwrap();
    // "now" moved back to inside the record's original window: the current
    // authority state is now signed in the future, so nothing authorizes.
    assert_eq!(
        b.verified_authority(&c, T0 + 10).unwrap_err(),
        RecordDeny::Stale
    );
    // A record signed ahead of authenticated now is refused.
    let future = control(&issuer(), 1, 2, T0 + 3 * STALE);
    assert_eq!(
        b.ingest(&future, T0 + 2 * STALE).unwrap_err(),
        RecordDeny::Forged
    );
}

#[test]
fn replaying_newest_record_after_stale_maximum_does_not_restore_privilege() {
    let (mut b, h) = book_with_genesis(T0);
    let s = snapshot(&h, 1, T0 + 60, vec![pk(&issuer())]);
    let c = control(&issuer(), 1, 1, T0 + 60);
    b.ingest(&s, T0 + 60).unwrap();
    b.verified_authority(&c, T0 + 60).unwrap();

    let late = T0 + 60 + STALE + 1;
    // A peer replays the newest authority-state record and the newest control.
    assert_eq!(b.ingest(&s, late), Ok(Ingested::AlreadyApplied));
    assert_eq!(b.ingest(&h, late), Ok(Ingested::AlreadyApplied));
    assert_eq!(b.freshness(), Ok(Some(T0 + 60)));
    assert_eq!(
        b.verified_authority(&c, late).unwrap_err(),
        RecordDeny::Stale
    );

    // The same replay at a cold consumer does not restore privilege either.
    let mut cold = book();
    assert_eq!(cold.ingest(&h, late), Ok(Ingested::Applied));
    assert_eq!(cold.ingest(&s, late), Ok(Ingested::Applied));
    assert_eq!(
        cold.verified_authority(&c, late).unwrap_err(),
        RecordDeny::Stale
    );
}

// --- Freshness advances only from owner-signed authority state ------------

#[test]
fn withdrawn_issuers_newer_records_do_not_keep_privilege_alive() {
    // Partitioned with the withdrawn issuer; this node never saw the
    // withdrawal. The issuer keeps signing fresh control records, and the
    // owner's unrelated withdrawal of someone else still flows.
    let mut b = book();
    let h = head(1, T0, [0; 32], vec![pk(&issuer()), pk(&other_issuer())]);
    b.ingest(&h, T0).unwrap();
    let mut last = None;
    for (i, at) in [T0 + 3600, T0 + STALE - 1, T0 + STALE + 1, T0 + 3 * STALE]
        .into_iter()
        .enumerate()
    {
        let c = control(&issuer(), 1, i as u64 + 1, at);
        let owner_control = control(&owner(), 1, i as u64 + 1, at);
        b.ingest(&c, at).unwrap();
        b.ingest(&owner_control, at).unwrap();
        assert_eq!(
            b.freshness(),
            Ok(Some(T0)),
            "non-authority-state record advanced freshness"
        );
        last = Some((c, owner_control, at));
    }
    let w_other = withdrawal(1, 1, T0 + 3 * STALE, &other_issuer());
    b.ingest(&w_other, T0 + 3 * STALE).unwrap();
    assert_eq!(
        b.freshness(),
        Ok(Some(T0)),
        "a withdrawal advanced freshness"
    );

    let (c, owner_control, at) = last.unwrap();
    assert_eq!(b.verified_authority(&c, at).unwrap_err(), RecordDeny::Stale);
    assert_eq!(
        b.verified_authority(&owner_control, at).unwrap_err(),
        RecordDeny::Stale
    );

    // Only a newer owner-signed authority-state record advances freshness.
    let s = snapshot(&h, 1, at, vec![pk(&issuer())]);
    b.ingest(&s, at).unwrap();
    assert_eq!(b.freshness(), Ok(Some(at)));
    b.verified_authority(&c, at).unwrap();
}

#[test]
fn non_owner_cannot_sign_authority_state() {
    let (mut b, h) = book_with_genesis(T0);
    let i = issuer();
    let fake_snapshot = sign(
        &i,
        header(&i, 1, 1, T0 + 100, digest(&h)),
        RecordBody::Snapshot {
            issuers: vec![pk(&i)],
            revoked_lineage: Vec::new(),
        },
    );
    assert_eq!(
        b.ingest(&fake_snapshot, T0 + 100).unwrap_err(),
        RecordDeny::Forged
    );
    let fake_successor = sign(
        &i,
        header(&i, 2, 0, T0 + 100, digest(&h)),
        RecordBody::Head {
            owner: pk(&i),
            profile: SecurityProfile::Relaxed,
            issuers: vec![pk(&i)],
            revoked_lineage: Vec::new(),
        },
    );
    assert_eq!(
        b.ingest(&fake_successor, T0 + 100).unwrap_err(),
        RecordDeny::Forged
    );
    assert_eq!(b.freshness(), Ok(Some(T0)));
}

// --- Withdrawal --------------------------------------------------------------

#[test]
fn old_withdrawal_still_removes_the_issuer() {
    // The owner signed the withdrawal at T0+60 under a 5-minute ownership
    // authorization. This node sees it after those 5 minutes and after the
    // stale maximum.
    let (mut b, _) = book_with_genesis(T0);
    let c = control(&issuer(), 1, 1, T0 + 30);
    b.ingest(&c, T0 + 30).unwrap();
    b.verified_authority(&c, T0 + 30).unwrap();

    let w = withdrawal(1, 1, T0 + 60, &issuer());
    let seen = T0 + 60 + 5 * 60 + STALE + 1;
    assert_eq!(
        b.ingest(&w, seen),
        Ok(Ingested::Applied),
        "withdrawal refused for age"
    );

    // The issuer is removed, by name, not merely stale.
    assert_eq!(
        b.verified_authority(&c, seen).unwrap_err(),
        RecordDeny::WithdrawnIssuer
    );
    let newer = control(&issuer(), 1, 2, seen);
    assert_eq!(
        b.ingest(&newer, seen).unwrap_err(),
        RecordDeny::WithdrawnIssuer
    );

    // It survives persistence.
    let restored = AuthorityBook::restore(*b.anchor(), Some(&b.to_bytes().unwrap()));
    let mut restored = restored;
    assert_eq!(
        restored.verified_authority(&c, seen).unwrap_err(),
        RecordDeny::WithdrawnIssuer
    );
}

#[test]
fn withdrawal_covers_unexpired_grants_and_earlier_records() {
    let (mut b, h) = book_with_genesis(T0);
    let s = snapshot(&h, 1, T0 + 10, vec![pk(&issuer())]);
    b.ingest(&s, T0 + 10).unwrap();
    // Signed before the withdrawal, still unexpired.
    let c = control(&issuer(), 1, 1, T0 + 20);
    b.ingest(&c, T0 + 20).unwrap();
    b.ingest(&withdrawal(1, 1, T0 + 30, &issuer()), T0 + 30)
        .unwrap();
    assert_eq!(
        b.verified_authority(&c, T0 + 40).unwrap_err(),
        RecordDeny::WithdrawnIssuer
    );
    // The current snapshot still lists the issuer, but it predates the
    // withdrawal, so it does not lift it.
    assert_eq!(b.freshness(), Ok(Some(T0 + 10)));
}

#[test]
fn only_the_owner_withdraws() {
    let mut b = book();
    let h = head(1, T0, [0; 32], vec![pk(&issuer()), pk(&other_issuer())]);
    b.ingest(&h, T0).unwrap();
    let c = control(&issuer(), 1, 1, T0);
    b.ingest(&c, T0).unwrap();
    let peer = other_issuer();
    let by_peer = sign(
        &peer,
        header(&peer, 1, 1, T0 + 10, pk(&issuer())),
        RecordBody::Withdrawal,
    );
    assert_eq!(b.ingest(&by_peer, T0 + 10).unwrap_err(), RecordDeny::Forged);
    b.verified_authority(&c, T0 + 20).unwrap();
}

// --- Fork -----------------------------------------------------------------

#[test]
fn forked_heads_refuse_until_a_higher_epoch_successor() {
    let (mut b, g) = book_with_genesis(T0);
    let a = head(2, T0 + 10, digest(&g), vec![pk(&issuer())]);
    let z = head(
        2,
        T0 + 20,
        digest(&g),
        vec![pk(&issuer()), pk(&other_issuer())],
    );
    b.ingest(&a, T0 + 30).unwrap();
    b.ingest(&z, T0 + 30).unwrap();
    assert_eq!(b.current_head().unwrap_err(), RecordDeny::ForkedHead);

    // Neither sibling, nor the common ancestor's epoch, authorizes.
    for c in [
        control(&issuer(), 2, 1, T0 + 30),
        control(&issuer(), 1, 1, T0 + 30),
    ] {
        assert!(matches!(
            b.verified_authority(&c, T0 + 30).unwrap_err(),
            RecordDeny::ForkedHead
        ));
    }
    // Snapshots cannot advance freshness on either branch during the fork.
    assert_eq!(
        b.ingest(&snapshot(&a, 1, T0 + 40, vec![]), T0 + 40)
            .unwrap_err(),
        RecordDeny::ForkedHead
    );

    // A higher-epoch successor naming one branch supersedes both siblings.
    let next = head(3, T0 + 50, digest(&z), vec![pk(&other_issuer())]);
    b.ingest(&next, T0 + 50).unwrap();
    assert_eq!(b.current_head(), Ok(Some(digest(&next))));
    let c3 = control(&other_issuer(), 3, 1, T0 + 50);
    b.verified_authority(&c3, T0 + 60).unwrap();
    // Sibling-epoch records are superseded.
    let c2 = control(&other_issuer(), 2, 1, T0 + 50);
    assert_eq!(
        b.verified_authority(&c2, T0 + 60).unwrap_err(),
        RecordDeny::Replayed
    );
}

// --- Freshness state ------------------------------------------------------

#[test]
fn unreadable_freshness_denies_and_is_never_reset() {
    let (mut b, _) = book_with_genesis(T0);
    let c = control(&issuer(), 1, 1, T0);
    b.ingest(&c, T0).unwrap();
    let stored = b.to_bytes().unwrap();

    let anchor = *b.anchor();
    let mut corrupt = stored.clone();
    let last = corrupt.len() - 1;
    corrupt[last] ^= 1; // flips a signature bit
    for state in [
        None,
        Some(&corrupt[..]),
        Some(&stored[..stored.len() - 1]),
        Some(&[][..]),
    ] {
        let mut r = AuthorityBook::restore(anchor, state);
        assert!(!r.is_readable());
        assert_eq!(
            r.verified_authority(&c, T0).unwrap_err(),
            RecordDeny::FreshnessUnreadable
        );
        assert_eq!(
            r.ingest(&c, T0).unwrap_err(),
            RecordDeny::FreshnessUnreadable
        );
        assert_eq!(r.to_bytes().unwrap_err(), RecordDeny::FreshnessUnreadable);
        assert_eq!(r.freshness().unwrap_err(), RecordDeny::FreshnessUnreadable);
    }
    // Stored state for another anchor is unreadable here.
    let elsewhere = AuthorityAnchor::new([0x77; 32], pk(&owner()));
    assert!(!AuthorityBook::restore(elsewhere, Some(&stored)).is_readable());

    let mut ok = AuthorityBook::restore(anchor, Some(&stored));
    ok.verified_authority(&c, T0).unwrap();
}

#[test]
fn restore_round_trips_after_snapshot_compaction() {
    // H lists nobody; S1 approves the issuer; the issuer's control record is
    // applied under S1; S2 supersedes S1 and still lists the issuer.
    let mut b = book();
    let h = head(1, T0, [0; 32], vec![]);
    b.ingest(&h, T0).unwrap();
    b.ingest(&snapshot(&h, 1, T0 + 10, vec![pk(&issuer())]), T0 + 10)
        .unwrap();
    let c = control(&issuer(), 1, 1, T0 + 20);
    b.ingest(&c, T0 + 20).unwrap();
    b.ingest(&snapshot(&h, 2, T0 + 30, vec![pk(&issuer())]), T0 + 30)
        .unwrap();
    b.verified_authority(&c, T0 + 40).unwrap();

    let mut restored = AuthorityBook::restore(*b.anchor(), Some(&b.to_bytes().unwrap()));
    assert!(
        restored.is_readable(),
        "legitimately applied state restored as unreadable"
    );
    assert_eq!(restored.freshness(), Ok(Some(T0 + 30)));
    restored.verified_authority(&c, T0 + 40).unwrap();
    assert_eq!(restored.ingest(&c, T0 + 40), Ok(Ingested::AlreadyApplied));
}

#[test]
fn restore_rechecks_signer_at_point_of_use() {
    // Same compaction, but S2 drops the issuer: the restored book is readable
    // and the control record no longer authorizes.
    let mut b = book();
    let h = head(1, T0, [0; 32], vec![]);
    b.ingest(&h, T0).unwrap();
    b.ingest(&snapshot(&h, 1, T0 + 10, vec![pk(&issuer())]), T0 + 10)
        .unwrap();
    let c = control(&issuer(), 1, 1, T0 + 20);
    b.ingest(&c, T0 + 20).unwrap();
    b.ingest(&snapshot(&h, 2, T0 + 30, vec![]), T0 + 30)
        .unwrap();
    assert_eq!(
        b.verified_authority(&c, T0 + 40).unwrap_err(),
        RecordDeny::Forged
    );

    let mut restored = AuthorityBook::restore(*b.anchor(), Some(&b.to_bytes().unwrap()));
    assert!(restored.is_readable());
    assert_eq!(
        restored.verified_authority(&c, T0 + 40).unwrap_err(),
        RecordDeny::Forged
    );
}

// --- Node-vouched facts ---------------------------------------------------

#[test]
fn node_vouched_fact_never_yields_verified_authority() {
    let (mut b, _) = book_with_genesis(T0);
    let c = control(&issuer(), 1, 1, T0);
    b.ingest(&c, T0).unwrap();

    // Even a vouched wrapper around a genuine owner-signed record, delivered
    // by a peer that "already verified" it, is node-vouched.
    let vouched = NodeVouched::new(c.clone(), Some([0x55; 32]));
    assert_eq!(
        b.verified_authority(&vouched, T0).unwrap_err(),
        RecordDeny::NodeVouched
    );
    let address = NodeVouched::new("10.42.7.1", None);
    assert_eq!(
        b.verified_authority(&address, T0).unwrap_err(),
        RecordDeny::NodeVouched
    );
    assert_eq!(
        b.verified_authority(Evidence::NodeVouched { vouched_by: None }, T0)
            .unwrap_err(),
        RecordDeny::NodeVouched
    );
    // Only the verifier's own result exposes authority.
    let v = b.verified_authority(&c, T0).unwrap();
    assert_eq!(v.authority().node_id(), &NODE);
    assert_eq!(v.authority().grant_class(), GrantClass::NetworkRadio);
    assert_eq!(v.time_evidence(T0).unwrap().authority_verified_at(), T0);
}
