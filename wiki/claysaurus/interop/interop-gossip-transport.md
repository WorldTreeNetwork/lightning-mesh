---
title: Reticulum as a gossip transport
created: 2026-11-05
updated: 2026-11-06
type: interop
tags: [interop, reticulum, transport-swap]
sources: [crates/mjolnir-mesh/src/crdt/sync.rs, wiki/claysaurus/raw/notes/2026-11-06-reticulum-gossip.md]
state: [FUTURE]
confidence: low
---

# Reticulum as a gossip transport

> Sources: sync.rs and Clay's Reticulum notes, 2026-11-06
> Raw: [Reticulum gossip](../raw/notes/2026-11-06-reticulum-gossip.md)
> Updated: 2026-11-06

`GossipTransport` in `crates/mjolnir-mesh/src/crdt/sync.rs` is async `broadcast` and `recv` of raw bytes. `GossipSync` postcard-encodes `GossipMessage` and does not know which carrier delivered the bytes. The production implementation is `IrohGossipTransport` inside `mjolnir-meshd`. Tests have fakes. There is no `ReticulumTransport`, and there is no Cargo feature named `reticulum-transport`.

The notes say a Reticulum `Channel` can send and receive bytes, that `Channel.get` blocks, and that a Python sidecar would have to bridge that into the async trait. This repo does not contain `Channel.py`, so those method names stay the draft's claim.

Replacing iroh is not the mesh design. LAN mode is the default. Internet mode is opt-in. A second carrier would be a new implementation of the trait beside iroh, and it would not inherit QUIC. That work has not been accepted.
