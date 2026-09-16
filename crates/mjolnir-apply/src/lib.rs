//! Durable, target-owned network configuration transactions.
//!
//! This crate owns the journal and recovery state machine. It deliberately does
//! not contain an OpenWrt service adapter, launcher, init script, or radio code.

mod engine;
mod model;
mod openwrt;
mod storage;

pub use engine::{
    ApplyAdapter, Engine, EngineError, FaultInjector, FaultPoint, NoFaults, Runtime, SystemRuntime,
};
pub use model::{
    DEFAULT_TIMEOUT_SECS, Journal, JournalState, MAX_PLAN_BYTES, MAX_RECEIPTS, MAX_SNAPSHOT_BYTES,
    MAX_TIMEOUT_SECS, Observation, ObservationKind, ObservationStatus, Outcome, Plan, Receipt,
    RecoveryTrigger, SCHEMA_VERSION, Tombstone, Tombstones, UciConfig, Vantage,
};
pub use openwrt::{OpenWrtAdapter, active_plan, write_result_from_receipt};
pub use storage::{NodeLock, TxnPaths};

#[cfg(test)]
mod tests;
