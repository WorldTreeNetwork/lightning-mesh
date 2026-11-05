---
title: Overlay TUN
created: 2025-11-05
updated: 2025-11-05
type: concept
tags: [mesh-core, tun]
sources: [crates/mjolnir-mesh/src/tun/]
state: [CURRENT]
confidence: high
---

# Overlay TUN

The overlay TUN device (`mjolnir0`) carries cross-site Iroh traffic between mesh nodes.

## Architecture

- **`spawn_overlay_tun()`** -- creates the TUN device with an identity-derived IPv6 address
- **`spawn_tunnel()`** -- creates per-peer TUN tunnels over Iroh QUIC datagrams
- **`EncapHandles`** -- manages the encap/decap loops; cancellation is `abort`, not drop
- **`Fib`** -- forward information base for routing decisions
- **`NeighborRoutes`** -- tracks routes to known peers

## Addressing

- Overlay link-local uses the blake3 interface ID, not the last 16 bits of 10.254
- Each peer gets a /128 on the overlay
- The mesh's house /48 is derived from the guild ID

## Platform Support

`spawn_overlay_tun` is Linux-only (`#[cfg(target_os = "linux")]`). The `mjolnir-node` standalone binary provides a gossip-only mode that does not require a TUN.
