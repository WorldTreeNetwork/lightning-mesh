use crate::model::{
    Journal, JournalState, MAX_PLAN_BYTES, MAX_TIMEOUT_SECS, Observation, ObservationKind,
    ObservationStatus, Outcome, Plan, Receipt, RecoveryTrigger, SCHEMA_VERSION, Tombstone,
    Tombstones, UciConfig, Vantage,
};
use crate::storage::{
    NodeLock, TxnPaths, active_ids, atomic_json, initialize, prune_receipts, read_json,
    remove_active, restore_snapshot, snapshot,
};
use std::collections::BTreeSet;
use std::fs;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultPoint {
    Snapshot(UciConfig),
    AfterState(JournalState),
    AfterHealthPass,
}

pub trait FaultInjector: Send + Sync {
    fn check(&self, point: FaultPoint) -> Result<(), EngineError>;
}

#[derive(Debug, Default)]
pub struct NoFaults;

impl FaultInjector for NoFaults {
    fn check(&self, _point: FaultPoint) -> Result<(), EngineError> {
        Ok(())
    }
}

pub trait Runtime: Send + Sync {
    fn boot_id(&self) -> Result<String, EngineError>;
    fn monotonic_ms(&self) -> Result<u64, EngineError>;
    fn wall_time_ms(&self) -> i64;
}

#[derive(Debug)]
pub struct SystemRuntime {
    process_start: Instant,
}

impl Default for SystemRuntime {
    fn default() -> Self {
        Self {
            process_start: Instant::now(),
        }
    }
}

impl Runtime for SystemRuntime {
    fn boot_id(&self) -> Result<String, EngineError> {
        let value = fs::read_to_string("/proc/sys/kernel/random/boot_id")?;
        let value = value.trim();
        if value.is_empty() {
            return Err(EngineError::Clock("empty boot_id".into()));
        }
        Ok(value.to_owned())
    }

    fn monotonic_ms(&self) -> Result<u64, EngineError> {
        // `/proc/uptime` is CLOCK_BOOTTIME on Linux and remains stable across
        // helper process restarts within one boot.
        let uptime = fs::read_to_string("/proc/uptime")?;
        let seconds: f64 = uptime
            .split_whitespace()
            .next()
            .ok_or_else(|| EngineError::Clock("missing /proc/uptime value".into()))?
            .parse()
            .map_err(|error| EngineError::Clock(format!("invalid /proc/uptime: {error}")))?;
        if !seconds.is_finite() || seconds.is_sign_negative() {
            return Err(EngineError::Clock("invalid monotonic value".into()));
        }
        Ok((seconds * 1000.0) as u64)
    }

    fn wall_time_ms(&self) -> i64 {
        let _ = self.process_start;
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
            .unwrap_or_default()
    }
}

pub trait ApplyAdapter {
    fn current_revision(&mut self) -> Result<String, String>;
    fn apply(&mut self, plan: &Plan) -> Result<(), String>;
    fn check_health(&mut self, plan: &Plan) -> Result<Vec<Observation>, String>;
    fn verify_restoration(&mut self, previous_revision: &str) -> Result<Vec<Observation>, String>;
}

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("another network transaction owns the node lock")]
    Busy,
    #[error("invalid plan: {0}")]
    InvalidPlan(String),
    #[error("stale revision: expected {expected}, current {current}")]
    StaleRevision { expected: String, current: String },
    #[error("transaction ID {id} is already bound to another plan")]
    DuplicateIdConflict { id: String },
    #[error("durable recovery is required before admission: {0}")]
    RecoveryRequired(String),
    #[error("durable state is corrupt: {0}")]
    Corrupt(String),
    #[error("snapshot is incomplete or has been changed")]
    IncompleteSnapshot,
    #[error("snapshot exceeds the 8 MiB ceiling")]
    SnapshotTooLarge,
    #[error("restoration failed: {0}")]
    RestorationFailed(String),
    #[error("storage failure: {0}")]
    Storage(String),
    #[error("clock failure: {0}")]
    Clock(String),
    #[error("adapter failure: {0}")]
    Adapter(String),
    #[error("injected crash at {0:?}")]
    InjectedCrash(FaultPoint),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub struct Engine {
    paths: TxnPaths,
    runtime: Arc<dyn Runtime>,
    faults: Arc<dyn FaultInjector>,
}

impl Engine {
    pub fn new(paths: TxnPaths) -> Self {
        Self {
            paths,
            runtime: Arc::new(SystemRuntime::default()),
            faults: Arc::new(NoFaults),
        }
    }

