---
title: Iroh Transport
created: 2025-11-05
updated: 2026-11-05
type: concept
tags: [mesh-core, iroh]
sources: [crates/mjolnir-mesh/src/crdt/sync.rs, crates/mjolnir-mesh/src/tun/]
state: [CURRENT]
confidence: high
---

# Iroh Transport

Iroh is the default transport for Lightning Mesh. It provides QUIC-based connectivity, NAT traversal, and cryptographic identity.

## Role in the Mesh

1. **Gossip transport** -- `IrohGossipTransport` in `mjolnir-meshd` implements `GossipTransport` over `iroh_gossip`. The daemon accepts `iroh_gossip::ALPN` on the endpoint router.
2. **Overlay transport** -- Per-peer TUN tunnels, and the optional `mjolnir0` overlay TUN, carry cross-site traffic over Iroh QUIC datagrams.
3. **Identity** -- Each node has an `EndpointId`, which is the hex Ed25519 public key of an `iroh::SecretKey`.

## NAT Traversal

Internet mode uses `RelayMode::Staging` (the n0 staging relays) plus hole-punching. LAN mode disables relays and discovers peers with mDNS. The preset and the relay list are chosen in `build_endpoint` in `mjolnir-meshd`.

## Key Derivation

The node id is that Ed25519 public key. `blake3` derives the identity ULA and the `10.254` backhaul address from the node id. It does not derive the node id itself. A Reticulum identity is a separate 64-byte blob (X25519 plus Ed25519) and is not this key.
