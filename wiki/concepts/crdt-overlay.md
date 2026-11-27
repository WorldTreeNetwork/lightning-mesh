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

- **SubnetClaim** -- A node claims a routed /24 from 10.42.0.0/16. `claimed_at` is a hybrid logical clock. The same owner, with a newer clock, is an update. A different owner is a `Conflict`: the earlier clock wins, and an equal clock breaks the tie on `owner_node_id` (`merge_subnet_claim`, `resolve_subnet_conflict`).
- **LeaseBook** -- Client DHCP leases. Per-entry last-write-wins.
- **ServiceBook** / **ServiceBookV2** -- v1 services are last-write-wins. v2 services are owner-bound: the same owner updates by `updated_at`, and a different owner wins on the earlier `first_claimed_at`. Reserved names are rejected before that compare (`merge_service_v2`).
- **LeasedNameBook** -- Key-owned `.mesh` names. The same owner renews on a newer `renewed_at`. A different owner is a `Conflict` (`merge_leased_name` in `leased_name.rs`).
- **NodeNameBook** -- Human router names. Last-write-wins on `announced_at`.
- **PeerAddrBook** -- Known peer addresses. A newer `announced_at` wins only when the address set or the relay changed. A stamp-only refresh is discarded (`merge_peer_addr`).
- **UserBook** -- Directory users. Last-write-wins.
- **CoordinateBook** -- WGS84 stamps. Last-write-wins on `stamped_at_unix`, then on the `stamper` string. Not an HLC (`merge_coordinate`).
- **`DnsUpdate`** -- A gossip variant in `gossip.rs`. `merge.rs` has no DNS merge function.

## Merge Strategy

There is no single merge rule. The functions live in `crates/mjolnir-mesh/src/crdt/merge.rs` except leased names, which live beside `LeasedName`. Several books order events with a hybrid logical clock. Coordinates do not.

## Key Design Decisions

- CRDT convergence is symmetric. Household ownership is the owner-claim ceremony, not a leader for these books
- Full state replication -- no sharding
- Gossip-based convergence -- no leader election for this state
- Flash writes are minimized -- liveness is decoupled from durable state (see [[liveness-plane]])
