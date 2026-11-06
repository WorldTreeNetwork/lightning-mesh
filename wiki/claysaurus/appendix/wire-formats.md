---
title: Reticulum packet notes
created: 2026-11-05
updated: 2026-11-05
type: interop
tags: [interop, reticulum, transport]
sources: [wiki/claysaurus/raw/notes/2026-11-05-reticulum-packets.md]
state: [FUTURE]
confidence: low
---

# Reticulum packet notes

> Sources: draft concept atlas, 2026-11-05
> Raw: [Reticulum packet notes](../raw/notes/2026-11-05-reticulum-packets.md)
> Updated: 2026-11-05

Moved out of the shared wire-format appendix. This is not the Lightning Mesh gossip encoding.

Serialization is msgpack (via `umsgpack`) for control, and raw bytes for payload. The envelope is `RNS.Packet.Packet` with a header plus an encrypted payload.

Header: destination hash (20B) + source hash (20B) + type (1B) + flags (1B) + TTL (1B) + hops (2B) + transport ID (20B).

Payload: encrypted with an ephemeral X25519 key and an AES-256-CBC per-packet key. Token format: `ephemeral_pub_bytes (32B) + AES_ciphertext (variable)`.

The draft claimed a typical size of 100-500 bytes and also a minimum of about 500 bytes. Those two claims disagree. Treat the size as unverified.

| Type | Value | Purpose |
|---|---|---|
| DATA | 0x01 | Application data |
| ANNOUNCE | 0x02 | Service discovery (identity + name + optional ratchet) |
| PATH_REQUEST | 0x03 | Route discovery (flooded) |
| PATH_REPLY | 0x04 | Route reply (unicast) |
| PROOF | 0x05 | Identity proof (Ed25519 signature) |
| LINK_ESTABLISHMENT | 0x06 | 3-packet link handshake |

Announce layout from that draft: public key (64 bytes), name hash (10 bytes) as SHA-256(name + identity_hash)[:10], random hash (10 bytes), optional ratchet (32 bytes), signature (64 bytes) over destination_hash + public_key + name_hash + random_hash + ratchet + app_data, then app data.

postcard and msgpack are both binary formats with different type systems. Reticulum packets include routing headers. Mesh gossip messages are application-layer only.
