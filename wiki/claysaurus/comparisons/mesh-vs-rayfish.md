---
title: Lightning Mesh vs Rayfish
created: 2026-11-05
updated: 2026-11-06
type: comparison
tags: [interop, rayfish, mesh-core, vpn]
sources: [crates/mjolnir-mesh/src/tun/link.rs, wiki/claysaurus/raw/notes/2026-11-06-mesh-vs-rayfish.md]
state: [FUTURE]
confidence: medium
---

# Lightning Mesh vs Rayfish

> Sources: Clay's Rayfish notes, corrected against this tree, 2026-11-06
> Raw: [mesh vs Rayfish](../raw/notes/2026-11-06-mesh-vs-rayfish.md)
> Updated: 2026-11-06

Rayfish is another iroh program. These rows are a reading of it, not a decision to adopt it.

| Dimension | Lightning Mesh | Rayfish |
|---|---|---|
| Mission | Router mesh. CRDT books, Babel, hello.mesh | P2P VPN. One TUN, Magic DNS, a firewall |
| Transport | iroh. LAN mode is the default. Internet mode is opt-in | iroh, with a versioned ALPN |
| State | Per-book CRDT. No coordinator | Signed GroupBlob, published through pkarr. A coordinator holds the network key |
| Node address | Derived. `backhaul_addr` is one host in `10.254.0.0/16` from `blake3(node id)`. `ula_addr` is the identity ULA from the same id | Overlay address derived. IPv6 in `200::/7` from blake3. IPv4 in `100.64.0.0/10` is for exit egress. The overlay is IPv6-only |
| Client LAN | A claimed `/24` from `10.42.0.0/16`. First-writer-wins on an HLC | No separate claimed client subnet. The overlay address is the identity address |
| Identity | Ed25519 `iroh::SecretKey`. The node id is the hex public key | Same endpoint key, plus a user-signed device cert |
| Admission | Owner claim: device proof, card HMAC, 120-second button | Signed invite, GroupBlob, a coordinator |
| Routing | procd's `mjolnir-babeld` over 802.11s and wired node-links | Per-peer QUIC datagrams |
| Overlay firewall | No stateful conntrack on `mjolnir0` | Stateful conntrack, per-peer rules, deny inbound TCP/UDP by default |
| DNS | `.mesh` on the node | `.ray` Magic DNS |
| This repo | About 55K lines of Rust across 6 crates | Not in this repo. Draft line counts disagreed with each other and are omitted |

Mesh does not run ACME inside `mjolnir-meshd`. Three radios are live. A larger line count is not evidence that a pattern is battle-tested.

The gossip trait is the seam that keeps iroh out of `mjolnir-mesh`. It is not a slot for Rayfish components.
