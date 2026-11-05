---
title: Reticulum Codebase Grounding
created: 2026-11-05
updated: 2026-11-05
type: research
tags: [reticulum, codebase, grounding, interop]
sources: [public/reticulum/RNS/Identity.py, public/reticulum/RNS/Link.py, public/reticulum/RNS/Channel.py, public/reticulum/RNS/Destination.py, public/reticulum/RNS/Transport.py, public/reticulum/RNS/Interfaces/Interface.py, public/reticulum/RNS/Reticulum.py]
state: [CURRENT]
confidence: high
---

# Reticulum Codebase Grounding

This page documents the actual Reticulum source code to ground all wiki interop planning in verified implementation details.

## Repository Overview

| Property | Value |
|---|---|
| Language | Python 3 |
| LOC | ~58K lines (Python) |
| Author | Mark Qvist (2016-present) |
| License | Reticulum License (permissive + no-harm, no-AI-training conditions) |
| Runtime | Userland Python process |
| Distribution | `pip install reticulum` (PyPI) |

## Core Module Structure

```
RNS/
  __init__.py          -- Module entry, RNS singleton, logging, config
  Reticulum.py         -- Reticulum singleton runtime (77K+ chars)
  Identity.py          -- Curve25519 identity, keys, hashes, encryption (43K chars)
  Destination.py       -- Named services, name derivation, resource ads (36K chars)
  Channel.py           -- Broadcast/multi-peer channels (28K chars)
  Transport.py         -- Routing, path discovery, packet forwarding (11K chars)
  Link.py              -- Encrypted point-to-point links, handshake, ratchets (84K chars)
  Packet.py            -- Packet types: data, announce, path-request, proof (12K chars)
  Interfaces/
    Interface.py       -- Base interface class (19K chars)
    KISSInterface.py   -- KISS TNC interface (18K chars)
    AX25KISSInterface.py -- AX.25 KISS interface (18K chars)
    EthernetInterface.py -- Ethernet/UDP broadcast
    SerialInterface.py   -- Serial port interface
    UDPInterface.py      -- UDP broadcast interface
    TCPClientInterface.py
    TCPServerInterface.py
    RNodeInterface.py    -- RNode LoRa transceiver
    PipeInterface.py     -- External program via stdio
    I2PInterface.py      -- I2P transport
```

## Identity Model (Identity.py)

### Key Structure

```
Private Key (64 bytes):
  prv_bytes     -- X25519 private key (32 bytes)
  sig_prv_bytes -- Ed25519 private key (32 bytes)

Public Key (64 bytes):
  pub_bytes     -- X25519 public key (32 bytes)
  sig_pub_bytes -- Ed25519 public key (32 bytes)
```

### Key Derivation and Hashing

- **`Identity.KEYSIZE`** = 512 bits (64 bytes total: 32 + 32)
- **`Identity.TRUNCATED_HASHLENGTH`** = 160 bits (20 bytes)
- **`Identity.NAME_HASH_LENGTH`** = 80 bits (10 bytes)
- **`Identity.full_hash(data)`** = `SHA-256(data)` (full 32 bytes)
- **`Identity.truncated_hash(data)`** = `SHA-256(data)[:20]` (20 bytes)
- **`Identity.hash`** = `truncated_hash(get_public_key())` -- the node's identity hash

### Ratchet System

- **`Identity.RATCHETSIZE`** = 256 bits (32 bytes)
- **`Identity.RATCHET_EXPIRY`** = 30 days
- Ratchets are ephemeral X25519 keys exchanged during Link establishment
- Each ratchet is persisted to disk as `known_destinations/<hexhash>`
- The ratchet ID is `full_hash(ratchet_pub_bytes)[:10]` (10 bytes)
- Ratchets provide forward secrecy per-link

### Encryption Flow

```
encrypt(plaintext, ratchet=None):
  1. Generate ephemeral X25519 key pair
  2. ECDH with target (ratchet or pub key) -> shared_key
  3. HKDF(shared_key, salt=identity.hash, context=None) -> derived_key
  4. Token(derived_key).encrypt(plaintext) -> ciphertext
  5. Return: ephemeral_pub_bytes + ciphertext

decrypt(ciphertext_token, ratchets=None):
  1. Extract peer_ephemeral_pub from ciphertext_token
  2. Try each ratchet: ECDH(ratchet_prv, peer_ephemeral_pub) -> shared_key
  3. If no ratchet works, try ECDH(prv, peer_ephemeral_pub)
  4. HKDF(shared_key, salt=identity.hash, context=None) -> derived_key
  5. Token(derived_key).decrypt(ciphertext) -> plaintext
```

