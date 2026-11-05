---
title: Wire Format Reference
created: 2025-11-05
updated: 2025-11-05
type: query
tags: [mesh-core, interop]
sources: [crates/mjolnir-mesh/src/crdt/gossip.rs, public/reticulum/RNS/Packet.py]
state: [CURRENT]
confidence: high
---

# Wire Format Reference

## Lightning Mesh Gossip

- **Serialization:** postcard (binary, zero-copy)
- **Envelope:** raw `bytes::Bytes`
- **Message types:** `GossipMessage` enum variants (SubnetClaim, ServicePublish, Lease, etc.)
- **Framing:** none -- each broadcast is a complete postcard message
- **Size:** typically 200-2000 bytes per message

### GossipMessage Variants

```rust
pub enum GossipMessage {
    SubnetClaim { ... },
    SubnetClaimRelease { ... },
    ServicePublish { ... },
    ServiceUnpublish { ... },
    Lease { ... },
    LeasedName { ... },
    User { ... },
    PeerAddr { ... },
    NodeName { ... },
    Coordinate { ... },
    LivenessBeacon { ... },
}
```

## Reticulum Packets

- **Serialization:** msgpack (via `umsgpack`) for control, raw bytes for payload
- **Envelope:** `RNS.Packet.Packet` with header + encrypted payload
- **Header:** destination hash (20B) + source hash (20B) + type (1B) + flags (1B) + TTL (1B) + hops (2B) + transport ID (20B)
- **Payload:** encrypted with ephemeral X25519 key + AES-256-CBC per-packet key
- **Token format:** `ephemeral_pub_bytes (32B) + AES_ciphertext (variable)`
- **Size:** typically 100-500 bytes per packet (optimized for low bandwidth, minimum ~500B)

### Packet Types

| Type | Value | Purpose |
|---|---|---|
| DATA | 0x01 | Application data |
| ANNOUNCE | 0x02 | Service discovery (identity + name + optional ratchet) |
| PATH_REQUEST | 0x03 | Route discovery (flooded) |
| PATH_REPLY | 0x04 | Route reply (unicast) |
| PROOF | 0x05 | Identity proof (Ed25519 signature) |
| LINK_ESTABLISHMENT | 0x06 | 3-packet link handshake |

### Announce Packet Data Structure

```
[Public Key (64 bytes)]
[Name Hash (10 bytes)] -- SHA-256(name + identity_hash)[:10]
[Random Hash (10 bytes)]
[Ratchet (32 bytes)] -- if context flag is set, else empty
[Signature (64 bytes)] -- Ed25519 over: destination_hash + public_key + name_hash + random_hash + ratchet + app_data
[App Data (variable)]
```

## Translation Considerations

- postcard and msgpack are both binary formats but have different type systems
- A translation layer would need to convert between the two serialization formats
- Mesh messages are larger (no low-bandwidth optimization)
- Reticulum packets include routing headers; Mesh messages are application-layer only
