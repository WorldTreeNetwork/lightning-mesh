---
title: Wiki SCHEMA — Lightning Mesh
created: 2026-11-05
updated: 2026-11-05
type: summary
tags: [wiki-admin]
sources: [wiki/SCHEMA.md]
---

# Wiki SCHEMA — Lightning Mesh

## Purpose

This wiki captures the Lightning Mesh codebase, architecture decisions, and cross-stack interoperability research. It is the single source of truth for new contributors and the reference for active development.

## Directory Structure

```
wiki/
├── SCHEMA.md          # This file — conventions and tag taxonomy
├── index.md           # Content catalog — all pages listed by category
├── log.md             # Action log — wiki creation and updates
├── concepts/          # Concept and entity pages (code, protocols, architecture)
├── comparisons/       # Side-by-side comparisons (mesh vs reticulum, etc.)
├── interop/           # Interoperability research and integration patterns
├── appendix/          # Glossary, crypto reference, wire format tables
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
type: concept | comparison | interop | query | summary
tags: [from taxonomy below]
sources: [raw/source-document.md]
state: [CURRENT] | [FUTURE]
# Optional quality signals:
confidence: high | medium | low
contested: true
contradictions: [other-page-slug]
---
```

### State Indicators

- `[CURRENT]` — current state of repo/code/features
- `[FUTURE]` — intended future state of repo/code/features

### Source References

Every claim must have a source reference. The `sources:` field points to the raw source (file, commit, or document) that backs the content.

## Tag Taxonomy

- **mesh-core:** mesh-core, crdt, gossip, babel, tun, iroh, overlay
- **mesh-components:** meshd, meshctl, hello, apply, admin-capabilities, app-tun
- **interop:** reticulum, transport-swap, identity-sharing, name-translation, carrier-extension
- **hardware:** openwrt, routeros, ap3000, tr3000, lorawan, serial
- **security:** crypto, curve25519, ed25519, x25519, aes-256, hkdf
- **operations:** fleet, deploy, node-operations, backhaul, roaming
- **admin:** guilds, keyspace, owner-claim, admin-capabilities, ceremony
- **wiki-admin:** wiki-admin (only on SCHEMA.md and index.md)

Rule: every tag on a page must appear in this taxonomy. If a new tag is needed, add it here first, then use it.

## Page Thresholds

- Create a page when a concept appears in 3+ source locations or is referenced by 2+ other wiki pages
- Do NOT create pages for passing mentions
- Use `comparisons/` when two systems are discussed side-by-side
- Use `interop/` for integration patterns between Lightning Mesh and external systems