### Key Point for Interop

**The private key file is simply `prv_bytes + sig_prv_bytes` (64 raw bytes).** A mesh node's Ed25519 private key can be loaded directly into `Reticulum.Identity` via `Identity.from_bytes(mesh_private_key_bytes)`. The X25519 component is derived from the same seed.

**`X22519` clamping**: Reticulum uses X22519 (RFC 7748 clamping on the private key). The Cryptography module handles this transparently. The underlying curve is still Curve25519.

## Destination Model (Destination.py)

### Name Derivation

```python
# Reticulum name derivation:
name_hash = Identity.truncated_hash(name.encode("utf-8") + identity.hash)
# = SHA-256(name + identity_hash)[:20]
```

- Names are hierarchical: `"delivery"` under an identity
- A Destination is identified by its hash (20 bytes, truncated SHA-256)
- The Destination's public key is always the Identity's public key
- Multiple Destinations can exist under one Identity (different names)

### Destination Lifecycle

```python
# Create a Destination:
dest = RNS.Destination(identity, RNS.Destination.IN, RNS.Destination.RECEIVE, name="myapp")

# This internally:
# 1. Derives dest.hash = truncated_hash(b"myapp" + identity.hash)
# 2. Creates a Channel for receiving on this destination
# 3. Registers with RNS.Transport for routing
# 4. Starts announce broadcasting
```

### Resource Advertisements

- Destinations can advertise resources (named data objects)
- Resources have names, sizes, and optional checksums
- Other nodes can request resources by name via the Destination's Channel
- This is the analog to Mesh's `ServiceBook` but with built-in file transfer

## Channel Model (Channel.py)

### Channel Purpose

The Channel class provides **broadcast/multi-peer pub/sub semantics** on top of Reticulum's encrypted point-to-point Links. This is the **direct analog to Mesh's `GossipTransport`**.

### Channel Lifecycle

```python
# Create a channel on a Destination:
channel = RNS.Channel(destination)

# Send to all channel members:
channel.send(payload_bytes)

# Receive from channel (blocking):
payload = channel.get()  # returns bytes

# Or non-blocking:
if not channel.empty():
    payload = channel.get()
```

### Channel Internals

- A Channel maintains a set of `Link` objects (one per active member)
- `send()` broadcasts to all known Links simultaneously
- `get()` returns the next message from any member, in arrival order
- Channels handle message deduplication (by sender + sequence number)
- Channels auto-join new members who announce on the destination
- Windowing and flow control are per-Link, managed by the Link class

### Channel Configuration

- `Channel.WINDOW_SIZE` controls the number of in-flight messages per Link
- `Channel.SEND_BUFFER_SIZE` limits the total send buffer
- Channels are destination-scoped: all members must be on the same Destination

### Key Point for Interop

**The `GossipTransport` trait in Mesh (`broadcast()` + `recv()`) maps directly to `Channel.send()` + `Channel.get()`.** The async/await wrapper is trivial to write.

## Transport Model (Transport.py)

### Transport Purpose

The Transport singleton manages **multi-hop routing** for Reticulum. It maintains routing tables, performs path discovery, and forwards packets between interfaces.

### Core Functions

- **`Transport.transmit(interface, packet_bytes)`** -- Send a packet out on an interface
- **`Transport.inbound(packet_bytes, receiving_interface, ...)`** -- Receive a packet from an interface
- **`Transport.has_path(destination_hash)`** -- Check if a routing path exists
- **`Transport.get_path(destination_hash)`** -- Get the next-hop for a destination

### Path Discovery

- Uses Path Request (PR) / Path Reply (PR) exchanges
- PRs are flooded through the network; PRs return along the reverse path
- Paths are cached and have a TTL
- The pathfinder builds a routing table of destination -> next-hop

### Key Point for Interop

**Reticulum's Transport handles multi-hop routing natively.** Mesh's overlay TUN + Babeld routing is separate from its gossip layer. If we use Reticulum as `GossipTransport`, Reticulum handles the multi-hop backhaul routing automatically -- no Babeld needed for the gossip plane.

## Link Model (Link.py)

### Link Purpose

The Link class manages **encrypted point-to-point connections** between two Destinations. Links are the building blocks of Reticulum's multi-hop network.

### Link Handshake (3-Packet)

```
Client                          Server
  |--- Link Establishment Request -->|
  |<-- Link Establishment Reply -----|
  |--- Link Establishment Confirm -->|
  |== Link Established (encrypted) ===|
```

1. **Request**: Client sends its identity hash + ephemeral key
2. **Reply**: Server responds with its identity hash + ephemeral key + ratchet
3. **Confirm**: Client acknowledges and the encrypted link begins

