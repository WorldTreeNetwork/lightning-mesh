---
title: CRDT Overlay
created: 2025-11-05
updated: 2026-11-05
type: concept
tags: [mesh-core, crdt]
sources: [crates/mjolnir-mesh/src/crdt/]
state: [CURRENT]
confidence: high
---

# CRDT Overlay

Lightning Mesh uses Conflict-free Replicated Data Types (CRDTs) for shared state across all nodes. Every node holds a full copy of the mesh state and converges through gossip.

## CRDT Types

- **SubnetClaim** -- A node claims a routed /24 from 10.42.0.0/16. A different owner loses to the earlier `claimed_at`. The same owner refreshing the claim is an update. See `merge_subnet_claim` in `crates/mjolnir-mesh/src/crdt/merge.rs`.
- **LeaseBook** -- Client DHCP leases. Per-entry last-write-wins.
- **ServiceBook** / **ServiceBookV2** -- v1 services are last-write-wins. v2 services are owner-bound: the same owner updates by `updated_at`, and a different owner wins on the earlier `first_claimed_at` (`merge_service_v2`).
- **LeasedNameBook** -- Key-owned `.mesh` names. Merge is the lease/reclaim rule in `crates/mjolnir-mesh/src/crdt/leased_name.rs`.
- **NodeNameBook** -- Human router names. Last-write-wins.
- **PeerAddrBook** -- Known peer addresses. Last-write-wins.
- **UserBook** -- Directory users. Last-write-wins.
- **CoordinateBook** -- WGS84 stamps. Last-write-wins on `stamped_at_unix`, not an HLC (`merge_coordinate`).

## Merge Strategy

There is no single merge rule. The functions live in `crates/mjolnir-mesh/src/crdt/merge.rs` except leased names, which live beside `LeasedName`. Several books order events with a hybrid logical clock. Coordinates do not.

## Key Design Decisions

- No central authority -- every node is symmetric
- Full state replication -- no sharding
- Gossip-based convergence -- no leader election for state
- Flash writes are minimized -- liveness is decoupled from durable state (see [[liveness-plane]])
