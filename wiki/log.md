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

## 2026-11-05 — contributor intake

- New concepts, links, and papers land in `wiki/<name>/` first, with the same folders as this notebook.
- `wiki/.contributor` is gitignored and selects the directory. The shared pages are the accepted set.

## 2026-11-05 — concept atlas

- Added nine concept pages and two appendix pages.
- Gossip variant names, merge rules, babel ownership, the node id, and the meshd CLI match the tree.

## 2026-11-05 — atlas corrected

- Addressing, merge rules, the LAN-mode default, owner-claim HMAC, babel interface types, and hello.mesh embed rules now match the tree.
- `mjolnir-meshd` does not start ACME. The Reticulum crypto and packet catalogs moved to `wiki/claysaurus/`.

## 2026-11-06 — Rayfish notes

- Clay's Rayfish reading lives in `wiki/claysaurus/`. It is not the shared notebook.
- The node backhaul (`10.254`) and the identity ULA are already derived from the node id. A client `/24` is a claim.
