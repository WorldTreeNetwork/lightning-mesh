//! Authorization-engine vectors. Fixtures are local, not live recovery proof.

use biscuit_auth::macros::biscuit;
use biscuit_auth::{Biscuit, KeyPair, PublicKey};
use ed25519_dalek::{Signer, SigningKey};
use mjolnir_admin_capabilities::{
    AdminRequest, AuthzError, GrantClass, HighWaterFloor, HolderAlgorithm, HolderKey, Operation,
    SecurityProfile, TimeEvidence, TokenIdentity, VerifiedAuthority, authorize,
    verify_holder_proof,
};

fn b32(v: u8) -> [u8; 32] {
    [v; 32]
}

fn holder() -> (SigningKey, HolderKey) {
    let sk = SigningKey::from_bytes(&[7u8; 32]);
    let hk = HolderKey::new(HolderAlgorithm::Ed25519, sk.verifying_key().to_bytes());
    (sk, hk)
}

fn hex_key(hk: &HolderKey) -> String {
    hk.key_bytes().iter().map(|b| format!("{b:02x}")).collect()
}

fn mint(
    root: &KeyPair,
    holder: &HolderKey,
    right: &str,
    issued_at: i64,
    expires_at: i64,
) -> Vec<u8> {
    let h = hex_key(holder);
    let token = biscuit!(
        r#"
        right({right});
        holder({h});
        issued_at({issued_at});
        expires_at({expires_at});
        "#
    )
    .build(root)
    .expect("mint");
    token.to_vec().expect("serialize")
}

fn proof_for(
    op: Operation,
    raw: &[u8],
    sk: &SigningKey,
    hk: &HolderKey,
    epoch: u64,
) -> (Vec<u8>, mjolnir_admin_capabilities::VerifiedHolderProof) {
    let request = AdminRequest::new(
        b32(0x11),
        b32(0x22),
        epoch,
        op,
        TokenIdentity::from_raw_token(raw).unwrap(),
        b32(0x44),
        b32(0x55),
    )
    .unwrap();
    let canonical = request.encode().unwrap();
    let sig = sk.sign(&canonical);
    let proof = verify_holder_proof(&canonical, raw, hk, &sig.to_bytes()).unwrap();
    (canonical, proof)
}

fn authority(issuer: PublicKey, class: GrantClass, profile: SecurityProfile) -> VerifiedAuthority {
    VerifiedAuthority {
        issuer,
        house_id: b32(0x11),
        node_id: b32(0x22),
        authority_epoch: 1,
        profile,
        grant_class: class,
        revoked_lineage: Vec::new(),
        issuer_revoked: false,
    }
}

fn time(now: u64, verified_at: u64) -> TimeEvidence {
    TimeEvidence::authenticated(now, verified_at).unwrap()
}

#[test]
fn attenuated_diagnostics_succeeds() {
    let root = KeyPair::new();
    let (sk, hk) = holder();
    let raw = mint(&root, &hk, "diagnostics_status", 1000, 100_000);
    let (_, proof) = proof_for(Operation::DiagnosticsStatus, &raw, &sk, &hk, 1);
    let auth = authority(
        root.public(),
        GrantClass::DiagnosticsRead,
        SecurityProfile::Standard,
    );
    let mut floor = HighWaterFloor::default();
    let grant = authorize(&proof, &raw, &hk, &auth, time(5_000, 4_000), &mut floor).unwrap();
    assert!(grant.effective_expiry.is_none());
    assert_eq!(floor.get(), 5_000);
}

#[test]
fn wrong_issuer_fails() {
    let root = KeyPair::new();
    let other = KeyPair::new();
    let (sk, hk) = holder();
    let raw = mint(&root, &hk, "diagnostics_status", 1000, 100_000);
    let (_, proof) = proof_for(Operation::DiagnosticsStatus, &raw, &sk, &hk, 1);
    let auth = authority(
        other.public(),
        GrantClass::DiagnosticsRead,
        SecurityProfile::Standard,
    );
    let mut floor = HighWaterFloor::default();
    let err = authorize(&proof, &raw, &hk, &auth, time(5_000, 4_000), &mut floor).unwrap_err();
    assert!(matches!(err, AuthzError::Token(_)));
}

#[test]
fn expired_with_fresh_authority_refuses() {
    let root = KeyPair::new();
    let (sk, hk) = holder();
    let raw = mint(&root, &hk, "apply_network_plan", 1000, 2000);
    let (_, proof) = proof_for(
        Operation::ApplyNetworkPlan {
            plan_id: b32(1),
            expected_revision: b32(2),
            plan_digest: b32(3),
        },
        &raw,
        &sk,
        &hk,
        1,
    );
    let auth = authority(
        root.public(),
        GrantClass::NetworkRadio,
        SecurityProfile::Standard,
    );
    let mut floor = HighWaterFloor::default();
    let err = authorize(&proof, &raw, &hk, &auth, time(10_000, 9_900), &mut floor).unwrap_err();
    assert_eq!(err, AuthzError::Expired);
}

