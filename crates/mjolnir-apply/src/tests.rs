use super::*;
use std::fs;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

#[derive(Debug)]
struct FakeRuntime {
    boot_id: Mutex<String>,
    monotonic_ms: Mutex<u64>,
    wall_time_ms: Mutex<i64>,
}

impl FakeRuntime {
    fn new() -> Self {
        Self {
            boot_id: Mutex::new("boot-a".into()),
            monotonic_ms: Mutex::new(1_000),
            wall_time_ms: Mutex::new(10_000),
        }
    }

    fn reboot(&self) {
        *self.boot_id.lock().unwrap() = "boot-b".into();
        *self.monotonic_ms.lock().unwrap() = 1;
    }
}

impl Runtime for FakeRuntime {
    fn boot_id(&self) -> Result<String, EngineError> {
        Ok(self.boot_id.lock().unwrap().clone())
    }

    fn monotonic_ms(&self) -> Result<u64, EngineError> {
        Ok(*self.monotonic_ms.lock().unwrap())
    }

    fn wall_time_ms(&self) -> i64 {
        *self.wall_time_ms.lock().unwrap()
    }
}

#[derive(Debug)]
struct OneShotFault {
    point: FaultPoint,
    fired: Mutex<bool>,
}

impl OneShotFault {
    fn new(point: FaultPoint) -> Self {
        Self {
            point,
            fired: Mutex::new(false),
        }
    }
}

impl FaultInjector for OneShotFault {
    fn check(&self, point: FaultPoint) -> Result<(), EngineError> {
        let mut fired = self.fired.lock().unwrap();
        if !*fired && point == self.point {
            *fired = true;
            return Err(EngineError::InjectedCrash(point));
        }
        Ok(())
    }
}

#[derive(Debug)]
struct FakeAdapter {
    config_root: std::path::PathBuf,
    revision: String,
    apply_calls: usize,
    restore_calls: usize,
    fail_apply: bool,
    fail_restore: bool,
    health: Vec<Observation>,
}

impl FakeAdapter {
    fn new(config_root: std::path::PathBuf) -> Self {
        Self {
            config_root,
            revision: "rev-1".into(),
            apply_calls: 0,
            restore_calls: 0,
            fail_apply: false,
            fail_restore: false,
            health: vec![observation(
                ObservationKind::ManagementReachability,
                ObservationStatus::Passed,
                Vantage::ManagementPeer,
            )],
        }
    }
}

impl ApplyAdapter for FakeAdapter {
    fn current_revision(&mut self) -> Result<String, String> {
        Ok(self.revision.clone())
    }

    fn apply(&mut self, plan: &Plan) -> Result<(), String> {
        self.apply_calls += 1;
        fs::write(self.config_root.join("wireless"), b"new-wireless")
            .map_err(|error| error.to_string())?;
        self.revision = plan.proposed_revision.clone();
        if self.fail_apply {
            Err("apply failed after a partial write".into())
        } else {
            Ok(())
        }
    }

    fn check_health(&mut self, _plan: &Plan) -> Result<Vec<Observation>, String> {
        Ok(self.health.clone())
    }

    fn verify_restoration(&mut self, previous_revision: &str) -> Result<Vec<Observation>, String> {
        self.restore_calls += 1;
        if self.fail_restore {
            return Err("management did not recover".into());
        }
        self.revision = previous_revision.into();
        Ok(vec![observation(
            ObservationKind::AppliedRevision,
            ObservationStatus::Passed,
            Vantage::TargetLocal,
        )])
    }
}

struct Fixture {
    _temp: TempDir,
    paths: TxnPaths,
    runtime: Arc<FakeRuntime>,
    adapter: FakeAdapter,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let config_root = temp.path().join("config");
        fs::create_dir_all(&config_root).unwrap();
        for config in UciConfig::ALL {
            fs::write(
                config_root.join(config.file_name()),
                format!("old-{}", config.file_name()),
            )
            .unwrap();
        }
        let paths = TxnPaths::new(temp.path().join("txn"), &config_root);
        let runtime = Arc::new(FakeRuntime::new());
        let adapter = FakeAdapter::new(config_root);
        Self {
            _temp: temp,
            paths,
            runtime,
            adapter,
        }
    }

    fn engine(&self) -> Engine {
        Engine::new(self.paths.clone()).with_runtime(self.runtime.clone())
    }

    fn plan(&self, id: &str) -> Plan {
        Plan {
            schema_version: SCHEMA_VERSION,
            transaction_id: id.into(),
            node_id: "node-a".into(),
            expected_revision: "rev-1".into(),
            proposed_revision: "rev-2".into(),
            resources: vec![UciConfig::Wireless],
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            required_health: vec![ObservationKind::ManagementReachability],
        }
    }

    fn wireless(&self) -> Vec<u8> {
        fs::read(self.paths.config(UciConfig::Wireless)).unwrap()
    }
}

