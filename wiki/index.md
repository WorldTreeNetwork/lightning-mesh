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

Specified behavior is in `openspec/specs/`. Design is in `docs/`. This index lists notebook pages.

## Concepts

- [[babel-routing]] — Babeld over the backhaul. procd owns the process
- [[crdt-overlay]] — Per-book merge rules for mesh state
- [[gossip-transport]] — `GossipTransport` and `IrohGossipTransport`
- [[hello-mesh]] — hello.mesh front desk and service directory
- [[iroh-transport]] — Ed25519 identity, staging relays, QUIC
- [[liveness-plane]] — Ephemeral per-node liveness beacons
- [[meshd-binary]] — `mjolnir-meshd` and the `mesh` subcommand
- [[overlay-tun]] — Per-peer /31 tunnels and the `mjolnir0` overlay
- [[owner-claim]] — Device proof, card HMAC, and the button window

## Interoperability

### Rayfish

- [[rayfish-overview]] — Rayfish as an iroh VPN with a coordinator
- [[rayfish-identity-model]] — One Ed25519 key, device certs, positional msgpack
- [[rayfish-codebase-grounding]] — Notes from the Rayfish sources
- [[interop-rayfish-transport]] — Transport patterns
- [[interop-rayfish-addressing]] — Identity-derived addresses
- [[interop-rayfish-firewall]] — Stateful firewall on an overlay TUN
- [[interop-rayfish-dns]] — Magic DNS patterns
- [[interop-rayfish-invite]] — Invite and admission

### Reticulum

- [[reticulum-codebase-grounding]] — Notes from the Reticulum sources
- [[interop-gossip-transport]] — A Reticulum channel as a future `GossipTransport`
- [[interop-identity-sharing]] — The two identity blobs, and a derivation that is not built
- [[interop-name-translation]] — `.mesh` names and Reticulum destinations
- [[interop-carrier-extension]] — Carrier ideas. meshd does not spawn Reticulum interfaces
- [[interop-bulk-sync]] — Resource sync for CRDT reconciliation
- [[interop-protocol-bridge]] — A gateway, marked future

## Comparisons

- [[mesh-vs-rayfish]] — CRDT mesh beside a coordinator-admitted iroh VPN
- [[mesh-vs-reticulum]] — Ed25519 iroh mesh beside Reticulum's 64-byte identity

## Research

- [[sdr-radio-integration]] — `GossipTransport` as the seam for a second carrier. `radio.rs` is telemetry
- [[sdr-generic-soapysdr]] — One proposed SoapySDR transport for many devices
- [[limesdr-bladerf-carrier-mvp]] — LimeSDR and bladeRF as a point-to-point carrier
- [[baofeng-tnc-aprs-reticulum]] — Baofeng, DireWolf KISS, and a proposed `broadcast` / `recv` impl

## Appendix

- [[crypto-primitives]] — Mesh Ed25519 identity, and Reticulum's separate key blob
- [[wire-formats]] — Postcard `GossipMessage` names, and Reticulum packets
