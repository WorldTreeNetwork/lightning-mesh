---
title: Wire Format Reference
created: 2025-11-05
updated: 2026-11-05
type: query
tags: [mesh-core, gossip]
sources: [crates/mjolnir-mesh/src/crdt/gossip.rs]
state: [CURRENT]
confidence: high
---

# Wire Format Reference

## Lightning Mesh Gossip

- **Serialization:** postcard. The discriminant is one byte. This is not a zero-copy format.
- **Envelope:** raw `bytes::Bytes`
- **Message types:** the `GossipMessage` enum in `crates/mjolnir-mesh/src/crdt/gossip.rs`. Variant order is the postcard discriminant.
- **Framing:** none. Each broadcast is one postcard message
- **Names, in discriminant order:** `LeaseUpdate`, `LeaseRelease`, `DnsUpdate`, `ServiceUpdate`, `SubnetClaimUpdate`, `SubnetClaimRelease`, `PeerAddrUpdate`, `UserUpdate`, `ServicePublishV2`, `ServiceUnpublishV2`, `LivenessBeacon`, `NodeNameAnnounce`, `LeasedNamePublish`, `CoordinateAnnounce`

Short names such as `SubnetClaim`, `ServicePublish`, and `Lease` are not variants. A decoder that uses them will not read this fleet.

Notes on another stack's packets are a contributor page: `wiki/claysaurus/appendix/wire-formats.md`.
