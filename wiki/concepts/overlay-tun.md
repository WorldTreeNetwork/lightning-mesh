---
title: Overlay TUN
created: 2025-11-05
updated: 2026-11-05
type: concept
tags: [mesh-core, tun]
sources: [crates/mjolnir-mesh/src/tun/]
state: [CURRENT]
confidence: high
---

# Overlay TUN

Cross-site traffic rides Iroh. Two TUN shapes exist in `crates/mjolnir-mesh/src/tun/`.

## Architecture

- **Per-peer tunnels** -- `spawn_encap_pair` (re-exported from `tun/encap.rs`) bridges one TUN to one iroh connection. `EncapHandles` owns those tasks.
- **`mjolnir0` overlay** -- `spawn_overlay` and `spawn_overlay_routed` in `tun/overlay.rs` multiplex one TUN across many peer connections. `OverlayHandles` tears that down by aborting the tasks.
- **`Fib`** (`tun/fib.rs`) and **`NeighborRoutes`** (`tun/neighbor.rs`) are the forwarding tables beside those devices.
- **Underlay policy** is `tun/underlay.rs`: outer routing table 210 and the relay-route rules. It is not a second TUN.

## Addressing

- The client LAN is a routed `10.42.0.0/16` /24 per node.
- The backhaul address is `10.254.<blake3(node id)>/16` (`backhaul_addr` in `tun/link.rs`).
- Per-peer tunnel links are IPv4 /31s from `pick_link_31`.
- The identity ULA and the house prefix come from `ula_addr` and the guild id, not from the `10.254` address.

## Platform Support

The TUN data plane is Linux-only. Gossip does not require a TUN.
