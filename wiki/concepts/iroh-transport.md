---
title: Iroh Transport
created: 2025-11-05
updated: 2026-11-05
type: concept
tags: [mesh-core, iroh]
sources: [crates/mjolnir-mesh/src/bin/mjolnir-meshd.rs, crates/mjolnir-mesh/src/tun/link.rs]
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

`mesh` runs LAN mode unless internet mode is opted into. LAN mode uses `RelayMode::Disabled` and mDNS, with no pkarr. Internet mode uses the N0 preset, `RelayMode::Staging` (the n0 staging relays) plus hole-punching, and still adds mDNS. A wildcard bind is rejected. `build_endpoint` in `mjolnir-meshd` chooses the preset and the relay list. The embassy radios that run internet mode were switched to it; the binary does not default there.

## Key Derivation

The node id is that Ed25519 public key. `blake3` derives the identity ULA and the `10.254` backhaul host from the node id. It does not derive the node id itself. A Reticulum identity is a separate 64-byte blob (X25519 plus Ed25519) and is not this key.
