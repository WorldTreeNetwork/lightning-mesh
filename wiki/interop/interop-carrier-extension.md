---
title: Reticulum Carriers on OpenWrt Hardware
created: 2025-11-05
updated: 2025-11-05
type: interop
tags: [interop, reticulum, carrier-extension, openwrt]
state: [FUTURE]
confidence: medium
---

# Reticulum Carriers on OpenWrt Hardware

Extend Lightning Mesh's physical carriers to include Reticulum's LoRa, serial, and KISS interfaces.

## Concept

Many OpenWrt routers have serial ports, USB, or can attach LoRa radios. Reticulum has native drivers for RNode (LoRa), KISS TNCs, serial ports, and AX.25. These become additional transports for the mesh.

## Interface Mapping

|| Reticulum Interface | OpenWrt Hardware | Mesh Use Case | MTU |
|---|---|---|---|---|
| `SerialInterface` | UART/USB serial | Device management, debugging | 1024 |
| `KISSInterface` | KISS TNC (DireWolf) | Amateur radio mesh | 1024 |
| `AX25KISSInterface` | AX.25 KISS TNC | Amateur radio mesh | 1024 |
| `RNodeInterface` | RNode LoRa transceiver | Long-range low-bandwidth backhaul | 256 |
| `PipeInterface` | External program via stdio | Custom hardware controllers | 1024 |
| `EthernetInterface` | Ethernet/UDP broadcast | LAN/WiFi backhaul | Auto (from bitrate) |
| `UDPInterface` | UDP broadcast | LAN discovery | Auto |
| `TCPClientInterface` | TCP | Remote connections | 65536 |
| `TCPServerInterface` | TCP server | Remote connections | 65536 |
| `I2PInterface` | I2P network | Anonymous overlay | 500 |

**Verified from `Interface.py`:** Each Interface subclass implements `transmit(packet_bytes)` and `receive()` (blocking). The base class provides ingress/egress control, announce rate limiting, MTU auto-calculation from bitrate, and interface mode flags (MODE_FULL, MODE_POINT_TO_POINT, etc.).

## Architecture

```
meshd
  +-- IrohTransport (WiFi/Ethernet backhaul)
  +-- ReticulumTransport
        +-- SerialInterface -> mesh management
        +-- KISSInterface -> amateur radio
        +-- RNodeInterface -> LoRa backhaul
```

## Implementation

- Bundle Reticulum interface drivers as optional mesh transports
- A future build could run a Reticulum process beside meshd and bridge one `GossipTransport`
- `tun/underlay.rs` is the outer routing table (table 210 and relay-route policy). It is not a multi-link `DatagramConn`, and meshd does not spawn a Reticulum `SerialInterface`
- Babel routes IP prefixes. It does not absorb a Reticulum interface by itself

## Benefits

- Mesh nodes connected via LoRa at 50kbps or serial at 115200bps
- The mesh scales to environments where WiFi backhaul is unavailable
- Amateur radio operators can join the mesh using standard equipment
- LoRa extends mesh range to kilometers with minimal power

## Risks

- Reticulum's Python runtime on OpenWrt adds memory overhead
- LoRa carriers have very low bandwidth CRDT sync may be slow
- KISS/AX.25 requires specialized hardware and knowledge

## Cargo Feature

Gate behind `--features reticulum-carriers`. The carriers are optional and only loaded when configured.
