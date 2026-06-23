//! mjolnir-mesh — networking substrate for the mjolnir router mesh.
//!
//! This crate provides the coordination primitives used by routers running
//! mjolnir-mesh on OpenWrt: a CRDT data model for shared mesh state (leases,
//! DNS, services, subnet claims), per-peer TUN tunnel interfaces over Iroh
//! QUIC, and a `babeld` config-and-supervisor layer for cross-site routing.
//!
//! Modules:
//! - [`crdt`] — shared-state types and FWW merge
//! - [`alloc`] / [`claim_cooldown`] — subnet claim allocation on first boot
//! - [`tun`] — per-peer TUN lifecycle and Iroh-datagram encap/decap loops
//! - [`babel`] — babeld config rendering and process supervision
//!
//! See `docs/network-coordination/` in the repo root for the design specs.

pub mod alloc;
pub mod babel;
pub mod claim_cooldown;
pub mod crdt;
pub mod roster;
pub mod tun;

pub use crdt::{
    dns::DnsEntry,
    gossip::GossipMessage,
    hlc::HLC,
    lease::LeaseEntry,
    merge::{merge_subnet_claim, resolve_subnet_conflict, MergeResult},
    service::ServiceEntry,
    subnet::SubnetClaim,
    sync::{GossipError, GossipSync, GossipTransport},
};
pub use roster::{PeerEntry, PeerRoster, RosterError};
