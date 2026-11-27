use crate::storage::{atomic_bytes, sync_dir};
use crate::{
    ApplyAdapter, EngineError, Observation, ObservationKind, ObservationStatus, Outcome, Plan,
    Receipt, TxnPaths, UciConfig, Vantage,
};
use crate::{Passphrase, ProvesUpstream, UplinkPlan, check_restoration};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Strict OpenWrt adapter for the v1 mutation allowlist. It copies desired UCI
/// files; it never runs package operations, swaps wpad, or replaces binaries.
pub struct OpenWrtAdapter {
    paths: TxnPaths,
    source: Option<PathBuf>,
    probes: Box<dyn OpenWrtProbes>,
    secret: Option<(String, Passphrase)>,
}

impl OpenWrtAdapter {
    pub fn new(paths: TxnPaths) -> Self {
        Self {
            paths,
            source: None,
            probes: Box::new(LiveProbes::default()),
            secret: None,
        }
    }

    pub fn for_apply(paths: TxnPaths, source: impl Into<PathBuf>, _plan: Plan) -> Self {
        Self {
            paths,
            source: Some(source.into()),
            probes: Box::new(LiveProbes::default()),
            secret: None,
        }
    }

    /// Inject probes and reload operations for offline tests.
    pub fn with_probes(mut self, probes: impl OpenWrtProbes + 'static) -> Self {
        self.probes = Box::new(probes);
        self
    }

    /// Configure the interfaces and peer targets used by real probes.
    pub fn with_probe_targets(self, targets: ProbeTargets) -> Self {
        self.with_probes(LiveProbes { targets })
    }

    /// Keep credentials in memory until apply holds the transaction lock.
    pub fn with_uplink_secret(mut self, candidate_id: String, passphrase: Passphrase) -> Self {
        self.secret = Some((candidate_id, passphrase));
        self
    }