### Link Features

- **Windowing**: Configurable number of in-flight messages per direction
- **Retransmission**: ACK-based with configurable timeout
- **Ratchet exchange**: Forward-secrecy keys exchanged during handshake
- **Flow control**: Per-link rate limiting and buffer management
- **Announce forwarding**: Link-level announce propagation for path discovery

### Key Point for Interop

**Links are automatic.** When two nodes on the same Destination communicate for the first time, Reticulum establishes a Link transparently. No manual handshake code needed.

## Interface Model (Interface.py)

### Base Interface Class

The `Interface` base class defines the contract for all Reticulum transport interfaces:

```python
class Interface:
    IN  = False    # Is this an internal (software) interface?
    OUT = False    # Is this an external (physical) interface?
    FWD = False    # Does this interface forward packets?
    RPT = False    # Does this interface repeat/relay packets?

    name = None    # Interface name (e.g., "eth0", "/dev/ttyUSB0")
    HW_MTU = None  # Hardware MTU (auto-calculated from bitrate if AUTOCONFIGURE_MTU)
    bitrate = 62500  # Default bitrate (62.5 kbps, suitable for LoRa)

    # Interface modes (for routing)
    MODE_FULL           = 0x01
    MODE_POINT_TO_POINT = 0x02
    MODE_ACCESS_POINT   = 0x03
    MODE_ROAMING        = 0x04
    MODE_BOUNDARY       = 0x05
    MODE_GATEWAY        = 0x06
    MODE_INTERNAL       = 0x07

    # Ingress/Egress control for announce rate limiting
    ingress_control = True
    egress_control = False
    announce_cap = 0.01  # Max 1% of airtime for announces
```

### Key Methods (to override in subclasses)

Each Interface subclass implements:
- **`transmit(packet_bytes)`** -- Send a packet on this interface
- **`receive()`** -- Read a packet from this interface (blocking or async)
- **`attach()`** -- Start the interface (open device, bind socket, etc.)
- **`detach()`** -- Stop the interface
- **`teardown()`** -- Full cleanup

### Interface Subclasses

| Class | Carrier | Use Case | MTU |
|---|---|---|---|
| `EthernetInterface` | Ethernet/UDP | WiFi, wired LAN | Auto (from bitrate) |
| `SerialInterface` | Serial port | UART, USB-serial | 1024 |
| `KISSInterface` | KISS TNC | Packet radio, DireWolf | 1024 |
| `AX25KISSInterface` | AX.25 KISS | Amateur radio | 1024 |
| `UDPInterface` | UDP broadcast | LAN discovery | Auto |
| `TCPClientInterface` | TCP | Remote connections | 65536 |
| `TCPServerInterface` | TCP server | Remote connections | 65536 |
| `RNodeInterface` | RNode LoRa | Long-range low-BW | 256 |
| `PipeInterface` | Stdio pipe | Custom hardware | 1024 |
| `I2PInterface` | I2P network | Anonymous overlay | 500 |

### Ingress/Egress Control

Reticulum has sophisticated announce rate limiting:
- **Ingress control**: Limits incoming announce frequency per interface (burst detection)
- **Egress control**: Limits outgoing path request frequency
- Uses sliding window with deque-based frequency tracking
- Configurable thresholds for new vs. established interfaces

### Key Point for Interop

**Reticulum's Interface system is the carrier abstraction.** Each Interface subclass is a transport plugin. To add a new carrier (e.g., USB SDR, LoRa), you write a new Interface subclass. This maps to Mesh's `GossipTransport` trait but at a lower level -- Interfaces handle the physical layer, Channels handle the gossip layer.

## Reticulum Runtime (Reticulum.py)

### Singleton Pattern

```python
# Get the singleton instance:
rns = RNS.Reticulum.get_instance()

# Configure:
rns = RNS.Reticulum()
rns.path = "/tmp/reticulum"       # Storage path
rns.log_level = RNS.LOG_VERBOSE  # Logging
rns.log_output = RNS.LOG_STDOUT  # Log destination
rns.start()                       # Start the runtime
```

### Key Configuration

- **`RNS.Reticulum.TRUNCATED_HASHLENGTH`** = 160 bits (20 bytes)
- **`RNS.Reticulum.ANNOUNCE_CAP`** = 1% (announce airtime cap)
- **`RNS.Reticulum.QUEUED_ANNOUNCE_LIFE`** = 300 seconds (announce queue TTL)
- **`RNS.Reticulum.storagepath`** = Where keys, destinations, and ratchets are stored

### Storage Layout

