---
title: Gossip Transport
created: 2025-11-05
updated: 2026-11-05
type: concept
tags: [mesh-core, crdt, iroh]
sources: [crates/mjolnir-mesh/src/crdt/sync.rs]
state: [CURRENT]
confidence: high
---

# Gossip Transport

`GossipTransport` is the seam that lets `mjolnir-mesh` gossip CRDT state without depending on iroh. The daemon supplies the iroh implementation.

## Architecture

- **`GossipTransport` trait** -- async `broadcast(&self, payload: Bytes) -> Result<(), GossipError>` and `recv(&self) -> Result<Bytes, GossipError>`. It lives in `crates/mjolnir-mesh/src/crdt/sync.rs`.
- **`GossipSync<T>`** -- encodes and decodes `GossipMessage` with postcard and runs the receive loop. It owns no CRDT books.
- **Production impl** -- `IrohGossipTransport` in `crates/mjolnir-mesh/src/bin/mjolnir-meshd.rs` wraps `iroh_gossip::GossipSender` and `GossipReceiver`. `iroh_gossip::Gossip` itself is not a `GossipTransport`. Tests also implement the trait (`MockTransport`, `ChannelTransport`, `NullTransport`).

## Wire Format

Messages are postcard-serialized `GossipMessage` variants from `crates/mjolnir-mesh/src/crdt/gossip.rs`:

- `LeaseUpdate`, `LeaseRelease`
- `DnsUpdate`
- `ServiceUpdate`, `ServicePublishV2`, `ServiceUnpublishV2`
- `SubnetClaimUpdate`, `SubnetClaimRelease`
- `PeerAddrUpdate`, `UserUpdate`, `NodeNameAnnounce`, `LeasedNamePublish`
- `CoordinateAnnounce`, `LivenessBeacon`

The discriminant is a postcard integer. A hand-rolled enum with different names is not wire-compatible.

## Invariants

- Decode failure on a single payload is logged and skipped -- never fatal
- The loop exits only on `GossipError::Closed`
- This file is iroh-free -- the transport deals in raw `bytes::Bytes`
