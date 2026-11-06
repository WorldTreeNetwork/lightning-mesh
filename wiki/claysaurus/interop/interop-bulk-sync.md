---
title: Bulk sync
created: 2025-11-05
updated: 2026-11-06
type: interop
tags: [interop, reticulum, crdt]
sources: [wiki/claysaurus/raw/notes/2026-11-06-reticulum-bulk-sync.md]
state: [FUTURE]
confidence: low
---

# Bulk sync

> Sources: Clay's Reticulum notes, 2026-11-06
> Raw: [Reticulum bulk sync](../raw/notes/2026-11-06-reticulum-bulk-sync.md)
> Updated: 2026-11-06

The draft says a Reticulum `Resource` could carry a full CRDT snapshot after a partition. This tree has no `serialize_snapshot`, no `apply_snapshot`, and no Reticulum resource handler.

Gossip today is one postcard `GossipMessage` per broadcast. Each book merges on its own rule. A full snapshot would be a new message and a new merge path. Catch-up is not "the HLC is N ticks behind," and the books do not share one clock that means that.

Using another stack's file transfer for that snapshot is a proposal. It is not how a node rejoins now.
