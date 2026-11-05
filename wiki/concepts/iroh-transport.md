---
title: Iroh Transport
created: 2025-11-05
updated: 2025-11-05
type: concept
tags: [mesh-core, iroh]
sources: [crates/mjolnir-mesh/src/crdt/sync.rs, crates/mjolnir-mesh/src/tun/]
state: [CURRENT]
confidence: high
---

# Iroh Transport

Iroh is the default transport for Lightning Mesh. It provides QUIC-based connectivity, NAT traversal, and cryptographic identity.

## Role in the Mesh

1. **Gossip transport** -- `iroh_gossip::Gossip` implements `GossipTransport` for CRDT state propagation
2. **Overlay transport** -- Per-peer TUN tunnels over Iroh QUIC datagrams carry cross-site traffic
3. **Identity** -- Each node has an Iroh `EndpointId` derived from a `SecretKey` (Curve25519)

## NAT Traversal

Iroh uses relays and hole-punching to connect nodes behind symmetric NATs. The relay configuration is loaded from the node's runtime config.

## Key Derivation

The mesh node `SecretKey` is a Curve25519 key. The Iroh `EndpointId` is derived from the public key. This same key can serve as a Reticulum Identity (see [[interop-identity-sharing]]).

## Build Note

`spawn_overlay_tun` is gated behind `#[cfg(target_os = "linux")]`. Building on macOS requires the `mjolnir-node` binary or a Linux cross-target.
