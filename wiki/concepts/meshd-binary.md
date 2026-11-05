---
title: mjolnir-meshd Binary
created: 2025-11-05
updated: 2026-11-05
type: concept
tags: [mesh-components, meshd]
sources: [crates/mjolnir-mesh/src/bin/mjolnir-meshd.rs]
state: [CURRENT]
confidence: high
---

# mjolnir-meshd Binary

The headless iroh-transport router daemon. Runs on OpenWrt routers as the core mesh process.

## Subcommands

The `Command` enum in `mjolnir-meshd.rs` is the contract. Router deployments run `mesh`.

- `mesh` -- the daemon: roster, gossip, babel config, DNS, tunnels
- `id` -- print this node's EndpointId and a shareable address blob
- `status` -- read identity, addresses, and kernel routes without a running daemon
- `publish` / `unpublish` -- talk to the running daemon's control API
- `listen` / `connect` -- datagram echo probes
- `tun-test` / `tun-listen` / `tun-connect` -- TUN bring-up probes

`daemon` is `required-features` for the binary. It is how the crate's iroh dependencies are turned on, not a second mode that upgrades the echo probes into the daemon.

## What mesh runs

- CRDT books and gossip sync through `IrohGossipTransport`
- Babeld configuration rendering. procd's `mjolnir-babeld` owns the process
- DNS for the `.mesh` zone
- Optional ACME HTTPS issuance
- Per-peer tunnels, and the `mjolnir0` overlay when that mode is on
- Owner claim and the capability beacon

## Build

- Cross-compiled static aarch64 for OpenWrt via `deploy/openwrt/build.sh`
- Installed with `deploy/openwrt/install-node.sh`
- The TUN data plane is Linux-only. Host tests of the CRDT and the babel renderer do not need a TUN

## File Size

`mjolnir-meshd.rs` is about 11,500 lines, the largest source file in the crate.