    pub fn with_runtime(mut self, runtime: Arc<dyn Runtime>) -> Self {
        self.runtime = runtime;
        self
    }

    pub fn with_faults(mut self, faults: Arc<dyn FaultInjector>) -> Self {
        self.faults = faults;
        self
    }

    pub fn paths(&self) -> &TxnPaths {
        &self.paths
    }

    pub fn apply(
        &self,
        plan: &Plan,
        adapter: &mut dyn ApplyAdapter,
    ) -> Result<Receipt, EngineError> {
        let lock = NodeLock::acquire(&self.paths)?;
        self.apply_locked(&lock, plan, adapter)
    }

    /// Apply while using a lock already held by the caller. This is the entry
    /// point for the OpenWrt helper, whose fd 9 is inherited from the ash
    /// launcher rather than reopening the lock path.
    pub fn apply_locked(
        &self,
        _lock: &NodeLock,
        plan: &Plan,
        adapter: &mut dyn ApplyAdapter,
    ) -> Result<Receipt, EngineError> {
        let (plan_bytes, plan_digest) = validate_plan(plan)?;
        initialize(&self.paths)?;

        self.recover_locked(adapter)?;
        if let Some(receipt) = self.lookup_id(&plan.transaction_id, &plan_digest)? {
            return Ok(receipt);
        }

        let current = adapter.current_revision().map_err(EngineError::Adapter)?;
        if current != plan.expected_revision {
            return Err(EngineError::StaleRevision {
                expected: plan.expected_revision.clone(),
                current,
            });
        }

        if !active_ids(&self.paths)?.is_empty() || self.paths.journal().exists() {
            return Err(EngineError::RecoveryRequired(
                "unexpected active transaction after recovery".into(),
            ));
        }

        if let Err(error) = snapshot(&self.paths, &plan.transaction_id, self.faults.as_ref()) {
            // No journal means `prepared` was never reached, so no mutation can
            // have occurred and this partial admission artifact is safe to drop.
            remove_active(&self.paths, &plan.transaction_id)?;
            return Err(error);
        }
        if let Err(error) =
            crate::storage::atomic_bytes(&self.paths.plan(&plan.transaction_id), &plan_bytes)
        {
            remove_active(&self.paths, &plan.transaction_id)?;
            return Err(error);
        }

        let boot_id = self.runtime.boot_id()?;
        let started = self.runtime.monotonic_ms()?;
        let deadline = started
            .checked_add(plan.timeout_secs.saturating_mul(1000))
            .ok_or_else(|| EngineError::Clock("deadline overflow".into()))?;
        let mut journal = Journal {
            schema_version: SCHEMA_VERSION,
            transaction_id: plan.transaction_id.clone(),
            plan_digest: plan_digest.clone(),
            state: JournalState::Prepared,
            boot_id,
            started_monotonic_ms: started,
            deadline_monotonic_ms: deadline,
            previous_revision: plan.expected_revision.clone(),
            proposed_revision: plan.proposed_revision.clone(),
            observations: Vec::new(),
            detail: None,
        };
        self.persist_state(&journal)?;

        journal.state = JournalState::Applying;
        self.persist_state(&journal)?;
        if let Err(detail) = adapter.apply(plan) {
            return self.restore_locked(
                &mut journal,
                adapter,
                RecoveryTrigger::ApplyFailed,
                Some(detail),
            );
        }

        journal.state = JournalState::Verifying;
        self.persist_state(&journal)?;
        let observations = match adapter.check_health(plan) {
            Ok(observations) => sanitize_observations(observations),
            Err(detail) => {
                return self.restore_locked(
                    &mut journal,
                    adapter,
                    RecoveryTrigger::HealthFailed,
                    Some(detail),
                );
            }
        };
        if let Some(missing) = missing_required_health(plan, &observations) {
            return self.restore_locked(
                &mut journal,
                adapter,
                RecoveryTrigger::HealthFailed,
                Some(format!(
                    "required health evidence did not pass: {missing:?}"
                )),
            );
        }
        if self.runtime.boot_id()? != journal.boot_id {
            return self.restore_locked(
                &mut journal,
                adapter,
                RecoveryTrigger::Reboot,
                Some("boot identity changed before commit".into()),
            );
        }
        if self.runtime.monotonic_ms()? > journal.deadline_monotonic_ms {
            return self.restore_locked(
                &mut journal,
                adapter,
                RecoveryTrigger::DeadlineExpired,
                Some("confirmation deadline expired before commit".into()),
            );
        }

        self.faults.check(FaultPoint::AfterHealthPass)?;
        journal.state = JournalState::Committed;
        journal.observations = observations.clone();
        self.persist_state(&journal)?;
        let receipt = Receipt {
            schema_version: SCHEMA_VERSION,
            transaction_id: plan.transaction_id.clone(),
            plan_digest,
            outcome: Outcome::Committed,
            previous_revision: plan.expected_revision.clone(),
            resulting_revision: plan.proposed_revision.clone(),
            observations,
            recovery_trigger: None,
            detail: None,
            wall_time_ms: self.runtime.wall_time_ms(),
        };
        self.finalize(&receipt, false)?;
        Ok(receipt)
    }

