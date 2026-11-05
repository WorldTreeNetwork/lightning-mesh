---
title: SDR and Radio Integration
created: 2026-11-05
updated: 2026-11-05
type: research
tags: [sdr, radio, soapysdr, carrier]
sources: [crates/mjolnir-mesh/src/crdt/sync.rs, crates/mjolnir-mesh/src/bin/mjolnir-meshd.rs, crates/mjolnir-mesh/src/tun/underlay.rs]
state: [FUTURE]
confidence: medium
---

# SDR and Radio Integration

Use when: designing new physical carriers for lightning-mesh beyond 802.11s WiFi backhaul.

## 1. The Problem

Lightning Mesh runs 802.11s on `br-mesh`. There is no `backhaul.rs`. The goal is to add other physical carriers:

- **USB SDR** — RTL-SDR, HackRF, etc., plugged directly into a node (OpenWrt router or laptop)
- **802.11ah HaLow** — long-range WiFi variant via USB dongle
- **LoRa** — via USB serial TNC or KISS protocol
- **SDR over IP** — remote SDR accessed via network protocol (WebSDR, etc.)

## 2. Current Architecture

`radio.rs` reads `iw dev`, `iw station dump`, and `iw mpath dump`. It writes telemetry for the browser. It does not transmit.

The seam for a new physical carrier is `GossipTransport` in `crates/mjolnir-mesh/src/crdt/sync.rs`:

```rust
#[async_trait::async_trait]
pub trait GossipTransport: Send + Sync {
    async fn broadcast(&self, payload: Bytes) -> Result<(), GossipError>;
    async fn recv(&self) -> Result<Bytes, GossipError>;
}
```

`GossipSync` postcard-encodes a `GossipMessage` and calls that trait. The only production impl is `IrohGossipTransport` inside `mjolnir-meshd`. It wraps iroh-gossip. It is not `DatagramConn`. `DatagramConn` is the TUN encapsulation seam, re-exported from the two-line `tun/encap.rs`.

A second carrier would be a second impl of the same trait, run beside `IrohGossipTransport`. The tree does not have one. A raw second transport does not inherit iroh's QUIC encryption.

| File | Role | About |
|---|---|---|
| `tun/underlay.rs` | Outer FIB policy: table 210, mark `0x4d4a`, `usable_backhaul_binding`, relay routes | 291 lines |
| `tun/overlay.rs` | `spawn_overlay`, `spawn_overlay_routed`, `OverlayHandles` | 462 lines |
| `tun/encap.rs` | Re-exports `DatagramConn`, `EncapError`, `EncapHandles`, `spawn_encap_pair` | 2 lines |
| `bin/meshd/transport.rs` | Endpoint build and the relay `OuterGuard` nft table | 755 lines |
| `bin/mjolnir-meshd.rs` | Daemon, including `IrohGossipTransport` | 11,555 lines |

This tree has no `UnderlayRoute`, `spawn_underlay_routing`, `spawn_overlay_routing`, `spawn_encapsulation`, or `spawn_transport`.

## 3. SDR Landscape Analysis

### 3.1 RTL-SDR (RTL2832U + R820T)

**Status:** ✅ Active, mature, large community
- GitHub: 4.4K stars, active development
- Linux driver: `rtl-sdr` (mainline kernel, `dvb-usb-rtl2832u`)
- OpenWrt: Available as `kmod-usb-dvb-rtl2832u` and `rtl-sdr` package
- Protocols: RTL-SDR native binary IQ stream, RTL-TCP (network streaming)
- Frequency range: 24 MHz – 1.766 GHz
- Sample rate: Up to 2.4 MSPS
- Cost: $10-30 USD
- Use case: Receiver-only, great for listening and passive monitoring

**Protocol detail:** RTL-SDR devices expose a USB DVB-T interface. The `rtl-sdr` userspace library communicates via libusb. The native protocol sends raw I/Q samples over USB. RTL-TCP adds a TCP server that streams these samples over IP, making the SDR accessible remotely.

**Mesh integration path:**
- **Direct USB:** Use `rtl-sdr` library (libusb) to read raw I/Q samples → demodulate → create virtual interface
- **RTL-TCP:** Connect to remote RTL-SDR via TCP, receive I/Q stream → demodulate → create virtual interface
- **Underlay:** RTL-SDR nodes discover each other via beaconing on shared frequency
- **GossipTransport:** a future impl would carry postcard bytes over that link. `IrohGossipTransport` stays on IP

