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
├── SCHEMA.md                 # This file — conventions and tag taxonomy
├── index.md                  # Shared catalog. Contributor pages are not listed
├── log.md                    # Shared notebook log
├── .contributor.example      # Copy to .contributor (gitignored) to choose a writer
├── concepts/                 # Accepted concept and entity pages
├── comparisons/              # Accepted side-by-side comparisons
├── interop/                  # Accepted interoperability notes
├── appendix/                 # Accepted reference tables
├── research/                 # Accepted carrier and hardware proposals
├── queries/                  # Accepted query pages
└── <name>/                   # One person's intake. Same folders as above
    ├── index.md
    ├── log.md
    ├── concepts/
    ├── comparisons/
    ├── interop/
    ├── appendix/
    ├── research/
    ├── queries/
    └── raw/                  # Immutable sources for that person's pages
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

## Contributors

A new concept, link, or paper enters in a contributor directory. The shared pages above are what this project has accepted as relevant to Lightning Mesh.

`wiki/.contributor` is gitignored. Copy `wiki/.contributor.example` and put one directory name on the first non-empty, non-comment line. The LLM wiki skill (`.grok/skills/llm-wiki`) reads that file and writes only under `wiki/<name>/`. If the file is missing, it does not guess a name and it does not write the shared notebook.

The name is one path segment of letters, digits, `.`, `_`, or `-`. It is not `.` or `..`, and it is not `concepts`, `comparisons`, `interop`, `appendix`, `research`, `queries`, or `raw`.

`wiki/<name>/` has the same folders as the shared notebook, plus `raw/` for sources that are kept as collected. It has its own `index.md` and `log.md`. It does not contain a second `SCHEMA.md`. Pages there use this file's frontmatter. A note that is not yet accepted is `[FUTURE]`.

The contributor `log.md` records ingest, query, and lint operations, including the `no material` heading the evidence check reads. The shared `wiki/log.md` records changes to the accepted notebook.

Promotion is explicit. When a concept, link, or paper is relevant here, write or update the matching shared page and cite the contributor page or its raw file. Leave the contributor page in place. The skill does not promote on its own.

## Page Thresholds

- Create a page when a concept appears in 3+ source locations or is referenced by 2+ other wiki pages
- Do not create pages for passing mentions
- Use `comparisons/` when two systems are discussed side-by-side
- Use `interop/` for integration patterns between Lightning Mesh and external systems
- Use `research/` for hardware and carrier proposals