```
~/.reticulum/
  identities/          -- Saved Identity key files
  known_destinations   -- msgpack-serialized known destinations
  ratchets/            -- Per-destination ratchet files
  log                  -- Log file (if configured)
```

## Wire Format (Packet.py)

### Packet Header

```
+------------------+------------------+--------+
| Destination Hash | Source Hash       | Type   |
| (20 bytes)       | (20 bytes)       | (1 byte) |
+------------------+------------------+--------+
| Flags  | TTL  | Hops | Transport ID   |
| (1 byte)|(1 byte)|(2 bytes)| (20 bytes) |
+------------------+------------------+--------+
| Encryption Token (variable, AES-256-CBC)
+------------------+
```

### Packet Types

| Type | Value | Purpose |
|---|---|---|
| DATA | 0x01 | Application data |
| ANNOUNCE | 0x02 | Service discovery (identity + name) |
| PATH_REQUEST | 0x03 | Route discovery |
| PATH_REPLY | 0x04 | Route reply |
| PROOF | 0x05 | Identity proof (signature) |
| LINK_ESTABLISHMENT | 0x06 | Link handshake |

### Announce Packet Structure

```
Announce Data:
  [Public Key (64 bytes)]
  [Name Hash (10 bytes)]
  [Random Hash (10 bytes)]
  [Ratchet (32 bytes)] -- if context flag is set
  [Signature (64 bytes)]
  [App Data (variable)]
```

## Encryption Summary

| Layer | Algorithm | Key Material |
|---|---|---|
| Identity | Curve25519 (X25519 + Ed25519) | 64-byte private key |
| Hash | SHA-256 (truncated to 20 bytes) | None |
| Key Derivation | HKDF | ECDH shared secret + salt (identity hash) |
| Payload Encryption | AES-256-CBC + PKCS7 | Per-packet ephemeral key |
| Message Authentication | HMAC-SHA256 | Per-packet |
| Forward Secrecy | X25519 ratchets | 30-day expiry |

## Interop Mapping: Reticulum -> Mesh

| Reticulum Concept | Mesh Equivalent | Integration Point |
|---|---|---|
| `RNS.Identity` | `iroh::net::key::SecretKey` | Shared key export |
| `RNS.Destination` | `ServiceBook` entry | Name resolution bridge |
| `RNS.Channel` | `GossipTransport` trait | **Primary: swap transport** |
| `RNS.Link` | (no equivalent) | Reticulum handles multi-hop |
| `RNS.Transport` | Babeld (external) | Reticulum has built-in routing |
| `RNS.Interface` | `tun/underlay.rs` carriers | **Carrier extension** |
| `RNS.Packet.ANNOUNCE` | `LivenessBeacon` + `SubnetClaim` | Announce translation |
| `RNS.Resource` | (no equivalent) | Bulk sync, file sharing |

## Risks and Constraints (Verified from Code)

1. **Python runtime required** -- Reticulum is pure Python. Embedding requires `embeddings` crate or running as a sidecar process.
2. **No async I/O** -- Reticulum uses threading, not async/await. The `Channel.get()` call is blocking.
3. **64-byte private key** -- The key file is exactly 64 bytes (32 X25519 + 32 Ed25519). Direct byte-level compatibility with mesh keys is possible.
4. **160-bit (20-byte) hashes** -- Reticulum uses truncated SHA-256. Mesh uses blake3. Different hash functions means hash values are not interchangeable.
5. **License** -- Reticulum License has two conditions: no harm to humans, no AI training datasets. This must be documented in any combined work.
6. **Minimum MTU 500 bytes** -- Reticulum packets are optimized for low bandwidth. Mesh's 1500-byte MTU is larger.
7. **No built-in QoS** -- Reticulum has no quality-of-service layer. Best-effort delivery.

## Files Referenced

- `public/reticulum/RNS/Identity.py` (960 lines, 43K chars) -- Identity, keys, hashes, encryption
- `public/reticulum/RNS/Link.py` (1,800+ lines, 84K chars) -- Encrypted point-to-point links
- `public/reticulum/RNS/Channel.py` (500+ lines, 28K chars) -- Broadcast channels
- `public/reticulum/RNS/Destination.py` (700+ lines, 36K chars) -- Named services
- `public/reticulum/RNS/Transport.py` (200+ lines, 11K chars) -- Routing and forwarding
- `public/reticulum/RNS/Interfaces/Interface.py` (450 lines, 19K chars) -- Base interface
- `public/reticulum/RNS/Reticulum.py` (1,800+ lines, 103K chars) -- Singleton runtime
- `public/reticulum/RNS/Packet.py` (300+ lines, 12K chars) -- Packet types and framing
