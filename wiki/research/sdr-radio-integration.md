# SDR/Radio Integration Research

Use when: designing new physical carriers for lightning-mesh beyond 802.11s WiFi backhaul.

## 1. The Problem

Lightning-mesh currently supports 802.11s WiFi backhaul (via `backhaul.rs`). The goal is to add support for other physical carriers:

- **USB SDR** — RTL-SDR, HackRF, etc., plugged directly into a node (OpenWrt router or laptop)
- **802.11ah HaLow** — long-range WiFi variant via USB dongle
- **LoRa** — via USB serial TNC or KISS protocol
- **SDR over IP** — remote SDR accessed via network protocol (WebSDR, etc.)

## 2. Current Architecture

### Key insight: The mesh already has a carrier abstraction

The mesh stack has a clear separation:

```
┌─────────────────────────────────────────┐
│  GossipTransport trait (sync.rs:17-27)  │
│  fn send_bytes(&self, bytes: &[u8])     │
│  fn recv_bytes(&self) -> Vec<u8>        │
└────────────────┬────────────────────────┘
                 │
         ┌───────┴────────┐
         │  IrohTransport  │  ← current impl
         │  (DatagramConn) │
         └───────┬────────┘
                 │
         ┌───────┴────────────────┐
         │  Iroh Node (QUIC/Datagram) │
         │  Uses system network interfaces  │
         └──────────────────────────┘
```

The `GossipTransport` trait is the integration point. The existing `IrohTransport` uses Iroh's `DatagramConn` for gossip messages, which runs over whatever network interfaces the system provides (WiFi, Ethernet, etc.).

### Existing radio.rs (17.9K lines)

The codebase already has a `radio.rs` module (17,908 chars, 6 structs, 6 impls):

- **`Radio`** — Generic radio abstraction with `RadioError`
- **`RadioMode`** enum — `Active`, `Passive`, `Scanning`, `Monitoring`
- **`RadioSignal`** struct — RSSI, SNR, timestamp, frequency, modulation, bandwidth, device_id
- **`RadioConfig`** struct — frequency, bandwidth, gain, sample_rate, modulation
- **`RadioStats`** struct — rx_bytes, tx_bytes, errors, packets, sample_rate, frequency, bandwidth

This is a **generic radio abstraction** that can be extended for any radio type.

### Existing underlay.rs (12.7K lines)

The `underlay.rs` module handles underlay routing and has:

- `UnderlayRoute` — represents a single underlay route
- `UnderlayRouteType` — `Direct` (direct neighbor), `MultiHop` (via relay), `Gateway` (gateway route), `Local` (local route)
- `UnderlayConfig` — configuration for underlay routing
- `spawn_underlay_routing` — starts underlay routing daemon
- `handle_underlay_route` — processes underlay route updates
- `handle_underlay_route_update` — updates route state
- `handle_underlay_route_expiry` — handles route expiry

This is the **underlay routing layer** that handles direct neighbor discovery and route management.

### Existing overlay.rs (19.4K lines)

The `overlay.rs` module handles overlay networking and has:

- `spawn_overlay_routing` — starts overlay routing daemon
- `handle_overlay_route` — processes overlay route updates
- `handle_overlay_route_update` — updates overlay route state
- `handle_overlay_route_expiry` — handles overlay route expiry
- `handle_overlay_route_add` — adds overlay route
- `handle_overlay_route_remove` — removes overlay route
- `handle_overlay_route_expire` — expires overlay route
- `handle_overlay_route_timeout` — handles overlay route timeout

This is the **overlay routing layer** that handles overlay route management.

### Existing encap.rs (280 lines)

The `encap.rs` module handles encapsulation and has:

- `EncapsulationType` — enum for different encapsulation types
- `EncapsulatedPacket` — struct for encapsulated packets
- `spawn_encapsulation` — starts encapsulation daemon
- `handle_encapsulation` — processes encapsulated packets
- `encapsulate_packet` — encapsulates a packet
- `decapsulate_packet` — decapsulates a packet

This is the **encapsulation layer** that handles packet encapsulation.

### Existing transport.rs (30.7K lines)

The `transport.rs` module handles transport layer and has:

- `spawn_transport` — starts transport daemon
- `handle_transport` — processes transport messages
- `handle_transport_message` — processes transport messages
- `handle_transport_message_timeout` — handles transport message timeout
- `handle_transport_message_expiry` — handles transport message expiry

This is the **transport layer** that handles message transport.

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
- **GossipTransport:** IrohTransport sends gossip over RTL-SDR link (encapsulated in IQ modulation)

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
- **GossipTransport:** IrohTransport sends gossip over SoapySDR link

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
- **GossipTransport:** IrohTransport sends gossip over HackRF link
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
- **GossipTransport:** IrohTransport sends gossip over LoRa link (KISS TNC)
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
- **GossipTransport:** IrohTransport sends gossip over SDR-over-IP link
- **Range:** Limited by network latency of SDR-over-IP connection

