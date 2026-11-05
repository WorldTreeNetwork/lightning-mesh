---
title: Lightning Mesh vs Reticulum
created: 2025-11-05
updated: 2026-11-05
type: comparison
tags: [interop, reticulum, mesh-core]
sources: [crates/mjolnir-mesh/, public/reticulum/]
state: [CURRENT]
confidence: high
---

# Lightning Mesh vs Reticulum

| Dimension | Lightning Mesh | Reticulum |
|---|---|---|
| **Language** | Rust | Python 3 |
| **Runtime** | Static binary (OpenWrt) | Userland Python process |
| **Transport** | Iroh (QUIC + NAT traversal) | Custom protocol (multi-hop) |
| **State Model** | CRDT, per-book merge | Transport tables + Resource sync |
| **Addressing** | Ed25519 node id, blake3 ULA and 10.254 address, /24 claims | Hierarchical destination hashes |
| **Crypto** | Ed25519 `iroh::SecretKey`, blake3, postcard | X25519 plus Ed25519 identity, SHA-256, AES-256-CBC, msgpack |
| **Routing** | Babeld (external process) | Built-in pathfinder |
| **Backhaul** | 802.11s WiFi mesh | Any interface (LoRa, serial, TCP, UDP, KISS) |
| **Overlay** | TUN device (mjolnir0) | Link-layer encrypted tunnels |
| **Service Discovery** | .mesh DNS + gossip CRDT | RNS name resolution + Resource ads |
| **Identity** | Iroh EndpointId (Ed25519 public key) | 64-byte Reticulum identity (X25519 + Ed25519) |
| **License** | AGPL-3.0 (dual: commercial) | Reticulum License (permissive + conditions) |
| **Protocol Status** | Public domain (protocol) | Public domain (since 2016) |
| **Implementation** | Licensed (AGPL-3.0) | Licensed (Reticulum License) |
| **Min Bandwidth** | ~1 Mbps (WiFi backhaul) | ~150 bps |
| **Min MTU** | ~1500 bytes | 500 bytes |
| **Target Hardware** | OpenWrt routers ($30-80) | Any Python 3 system + radios |
| **Field Status** | 5-router fleet, July 2026 | Global distributed backbone |

## Key Differences

### Strengths of Lightning Mesh

- **Performance** -- Rust binary, no Python overhead, optimized for router hardware
- **Maturity on target hardware** -- field-validated on OpenWrt routers
- **Rich service layer** -- hello.mesh directory, .mesh DNS, mini-apps, ACME HTTPS
- **Operational tooling** -- mjolnir-apply for staged updates, Lightning Admin desktop

### Strengths of Reticulum

- **Carrier diversity** -- LoRa, serial, KISS, AX.25, I2P, TCP, UDP, Pipe
- **Low bandwidth** -- works at 150 bps (vs Mesh's ~1 Mbps minimum)
- **Multi-hop routing** -- built-in, no overlay TUN needed
- **Established ecosystem** -- LXMF, NomadNet, Sideband, MeshChatX
- **Protocol maturity** -- 10+ years of development, global backbone

## Complementarity

The stacks are highly complementary:

1. **Mesh brings performance and operational maturity** to the OpenWrt router domain
2. **Reticulum brings carrier diversity** (LoRa, serial, KISS) that Mesh lacks
3. **The keys are different shapes.** A mesh node id is an Ed25519 public key. A Reticulum identity is a separate 64-byte blob. Reuse takes a derivation step, which [[interop-identity-sharing]] treats as future work
4. **GossipTransport trait** enables transport swap with minimal code

## Integration Priority

See the interop pages for detailed integration strategies:

- [[interop-gossip-transport]] -- Reticulum as GossipTransport (Phase 1)
- [[interop-identity-sharing]] -- Identity across the two key shapes (future)
- [[interop-name-translation]] -- Name resolution bridge (Phase 2)
- [[interop-carrier-extension]] -- LoRa/serial carriers (Phase 2)
- [[interop-bulk-sync]] -- CRDT reconciliation (Phase 3)
- [[interop-protocol-bridge]] -- Full gateway daemon (moon shot)
