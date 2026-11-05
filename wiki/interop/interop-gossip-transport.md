---
title: Reticulum as Gossip Transport
created: 2026-11-05
updated: 2026-11-05
type: interop
tags: [interop, reticulum, transport-swap]
sources: [crates/mjolnir-mesh/src/crdt/sync.rs, public/reticulum/RNS/Channel.py]
state: [FUTURE]
confidence: medium
---

# Reticulum as Gossip Transport

Replace iroh-gossip with Reticulum's Link + Channel for CRDT state propagation.

## Concept

Write a `ReticulumTransport` impl of the `GossipTransport` trait. Reticulum's `Channel` provides the exact same contract: send/receive bytes to/from all peers on a topic.

## Architecture

```
meshd
  └── GossipSync<ReticulumTransport>
        └── ReticulumChannel (broadcast/recv)
              └── RNS.Link (encrypted link)
                    └── RNS.Interface (Ethernet/LoRa/Serial/etc.)
```

## Implementation Sketch

The `GossipTransport` trait is defined in `crates/mjolnir-mesh/src/crdt/sync.rs` at line 44:

```rust
#[async_trait::async_trait]
pub trait GossipTransport: Send + Sync {
    async fn broadcast(&self, payload: Bytes) -> Result<(), GossipError>;
    async fn recv(&self) -> Result<Bytes, GossipError>;
}
```

A Reticulum implementation would:

1. Create an `RNS.Destination` with a shared secret (mesh network name)
2. Open an `RNS.Channel` on that destination for gossip
3. `broadcast()` calls `channel.send(payload)`
4. `recv()` calls `channel.get()` in an async loop

**Verified from `Channel.py`:**
- `Channel.send(payload)` broadcasts to all known Link members simultaneously
- `Channel.get()` is a **blocking** call (not async). Returns the next message in arrival order
- `channel.empty()` checks if messages are available without blocking
- Message deduplication is automatic (by sender + sequence number)
- Channels auto-join new members who announce on the destination

**Verified from `Link.py`:**
- Links are established automatically via a 3-packet handshake (Request/Reply/Confirm)
- Links use windowing (configurable in-flight messages) and ACK-based retransmission
- Forward secrecy via X25519 ratchet exchange during handshake

**Async wrapper needed:** Since `Channel.get()` is blocking, the `GossipTransport.recv()` method needs an async bridge (e.g., `asyncio.to_thread(channel.get)` or a dedicated receiver thread with a queue).

## Configuration

- One `RNS.Destination` per mesh (shared secret = mesh network name)
- One `RNS.Channel` per gossip topic
- Link establishment is automatic via Reticulum's multi-hop transport

## Benefits

- Nodes behind strict NATs that iroh cannot hole-punch can participate
- LoRa, serial, and KISS carriers become first-class mesh transports
- Multi-hop routing is built-in (no overlay TUN needed for backhaul)

## Risks

- Reticulum is Python — requires embedding a Python runtime or running as a sidecar
- Link establishment is slower than iroh's QUIC (3-packet handshake vs connectionless)
- Reticulum's license has conditions (no harm, no AI training)

## Cargo Feature

Gate behind `--features reticulum-transport`. Default remains iroh-gossip.
