---
title: Crypto Primitives Reference
created: 2025-11-05
updated: 2026-11-05
type: query
tags: [security, crypto, ed25519]
sources: [crates/mjolnir-mesh/src/tun/link.rs, crates/mjolnir-mesh/src/bin/mjolnir-meshd.rs]
state: [CURRENT]
confidence: high
---

# Crypto Primitives Reference

| Primitive | Algorithm | Source | Notes |
|---|---|---|---|
| Identity key | Ed25519 | `iroh::SecretKey` | 32-byte seed. The node id is the hex public key (`EndpointId`) |
| Identity ULA | blake3 | `ula_addr` in `tun/link.rs` | Prefix from `ula_prefix`, interface id from `blake3(node id)`. This does not derive the node id |
| Backhaul address | blake3 | `backhaul_addr` in `tun/link.rs` | One host in `10.254.0.0/16`, from the first 16 bits of `blake3(node id)` |
| Link encryption | QUIC TLS | iroh | Mesh links are not a second cipher we choose |

A Reticulum identity is a separate 64-byte blob (X25519 plus Ed25519) and is not this key. That catalog is a contributor note: `wiki/claysaurus/appendix/crypto-primitives.md`.
