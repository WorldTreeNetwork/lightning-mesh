use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_TIMEOUT_SECS: u64 = 120;
pub const MAX_TIMEOUT_SECS: u64 = 600;
pub const MAX_PLAN_BYTES: usize = 64 * 1024;
pub const MAX_SNAPSHOT_BYTES: u64 = 8 * 1024 * 1024;
pub const MAX_RECEIPTS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UciConfig {
    Wireless,
    Network,
    Firewall,
    Mjolnir,
}

impl UciConfig {
    pub const ALL: [Self; 4] = [Self::Wireless, Self::Network, Self::Firewall, Self::Mjolnir];

    pub fn file_name(self) -> &'static str {
        match self {
            Self::Wireless => "wireless",
            Self::Network => "network",
            Self::Firewall => "firewall",
            Self::Mjolnir => "mjolnir",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    pub schema_version: u32,
    pub transaction_id: String,
    pub node_id: String,
    pub expected_revision: String,
    pub proposed_revision: String,
    pub resources: Vec<UciConfig>,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    #[serde(default)]
    pub required_health: Vec<ObservationKind>,
}

fn default_timeout() -> u64 {
    DEFAULT_TIMEOUT_SECS
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JournalState {
    Prepared,
    Applying,
    Verifying,
    Committed,
    Restoring,
    Restored,
    RecoveryRequired,
}

impl JournalState {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Committed | Self::Restored | Self::RecoveryRequired
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Journal {
    pub schema_version: u32,
    pub transaction_id: String,
    pub plan_digest: String,
    pub state: JournalState,
    pub boot_id: String,
    pub started_monotonic_ms: u64,
    pub deadline_monotonic_ms: u64,
    pub previous_revision: String,
    pub proposed_revision: String,
    #[serde(default)]
    pub observations: Vec<Observation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    Committed,
    Restored,
    RecoveryRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecoveryTrigger {
    Interrupted,
    DeadlineExpired,
    Reboot,
    ApplyFailed,
    HealthFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObservationKind {
    AppliedRevision,
    ManagementReachability,
    MeshReachability,
    UpstreamAssociation,
    UpstreamAddress,
    UpstreamRoute,
    UpstreamBoundDnsHttps,
    DownstreamClientForwarding,
    ClientInternet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObservationStatus {
    Passed,
    Failed,
    Unknown,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Vantage {
    TargetLocal,
    ManagementPeer,
    MeshPeer,
    UpstreamInterface,
    DownstreamClient,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    pub kind: ObservationKind,
    pub status: ObservationStatus,
    pub vantage: Vantage,
    pub target: String,
    pub interface: String,
    pub boot_id: String,
    pub monotonic_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl Observation {
    pub fn is_qualifying_pass(&self) -> bool {
        if self.status != ObservationStatus::Passed {
            return false;
        }
        match self.kind {
            ObservationKind::ClientInternet | ObservationKind::DownstreamClientForwarding => {
                self.vantage == Vantage::DownstreamClient
                    && !self.target.is_empty()
                    && !self.interface.is_empty()
            }
            _ => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    pub schema_version: u32,
    pub transaction_id: String,
    pub plan_digest: String,
    pub outcome: Outcome,
    pub previous_revision: String,
    pub resulting_revision: String,
    pub observations: Vec<Observation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery_trigger: Option<RecoveryTrigger>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub wall_time_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tombstone {
    pub plan_digest: String,
    pub outcome: Outcome,
    pub previous_revision: String,
    pub resulting_revision: String,
    pub wall_time_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tombstones {
    pub schema_version: u32,
    pub entries: BTreeMap<String, Tombstone>,
}

impl Default for Tombstones {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            entries: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SnapshotEntry {
    pub config: UciConfig,
    pub missing: bool,
    pub bytes: u64,
    pub digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SnapshotManifest {
    pub schema_version: u32,
    pub entries: Vec<SnapshotEntry>,
    pub total_bytes: u64,
}