fn observation(kind: ObservationKind, status: ObservationStatus, vantage: Vantage) -> Observation {
    Observation {
        kind,
        status,
        vantage,
        target: "probe.example".into(),
        interface: "eth-test".into(),
        boot_id: "boot-a".into(),
        monotonic_ms: 1_001,
        detail: None,
    }
}

#[test]
fn stale_revision_refuses_before_mutation() {
    let mut fixture = Fixture::new();
    fixture.adapter.revision = "rev-newer".into();
    let error = fixture
        .engine()
        .apply(&fixture.plan("stale"), &mut fixture.adapter)
        .unwrap_err();
    assert!(matches!(error, EngineError::StaleRevision { .. }));
    assert_eq!(fixture.adapter.apply_calls, 0);
    assert_eq!(fixture.wireless(), b"old-wireless");
    assert!(!fixture.paths.journal().exists());
}

#[test]
fn concurrent_apply_is_refused_by_process_lock() {
    let mut fixture = Fixture::new();
    fs::create_dir_all(fixture.paths.root()).unwrap();
    let _held = NodeLock::acquire(&fixture.paths).unwrap();
    let error = fixture
        .engine()
        .apply(&fixture.plan("busy"), &mut fixture.adapter)
        .unwrap_err();
    assert!(matches!(error, EngineError::Busy));
    assert_eq!(fixture.adapter.apply_calls, 0);
}

#[test]
fn duplicate_id_is_idempotent_but_conflicting_plan_is_refused() {
    let mut fixture = Fixture::new();
    let plan = fixture.plan("same-id");
    let first = fixture.engine().apply(&plan, &mut fixture.adapter).unwrap();
    let second = fixture.engine().apply(&plan, &mut fixture.adapter).unwrap();
    assert_eq!(first, second);
    assert_eq!(fixture.adapter.apply_calls, 1);

    let mut conflicting = plan;
    conflicting.proposed_revision = "rev-other".into();
    let error = fixture
        .engine()
        .apply(&conflicting, &mut fixture.adapter)
        .unwrap_err();
    assert!(matches!(error, EngineError::DuplicateIdConflict { .. }));
    assert_eq!(fixture.adapter.apply_calls, 1);
}

#[test]
fn crash_after_every_nonterminal_transition_recovers_snapshot() {
    for state in [
        JournalState::Prepared,
        JournalState::Applying,
        JournalState::Verifying,
    ] {
        let mut fixture = Fixture::new();
        let engine = fixture
            .engine()
            .with_faults(Arc::new(OneShotFault::new(FaultPoint::AfterState(state))));
        let error = engine
            .apply(
                &fixture.plan(&format!("crash-{state:?}")),
                &mut fixture.adapter,
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::InjectedCrash(_)));
        let receipt = fixture
            .engine()
            .recover(&mut fixture.adapter)
            .unwrap()
            .unwrap();
        assert_eq!(receipt.outcome, Outcome::Restored);
        assert_eq!(fixture.wireless(), b"old-wireless");
    }
}

#[test]
fn crash_after_health_pass_but_before_commit_restores() {
    let mut fixture = Fixture::new();
    let engine = fixture
        .engine()
        .with_faults(Arc::new(OneShotFault::new(FaultPoint::AfterHealthPass)));
    let error = engine
        .apply(&fixture.plan("health-gap"), &mut fixture.adapter)
        .unwrap_err();
    assert!(matches!(error, EngineError::InjectedCrash(_)));
    assert_eq!(fixture.wireless(), b"new-wireless");

    let receipt = fixture
        .engine()
        .recover(&mut fixture.adapter)
        .unwrap()
        .unwrap();
    assert_eq!(receipt.outcome, Outcome::Restored);
    assert_eq!(fixture.wireless(), b"old-wireless");
}

