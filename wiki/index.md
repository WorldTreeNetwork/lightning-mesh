---
title: Wiki Index — Lightning Mesh
created: 2026-11-05
updated: 2026-11-05
type: summary
tags: [wiki-admin]
sources: [wiki/index.md]
state: [CURRENT]
---

# Wiki Index — Lightning Mesh

## Concepts

- [[crdt-overlay]] — CRDT-based shared state for mesh coordination
- [[gossip-transport]] — Transport-agnostic gossip sync seam
- [[iroh-transport]] — Iroh as the default gossip and overlay transport
- [[babel-routing]] — Babeld routing over the mesh backhaul
- [[overlay-tun]] — mjolnir0 TUN device for cross-site Iroh traffic
- [[hello-mesh]] — hello.mesh front desk and service directory
- [[owner-claim]] — Household owner-claim ceremony and recovery
- [[liveness-plane]] — Ephemeral per-node liveness tracking
- [[meshd-binary]] — The mjolnir-meshd daemon binary

## Interoperability

### Reticulum

- [[reticulum-codebase-grounding]] — Verified Reticulum source code analysis (Identity, Link, Channel, Destination, Transport, Interface)
- [[interop-gossip-transport]] — Reticulum as GossipTransport (Phase 1)
- [[interop-identity-sharing]] — Shared Curve25519 identity across stacks
- [[interop-name-translation]] — .mesh ↔ Reticulum name resolution bridge
- [[interop-carrier-extension]] — Reticulum carriers on OpenWrt hardware
- [[interop-bulk-sync]] — Reticulum Resource sync for CRDT reconciliation
- [[interop-protocol-bridge]] — Full mesh↔reticulum gateway daemon (moon shot)

### Rayfish

- [[rayfish-codebase-grounding]] — Verified Rayfish source code analysis (Identity, Membership, Transport, Forward, Firewall, DNS, Invite, Peers)
- [[rayfish-identity-model]] — Single Ed25519 key, device certs, derive_ipv6(), ExitFamilies, positional msgpack wire
- [[rayfish-overview]] — Rayfish P2P VPN: architecture and key facts
- [[interop-rayfish-transport]] — Transport hardening patterns (Phase 1)
- [[interop-rayfish-addressing]] — Deterministic addressing from identity (Phase 1)
- [[interop-rayfish-firewall]] — Stateful firewall on overlay TUN (Phase 2)
- [[interop-rayfish-dns]] — Magic DNS patterns (Phase 2)
- [[interop-rayfish-invite]] — Invite and admission model (Phase 3)

## Comparisons

- [[mesh-vs-reticulum]] — Lightning Mesh vs Reticulum: architecture comparison
- [[mesh-vs-rayfish]] — Lightning Mesh vs Rayfish: CRDT vs coordinator model

## Research

- [[sdr-radio-integration]] — SDR/Radio integration overview: landscape, integration strategy, GFSK modem
- [[baofeng-tnc-aprs-reticulum]] — Baofeng VHF/UHF radio + DireWolf TNC + APRS/AX.25 + Reticulum integration
- [[limesdr-bladerf-carrier-mvp]] — LimeSDR/bladeRF MVP design: GFSK modem, SdrGossipTransport, two-node plan
- [[sdr-generic-soapysdr]] — Generic SDR abstraction via SoapySDR: one codebase for all devices, RX-only support, SoapyRemote

## Appendix

- [[crypto-primitives]] — Authoritative crypto primitives for both stacks
- [[wire-formats]] — Serialization formats: postcard, msgpack, Fernet tokens
