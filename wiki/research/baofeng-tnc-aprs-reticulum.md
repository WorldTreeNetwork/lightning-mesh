---
title: Baofeng Radio + TNC/APRS + Reticulum Integration
created: 2026-11-05
updated: 2026-11-05
type: research
tags: [sdr, radio, tnc, aprs, reticulum, baofeng, kiss, ax.25]
state: [FUTURE]
confidence: high
---

# Baofeng Radio + TNC/APRS + Reticulum Integration

Use when: designing carriers using VHF/UHF FM radios (Baofeng-class) with TNC/APRS for mesh backhaul.

## 1. The Hardware Stack

### Baofeng UV-5R / UV-5RE (and equivalents)

- **Price:** $15-30 USD
- **Frequency:** 136-174 MHz (VHF) + 400-520 MHz (UHF)
- **Power:** 5-8W TX
- **Modes:** Wide FM, NARROW FM
- **APRS frequency:** 144.390 MHz (North America), 144.800 MHz (Europe)
- **Config:** Simple — set frequency, disable BCLO/VOX, set SQL=0-2, PTT via cable
- **Status:** ✅ Extremely active community, widely available, proven for APRS

### TNC (Terminal Node Controller)

A TNC bridges digital data to RF. Two approaches:

**A. Software TNC (DireWolf)** — recommended
- Software that turns a soundcard (or USB audio interface) into a modem
- Supports 1200 baud AFSK (standard VHF packet radio)
- Exposes KISS over TCP (port 8001), AGWPE (port 8000), serial KISS
- Actively developed (v1.8, October 2025), WB2OSZ, MPL-2.0
- Runs on Raspberry Pi, OpenWrt (via package), Linux, macOS, Windows
- Supports FX.25 forward error correction

**B. Hardware TNC**
- Dedicated TNC hardware (TNC-X, TNC-PI, OpenModem)
- Connects via UART/serial to the radio
- More expensive ($30-100), but more reliable for permanent installations
- DigiPi 2.0 is a purpose-built Linux distro for APRS digipeaters/iGates

### Audio/PTT Interface

For software TNC, you need:
- **USB audio interface** (e.g., NA6D AIOC — All-In-One-Cable, $20-30)
  - CM108-based USB audio chip
  - Provides clean audio paths + PTT control
  - Eliminates instability of generic USB sound cards
- **PTT (Push-to-Talk)** — GPIO or serial RTS signal to key the radio
- **Audio isolation** — transformers/optocouplers to protect radio and PC

### Complete Hardware Chain

```
[Baofeng UV-5R] --audio--> [NA6D AIOC / USB sound card] --USB--> [Raspberry Pi / OpenWrt router]
                              |                                       |
                              +-- PTT control ------------------------+
                                                              [DireWolf TNC]
                                                              [Reticulum / Mesh]
```

## 2. Protocol Stack

### APRS (Automatic Packet Reporting System)

- **Modulation:** 1200 baud AFSK (Audio Frequency Shift Keying)
  - Mark tone: 1200 Hz (binary 1)
  - Space tone: 2200 Hz (binary 0)
- **Encoding:** AX.25 Link Layer (amateur radio variant of HDLC)
- **Frame rate:** ~10-20 packets/second theoretical max, ~1-5 practical
- **MTU:** ~128 bytes per AX.25 frame
- **Use:** Position reporting, messaging, telemetry

### AX.25 (Amateur Packet Radio Layer 2)

- Extension of HDLC for amateur radio
- Frame structure:
  ```
  [Dest Address 6+1 bytes] [Src Address 6+1 bytes] [Control 0x03] [PID 0xF0] [Data N bytes] [CRC 2 bytes]
  ```
- Default destination: `APZRNS-0` (Reticulum)
- Control field: `0x03` = UI frame (Unnumbered Information)
- PID field: `0xF0` = No Layer 3 protocol (raw data)

### KISS (Keep It Simple, Stupid)

- Host-to-TNC protocol over serial/USB/TCP
- Frame format:
  ```
  0xC0 [Command Byte] [Data N bytes] 0xC0
  ```
- Commands:
  - `0x00` — Data frame (transmit/receive packet)
  - `0x01` — Set TX delay (preamble)
  - `0x02` — Set persistence
  - `0x03` — Set slot time
  - `0x04` — Set TX tail
  - `0x05` — Set full-duplex
  - `0x06` — Set hardware speed
  - `0xFF` — Exit KISS mode

### DireWolf Interfaces

DireWolf exposes 3 interfaces to host software:

