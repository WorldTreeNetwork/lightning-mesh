---
title: Wiki Index — Lightning Mesh
created: 2026-11-05
updated: 2026-11-05
type: summary
tags: [wiki-admin]
sources: [wiki/SCHEMA.md]
state: [CURRENT]
---

# Wiki Index — Lightning Mesh

Specified behavior is in `openspec/specs/`. Design is in `docs/`. This index lists the shared notebook.

Contributor notes live under `wiki/<name>/` and are not listed here. `wiki/.contributor` chooses the directory this checkout writes. The rule is in `wiki/SCHEMA.md`.

## Concepts

- [[babel-routing]] — Babeld over the backhaul. procd owns the process
- [[crdt-overlay]] — Per-book merge rules for mesh state
- [[gossip-transport]] — `GossipTransport` and `IrohGossipTransport`
- [[hello-mesh]] — hello.mesh front desk and service directory
- [[iroh-transport]] — Ed25519 identity. LAN is the default; internet mode is opt-in
- [[liveness-plane]] — Ephemeral per-node liveness beacons
- [[meshd-binary]] — `mjolnir-meshd` and the `mesh` subcommand
- [[overlay-tun]] — Per-peer /31 tunnels and the `mjolnir0` overlay
- [[owner-claim]] — Device proof, card HMAC, and the button window

## Research

- [[sdr-radio-integration]] — `GossipTransport` as the seam for a second carrier. `radio.rs` is telemetry
- [[sdr-generic-soapysdr]] — One proposed SoapySDR transport for many devices
- [[limesdr-bladerf-carrier-mvp]] — LimeSDR and bladeRF as a point-to-point carrier
- [[baofeng-tnc-aprs-reticulum]] — Baofeng, DireWolf KISS, and a proposed `broadcast` / `recv` impl

## Appendix

- [[crypto-primitives]] — Mesh Ed25519 identity and blake3 addresses
- [[wire-formats]] — Postcard `GossipMessage` names
