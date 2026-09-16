use crate::storage::{atomic_bytes, sync_dir};
use crate::{
    ApplyAdapter, EngineError, Observation, ObservationKind, ObservationStatus, Outcome, Plan,
    Receipt, TxnPaths, UciConfig, Vantage,
};
use std::fs;
use std::path::{Path, PathBuf};

/// Strict OpenWrt adapter for the v1 mutation allowlist. It copies desired UCI
/// files; it never runs package operations, swaps wpad, or replaces binaries.
pub struct OpenWrtAdapter {
    paths: TxnPaths,
    source: Option<PathBuf>,
    plan: Option<Plan>,
}

impl OpenWrtAdapter {
    pub fn new(paths: TxnPaths) -> Self {
        Self {
            paths,
            source: None,
            plan: None,
        }
    }

    pub fn for_apply(paths: TxnPaths, source: impl Into<PathBuf>, plan: Plan) -> Self {
        Self {
            paths,
            source: Some(source.into()),
            plan: Some(plan),
        }
    }

    fn revision(&self) -> Result<String, String> {
        let mut hasher = blake3::Hasher::new();
        for config in UciConfig::ALL {
            hasher.update(config.file_name().as_bytes());
            let path = self.paths.config(config);
            match fs::read(&path) {
                Ok(bytes) => {
                    hasher.update(b"\0present\0");
                    hasher.update(&(bytes.len() as u64).to_le_bytes());
                    hasher.update(&bytes);
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    hasher.update(b"\0missing\0");
                }
                Err(error) => return Err(format!("read {}: {error}", path.display())),
            }
        }
        Ok(hasher.finalize().to_hex().to_string())
    }

    fn observation(
        &self,
        kind: ObservationKind,
        status: ObservationStatus,
        detail: Option<String>,
    ) -> Observation {
        let boot_id = fs::read_to_string("/proc/sys/kernel/random/boot_id")
            .unwrap_or_default()
            .trim()
            .to_owned();
        let monotonic_ms = fs::read_to_string("/proc/uptime")
            .ok()
            .and_then(|value| value.split_whitespace().next()?.parse::<f64>().ok())
            .map(|seconds| (seconds * 1000.0) as u64)
            .unwrap_or_default();
        Observation {
            kind,
            status,
            vantage: Vantage::TargetLocal,
            target: "openwrt-node".into(),
            interface: "uci".into(),
            boot_id,
            monotonic_ms,
            detail,
        }
    }

    fn revision_observation(&self, expected: &str) -> Result<Observation, String> {
        let current = self.revision()?;
        let passed = current == expected;
        Ok(self.observation(
            ObservationKind::AppliedRevision,
            if passed {
                ObservationStatus::Passed
            } else {
                ObservationStatus::Failed
            },
            (!passed).then(|| format!("expected revision {expected}, observed {current}")),
        ))
    }
}

impl ApplyAdapter for OpenWrtAdapter {
    fn current_revision(&mut self) -> Result<String, String> {
        self.revision()
    }

    fn apply(&mut self, plan: &Plan) -> Result<(), String> {
        let source = self
            .source
            .as_ref()
            .ok_or_else(|| "apply source directory was not provided".to_owned())?;
        for config in &plan.resources {
            let desired = source.join(config.file_name());
            let missing = source.join(format!("{}.missing", config.file_name()));
            let target = self.paths.config(*config);
            if desired.is_file() && !missing.exists() {
                let bytes = fs::read(&desired)
                    .map_err(|error| format!("read {}: {error}", desired.display()))?;
                atomic_bytes(&target, &bytes).map_err(|error| error.to_string())?;
            } else if missing.is_file() && !desired.exists() {
                match fs::remove_file(&target) {
                    Ok(()) => sync_dir(
                        target
                            .parent()
                            .ok_or_else(|| format!("{} has no parent", target.display()))?,
                    )
                    .map_err(|error| error.to_string())?,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(format!("remove {}: {error}", target.display())),
                }
            } else {
                return Err(format!(
                    "source must contain exactly one of {} or {}",
                    desired.display(),
                    missing.display()
                ));
            }
        }
        let observed = self.revision()?;
        if observed != plan.proposed_revision {
            return Err(format!(
                "desired files produced revision {observed}, expected {}",
                plan.proposed_revision
            ));
        }
        Ok(())
    }

    fn check_health(&mut self, plan: &Plan) -> Result<Vec<Observation>, String> {
        let mut observations = vec![self.revision_observation(&plan.proposed_revision)?];
        for required in &plan.required_health {
            if *required != ObservationKind::AppliedRevision {
                observations.push(self.observation(
                    *required,
                    ObservationStatus::Unknown,
                    Some("no target/vantage was supplied for this reachability probe".into()),
                ));
            }
        }
        Ok(observations)
    }

    fn verify_restoration(&mut self, previous_revision: &str) -> Result<Vec<Observation>, String> {
        let revision = self.revision_observation(previous_revision)?;
        if revision.status != ObservationStatus::Passed {
            return Err(revision
                .detail
                .clone()
                .unwrap_or_else(|| "restored revision does not match".into()));
        }
        let mut observations = vec![revision];
        if let Some(plan) = &self.plan {
            for required in &plan.required_health {
                if *required != ObservationKind::AppliedRevision {
                    return Err(format!(
                        "required {required:?} reachability cannot be proved without a target and vantage"
                    ));
                }
            }
        }
        observations.shrink_to_fit();
        Ok(observations)
    }
}

pub fn active_plan(paths: &TxnPaths) -> Result<Option<Plan>, EngineError> {
    if !paths.journal().exists() {
        return Ok(None);
    }
    let journal: crate::Journal = serde_json::from_slice(&fs::read(paths.journal())?)?;
    let plan = serde_json::from_slice(&fs::read(paths.plan(&journal.transaction_id))?)?;
    Ok(Some(plan))
}

/// Preserve the installer's compatibility result, but derive every byte from a
/// durable receipt which can only exist after a terminal journal fsync.
pub fn write_result_from_receipt(path: &Path, receipt: &Receipt) -> Result<(), EngineError> {
    let line = match receipt.outcome {
        Outcome::Committed => format!("OK: transaction {} committed\n", receipt.transaction_id),
        Outcome::Restored => format!(
            "ROLLED_BACK: transaction {} restored\n",
            receipt.transaction_id
        ),
        Outcome::RecoveryRequired => format!(
            "FAILED: transaction {} requires recovery\n",
            receipt.transaction_id
        ),
    };
    atomic_bytes(path, line.as_bytes())
}