| Interface | Port | Protocol | Use Case |
|---|---|---|---|
| AGWPE | TCP 8000 | AGW Network Protocol | Xastir, Winlink, legacy apps |
| KISS TCP | TCP 8001 | KISS over TCP | Reticulum, BPQ32, mesh |
| KISS Serial | pseudo-tty | KISS over serial | kissattach, kernel AX.25 |

## 3. How Reticulum Handles These Carriers

Reticulum has **native support** for packet radio TNCs via 3 interface types:

### Reticulum Interface Types for Radio

| Interface | Purpose | Protocol | MTU | Bitrate |
|---|---|---|---|---|
| `SerialInterface` | Direct serial | HDLC framing | 564 bytes | Port speed |
| `KISSInterface` | KISS TNC | KISS protocol | 564 bytes | 1200 bps (guess) |
| `AX25KISSInterface` | AX.25 TNC | AX.25 + KISS | 564 bytes | 1200 bps (guess) |

### KISSInterface Implementation

Reticulum's `KISSInterface` (from `RNS/Interfaces/KISSInterface.py`):

1. Opens a serial port with configurable baud rate, data bits, parity, stop bits
2. Sends KISS commands to configure the TNC (preamble, tx tail, persistence, slot time)
3. Reads incoming KISS frames, extracts data, passes to Reticulum stack
4. Writes outgoing Reticulum packets as KISS data frames to the serial port
5. Handles escape sequences (0xDB → 0xDB 0xDD, 0xC0 → 0xDB 0xDC)

### AX25KISSInterface Implementation

Reticulum's `AX25KISSInterface` (from `RNS/Interfaces/AX25KISSInterface.py`):

1. Same as KISSInterface, but wraps Reticulum packets in AX.25 UI frames
2. Adds 16-byte AX.25 header:
   - Destination: `APZRNS-0` (default)
   - Source: User-configured callsign + SSID
   - Control: `0x03` (UI frame)
   - PID: `0xF0` (no layer 3)
3. Allows Reticulum to operate within existing amateur radio AX.25 networks

### Reticulum Interface Modes

Reticulum uses interface modes to control how traffic flows between carriers:

| Mode | Purpose | Use Case |
|---|---|---|
| `full` | General interface | Default for most interfaces |
| `access_point` | Limits relayed announces | Fast segment (WiFi/Ethernet) |
| `roaming` | Restricts announce relaying | Mobile/intermittent links |
| `gateway` | Answers path requests | Client-facing gateway |
| `boundary` | Restricts announce propagation | Network boundary |

**Key insight:** A node with multiple interfaces (e.g., WiFi + LoRa + KISS) can route traffic between them. Reticulum automatically decides which interface to use based on path tables and interface modes.

## 4. Integration Architecture

### Option A: Reticulum as Gossip Transport (via KISS)

This builds on the existing `ReticulumTransport` idea from `interop-gossip-transport.md`.

```
meshd
  └── GossipSync<ReticulumTransport>
        └── Reticulum Channel
              └── AX25KISSInterface (or KISSInterface)
                    └── DireWolf (software TNC)
                          └── USB audio interface
                                └── Baofeng UV-5R
```

**Implementation:**

1. Spawn DireWolf on the node (via `subprocess` or as a sidecar)
2. Configure DireWolf to listen on KISS TCP port 8001
3. Reticulum's `AX25KISSInterface` connects to DireWolf via TCP
4. Reticulum's `Channel` provides gossip transport
5. Mesh's `GossipSync` uses Reticulum for CRDT state propagation

**Pros:**
- DireWolf is actively maintained (v1.8, Oct 2025)
- KISS TCP is simple and well-documented
- Reticulum handles routing, crypto, multi-hop automatically
- Works on OpenWrt (Reticulum package available)

