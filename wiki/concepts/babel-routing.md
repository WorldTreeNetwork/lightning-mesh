---
title: Babel Routing
created: 2025-11-05
updated: 2025-11-05
type: concept
tags: [mesh-core, babel]
sources: [crates/mjolnir-mesh/src/babel/]
state: [CURRENT]
confidence: high
---

# Babel Routing

Babeld is run as an external process on each node. Lightning Mesh does not own the babeld lifecycle -- it is managed by procd via the `mjolnir-babeld` service.

## Role

- Routes between nodes over the 802.11s backhaul (`br-mesh`)
- The mesh daemon renders babeld configuration via `render_babeld_conf()` and `render_overlay_babeld_conf()`
- Routes are supervised: the daemon checks that babeld is running and restarts it on failure

## Configuration

The mesh daemon writes babel config to `/etc/config/mjolnir-babeld` (or similar). The config includes:

- Overlay interface addresses
- Backhaul interface settings
- Admitted prefix lists
- Route export rules

## Key Design

- Babeld is a separate process, not linked into the Rust binary
- Configuration is rendered from CRDT state -- no manual config editing
- The daemon watches for config changes and triggers hot-reloads
