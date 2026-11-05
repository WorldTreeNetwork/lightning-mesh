---
title: mjolnir-meshd Binary
created: 2025-11-05
updated: 2025-11-05
type: concept
tags: [mesh-components, meshd]
sources: [crates/mjolnir-mesh/src/bin/mjolnir-meshd.rs]
state: [CURRENT]
confidence: high
---

# mjolnir-meshd Binary

The headless iroh-transport router daemon. Runs on OpenWrt routers as the core mesh process.

## Subcommands (MVP)

- `id` -- print this node's EndpointId and a shareable address blob
- `listen` -- accept inbound connections, echo ping datagrams
- `connect <addr>` -- dial a peer by address blob, measure a datagram round-trip

## Full Daemon

In production, the binary is compiled with the `daemon` feature and provides:

- CRDT state management (all book types)
- Gossip sync (iroh-gossip or alternative transport)
- Babeld configuration rendering and hot-reload
- DNS responder for .mesh zone
- ACME HTTPS issuance
- Overlay TUN management
- Owner claim ceremony
- Capability beacon and transport

## Build

- Cross-compiled static aarch64 for OpenWrt via `deploy/openwrt/build.sh`
- Pushed with `deploy/openwrt/install-node.sh`
- On macOS host: build fails without Linux target due to `spawn_overlay_tun` gating

## File Size

The binary source is ~11,555 lines -- the largest single file in the crate.
