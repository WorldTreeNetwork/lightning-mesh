---
title: Wiki Log — Lightning Mesh
created: 2026-11-05
updated: 2026-11-05
type: summary
tags: [wiki-admin]
sources: [wiki/SCHEMA.md]
state: [CURRENT]
---

# Wiki Log — Lightning Mesh

## 2026-11-05

- Added `SCHEMA.md`, `index.md`, and this log.
- The notebook points at `openspec/specs/` and `docs/` for specified behavior and design.

## 2026-11-05 — concept atlas

- Added nine concept pages and two appendix pages.
- Gossip variant names, merge rules, babel ownership, the node id, and the meshd CLI match the tree.

## 2026-11-05 — Rayfish notes

- Added the Rayfish overview, identity model, grounding notes, five interop pages, and `mesh-vs-rayfish`.
- These pages describe another codebase. They do not change Lightning Mesh.

## 2026-11-05 — Reticulum interop

- Added the Reticulum grounding notes, six interop pages, and `mesh-vs-reticulum`.
- Identity sharing stays a proposal. The mesh key is an Ed25519 `iroh::SecretKey`. A Reticulum identity is 64 bytes.

## 2026-11-05 — radio research

- Added four carrier notes: SDR survey, SoapySDR, LimeSDR/bladeRF, and Baofeng/KISS.
- The seam is `GossipTransport::broadcast` / `recv`. The only production impl is `IrohGossipTransport`.
- `radio.rs` is `iw` telemetry. `tun/underlay.rs` is table 210, not a second gossip transport.

## 2026-11-05 — contributor intake

- New concepts, links, and papers land in `wiki/<name>/` first, with the same folders as this notebook.
- `wiki/.contributor` is gitignored and selects the directory. The shared pages are the accepted set.
