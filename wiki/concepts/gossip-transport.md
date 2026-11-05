---
title: Gossip Transport
created: 2025-11-05
updated: 2025-11-05
type: concept
tags: [mesh-core, crdt, iroh]
sources: [crates/mjolnir-mesh/src/crdt/sync.rs]
state: [CURRENT]
confidence: high
---

# Gossip Transport

The gossip sync seam is a transport-agnostic abstraction for CRDT state propagation. It is the primary integration point for cross-stack interoperability.

## Architecture

- **`GossipTransport` trait** -- defines `broadcast(Bytes)` and `recv() -> Bytes`. The trait lives in `crates/mjolnir-mesh/src/crdt/sync.rs` (line 44).
- **`GossipSync<T>`** -- dispatcher that encodes/decodes `GossipMessage` (postcard) and runs the receive loop.
- **Concrete impl** -- `iroh_gossip::Gossip` is the default. A `ReticulumTransport` impl would be a feature-gated alternative.

## Wire Format

Messages are postcard-serialized `GossipMessage` variants:

- `SubnetClaim`, `SubnetClaimRelease`
- `ServicePublish`, `ServiceUnpublish`
- `Lease`, `LeasedName`
- `User`, `PeerAddr`, `NodeName`
- `Coordinate`, `LivenessBeacon`

## Invariants

- Decode failure on a single payload is logged and skipped -- never fatal
- The loop exits only on `GossipError::Closed`
- This file is iroh-free -- the transport deals in raw `bytes::Bytes`