### 3.2 SoapySDR / SoapyRemote

**Status:** ✅ Active, framework approach
- GitHub: SoapySDR 3.3K stars, SoapyRemote for network SDR
- Cross-platform: Linux, macOS, Windows, OpenWrt
- Abstraction layer: Unified API for multiple SDR hardware backends
- SoapyRemote: Client-server protocol for remote SDR access
- Protocols: SoapySDR native, SoapyRemote (TCP)
- Supported devices: RTL-SDR, HackRF, BladeRF, Airspy, SDRplay, LimeSDR, etc.
- Use case: Multi-device, multi-protocol SDR framework

**Protocol detail:** SoapySDR provides a unified API for SDR devices. SoapyRemote adds a TCP server that allows remote access to SDR devices. The protocol uses a simple TCP connection with SoapySDR protocol framing.

**Mesh integration path:**
- **Framework:** Use SoapySDR as the abstraction layer for multiple SDR types
- **SoapyRemote:** Connect to remote SDRs via TCP
- **Underlay:** SoapySDR handles device discovery, mesh handles routing
- **GossipTransport:** a future impl beside `IrohGossipTransport`, not a change to the iroh path

### 3.3 HackRF One

**Status:** ✅ Active, full-duplex transceiver
- GitHub: GreatScottLab/hackrf, active development
- Linux driver: `hackrf` (mainline kernel, `rfkill`, `libusb`)
- OpenWrt: Available as `kmod-usb-acm` and `hackrf` package
- Frequency range: 1 MHz – 6 GHz
- Sample rate: Up to 20 MSPS
- Cost: $200-300 USD
- Use case: Full-duplex transceiver (TX+RX), more expensive than RTL-SDR

**Mesh integration path:**
- **Direct USB:** Use `libhackrf` to send/receive raw I/Q samples
- **Underlay:** HackRF nodes discover each other via beaconing
- **GossipTransport:** a future impl beside `IrohGossipTransport`
- **Range:** Better range than RTL-SDR (transceiver vs receiver-only)

### 3.4 BladeRF

**Status:** ✅ Active, full-duplex transceiver
- GitHub: Nuand/bladeRF, active development
- Linux driver: `bladeRF` (mainline kernel, `libusb`)
- OpenWrt: Available as `bladeRF` package
- Frequency range: 47 MHz – 6 GHz
- Sample rate: Up to 40 MSPS
- Cost: $300-500 USD
- Use case: Full-duplex transceiver, FPGA programmable

**Mesh integration path:**
- Similar to HackRF, but with FPGA programmability for custom waveforms

### 3.5 SDR# / SDSuperScope / WebSDR

**Status:** ⚠️ WebSDR is active but limited (receiver-only)
- WebSDR: Netherlands, receiver-only, HTTP-based protocol
- SDSuperScope: More modern, supports more devices
- Use case: Remote receiver access, not transceiver

