---
title: Wiki SCHEMA — Lightning Mesh
created: 2026-11-05
updated: 2026-11-05
type: summary
tags: [wiki-admin]
sources: [openspec/AGENTS.md, docs/]
---

# Wiki SCHEMA — Lightning Mesh

## Purpose

This directory is a research notebook: concept notes, comparisons, and interoperability research. Specified behavior lives in `openspec/specs/`. Design lives in `docs/`. A page marked `[CURRENT]` describes the tree as it is. A page marked `[FUTURE]` is a proposal.

## Directory Structure

```
wiki/
├── SCHEMA.md          # This file — conventions and tag taxonomy
├── index.md           # Content catalog — all pages listed by category
├── log.md             # Action log — wiki creation and updates
├── concepts/          # Concept and entity pages (code, protocols, architecture)
├── comparisons/       # Side-by-side comparisons
├── interop/           # Interoperability research and integration patterns
├── appendix/          # Crypto reference and wire format tables
├── research/          # Carrier and hardware proposals
├── raw/               # Raw source documents (meeting notes, external docs)
└── queries/           # Query and search reference pages
```

## Frontmatter Convention

Every page starts with YAML frontmatter:

```yaml
---
title: Page Title
created: YYYY-MM-DD
updated: YYYY-MM-DD
type: concept | comparison | interop | query | summary | research
tags: [from taxonomy below]
sources: [path/in/repo.md]
state: [CURRENT] | [FUTURE]
confidence: high | medium | low
---
```

### State Indicators

- `[CURRENT]` — matches the code or docs in this repository
- `[FUTURE]` — a proposal. It is not something the tree implements

### Source References

Claims about this repository cite a file under `crates/`, `docs/`, or `openspec/`. Claims about another project cite that project. A page does not invent symbols for this tree.

## Tag Taxonomy

- **mesh-core:** mesh-core, crdt, gossip, babel, tun, iroh, overlay
- **mesh-components:** meshd, meshctl, hello, apply, admin-capabilities, app-tun
- **interop:** reticulum, rayfish, transport-swap, identity-sharing, name-translation, carrier-extension, vpn, invite, admission, firewall, addressing, transport, dns, naming, identity, device-cert, overlay-filter, codebase, grounding
- **hardware:** openwrt, routeros, ap3000, tr3000, lorawan, serial, sdr, soapysdr, radio, limesdr, bladerf, tnc, aprs, baofeng, kiss, ax.25, carrier, generic, rx-only, abstraction, mvp
- **security:** crypto, curve25519, ed25519, x25519, aes-256, hkdf
- **operations:** fleet, deploy, node-operations, backhaul, roaming
- **admin:** guilds, keyspace, owner-claim, admin-capabilities, ceremony
- **wiki-admin:** wiki-admin (only on SCHEMA.md, index.md, and log.md)

Add a tag to this list in the same change that first uses it.

## Page Thresholds

- Create a page when a concept appears in 3+ source locations or is referenced by 2+ other wiki pages
- Do not create pages for passing mentions
- Use `comparisons/` when two systems are discussed side-by-side
- Use `interop/` for integration patterns between Lightning Mesh and external systems
- Use `research/` for hardware and carrier proposals
