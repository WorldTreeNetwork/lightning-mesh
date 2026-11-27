---
title: Rayfish invite
created: 2026-11-05
updated: 2026-11-06
type: interop
tags: [interop, rayfish, invite, admission]
sources: [wiki/claysaurus/raw/notes/2026-11-06-rayfish-invite.md]
state: [FUTURE]
confidence: low
---

# Rayfish invite

> Sources: Clay's Rayfish notes, 2026-11-06
> Raw: [Rayfish invite](../raw/notes/2026-11-06-rayfish-invite.md)
> Updated: 2026-11-06

Rayfish admission is a single-use invite checked by a coordinator, then a signed GroupBlob. The coordinator holds the network key and can admit, kick, and republish.

Mesh admission for a router is the owner-claim ceremony. Lightning Admin proves the node: a signature against the node id on the card, an HMAC under the derived card key, and a fresh 120-second button window. There is no coordinator and no invite code. A guild assigns a node after that ownership exists. Replacing the ceremony with Rayfish invites is not the household model.