    /// Run recovery under the node-wide lock without admitting a new plan.
    pub fn recover(&self, adapter: &mut dyn ApplyAdapter) -> Result<Option<Receipt>, EngineError> {
        let lock = NodeLock::acquire(&self.paths)?;
        self.recover_with_lock(&lock, adapter)
    }

    pub fn recover_with_lock(
        &self,
        _lock: &NodeLock,
        adapter: &mut dyn ApplyAdapter,
    ) -> Result<Option<Receipt>, EngineError> {
        initialize(&self.paths)?;
        self.recover_locked(adapter)
    }

    /// Early-boot recovery phase. Restore the allowlisted files before netifd
    /// starts, but deliberately leave the journal in `restoring` until the late
    /// service-time verifier has proved the restored network.
    pub fn restore_before_network(&self, _lock: &NodeLock) -> Result<bool, EngineError> {
        initialize(&self.paths)?;
        let Some(mut journal) = self.load_active_journal()? else {
            return Ok(false);
        };
        match journal.state {
            JournalState::Committed | JournalState::Restored => return Ok(false),
            JournalState::RecoveryRequired => {
                return Err(EngineError::RecoveryRequired(
                    journal
                        .detail
                        .unwrap_or_else(|| "previous restoration could not be proved".into()),
                ));
            }
            JournalState::Prepared
            | JournalState::Applying
            | JournalState::Verifying
            | JournalState::Restoring => {}
        }

        journal.state = JournalState::Restoring;
        journal.detail =
            Some("snapshot restored before network startup; verification pending".into());
        self.persist_state(&journal)?;
        if let Err(error) = restore_snapshot(&self.paths, &journal.transaction_id) {
            let trigger = self.recovery_trigger(&journal)?;
            self.fail_restoration(&mut journal, trigger, &error)?;
            return Err(EngineError::RecoveryRequired(error.to_string()));
        }
        Ok(true)
    }

    /// Late-boot recovery phase. It never performs the early file copy: if the
    /// restore phase was missed, the honest outcome is recovery-required.
    pub fn verify_after_services(
        &self,
        _lock: &NodeLock,
        adapter: &mut dyn ApplyAdapter,
    ) -> Result<Option<Receipt>, EngineError> {
        initialize(&self.paths)?;
        let Some(mut journal) = self.load_active_journal()? else {
            return Ok(None);
        };
        match journal.state {
            JournalState::Restoring => {
                let trigger = self.recovery_trigger(&journal)?;
                let observations = match adapter.verify_restoration(&journal.previous_revision) {
                    Ok(observations) => sanitize_observations(observations),
                    Err(detail) => {
                        let error = EngineError::RestorationFailed(detail);
                        self.fail_restoration(&mut journal, trigger, &error)?;
                        return Err(EngineError::RecoveryRequired(error.to_string()));
                    }
                };
                journal.state = JournalState::Restored;
                journal.observations = observations.clone();
                journal.detail =
                    Some("snapshot restoration verified after network services".into());
                self.persist_state(&journal)?;
                let receipt =
                    self.restored_receipt(&journal, trigger, observations, journal.detail.clone());
                self.finalize(&receipt, false)?;
                Ok(Some(receipt))
            }
            JournalState::Committed | JournalState::Restored => self.recover_locked(adapter),
            JournalState::RecoveryRequired => Err(EngineError::RecoveryRequired(
                journal
                    .detail
                    .unwrap_or_else(|| "previous restoration could not be proved".into()),
            )),
            JournalState::Prepared | JournalState::Applying | JournalState::Verifying => {
                let detail = "early restore phase did not run before network startup";
                let error = EngineError::RestorationFailed(detail.into());
                let trigger = self.recovery_trigger(&journal)?;
                self.fail_restoration(&mut journal, trigger, &error)?;
                Err(EngineError::RecoveryRequired(detail.into()))
            }
        }
    }