    fn probe(&mut self, kind: ObservationKind) -> Observation {
        let result = self.probes.probe(kind);
        let mut observation = self.observation(
            kind,
            if result.passed {
                ObservationStatus::Passed
            } else {
                ObservationStatus::Failed
            },
            None,
        );
        observation.vantage = result.vantage;
        observation.target = result.target;
        observation.interface = result.interface;
        observation
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
        if let Some((name, passphrase)) = &self.secret {
            crate::storage::secret_name(name).map_err(|e| e.to_string())?;
            let target = self.paths.uplink_secrets().join(name);
            let previous = match fs::read(&target) {
                Ok(bytes) => Some(bytes),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(_) => return Err("cannot read previous uplink secret".into()),
            };
            crate::storage::atomic_json(
                &self
                    .paths
                    .snapshot(&plan.transaction_id)
                    .join("uplink-secret.json"),
                &crate::storage::SecretSnapshot {
                    name: name.clone(),
                    previous,
                },
            )
            .map_err(|_| "cannot snapshot uplink secret".to_owned())?;
            crate::storage::ensure_dir(&self.paths.uplink_secrets()).map_err(|e| e.to_string())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(
                    self.paths.uplink_secrets(),
                    fs::Permissions::from_mode(0o700),
                )
                .map_err(|e| e.to_string())?;
            }
            atomic_bytes(&target, passphrase.expose().as_bytes())
                .map_err(|_| "cannot write uplink secret".to_owned())?;
        }
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
        self.probes.reload_wireless()
    }

    fn check_health(&mut self, plan: &Plan) -> Result<Vec<Observation>, String> {
        let mut observations = vec![self.revision_observation(&plan.proposed_revision)?];
        for required in &plan.required_health {
            if *required != ObservationKind::AppliedRevision {
                observations.push(self.probe(*required));
            }
        }
        Ok(observations)
    }

    fn verify_restoration(&mut self, previous_revision: &str) -> Result<Vec<Observation>, String> {
        self.probes.reload_wireless()?;
        let observations = vec![
            self.revision_observation(previous_revision)?,
            self.probe(ObservationKind::ManagementReachability),
        ];
        check_restoration(&observations)
            .map_err(|kind| format!("restoration did not prove {kind:?}"))?;
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

impl ProvesUpstream for OpenWrtAdapter {
    fn prepare_uplink(&mut self, uplink: &UplinkPlan) -> Result<(), String> {
        let (ssid, bssid) = crate::uplink::station_identity(uplink)?;
        self.probes.bind_candidate(&ssid, bssid.as_deref());
        match (&uplink.secret, &self.secret) {
            (Some(expected), Some((name, _)))
                if expected.path == format!("{}/{}", crate::SECRET_DIR, name)
                    && *name == uplink.candidate_id =>
            {
                Ok(())
            }
            (None, None) => Ok(()),
            _ => Err("uplink secret was not staged for this candidate".into()),
        }
    }
}

/// A probe returns evidence metadata, never command output or credentials.
pub struct ProbeResult {
    pub passed: bool,
    pub vantage: Vantage,
    pub target: String,
    pub interface: String,
}

pub trait OpenWrtProbes {
    fn reload_wireless(&mut self) -> Result<(), String>;
    fn probe(&mut self, kind: ObservationKind) -> ProbeResult;
    fn bind_candidate(&mut self, _ssid: &str, _bssid: Option<&str>) {}
}

/// Explicit targets prevent a probe from silently using the management uplink.
/// DNS and HTTPS use curl with both --interface and --dns-interface; a curl
/// build without interface-bound DNS support fails closed.
#[derive(Default)]
pub struct ProbeTargets {
    pub upstream_interface: String,
    pub upstream_ssid: String,
    pub upstream_bssid: Option<String>,
    pub dns_server: String,
    pub https_url: String,
    pub mesh_interface: String,
    pub mesh_peer: String,
    pub management_interface: String,
    pub management_peer: String,
}

#[derive(Default)]
struct LiveProbes {
    targets: ProbeTargets,
}

// OpenWrt's timeout bounds every command, including reload. Output is never
// reflected into a receipt. No shell interpolation is involved.
fn command(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new("timeout")
        .arg("10")
        .arg(program)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

impl OpenWrtProbes for LiveProbes {
    fn bind_candidate(&mut self, ssid: &str, bssid: Option<&str>) {
        self.targets.upstream_ssid = ssid.into();
        self.targets.upstream_bssid = bssid.map(str::to_owned);
    }
    fn reload_wireless(&mut self) -> Result<(), String> {
        command("wifi", &["reload"])
            .map(|_| ())
            .ok_or_else(|| "wireless reload failed".into())
    }

    fn probe(&mut self, kind: ObservationKind) -> ProbeResult {
        use ObservationKind::*;
        let t = &self.targets;
        let (interface, target, vantage) = match kind {
            MeshReachability => (&t.mesh_interface, &t.mesh_peer, Vantage::MeshPeer),
            ManagementReachability => (
                &t.management_interface,
                &t.management_peer,
                Vantage::ManagementPeer,
            ),
            _ => (
                &t.upstream_interface,
                &t.https_url,
                Vantage::UpstreamInterface,
            ),
        };
        let passed = !interface.is_empty()
            && match kind {
                UpstreamAssociation => {
                    command("iw", &["dev", interface, "link"]).is_some_and(|out| {
                        !t.upstream_ssid.is_empty()
                            && out
                                .lines()
                                .any(|line| line.trim() == format!("SSID: {}", t.upstream_ssid))
                            && out.lines().any(|line| {
                                line.starts_with("Connected to ")
                                    && t.upstream_bssid.as_ref().is_none_or(|bssid| {
                                        line.split_whitespace()
                                            .nth(2)
                                            .is_some_and(|seen| seen.eq_ignore_ascii_case(bssid))
                                    })
                            })
                    })
                }
                UpstreamAddress => command(
                    "ip",
                    &[
                        "-4", "-o", "addr", "show", "dev", interface, "scope", "global",
                    ],
                )
                .is_some_and(|out| out.split_whitespace().any(|word| word == "inet")),
                UpstreamRoute => {
                    command("ip", &["-4", "route", "show", "default", "dev", interface])
                        .is_some_and(|out| out.lines().any(|line| line.starts_with("default ")))
                }
                UpstreamBoundDnsHttps => {
                    !t.dns_server.is_empty()
                        && target.starts_with("https://")
                        && command(
                            "curl",
                            &[
                                "--silent",
                                "--fail",
                                "--noproxy",
                                "*",
                                "--max-time",
                                "8",
                                "--interface",
                                interface,
                                "--dns-interface",
                                interface,
                                "--dns-servers",
                                &t.dns_server,
                                "--output",
                                "/dev/null",
                                "--write-out",
                                "%{http_code}",
                                target,
                            ],
                        )
                        .is_some_and(|code| code.trim() == "204")
                }
                MeshReachability | ManagementReachability => {
                    !target.is_empty()
                        && command("ping", &["-c", "1", "-W", "2", "-I", interface, target])
                            .is_some()
                }
                _ => false,
            };
        ProbeResult {
            passed,
            vantage,
            target: target.clone(),
            interface: interface.clone(),
        }
    }
}