#[test]
fn crash_after_committed_transition_preserves_new_configuration() {
    let mut fixture = Fixture::new();
    let engine = fixture
        .engine()
        .with_faults(Arc::new(OneShotFault::new(FaultPoint::AfterState(
            JournalState::Committed,
        ))));
    engine
        .apply(&fixture.plan("commit-gap"), &mut fixture.adapter)
        .unwrap_err();
    let receipt = fixture
        .engine()
        .recover(&mut fixture.adapter)
        .unwrap()
        .unwrap();
    assert_eq!(receipt.outcome, Outcome::Committed);
    assert_eq!(receipt.observations.len(), 1);
    assert_eq!(fixture.wireless(), b"new-wireless");
    assert_eq!(fixture.adapter.restore_calls, 0);
}

#[test]
fn crashes_during_restoring_and_restored_are_restart_safe() {
    for state in [JournalState::Restoring, JournalState::Restored] {
        let mut fixture = Fixture::new();
        fixture.adapter.fail_apply = true;
        let engine = fixture
            .engine()
            .with_faults(Arc::new(OneShotFault::new(FaultPoint::AfterState(state))));
        let error = engine
            .apply(
                &fixture.plan(&format!("restore-{state:?}")),
                &mut fixture.adapter,
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::InjectedCrash(_)));
        fixture.adapter.fail_apply = false;
        let receipt = fixture
            .engine()
            .recover(&mut fixture.adapter)
            .unwrap()
            .unwrap();
        assert_eq!(receipt.outcome, Outcome::Restored);
        assert_eq!(fixture.wireless(), b"old-wireless");
    }
}

#[test]
fn recovery_required_transition_remains_blocked_after_a_crash() {
    let mut fixture = Fixture::new();
    fixture.adapter.fail_apply = true;
    fixture.adapter.fail_restore = true;
    let engine = fixture
        .engine()
        .with_faults(Arc::new(OneShotFault::new(FaultPoint::AfterState(
            JournalState::RecoveryRequired,
        ))));
    let error = engine
        .apply(&fixture.plan("recovery-required-gap"), &mut fixture.adapter)
        .unwrap_err();
    assert!(matches!(error, EngineError::InjectedCrash(_)));
    let error = fixture.engine().recover(&mut fixture.adapter).unwrap_err();
    assert!(matches!(error, EngineError::RecoveryRequired(_)));
}

#[test]
fn partial_snapshot_and_unavailable_storage_never_mutate() {
    let mut fixture = Fixture::new();
    let engine = fixture
        .engine()
        .with_faults(Arc::new(OneShotFault::new(FaultPoint::Snapshot(
            UciConfig::Network,
        ))));
    let error = engine
        .apply(&fixture.plan("partial"), &mut fixture.adapter)
        .unwrap_err();
    assert!(matches!(error, EngineError::InjectedCrash(_)));
    assert_eq!(fixture.adapter.apply_calls, 0);
    assert!(!fixture.paths.journal().exists());
    assert!(
        !fixture.paths.active().exists()
            || fs::read_dir(fixture.paths.active())
                .unwrap()
                .next()
                .is_none()
    );

    let temp = tempfile::tempdir().unwrap();
    let root_is_file = temp.path().join("not-a-directory");
    fs::write(&root_is_file, b"read-only shape").unwrap();
    let paths = TxnPaths::new(&root_is_file, temp.path().join("config"));
    let mut adapter = FakeAdapter::new(temp.path().join("config"));
    let error = Engine::new(paths)
        .with_runtime(Arc::new(FakeRuntime::new()))
        .apply(&fixture.plan("storage"), &mut adapter)
        .unwrap_err();
    assert!(matches!(
        error,
        EngineError::Io(_) | EngineError::Storage(_)
    ));
    assert_eq!(adapter.apply_calls, 0);
}

#[test]
fn corrupt_journal_blocks_recovery_and_new_admission() {
    let mut fixture = Fixture::new();
    let engine = fixture
        .engine()
        .with_faults(Arc::new(OneShotFault::new(FaultPoint::AfterState(
            JournalState::Applying,
        ))));
    engine
        .apply(&fixture.plan("corrupt"), &mut fixture.adapter)
        .unwrap_err();
    fs::write(fixture.paths.journal(), b"{not-json").unwrap();

    let error = fixture.engine().recover(&mut fixture.adapter).unwrap_err();
    assert!(matches!(error, EngineError::RecoveryRequired(_)));
    let error = fixture
        .engine()
        .apply(&fixture.plan("later"), &mut fixture.adapter)
        .unwrap_err();
    assert!(matches!(error, EngineError::RecoveryRequired(_)));
}