    pub fn receipt(&self, id: &str) -> Result<Option<Receipt>, EngineError> {
        validate_id(id, "transaction_id")?;
        if !self.paths.receipt(id).exists() {
            return Ok(None);
        }
        Ok(Some(read_json(&self.paths.receipt(id))?))
    }

    fn recover_locked(
        &self,
        adapter: &mut dyn ApplyAdapter,
    ) -> Result<Option<Receipt>, EngineError> {
        let Some(mut journal) = self.load_active_journal()? else {
            return Ok(None);
        };

        match journal.state {
            JournalState::Committed => {
                let observed_revision = adapter.current_revision().map_err(EngineError::Adapter)?;
                if observed_revision != journal.proposed_revision {
                    return Err(EngineError::RecoveryRequired(format!(
                        "committed revision {} is not active (observed {observed_revision})",
                        journal.proposed_revision
                    )));
                }
                let receipt = Receipt {
                    schema_version: SCHEMA_VERSION,
                    transaction_id: journal.transaction_id.clone(),
                    plan_digest: journal.plan_digest.clone(),
                    outcome: Outcome::Committed,
                    previous_revision: journal.previous_revision.clone(),
                    resulting_revision: journal.proposed_revision.clone(),
                    observations: journal.observations.clone(),
                    recovery_trigger: None,
                    detail: Some("terminal commit recovered before receipt persistence".into()),
                    wall_time_ms: self.runtime.wall_time_ms(),
                };
                self.finalize(&receipt, false)?;
                Ok(Some(receipt))
            }
            JournalState::Restored => {
                let receipt = self.restored_receipt(
                    &journal,
                    RecoveryTrigger::Interrupted,
                    journal.observations.clone(),
                    Some("terminal restoration recovered before receipt persistence".into()),
                );
                self.finalize(&receipt, false)?;
                Ok(Some(receipt))
            }
            JournalState::RecoveryRequired => Err(EngineError::RecoveryRequired(
                journal
                    .detail
                    .unwrap_or_else(|| "previous restoration could not be proved".into()),
            )),
            JournalState::Prepared
            | JournalState::Applying
            | JournalState::Verifying
            | JournalState::Restoring => {
                let trigger = self.recovery_trigger(&journal)?;
                let receipt = self.restore_locked(&mut journal, adapter, trigger, None)?;
                Ok(Some(receipt))
            }
        }
    }

    fn load_active_journal(&self) -> Result<Option<Journal>, EngineError> {
        let ids = active_ids(&self.paths)?;
        if ids.len() > 1 {
            return Err(EngineError::RecoveryRequired(
                "more than one transaction exists under active/".into(),
            ));
        }
        if !self.paths.journal().exists() {
            if ids.is_empty() {
                return Ok(None);
            }
            return Err(EngineError::RecoveryRequired(format!(
                "active transaction {} has no journal",
                ids[0]
            )));
        }
        let journal: Journal = read_json(&self.paths.journal()).map_err(|error| {
            EngineError::RecoveryRequired(format!("journal cannot be trusted: {error}"))
        })?;
        validate_journal(&journal).map_err(|error| {
            EngineError::RecoveryRequired(format!("journal cannot be trusted: {error}"))
        })?;
        if ids.as_slice() != [journal.transaction_id.as_str()] {
            return Err(EngineError::RecoveryRequired(
                "journal and active/ disagree".into(),
            ));
        }
        let plan: Plan = read_json(&self.paths.plan(&journal.transaction_id)).map_err(|error| {
            EngineError::RecoveryRequired(format!("durable plan cannot be trusted: {error}"))
        })?;
        let (_, digest) = validate_plan(&plan).map_err(|error| {
            EngineError::RecoveryRequired(format!("durable plan cannot be trusted: {error}"))
        })?;
        if digest != journal.plan_digest || plan.transaction_id != journal.transaction_id {
            return Err(EngineError::RecoveryRequired(
                "journal does not match its durable plan".into(),
            ));
        }
        Ok(Some(journal))
    }

    fn recovery_trigger(&self, journal: &Journal) -> Result<RecoveryTrigger, EngineError> {
        if self.runtime.boot_id()? != journal.boot_id {
            return Ok(RecoveryTrigger::Reboot);
        }
        if self.runtime.monotonic_ms()? > journal.deadline_monotonic_ms {
            return Ok(RecoveryTrigger::DeadlineExpired);
        }
        Ok(RecoveryTrigger::Interrupted)
    }

