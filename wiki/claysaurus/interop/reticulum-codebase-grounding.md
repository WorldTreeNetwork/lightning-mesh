---
title: Reticulum grounding
created: 2025-11-05
updated: 2026-11-06
type: interop
tags: [reticulum, codebase, grounding, interop]
sources: [wiki/claysaurus/raw/notes/2026-11-06-reticulum-grounding.md]
state: [FUTURE]
confidence: low
---

# Reticulum grounding

> Sources: Clay's Reticulum notes, 2026-11-06
> Raw: [Reticulum grounding](../raw/notes/2026-11-06-reticulum-grounding.md)
> Updated: 2026-11-06

The draft named `Identity.py`, `Destination.py`, `Channel.py`, `Transport.py`, `Link.py`, `Interface.py`, and `Packet.py` in a Reticulum tree that is not this repository. Nothing here re-opened that tree. API details that were marked "verified" are the draft's reading, not a check against a checkout in this repo.

What the notes agree on, and what matches the mesh side:

- A Reticulum private identity is 64 bytes, X25519 then Ed25519. `Identity.load_private_key()` wants those 64 bytes.
- A mesh secret is a 32-byte `iroh::SecretKey`. Passing it to that loader does not produce a Reticulum identity.
- A destination hash is a truncated SHA-256, not the mesh node id.
- Links are a handshake with their own encryption. Mesh links are QUIC.

What this mesh does, checked here:

- One production `GossipTransport`: `IrohGossipTransport` in `mjolnir-meshd`.
- `tun/underlay.rs` is routing table 210. It is not a Reticulum interface.
- meshd does not spawn a `SerialInterface`, a KISS interface, or a LoRa radio.
