# source

# Lightning Mesh vs Reticulum

> Sources: Clay's Reticulum notes, corrected against this tree, 2026-11-06
> Raw: [mesh vs Reticulum](../raw/notes/2026-11-06-mesh-vs-reticulum.md)
> Updated: 2026-11-06

Reticulum is another network stack. These rows are a reading of it, not a decision to adopt it. The Reticulum tree is not in this repository.

| Dimension | Lightning Mesh | Reticulum, as the notes describe it |
|---|---|---|
| Language | Rust | Python 3 |
| Transport | iroh. LAN mode is the default. Internet mode is opt-in | Its own multi-hop transport |
| State | Per-book CRDT. No coordinator | Transport tables and resource transfer |
| Node address | Ed25519 public key. `blake3` derives the ULA and one host in `10.254.0.0/16`. A client `/24` is a claim from `10.42.0.0/16` | Destination hashes, SHA-256 truncated |
| Identity | 32-byte `iroh::SecretKey` | 64 bytes: X25519 private plus Ed25519 private |
| Routing | procd's `mjolnir-babeld` | Its own pathfinder |
| Backhaul | 802.11s and wired node-links | Whatever interface the notes list: serial, KISS, LoRa, TCP, UDP |
| Overlay | `mjolnir0` when that mode is on. Per-peer `/31` tunnels | Encrypted links, not this TUN |
| DNS | `.mesh` on the node | Name resolution in that stack |
| License of this repo | AGPL-3.0-or-later, plus a commercial license | Not this repo's license |

`mjolnir-meshd` does not start ACME. Three radios are live. This mesh has no minimum bandwidth of 1 Mbps and no minimum MTU of 1500 bytes. The gossip topic is `mjolnir/mesh/crdt/v0`. The protocol text is not a separate public-domain grant.

`GossipTransport` is the seam that keeps iroh out of `mjolnir-mesh`. The production implementation is `IrohGossipTransport`. It is not a slot that already accepts Reticulum.