    fn restore_locked(
        &self,
        journal: &mut Journal,
        adapter: &mut dyn ApplyAdapter,
        trigger: RecoveryTrigger,
        initiating_detail: Option<String>,
    ) -> Result<Receipt, EngineError> {
        journal.state = JournalState::Restoring;
        journal.detail = initiating_detail;
        self.persist_state(journal)?;

        let restore_result =
            restore_snapshot(&self.paths, &journal.transaction_id).and_then(|()| {
                adapter
                    .verify_restoration(&journal.previous_revision)
                    .map(sanitize_observations)
                    .map_err(EngineError::RestorationFailed)
            });
        match restore_result {
            Ok(observations) => {
                journal.state = JournalState::Restored;
                journal.observations = observations.clone();
                self.persist_state(journal)?;
                let receipt =
                    self.restored_receipt(journal, trigger, observations, journal.detail.clone());
                self.finalize(&receipt, false)?;
                Ok(receipt)
            }
            Err(error) => {
                self.fail_restoration(journal, trigger, &error)?;
                Err(EngineError::RecoveryRequired(error.to_string()))
            }
        }
    }

    fn fail_restoration(
        &self,
        journal: &mut Journal,
        trigger: RecoveryTrigger,
        error: &EngineError,
    ) -> Result<(), EngineError> {
        journal.state = JournalState::RecoveryRequired;
        journal.detail = Some(error.to_string());
        self.persist_state(journal)?;
        let receipt = Receipt {
            schema_version: SCHEMA_VERSION,
            transaction_id: journal.transaction_id.clone(),
            plan_digest: journal.plan_digest.clone(),
            outcome: Outcome::RecoveryRequired,
            previous_revision: journal.previous_revision.clone(),
            resulting_revision: journal.previous_revision.clone(),
            observations: Vec::new(),
            recovery_trigger: Some(trigger),
            detail: journal.detail.clone(),
            wall_time_ms: self.runtime.wall_time_ms(),
        };
        self.finalize(&receipt, true)
    }

    fn restored_receipt(
        &self,
        journal: &Journal,
        trigger: RecoveryTrigger,
        observations: Vec<Observation>,
        detail: Option<String>,
    ) -> Receipt {
        Receipt {
            schema_version: SCHEMA_VERSION,
            transaction_id: journal.transaction_id.clone(),
            plan_digest: journal.plan_digest.clone(),
            outcome: Outcome::Restored,
            previous_revision: journal.previous_revision.clone(),
            resulting_revision: journal.previous_revision.clone(),
            observations,
            recovery_trigger: Some(trigger),
            detail,
            wall_time_ms: self.runtime.wall_time_ms(),
        }
    }

    fn persist_state(&self, journal: &Journal) -> Result<(), EngineError> {
        atomic_json(&self.paths.journal(), journal)?;
        self.faults.check(FaultPoint::AfterState(journal.state))?;
        Ok(())
    }

    fn finalize(&self, receipt: &Receipt, retain_active: bool) -> Result<(), EngineError> {
        // Receipt is derived only after the terminal journal was made durable.
        atomic_json(&self.paths.receipt(&receipt.transaction_id), receipt)?;
        let mut tombstones: Tombstones = read_json(&self.paths.tombstones())?;
        if tombstones.schema_version != SCHEMA_VERSION {
            return Err(EngineError::Corrupt(
                "unsupported tombstone schema version".into(),
            ));
        }
        tombstones.entries.insert(
            receipt.transaction_id.clone(),
            Tombstone {
                plan_digest: receipt.plan_digest.clone(),
                outcome: receipt.outcome,
                previous_revision: receipt.previous_revision.clone(),
                resulting_revision: receipt.resulting_revision.clone(),
                wall_time_ms: receipt.wall_time_ms,
            },
        );
        atomic_json(&self.paths.tombstones(), &tombstones)?;
        prune_receipts(&self.paths)?;
        if !retain_active {
            remove_active(&self.paths, &receipt.transaction_id)?;
        }
        Ok(())
    }