**Cons:**
- Reticulum is Python — requires embedding or sidecar
- 1200 baud is slow (~120 bytes/sec)
- Half-duplex (can't transmit and receive simultaneously)

### Option B: Direct KISS Implementation in Rust

Instead of embedding Reticulum, implement KISS directly in Rust.

Proposal only. The trait in `crdt/sync.rs` is async and deals in `Bytes`. This sketch is not in the tree.

```rust
pub struct KissTransport {
    tcp_stream: TcpStream,  // Connection to DireWolf KISS TCP
    callsign: String,
    ssid: u8,
    dst_call: String,       // Default destination
    dst_ssid: u8,
}

#[async_trait::async_trait]
impl GossipTransport for KissTransport {
    async fn broadcast(&self, payload: Bytes) -> Result<(), GossipError> {
        // Wrap payload in an AX.25 header
        // Escape bytes for KISS (0xDB → 0xDB 0xDD, 0xC0 → 0xDB 0xDC)
        // Send as KISS data frame: 0xC0 0x00 [escaped data] 0xC0
    }

    async fn recv(&self) -> Result<Bytes, GossipError> {
        // Read KISS frames from the TCP stream
        // Find 0xC0 delimiters
        // Unescape bytes
        // Strip the AX.25 header
        // Return the payload
    }
}
```

**Pros:**
- No Python dependency
- Full control over KISS framing
- Can optimize for mesh traffic patterns

**Cons:**
- More code to maintain
- Need to implement AX.25 header construction
- Need to handle KISS escape sequences

### Option C: Reticulum as Carrier Bridge

Reticulum runs as a separate process on the node. It manages multiple carriers (WiFi, LoRa, KISS) and presents a unified interface to the mesh.

```
meshd
  └── IrohGossipTransport (the one production impl, over IP)
  └── Reticulum Bridge
        ├── KISSInterface -> DireWolf -> Baofeng (VHF APRS)
        ├── RNodeInterface -> LoRa radio (long-range)
        └── TCPClientInterface -> Remote Reticulum nodes
```

**Pros:**
- Reticulum handles carrier diversity
- Mesh stays simple (just uses Reticulum as one transport)
- Reticulum's multi-hop routing handles long-range

**Cons:**
- Adds Reticulum as a dependency
- Double encryption (Reticulum + Iroh)
- More complex deployment

## 5. Reticulum's Role in the System

Reticulum is **not just a carrier** — it's a complete networking stack that can:

1. **Manage multiple carriers simultaneously** — A single Reticulum instance can run over WiFi, LoRa, KISS, serial, TCP, etc.
2. **Route between carriers** — Traffic from KISS can flow to WiFi automatically
3. **Handle crypto and routing** — Reticulum's multi-hop transport, announce propagation, and path discovery work across all carriers
4. **Provide a unified API** — Applications talk to Reticulum's `Destination` and `Channel` APIs, regardless of underlying carrier

### How Reticulum Helps Implement the System

**For Baofeng/KISS specifically:**

1. **DireWolf runs as the modem** — converts between AFSK on air and digital frames
2. **DireWolf exposes KISS TCP** — host software connects via TCP to port 8001
3. **Reticulum's AX25KISSInterface connects to DireWolf** — Reticulum handles AX.25 framing, KISS protocol, and serial/TCP communication
4. **Reticulum's Channel is a proposal for the byte pipe** — wrapping it in `GossipTransport` is future work, not a type the tree has
5. **Mesh's ReticulumTransport wraps Reticulum's Channel** — Mesh uses Reticulum as its gossip transport

**For LoRa (RNode):**

1. **RNode radio connects via USB serial** — or TCP, or BLE
2. **Reticulum's RNodeInterface drives the radio** — handles LoRa framing, packet transmission
3. **Reticulum's Channel provides gossip** — same as above
4. **Mesh's ReticulumTransport wraps Reticulum's Channel** — same as above

**For Serial (direct):**

1. **Serial cable connects two nodes** — or serial modem
2. **Reticulum's SerialInterface handles HDLC framing** — direct serial communication
3. **Reticulum's Channel provides gossip** — same as above
4. **Mesh's ReticulumTransport wraps Reticulum's Channel** — same as above

### Key Insight

**Reticulum is the carrier abstraction layer.** It provides:
- `Interface` classes for different carriers (KISS, AX25KISS, Serial, RNode, TCP, etc.)
- `Destination` and `Channel` APIs for application-level communication
- Multi-hop routing, announce propagation, path discovery
- Crypto (Curve25519, AES-256-CBC, HMAC-SHA256, HKDF)
- Bandwidth-aware routing (slow carriers get less traffic)

**Mesh just needs to implement `GossipTransport` using Reticulum's `Channel`.** The carrier diversity is handled by Reticulum.

## 6. Comparison: Mesh vs Reticulum for Radio Carriers

| Aspect | Lightning Mesh | Reticulum |
|---|---|---|
| **Carrier abstraction** | `GossipTransport` trait (gossip only) | `Interface` classes (full networking) |
| **Radio support** | None (WiFi 802.11s only) | KISS, AX25KISS, Serial, RNode, TCP, UDP, etc. |
| **Multi-carrier** | One production impl (`IrohGossipTransport`). A second impl would be manual | Automatic (Reticulum manages multiple interfaces) |
| **Routing** | Babeld (IP-based) | Reticulum's own routing (crypto-backed) |
| **Crypto** | Ed25519 `iroh::SecretKey`, iroh-gossip over QUIC | X25519 plus Ed25519 identity, AES-256-CBC |
| **Bandwidth awareness** | Manual | Automatic (interface modes, announce caps) |
| **Python vs Rust** | Rust | Python (but can be embedded) |
| **License** | AGPL-3.0-or-later | Reticulum License (no harm, no AI) |

## 7. Recommended Approach

### Phase 1: DireWolf + KISS TCP + Direct Rust Implementation

**Effort:** 2-3 weeks
**Value:** High

1. Implement `KissTransport` in Rust (direct KISS over TCP)
2. Connect to DireWolf's KISS TCP port (8001)
3. Implement AX.25 header construction
4. Implement KISS escape/unescape
5. Test with Baofeng + USB audio interface

### Phase 2: Reticulum as Gossip Transport

**Effort:** 3-4 weeks
**Value:** High

1. Embed Reticulum Python runtime (or run as sidecar)
2. Implement `ReticulumTransport` using Reticulum's `Channel`
3. Configure Reticulum to use `AX25KISSInterface` for KISS
4. Test multi-carrier (WiFi + KISS + LoRa)

### Phase 3: Full Carrier Diversity

**Effort:** 4-6 weeks
**Value:** Medium-High

1. Add RNodeInterface for LoRa
2. Add SerialInterface for direct serial
3. Add TCPClientInterface for remote SDR
4. Configure interface modes for each carrier
5. Test routing between carriers

## 8. Hardware Recommendations

### Minimum Viable Setup

- **Radio:** Baofeng UV-5R ($15)
- **Audio interface:** Generic USB sound card ($5) or NA6D AIOC ($20)
- **Host:** Raspberry Pi Zero W ($15) or OpenWrt router
- **Total:** $35-55 per node

### Production Setup

- **Radio:** Baofeng UV-5RE ($20) or Kenwood TH-F6A ($200)
- **Audio interface:** NA6D AIOC ($30)
- **Host:** Raspberry Pi 4 ($35-55)
- **Antenna:** Premium antenna ($15-30)
- **Total:** $100-300 per node

### OpenWrt Integration

- Reticulum package available for OpenWrt
- DireWolf package available for OpenWrt (or compile from source)
- USB audio interface support via `kmod-sound-core`
- Baofeng connects via USB audio interface (no direct USB connection)

## 9. Configuration Example

### DireWolf Configuration (`direwolf.conf`)

```
# Callsign
MYCALL N0CALL-10

# Audio device
ADEVICE plughw:1,0

# Channel 0 — 1200 baud AFSK
CHANNEL 0
MODEM 1200

# PTT control
PTT /dev/ttyUSB0 RTS

# KISS TCP (for Reticulum or mesh)
KISSPORT 8001

# AGWPE (for legacy apps)
AGWPORT 8000

# Beacon
PBEACON delay=0 every=10 overlay=T symbol="igate" \
    lat=18.603986824928814 long=73.93357992388026 \
    power=4 height=001050 gain=2 \
    comment="Mesh Node" via=WIDE-1,WIDE2-2
```

### Reticulum Configuration (`~/.reticulum/config`)

```
[Interfaces.KISS]
type = KISS
name = aprs_vhf
port = /dev/ttyUSB0
speed = 9600
preamble = 350
txtail = 20
persistence = 64
slottime = 20
id_interval = 300
id_callsign = PE1RXF-10

[Interfaces.AX25KISS]
type = AX25KISS
name = aprs_ax25
port = /dev/ttyUSB0
speed = 9600
callsign = PE1RXF-10
ssid = 10
preamble = 350
txtail = 20
persistence = 64
slottime = 20
```

## 10. Risks and Mitigations

| Risk | Severity | Mitigation |
|---|---|---|
| 1200 baud is very slow | High | Use for gossip/control plane only; data plane over WiFi |
| Half-duplex (can't TX/RX simultaneously) | Medium | Implement CSMA-like backoff; Reticulum handles this |
| DireWolf instability | Low | Actively maintained; use NA6D AIOC for stability |
| Reticulum Python dependency | Medium | Embed via `pyo3` or run as sidecar |
| License restrictions | Low | Reticulum license is permissive (no harm, no AI) |
| RF interference | Medium | Use frequency hopping; Reticulum handles retransmission |
| OpenWrt package availability | Low | Reticulum and DireWolf packages available |

## 11. Next Steps

1. **Prototype:** Build minimum viable setup (Baofeng + USB sound card + Raspberry Pi + DireWolf)
2. **Test KISS TCP:** Verify DireWolf KISS TCP works with a simple client
3. **Implement `KissTransport`:** Write Rust implementation of KISS over TCP
4. **Test gossip:** Verify CRDT sync works over KISS link
5. **Add Reticulum:** Embed Reticulum and implement `ReticulumTransport`
6. **Test multi-carrier:** Add LoRa via RNodeInterface
7. **Productionize:** Optimize for OpenWrt, add interface modes, test routing
