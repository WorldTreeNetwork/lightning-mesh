---
title: Baofeng, KISS, and Reticulum
created: 2026-11-05
updated: 2026-11-06
type: interop
tags: [interop, transport, radio, baofeng, aprs]
sources: [wiki/claysaurus/raw/notes/2026-11-06-baofeng.md]
state: [FUTURE]
confidence: low
---

# Baofeng, KISS, and Reticulum

> Sources: Clay's radio notes, 2026-11-06
> Raw: [Baofeng and KISS](../raw/notes/2026-11-06-baofeng.md)
> Updated: 2026-11-06

The draft proposes a Baofeng handheld, a USB sound card, and DireWolf speaking KISS on TCP port 8001. A future `KissTransport` would implement `broadcast` and `recv` on that socket. That implementation does not exist. The mesh license is AGPL-3.0-or-later.

The draft then says the mesh only has to implement `GossipTransport` on a Reticulum `Channel`, and Reticulum will supply KISS, AX.25, and LoRa. That is not the design. meshd does not spawn a Reticulum interface. A KISS carrier would be our code. Bytes on it would not be inside iroh's QUIC.

DireWolf, a UV-5R, and port 8001 are the draft's bench setup. This tree does not connect to them. The live backhaul is still 802.11s and wired node-links.