    fn lookup_id(&self, id: &str, digest: &str) -> Result<Option<Receipt>, EngineError> {
        let tombstones: Tombstones = read_json(&self.paths.tombstones())?;
        let Some(tombstone) = tombstones.entries.get(id) else {
            return Ok(None);
        };
        if tombstone.plan_digest != digest {
            return Err(EngineError::DuplicateIdConflict { id: id.to_owned() });
        }
        if self.paths.receipt(id).exists() {
            let receipt: Receipt = read_json(&self.paths.receipt(id))?;
            if receipt.plan_digest != digest {
                return Err(EngineError::Corrupt(format!(
                    "receipt and tombstone disagree for {id}"
                )));
            }
            return Ok(Some(receipt));
        }
        Ok(Some(Receipt {
            schema_version: SCHEMA_VERSION,
            transaction_id: id.to_owned(),
            plan_digest: digest.to_owned(),
            outcome: tombstone.outcome,
            previous_revision: tombstone.previous_revision.clone(),
            resulting_revision: tombstone.resulting_revision.clone(),
            observations: Vec::new(),
            recovery_trigger: None,
            detail: Some("terminal receipt evicted; outcome retained by tombstone".into()),
            wall_time_ms: tombstone.wall_time_ms,
        }))
    }
}

fn validate_plan(plan: &Plan) -> Result<(Vec<u8>, String), EngineError> {
    if plan.schema_version != SCHEMA_VERSION {
        return Err(EngineError::InvalidPlan(
            "unsupported schema_version".into(),
        ));
    }
    validate_id(&plan.transaction_id, "transaction_id")?;
    validate_id(&plan.node_id, "node_id")?;
    validate_revision(&plan.expected_revision, "expected_revision")?;
    validate_revision(&plan.proposed_revision, "proposed_revision")?;
    if plan.timeout_secs == 0 || plan.timeout_secs > MAX_TIMEOUT_SECS {
        return Err(EngineError::InvalidPlan(format!(
            "timeout_secs must be in 1..={MAX_TIMEOUT_SECS}"
        )));
    }
    if plan.resources.is_empty() {
        return Err(EngineError::InvalidPlan("resources cannot be empty".into()));
    }
    let resources: BTreeSet<_> = plan.resources.iter().copied().collect();
    if resources.len() != plan.resources.len() {
        return Err(EngineError::InvalidPlan(
            "resources contain duplicates".into(),
        ));
    }
    let bytes = serde_json::to_vec(plan)?;
    if bytes.len() > MAX_PLAN_BYTES {
        return Err(EngineError::InvalidPlan(format!(
            "plan exceeds {MAX_PLAN_BYTES} bytes"
        )));
    }
    let digest = blake3::hash(&bytes).to_hex().to_string();
    Ok((bytes, digest))
}

fn validate_id(value: &str, field: &str) -> Result<(), EngineError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        || value == "."
        || value == ".."
    {
        return Err(EngineError::InvalidPlan(format!(
            "{field} is not a bounded opaque ID"
        )));
    }
    Ok(())
}

fn validate_revision(value: &str, field: &str) -> Result<(), EngineError> {
    if value.is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
        return Err(EngineError::InvalidPlan(format!("invalid {field}")));
    }
    Ok(())
}

fn validate_journal(journal: &Journal) -> Result<(), EngineError> {
    if journal.schema_version != SCHEMA_VERSION {
        return Err(EngineError::Corrupt(
            "unsupported journal schema version".into(),
        ));
    }
    validate_id(&journal.transaction_id, "journal transaction_id")
        .map_err(|error| EngineError::Corrupt(error.to_string()))?;
    if journal.boot_id.is_empty()
        || journal.plan_digest.len() != 64
        || journal.deadline_monotonic_ms < journal.started_monotonic_ms
    {
        return Err(EngineError::Corrupt("invalid journal fields".into()));
    }
    Ok(())
}

fn sanitize_observations(mut observations: Vec<Observation>) -> Vec<Observation> {
    for observation in &mut observations {
        if matches!(
            observation.kind,
            ObservationKind::ClientInternet | ObservationKind::DownstreamClientForwarding
        ) && observation.vantage != Vantage::DownstreamClient
            && observation.status == ObservationStatus::Passed
        {
            observation.status = ObservationStatus::Unknown;
            observation.detail =
                Some("local or upstream evidence cannot establish a downstream client path".into());
        }
    }
    observations
}

fn missing_required_health(plan: &Plan, observations: &[Observation]) -> Option<ObservationKind> {
    plan.required_health.iter().copied().find(|required| {
        !observations
            .iter()
            .any(|observation| observation.kind == *required && observation.is_qualifying_pass())
    })
}
