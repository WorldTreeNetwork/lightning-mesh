---
title: claysaurus — contributor index
created: 2026-11-05
updated: 2026-11-06
type: summary
tags: [wiki-admin]
sources: [wiki/SCHEMA.md]
state: [CURRENT]
---

# claysaurus

Intake notes for Clay. These pages are not the shared notebook. The shared catalog is `wiki/index.md`.

## Interoperability

- [[rayfish-overview]] — Rayfish, an iroh VPN with a coordinator
- [[rayfish-identity-model]] — One Ed25519 key and device certs
- [[rayfish-codebase-grounding]] — What these notes actually checked
- [[interop-rayfish-addressing]] — Rayfish derives the overlay address; mesh already derives the node address
- [[interop-rayfish-transport]] — Transport patterns that are not this mesh
- [[interop-rayfish-firewall]] — Stateful overlay firewall Rayfish has
- [[interop-rayfish-dns]] — `.ray` Magic DNS patterns
- [[interop-rayfish-invite]] — Signed invite and a coordinator

## Comparisons

- [[mesh-vs-rayfish]] — This mesh beside Rayfish
- [[mesh-vs-reticulum]] — This mesh beside Reticulum

## Reticulum

- [[reticulum-codebase-grounding]] — What these notes actually checked
- [[interop-identity-sharing]] — Two different private keys
- [[interop-gossip-transport]] — A proposed second `GossipTransport`, not built
- [[interop-name-translation]] — A proposed `.mesh` to destination bridge
- [[interop-carrier-extension]] — Radio and serial ideas. meshd does not spawn them
- [[interop-bulk-sync]] — A proposed snapshot transfer
- [[interop-protocol-bridge]] — A proposed gateway

## Research

- [[sdr-radio-integration]] — `radio.rs` is telemetry. A second carrier is not built
- [[sdr-generic-soapysdr]] — A proposed SoapySDR transport. Nothing here calls SoapySDR
- [[limesdr-bladerf-carrier-mvp]] — A proposed LimeSDR or bladeRF link
- [[baofeng-tnc-aprs-reticulum]] — A proposed Baofeng and KISS path

## Appendix

- [[crypto-primitives]] — Reticulum cipher and identity catalog
- [[wire-formats]] — Reticulum packet layout notes
