---
title: Rayfish overview
created: 2026-11-05
updated: 2026-11-06
type: interop
tags: [interop, rayfish, vpn]
sources: [wiki/claysaurus/raw/notes/2026-11-06-rayfish-overview.md]
state: [FUTURE]
confidence: low
---

# Rayfish overview

> Sources: Clay's Rayfish notes, 2026-11-06
> Raw: [Rayfish overview](../raw/notes/2026-11-06-rayfish-overview.md)
> Updated: 2026-11-06

Rayfish is a P2P VPN on iroh. One endpoint and one TUN are shared across networks. Peers exchange packets when they share a network. A signed GroupBlob, one pkarr record per network, is the membership record. A holder of the network key can admit, kick, and republish. The room id is a discovery key, not a credential.

The overlay address is IPv6 in `200::/7`, from blake3 of the identity, and it does not rotate. IPv4 in `100.64.0.0/10` is exit-node egress. The overlay itself is IPv6-only.

The notes give the license as MPL-2.0 and the version as 0.5.15, still experimental. The ALPN is version 7 and still accepts v5 and v6. A root daemon owns the TUN, DNS, and routes. Unprivileged clients use a Unix socket, and the daemon checks the peer uid.

Per-file line counts in the draft (tens of thousands of lines in single files, beside a 132K-line total) do not add up. They are omitted.