#[test]
fn boot_id_change_uses_reboot_recovery_not_wall_clock_expiry() {
    let mut fixture = Fixture::new();
    let engine = fixture
        .engine()
        .with_faults(Arc::new(OneShotFault::new(FaultPoint::AfterState(
            JournalState::Verifying,
        ))));
    engine
        .apply(&fixture.plan("reboot"), &mut fixture.adapter)
        .unwrap_err();
    fixture.runtime.reboot();
    let receipt = fixture
        .engine()
        .recover(&mut fixture.adapter)
        .unwrap()
        .unwrap();
    assert_eq!(receipt.outcome, Outcome::Restored);
    assert_eq!(receipt.recovery_trigger, Some(RecoveryTrigger::Reboot));
}

#[test]
fn missing_backup_and_failed_verification_require_recovery() {
    let mut fixture = Fixture::new();
    let plan = fixture.plan("missing-backup");
    let engine = fixture
        .engine()
        .with_faults(Arc::new(OneShotFault::new(FaultPoint::AfterState(
            JournalState::Verifying,
        ))));
    engine.apply(&plan, &mut fixture.adapter).unwrap_err();
    fs::remove_file(
        fixture
            .paths
            .snapshot(&plan.transaction_id)
            .join("wireless.data"),
    )
    .unwrap();
    let error = fixture.engine().recover(&mut fixture.adapter).unwrap_err();
    assert!(matches!(error, EngineError::RecoveryRequired(_)));
    assert!(fixture.paths.transaction(&plan.transaction_id).exists());
    let receipt = fixture
        .engine()
        .receipt(&plan.transaction_id)
        .unwrap()
        .unwrap();
    assert_eq!(receipt.outcome, Outcome::RecoveryRequired);

    let mut fixture = Fixture::new();
    fixture.adapter.fail_apply = true;
    fixture.adapter.fail_restore = true;
    let plan = fixture.plan("failed-restore");
    let error = fixture
        .engine()
        .apply(&plan, &mut fixture.adapter)
        .unwrap_err();
    assert!(matches!(error, EngineError::RecoveryRequired(_)));
    assert_eq!(
        fixture
            .engine()
            .receipt(&plan.transaction_id)
            .unwrap()
            .unwrap()
            .outcome,
        Outcome::RecoveryRequired
    );
    let error = fixture
        .engine()
        .apply(&fixture.plan("blocked-next"), &mut fixture.adapter)
        .unwrap_err();
    assert!(matches!(error, EngineError::RecoveryRequired(_)));
}

#[test]
fn local_only_probe_cannot_be_recorded_as_client_internet() {
    let mut fixture = Fixture::new();
    fixture.adapter.health = vec![observation(
        ObservationKind::ClientInternet,
        ObservationStatus::Passed,
        Vantage::TargetLocal,
    )];
    let mut plan = fixture.plan("honest-evidence");
    plan.required_health = vec![ObservationKind::ClientInternet];
    let receipt = fixture.engine().apply(&plan, &mut fixture.adapter).unwrap();
    assert_eq!(receipt.outcome, Outcome::Restored);
    assert!(!receipt.observations.iter().any(|observation| {
        observation.kind == ObservationKind::ClientInternet
            && observation.status == ObservationStatus::Passed
    }));
}

#[test]
fn only_allowlisted_files_are_snapshotted_and_missing_files_restore_as_absent() {
    let mut fixture = Fixture::new();
    fs::remove_file(fixture.paths.config(UciConfig::Mjolnir)).unwrap();
    fs::write(
        fixture
            .paths
            .config(UciConfig::Wireless)
            .with_file_name("secret-key"),
        b"never snapshot me",
    )
    .unwrap();
    let engine = fixture
        .engine()
        .with_faults(Arc::new(OneShotFault::new(FaultPoint::AfterState(
            JournalState::Verifying,
        ))));
    let plan = fixture.plan("allowlist");
    engine.apply(&plan, &mut fixture.adapter).unwrap_err();
    let snapshot = fixture.paths.snapshot(&plan.transaction_id);
    assert!(snapshot.join("mjolnir.missing").is_file());
    assert!(!snapshot.join("secret-key.data").exists());
    fs::write(
        fixture.paths.config(UciConfig::Mjolnir),
        b"created-during-apply",
    )
    .unwrap();
    fixture.engine().recover(&mut fixture.adapter).unwrap();
    assert!(!fixture.paths.config(UciConfig::Mjolnir).exists());
}
