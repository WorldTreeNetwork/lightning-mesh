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
- The backhaul address is one host in `10.254.0.0/16`. `backhaul_addr` takes the first 16 bits of `blake3(node id)`. The node does not own that `/16`.
- Per-peer tunnel links are IPv4 /31s in `10.255.0.0/16`, from `pick_link_31`.
- The identity ULA is `ula_addr(node_id)`. The house prefix is `house_ula_prefix` of the guild id in `/etc/mjolnir/house-guild-id`. Those prefixes differ, and neither is derived from the `10.254` address.

## Platform Support

The TUN data plane is Linux-only. Gossip does not require a TUN.
