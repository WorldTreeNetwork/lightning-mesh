//! Durable, target-owned network configuration transactions.
//!
//! This crate owns the journal and recovery state machine. It deliberately does
//! not contain an OpenWrt service adapter, launcher, init script, or radio code.

mod client_class;
mod engine;
mod model;
mod node_profile;
mod openwrt;
mod storage;
mod uplink;

pub use client_class::{
    ClassPlan, ClassRefusal, ClassRequest, ClientClass, Destination, FlowInput, OpenAttempt,
    RadioEffect, Reach, ReachDecision, Schedule, SchedulesClients, build_class_plan,
    submit_class_plan,
};
pub use engine::{
    ApplyAdapter, Engine, EngineError, FaultInjector, FaultPoint, NoFaults, Runtime, SystemRuntime,
};
pub use model::{
    DEFAULT_TIMEOUT_SECS, Journal, JournalState, MAX_PLAN_BYTES, MAX_RECEIPTS, MAX_SNAPSHOT_BYTES,
    MAX_TIMEOUT_SECS, Observation, ObservationKind, ObservationStatus, Outcome, Plan, Receipt,
    RecoveryTrigger, SCHEMA_VERSION, Tombstone, Tombstones, UciConfig, Vantage,
};
pub use node_profile::{
    Band, CableLoss, NodeConfig, NodeFacts, PORT_MAP_DIR, PlanIds, PortMap, PortRole,
    ProfileDecision, ProfileId, ProfilePlan, ProfileRefusal, ProfileRequest, ProvesReachability,
    RadioRole, build_plan, check_profile_safety, decide, submit_profile_plan,
};
pub use openwrt::{OpenWrtAdapter, active_plan, write_result_from_receipt};
pub use storage::{NodeLock, TxnPaths};
pub use uplink::{
    Authorization, Candidate, DefaultExport, EXPORT_GATE, HOUSE_CLIENT_SSID, MeshInventory,
    Passphrase, ProvesUpstream, REQUIRED_HEALTH, RESTORATION_HEALTH, SECRET_DIR, SECRET_MODE,
    SecretFile, UplinkEffect, UplinkFacts, UplinkPlan, UplinkRefusal, UplinkRequest,
    build_uplink_plan, check_restoration, default_export, lease_conflicts, submit_uplink_plan,
};

#[cfg(test)]
mod tests;