#[test]
fn unexpired_but_stale_authority_refuses() {
    let root = KeyPair::new();
    let (sk, hk) = holder();
    let raw = mint(&root, &hk, "apply_network_plan", 1000, 10_000_000);
    let (_, proof) = proof_for(
        Operation::ApplyNetworkPlan {
            plan_id: b32(1),
            expected_revision: b32(2),
            plan_digest: b32(3),
        },
        &raw,
        &sk,
        &hk,
        1,
    );
    let auth = authority(
        root.public(),
        GrantClass::NetworkRadio,
        SecurityProfile::Standard,
    );
    let mut floor = HighWaterFloor::default();
    // Standard stale max = 24h. now - verified = 24h+1
    let err = authorize(
        &proof,
        &raw,
        &hk,
        &auth,
        time(1000 + 24 * 3600 + 1, 1000),
        &mut floor,
    )
    .unwrap_err();
    assert_eq!(err, AuthzError::StaleAuthority);
}

#[test]
fn strict_uplink_past_one_hour_refuses_even_if_relaxed_would_allow() {
    let root = KeyPair::new();
    let (sk, hk) = holder();
    let raw = mint(&root, &hk, "apply_network_plan", 1000, 1000 + 7 * 24 * 3600);
    let (_, proof) = proof_for(
        Operation::ApplyNetworkPlan {
            plan_id: b32(1),
            expected_revision: b32(2),
            plan_digest: b32(3),
        },
        &raw,
        &sk,
        &hk,
        1,
    );
    let auth = authority(
        root.public(),
        GrantClass::NetworkRadio,
        SecurityProfile::Strict,
    );
    let mut floor = HighWaterFloor::default();
    let err = authorize(
        &proof,
        &raw,
        &hk,
        &auth,
        time(1000 + 3600, 1000 + 3500),
        &mut floor,
    )
    .unwrap_err();
    assert_eq!(err, AuthzError::Expired);
}

#[test]
fn unlimited_diagnostic_used_for_apply_refuses() {
    let root = KeyPair::new();
    let (sk, hk) = holder();
    let raw = mint(&root, &hk, "diagnostics_status", 1000, 100_000);
    let (_, proof) = proof_for(
        Operation::ApplyNetworkPlan {
            plan_id: b32(1),
            expected_revision: b32(2),
            plan_digest: b32(3),
        },
        &raw,
        &sk,
        &hk,
        1,
    );
    let auth = authority(
        root.public(),
        GrantClass::DiagnosticsRead,
        SecurityProfile::Standard,
    );
    let mut floor = HighWaterFloor::default();
    let err = authorize(&proof, &raw, &hk, &auth, time(5_000, 4_000), &mut floor).unwrap_err();
    assert_eq!(err, AuthzError::ClassDenied);
}

#[test]
fn token_asserted_time_fact_refuses() {
    let root = KeyPair::new();
    let (sk, hk) = holder();
    let h = hex_key(&hk);
    let token = biscuit!(
        r#"
        right("diagnostics_status");
        holder({h});
        issued_at(1000);
        expires_at(100000);
        time(1);
        "#
    )
    .build(&root)
    .unwrap();
    let raw = token.to_vec().unwrap();
    let (_, proof) = proof_for(Operation::DiagnosticsStatus, &raw, &sk, &hk, 1);
    let auth = authority(
        root.public(),
        GrantClass::DiagnosticsRead,
        SecurityProfile::Standard,
    );
    let mut floor = HighWaterFloor::default();
    let err = authorize(&proof, &raw, &hk, &auth, time(5_000, 4_000), &mut floor).unwrap_err();
    assert!(matches!(err, AuthzError::ReservedSymbol("time")));
}

#[test]
fn clock_rollback_after_accept_refuses() {
    let root = KeyPair::new();
    let (sk, hk) = holder();
    let raw = mint(&root, &hk, "diagnostics_status", 1000, 100_000);
    let (_, proof) = proof_for(Operation::DiagnosticsStatus, &raw, &sk, &hk, 1);
    let auth = authority(
        root.public(),
        GrantClass::DiagnosticsRead,
        SecurityProfile::Standard,
    );
    let mut floor = HighWaterFloor::default();
    authorize(&proof, &raw, &hk, &auth, time(5_000, 4_000), &mut floor).unwrap();
    let err = authorize(&proof, &raw, &hk, &auth, time(4_999, 4_000), &mut floor).unwrap_err();
    assert_eq!(err, AuthzError::RegressedTime);
}

#[test]
fn partition_stale_relaxed_refuses() {
    let root = KeyPair::new();
    let (sk, hk) = holder();
    let raw = mint(&root, &hk, "apply_network_plan", 1000, 10_000_000);
    let (_, proof) = proof_for(
        Operation::ApplyNetworkPlan {
            plan_id: b32(1),
            expected_revision: b32(2),
            plan_digest: b32(3),
        },
        &raw,
        &sk,
        &hk,
        1,
    );
    let mut auth = authority(
        root.public(),
        GrantClass::NetworkRadio,
        SecurityProfile::Relaxed,
    );
    auth.issuer_revoked = true;
    let mut floor = HighWaterFloor::default();
    let err = authorize(
        &proof,
        &raw,
        &hk,
        &auth,
        time(1000 + 7 * 24 * 3600 + 1, 1000),
        &mut floor,
    )
    .unwrap_err();
    assert_eq!(err, AuthzError::StaleAuthority);
}