**Mesh integration path:**
- **SDR over IP:** Connect to WebSDR/SDSuperScope server via TCP
- **Underlay:** Receive-only, limited use for mesh (can't transmit)
- **GossipTransport:** Limited — can only receive, not send

## 4. 802.11ah HaLow

**Status:** ⚠️ Limited Linux support, no mature OpenWrt driver
- Linux kernel: Experimental support in mainline (5.x+), but limited
- OpenWrt: No official support yet
- USB dongles: Some exist (e.g., Intel 3165, but no driver)
- Range: Up to 1 km, sub-GHz (900 MHz ISM band)
- Use case: Long-range IoT, but not ready for mesh backhaul

**Mesh integration path:**
- **Not recommended** for near-term integration — kernel support is experimental
- **Wait** for mature OpenWrt driver support
- **Alternative:** Use LoRa for long-range, WiFi for short-range

## 5. LoRa

**Status:** ✅ Active, mature, large community
- Linux driver: `ax25`, `kissftp`, `libserialport`
- OpenWrt: Available as `lora`, `tncax25`, `kiss` packages
- Protocols: KISS (Kitchen Sink Simple Protocol), P2P, LoRaWAN
- Frequency: 868 MHz (EU), 915 MHz (US), 433 MHz (Asia)
- Range: Up to 15 km (line of sight)
- Data rate: 0.3 kbps – 50 kbps (very slow)
- Cost: $5-20 USD
- Use case: Low-power, long-range, low-bandwidth mesh networking

**Mesh integration path:**
- **KISS protocol:** Standard protocol for TNC (Terminal Node Controller) devices
- **Underlay:** LoRa nodes discover each other via beaconing
- **GossipTransport:** a future KISS impl of `broadcast` / `recv`, beside `IrohGossipTransport`
- **Range:** Best range of all options (15 km)
- **Trade-off:** Very low bandwidth (0.3-50 kbps)

## 6. SDR Over IP

**Status:** ✅ Active, multiple protocols
- **RTL-TCP:** RTL-SDR TCP server, binary IQ stream
- **SoapyRemote:** SoapySDR TCP server, unified API
- **WebSDR:** HTTP-based protocol, receiver-only
- **SDSuperScope:** More modern, supports more devices
- **Osmocom IQ:** Osmocom I/Q streaming protocol
- **Use case:** Remote SDR access, SDR sharing

**Mesh integration path:**
- **SDR over IP:** Connect to remote SDR via TCP
- **Underlay:** Remote SDR provides IQ stream → demodulate → virtual interface
- **GossipTransport:** a future impl beside `IrohGossipTransport`. Latency is the SDR-over-IP path, not iroh
- **Range:** Limited by network latency of SDR-over-IP connection

## 7. Recommended Integration Strategy

### Phase 1: SoapySDR + GFSK Modem (5 weeks)

**Effort:** 5 weeks
**Value:** High (multi-device support, works with all SDRs including RX-only)

- Add `soapysdr` crate to workspace
- Implement single `SdrGossipTransport` using `soapysdr::Device` — works with ALL supported devices
- Auto-detect TX/RX capability (RX-only devices get `tx_stream: None`)
- Build GFSK modem (~500 lines Rust) on top of SoapySDR IQ samples
- Wire into meshd as optional second gossip channel
- Device selection via CLI: `--sdr-device "driver=rtlsdr,serial=00000001"`
- New hardware = install SoapySDR module, no code change

### Phase 2: Per-device optimization (as needed)

**Effort:** Variable
**Value:** Medium-High (device-specific features)

- Custom FPGA waveforms on LimeSDR (CSS/LoRa firmware)
- Maximum performance paths bypassing SoapySDR normalization
- Device-specific features not exposed by SoapySDR

## 8. Integration Architecture

The mesh uses a **single generic `SdrGossipTransport`** via SoapySDR, not per-device transports. See `sdr-generic-soapysdr.md` for the full design.

### Generic SdrGossipTransport (via SoapySDR)

```rust
pub struct SdrGossipTransport {
    device: soapysdr::Device,       // Any SoapySDR device
    rx_stream: Option<soapysdr::RxStream>,  // None for RX-only
    tx_stream: Option<soapysdr::TxStream>,  // None for RX-only
    frequency: f64,                  // Center frequency in Hz
    sample_rate: f64,               // Sample rate in SPS
}

#[async_trait::async_trait]
impl GossipTransport for SdrGossipTransport {
    async fn broadcast(&self, payload: Bytes) -> Result<(), GossipError> {
        // TX path: bytes → FCS framing → GFSK mod → IQ samples → write_stream()
    }

    async fn recv(&self) -> Result<Bytes, GossipError> {
        // RX path: read_stream() → GFSK demod → FCS parse → bytes
    }
}
```

One implementation. Works with RTL-SDR, LimeSDR, bladeRF, HackRF, Airspy, PlutoSDR, USRP, and more. New hardware = install SoapySDR module, no code change.

## 9. Research Gaps

- **GFSK modem implementation** — Need to implement GFSK modulator/demodulator in Rust (see `limesdr-bladerf-carrier-mvp.md` for channel plan)
- **SoapySDR thread safety** — Many SoapySDR driver modules have bugs; wrap in `tokio::spawn_blocking` (see `sdr-generic-soapysdr.md`)
- **OpenWrt SoapySDR packages** — Need to cross-compile libsoapysdr + device modules for OpenWrt targets
- **Performance testing** — Need to test throughput/latency/BER for each carrier type
- **Security** — A raw `GossipTransport` of postcard bytes is not inside iroh QUIC. The new carrier has to encrypt, or it sends the CRDT in the clear

## 10. Next Steps

1. **SoapySDR integration** — Add `soapysdr` crate, implement `SdrGossipTransport`
2. **GFSK modem** — Build TX/RX modem on top of SoapySDR IQ samples
3. **meshd integration** — Wire as optional second gossip channel
4. **End-to-end testing** — Two-node MVP (laptops first, OpenWrt second)
