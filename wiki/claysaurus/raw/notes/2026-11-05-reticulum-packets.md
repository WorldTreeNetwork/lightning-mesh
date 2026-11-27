# Reticulum packet notes

Collected from the draft concept atlas on 2026-11-05. This is a note about Reticulum packets, not the Lightning Mesh gossip encoding.

Serialization is msgpack (via `umsgpack`) for control, and raw bytes for payload. The envelope is `RNS.Packet.Packet` with a header plus an encrypted payload.

Header: destination hash (20B) + source hash (20B) + type (1B) + flags (1B) + TTL (1B) + hops (2B) + transport ID (20B).

Payload: encrypted with an ephemeral X25519 key and an AES-256-CBC per-packet key. Token format: `ephemeral_pub_bytes (32B) + AES_ciphertext (variable)`.

The draft claimed a typical size of 100-500 bytes and also a minimum of about 500 bytes. Those two claims disagree. Treat the size as unverified.

Packet types from that draft:

| Type | Value | Purpose |
|---|---|---|
| DATA | 0x01 | Application data |
| ANNOUNCE | 0x02 | Service discovery (identity + name + optional ratchet) |
| PATH_REQUEST | 0x03 | Route discovery (flooded) |
| PATH_REPLY | 0x04 | Route reply (unicast) |
| PROOF | 0x05 | Identity proof (Ed25519 signature) |
| LINK_ESTABLISHMENT | 0x06 | 3-packet link handshake |

Announce packet data structure from that draft:

[Public Key (64 bytes)]
[Name Hash (10 bytes)] -- SHA-256(name + identity_hash)[:10]
[Random Hash (10 bytes)]
[Ratchet (32 bytes)] -- if context flag is set, else empty
[Signature (64 bytes)] -- Ed25519 over: destination_hash + public_key + name_hash + random_hash + ratchet + app_data
[App Data (variable)]

postcard and msgpack are both binary formats with different type systems. A translation layer would have to convert between them. Reticulum packets include routing headers. Mesh gossip messages are application-layer only.