#[test]
fn fd_positive_accepts_inside_stale_window() {
    let root = KeyPair::new();
    let (sk, hk) = holder();
    let raw = mint(&root, &hk, "apply_network_plan", 1000, 10_000_000);
    let (_, proof) = proof_for(
        Operation::ApplyNetworkPlan {
            plan_id: b32(1),
            expected_revision: b32(2),
            plan_digest: b32(3),
        },
        &raw,
        &sk,
        &hk,
        1,
    );
    let mut auth = authority(
        root.public(),
        GrantClass::NetworkRadio,
        SecurityProfile::Standard,
    );
    auth.issuer_revoked = true;
    let mut floor = HighWaterFloor::default();
    // 1h since last verify, Standard window 24h
    authorize(
        &proof,
        &raw,
        &hk,
        &auth,
        time(1000 + 3600, 1000),
        &mut floor,
    )
    .unwrap();
}

#[test]
fn missing_time_facts_refuse() {
    let root = KeyPair::new();
    let (sk, hk) = holder();
    let h = hex_key(&hk);
    let token = biscuit!(
        r#"
        right("diagnostics_status");
        holder({h});
        "#
    )
    .build(&root)
    .unwrap();
    let raw = token.to_vec().unwrap();
    let (_, proof) = proof_for(Operation::DiagnosticsStatus, &raw, &sk, &hk, 1);
    let auth = authority(
        root.public(),
        GrantClass::DiagnosticsRead,
        SecurityProfile::Standard,
    );
    let mut floor = HighWaterFloor::default();
    let err = authorize(&proof, &raw, &hk, &auth, time(5_000, 4_000), &mut floor).unwrap_err();
    assert_eq!(err, AuthzError::MissingIssuanceTimes);
}

#[test]
fn no_expiry_diagnostic_past_stale_refuses() {
    let root = KeyPair::new();
    let (sk, hk) = holder();
    let raw = mint(&root, &hk, "diagnostics_status", 1000, 100_000);
    let (_, proof) = proof_for(Operation::DiagnosticsStatus, &raw, &sk, &hk, 1);
    let auth = authority(
        root.public(),
        GrantClass::DiagnosticsRead,
        SecurityProfile::Standard,
    );
    let mut floor = HighWaterFloor::default();
    let err = authorize(
        &proof,
        &raw,
        &hk,
        &auth,
        time(1000 + 24 * 3600 + 1, 1000),
        &mut floor,
    )
    .unwrap_err();
    assert_eq!(err, AuthzError::StaleAuthority);
}

#[test]
fn changed_holder_refuses() {
    let root = KeyPair::new();
    let (_sk, hk) = holder();
    let other_sk = SigningKey::from_bytes(&[9u8; 32]);
    let other = HolderKey::new(
        HolderAlgorithm::Ed25519,
        other_sk.verifying_key().to_bytes(),
    );
    let raw = mint(&root, &hk, "diagnostics_status", 1000, 100_000);
    let request = AdminRequest::new(
        b32(0x11),
        b32(0x22),
        1,
        Operation::DiagnosticsStatus,
        TokenIdentity::from_raw_token(&raw).unwrap(),
        b32(0x44),
        b32(0x55),
    )
    .unwrap();
    let canonical = request.encode().unwrap();
    let sig = other_sk.sign(&canonical);
    let proof = verify_holder_proof(&canonical, &raw, &other, &sig.to_bytes()).unwrap();
    let auth = authority(
        root.public(),
        GrantClass::DiagnosticsRead,
        SecurityProfile::Standard,
    );
    let mut floor = HighWaterFloor::default();
    let err = authorize(&proof, &raw, &other, &auth, time(5_000, 4_000), &mut floor).unwrap_err();
    assert!(matches!(err, AuthzError::PolicyDenied));
}

#[test]
fn biscuit_to_vec_is_not_required_preimage_identity_is_raw_bytes() {
    let root = KeyPair::new();
    let (sk, hk) = holder();
    let raw = mint(&root, &hk, "diagnostics_status", 1000, 100_000);
    let parsed = Biscuit::from(&raw, root.public()).unwrap();
    let reserialized = parsed.to_vec().unwrap();
    // Identity is over exact received bytes; re-serialize may differ.
    let id_raw = TokenIdentity::from_raw_token(&raw).unwrap();
    let id_again = TokenIdentity::from_raw_token(&reserialized).unwrap();
    if id_raw != id_again {
        let (_, proof) = proof_for(Operation::DiagnosticsStatus, &raw, &sk, &hk, 1);
        let auth = authority(
            root.public(),
            GrantClass::DiagnosticsRead,
            SecurityProfile::Standard,
        );
        let mut floor = HighWaterFloor::default();
        let err = authorize(
            &proof,
            &reserialized,
            &hk,
            &auth,
            time(5_000, 4_000),
            &mut floor,
        )
        .unwrap_err();
        assert!(matches!(err, AuthzError::Codec(_)));
    }
}
