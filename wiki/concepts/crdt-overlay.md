---
title: CRDT Overlay
created: 2025-11-05
updated: 2025-11-05
type: concept
tags: [mesh-core, crdt]
sources: [crates/mjolnir-mesh/src/crdt/]
state: [CURRENT]
confidence: high
---

# CRDT Overlay

Lightning Mesh uses Conflict-free Replicated Data Types (CRDTs) for shared state across all nodes. Every node holds a full copy of the mesh state and converges through gossip.

## CRDT Types

- **SubnetClaim** -- First-writer-wins. Each node claims a routed /24 from 10.42.0.0/16. Conflict resolved by HLC timestamp.
- **LeaseBook** -- Tracks client DHCP leases per node. Merged per-entry with last-write-wins.
- **ServiceBookV2** -- Published services with device/host labels. Gossip-converged.
- **LeasedNameBook** -- Operator-published .mesh names. Key-owned, time-limited leases.
- **NodeNameBook** -- Stable human-readable node names.
- **PeerAddrBook** -- Address book of known peers.
- **UserBook** -- User entries in the mesh directory.
- **CoordinateBook** -- WGS84 coordinates for coverage survey (LWW unix timestamp).

## Merge Strategy

All CRDTs use a [HLC](concepts/hlc.md) (Hybrid Logical Clock) for ordering. Merge is first-writer-wins with HLC tiebreaking. The merge function for each type is in `crates/mjolnir-mesh/src/crdt/merge.rs`.

## Key Design Decisions

- No central authority -- every node is symmetric
- Full state replication -- no sharding
- Gossip-based convergence -- no leader election for state
- Flash writes are minimized -- liveness is decoupled from durable state (see [[liveness-plane]])
