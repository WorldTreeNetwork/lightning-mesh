---
title: Rayfish transport patterns
created: 2026-11-05
updated: 2026-11-06
type: interop
tags: [interop, rayfish, transport]
sources: [wiki/claysaurus/raw/notes/2026-11-06-rayfish-transport.md]
state: [FUTURE]
confidence: low
---

# Rayfish transport patterns

> Sources: Clay's Rayfish notes, 2026-11-06
> Raw: [Rayfish transport](../raw/notes/2026-11-06-rayfish-transport.md)
> Updated: 2026-11-06

The draft called `transport.rs` a 57K-line catalog of drop-in hardening. That size is not credible next to the same draft's 132K-line repository, and this repo does not contain the file. Nothing here is a patch for `mjolnir-meshd`.

The notes describe patterns Rayfish has and this mesh does not claim:

- A versioned ALPN, current version 7, still accepting v5 and v6. Mesh gossip uses one topic, `mjolnir/mesh/crdt/v0`, and does not keep older ALPNs alive.
- Filtering overlay ranges out of iroh's direct addresses. Mesh refuses Unique Local candidates. It does not drop `100.64.0.0/10`.
- One QUIC connection per peer device, with the network chosen in band.

`GossipTransport` stays the iroh seam inside `mjolnir-mesh`. These patterns are not a second implementation of that trait.