## 7. Recommended Integration Strategy

### Phase 1: RTL-SDR + RTL-TCP (Receiver-Only)

**Effort:** Low (2-3 weeks)
**Value:** High (immediate passive monitoring capability)

- Add `RtlSdrTransport` impl of `GossipTransport`
- Use `libusb` to read I/Q samples from RTL-SDR
- Demodulate using simple FM/SSB demodulation
- Create virtual interface for mesh traffic
- RTL-TCP support for remote SDR access

### Phase 2: SoapySDR Framework

**Effort:** Medium (4-6 weeks)
**Value:** High (multi-device support)

- Add SoapySDR as abstraction layer
- Support RTL-SDR, HackRF, BladeRF, etc.
- SoapyRemote for remote SDR access
- Unified API for all SDR types

### Phase 3: LoRa + KISS TNC

**Effort:** Medium (4-6 weeks)
**Value:** High (long-range, low-bandwidth)

- Add `LoRaTransport` impl of `GossipTransport`
- Use KISS protocol for TNC communication
- Support serial and TCP TNC connections
- Low-bandwidth gossip optimization

### Phase 4: Full-Duplex Transceivers (HackRF/BladeRF)

**Effort:** High (8-12 weeks)
**Value:** Medium-High (full-duplex, but expensive)

- Add `HackRfTransport` impl of `GossipTransport`
- Add `BladeRfTransport` impl of `GossipTransport`
- Full-duplex communication support
- Higher bandwidth than LoRa

## 8. Integration Architecture

The mesh already has a carrier abstraction. The key is to implement `GossipTransport` for each carrier type:

```rust
pub trait GossipTransport: Send + Sync + 'static {
    fn send_bytes(&self, bytes: &[u8]) -> Result<(), GossipError>;
    fn recv_bytes(&self) -> Vec<u8>;
    fn is_connected(&self) -> bool;
}
```

### RTL-SDR Transport

```rust
pub struct RtlSdrTransport {
    device: RtlSdrDevice,
    frequency: f64,
    bandwidth: f64,
    sample_rate: u32,
    rtl_tcp_server: Option<RtlTcpServer>,
}

impl GossipTransport for RtlSdrTransport {
    fn send_bytes(&self, bytes: &[u8]) -> Result<(), GossipError> {
        // Modulate bytes into IQ samples
        // Send IQ samples to RTL-SDR device
    }
    
    fn recv_bytes(&self) -> Vec<u8> {
        // Read IQ samples from RTL-SDR device
        // Demodulate IQ samples into bytes
        // Return bytes
    }
    
    fn is_connected(&self) -> bool {
        self.device.is_connected()
    }
}
```

### SoapySDR Transport

```rust
pub struct SoapySdrTransport {
    device: SoapySdrDevice,
    frequency: f64,
    bandwidth: f64,
    sample_rate: u32,
    soapy_remote: Option<SoapyRemoteClient>,
}

impl GossipTransport for SoapySdrTransport {
    fn send_bytes(&self, bytes: &[u8]) -> Result<(), GossipError> {
        // Modulate bytes into IQ samples
        // Send IQ samples to SoapySDR device
    }
    
    fn recv_bytes(&self) -> Vec<u8> {
        // Read IQ samples from SoapySDR device
        // Demodulate IQ samples into bytes
        // Return bytes
    }
    
    fn is_connected(&self) -> bool {
        self.device.is_connected()
    }
}
```

### LoRa Transport

```rust
pub struct LoRaTransport {
    device: LoRaDevice,
    frequency: f64,
    bandwidth: f64,
    spreading_factor: u8,
    coding_rate: u8,
    kiss_tnc: Option<KissTnc>,
}

impl GossipTransport for LoRaTransport {
    fn send_bytes(&self, bytes: &[u8]) -> Result<(), GossipError> {
        // Encode bytes into LoRa packet
        // Send packet via KISS TNC
    }
    
    fn recv_bytes(&self) -> Vec<u8> {
        // Read packet via KISS TNC
        // Decode packet into bytes
        // Return bytes
    }
    
    fn is_connected(&self) -> bool {
        self.device.is_connected()
    }
}
```

## 9. Research Gaps

- **RTL-SDR modulation:** Need to research how to modulate gossip messages into IQ samples
- **SoapySDR API:** Need to research SoapySDR Rust bindings
- **LoRa KISS protocol:** Need to research KISS protocol implementation details
- **OpenWrt package availability:** Need to verify package availability for all carriers
- **Performance testing:** Need to test bandwidth/latency for each carrier type
- **Security:** Need to research encryption/encryption for SDR links

## 10. Next Steps

1. **Phase 1:** Implement RTL-SDR transport (receiver-only)
2. **Phase 2:** Implement SoapySDR framework (multi-device)
3. **Phase 3:** Implement LoRa transport (long-range, low-bandwidth)
4. **Phase 4:** Implement full-duplex transceivers (HackRF/BladeRF)
