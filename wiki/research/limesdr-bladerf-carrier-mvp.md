---
title: LimeSDR and bladeRF as Mesh Carrier Links — MVP Brainstorm
created: 2026-11-05
updated: 2026-11-05
type: research
tags: [sdr, carrier, limesdr, bladerf, openwrt, mvp]
state: [FUTURE]
confidence: medium
---

# LimeSDR and bladeRF as Mesh Carrier Links — MVP Brainstorm

**Goal:** Two OpenWrt/laptop lightning-mesh nodes talk over their SDR wireless link. MVP = 2 nodes, point-to-point, same frequency, half-duplex or full-duplex depending on hardware.

## 1. Hardware Candidates

### LimeSDR-USB

| Property | Value |
|---|---|
| Vendor | MyriadRF (Open Source) |
| Interface | USB 3.0 (Micro-B) |
| Driver | `libusb-1.0`, no kernel module |
| Library | `liblimesuite` (C/C++) + FPGAs loaded from flash |
| FPGA | Xilinx Artix-7 (user-programmable) |
| RX chain | 2× 12-bit ADC, max 61.44 MSPS |
| TX chain | 2× 12-bit DAC, max 61.44 MSPS |
| Frequency | 10 MHz – 3.8 GHz (dual Rx, dual Tx) |
| Power | USB bus-powered (no external PSU) |
| Cost | ~$200-300 |
| OpenWrt | USB 3.0 works on OpenWrt; liblimesuite must be cross-compiled |
| Linux | Mainline: no kernel driver needed (pure libusb) |

### bladeRF 2.0 micro

| Property | Value |
|---|---|
| Vendor | Nuand (closed FPGA, open hardware design) |
| Interface | USB 3.0 (Micro-B) |
| Driver | `libbladeRF` (C/C++), no kernel module |
| FPGA | Xilinx Artix-7 (Nuand-firmware, not user-replaceable) |
| RX chain | 1× 14-bit ADC, max 38 MSPS |
| TX chain | 1× 12-bit DAC, max 38 MSPS |
| Frequency | 47 MHz – 6 GHz |
| Power | USB bus-powered |
| Cost | ~$300-500 |
| OpenWrt | Alpine Linux has `bladerf-libs` package for aarch64 |
| Linux | Mainline: no kernel driver needed (pure libusb) |

### Key comparison

| | LimeSDR-USB | bladeRF 2.0 micro |
|---|---|---|
| Open source FPGA | Yes (user-replaceable) | No (Nuand firmware) |
| Dual Rx/Tx | Yes (2×2) | Yes (1×1) |
| Max sample rate | 61.44 MSPS | 38 MSPS |
| License | MIT (LimeSuite) | GPL-2.0 + LGPL-2.1 + MIT |
| OpenWrt SDK | Needs cross-compile | Alpine edge aarch64 package exists |
| Community | MyriadRF (very active) | Nuand (moderate) |

**Recommendation:** LimeSDR-USB for MVP. More open (user-replaceable FPGA = custom waveforms), higher sample rate, larger community (MyriadRF forum + wiki). bladeRF is a strong second choice.

## 2. Architecture: Where SDR Fits in the Mesh Stack

The SDR carrier would be a new `GossipTransport` beside `IrohGossipTransport`, not a replacement for it:

```
┌─────────────────────────────────────────────────┐
│  GossipTransport trait (crdt/sync.rs)           │
│  async fn broadcast(payload: Bytes)             │
│  async fn recv() -> Bytes                       │
└────────────────┬────────────────────────────────┘
                 │
         ┌───────┴────────┐
         │  IrohGossipTransport  │  ← the one production impl (iroh-gossip)
         └───────┬────────┘
         ┌───────┴────────┐
         │  SdrTransport   │  ← NEW: SDR carrier link
         └───────┬────────┘
                 │
         ┌───────┴──────────────────────┐
         │  LimeSDR / bladeRF hardware   │
         │  liblimesuite / libbladeRF    │
         │  USB 3.0                      │
         └──────────────────────────────┘
```

**What the SDR transport does:**
1. **TX path:** Takes postcard-encoded `GossipMessage` bytes → modulates into baseband IQ samples → sends to SDR hardware
2. **RX path:** Reads raw IQ samples from SDR hardware → demodulates → decodes postcard → delivers `GossipMessage` to `GossipSync`

**What the SDR transport does NOT do:**
- Does NOT handle routing, addressing, or CRDT state
- Does NOT manage peer discovery (that's iroh's job, or the SDR's beaconing)
- Does NOT inherit iroh's QUIC encryption. Postcard bytes on this path are cleartext unless this transport encrypts them

## 3. Modulation Strategy for Gossip Messages

Gossip messages are small (typically < 1 KB). We need a robust, low-bandwidth digital modulation scheme.

### Option A: LoRa-style Chirp Spread Spectrum (CSS)
- **Pros:** Extremely robust, long range, low SNR tolerance
- **Cons:** Very low data rate (0.3-50 kbps), complex to implement in firmware
- **Fit:** Good for low-bandwidth gossip, but LimeSDR's FPGA could handle this

### Option B: GFSK (Gaussian Frequency Shift Keying)
- **Pros:** Simple, well-understood, narrow bandwidth, easy to demodulate
- **Cons:** Lower spectral efficiency, moderate range
- **Fit:** Good MVP choice. 9600 baud GFSK in 200 kHz channel = reliable at -100 dBm
- **Implementation:** Pure software, no FPGA changes needed

### Option C: OFDM (like 802.11ah HaLow, simplified)
- **Pros:** High spectral efficiency, good range
- **Cons:** Complex, needs precise timing synchronization, FPGA changes
- **Fit:** Overkill for MVP, good for Phase 2

### Option D: BPSK/QPSK with convolutional coding
- **Pros:** Simple, moderate data rate (100 kbps - 1 Mbps), well-understood
- **Cons:** Needs carrier synchronization, less robust than CSS
- **Fit:** Good balance for MVP

**Recommendation for MVP:** **GFSK at 9600 baud** in a 200 kHz channel. Simple, proven, works with any SDR, no FPGA changes needed. Can be implemented entirely in software on the host CPU.

### GFSK Channel Plan

| Parameter | Value |
|---|---|
| Center frequency | 915 MHz (US ISM) or 868 MHz (EU ISM) |
| Channel bandwidth | 200 kHz |
| Modulation | GFSK, BT = 0.5 |
| Baud rate | 9600 baud |
| Symbol rate | 9600 symbols/sec |
| Sample rate | 256 kSPS (2.67× oversampling) |
| Max data rate | ~9.6 kbps (after framing/encoding) |
| Range (LOS) | ~5-10 km (915 MHz, 0 dBm TX) |

## 4. MVP Design: Two Nodes, Point-to-Point

```
Node A (OpenWrt router, LimeSDR-USB)          Node B (Laptop, LimeSDR-USB)
─────────────────────────────────────────      ──────────────────────────────────
lightning-meshd                                lightning-meshd
├── GossipSync<IrohGossipTransport>  ──┐       ├── GossipSync<IrohGossipTransport>  ──┐
│   (iroh-gossip over ethernet)         │       │   (iroh-gossip over ethernet)         │
├── GossipSync<SdrGossipTransport>  ────┤       ├── GossipSync<SdrGossipTransport>  ────┤
│   (gossip over SDR link)              │       │   (gossip over SDR link)              │
│                                       │       │                                       │
├── Iroh endpoint (QUIC)                │       ├── Iroh endpoint (QUIC)                │
│   (over ethernet + SDR underlay)      │       │   (over ethernet + SDR underlay)      │
│                                       │       │                                       │
├── Babeld (underlay routing)           │       ├── Babeld (underlay routing)           │
├── TUN (mjolnir0 overlay)              │       ├── TUN (mjolnir0 overlay)              │
└── radio.json (telemetry)              │       └── radio.json (telemetry)              │
```

**Key insight:** The MVP does NOT replace the iroh transport. It **adds** the SDR as a second gossip channel. Both transports feed the same `GossipSync` — gossip messages arrive on whichever path is available. The iroh endpoint uses the SDR as an underlay route (via the existing `OuterGuard` mechanism).

## 5. SDR Transport Implementation

### 5.1 High-Level Design

```rust
// New crate: mjolnir-sdr (or module in mjolnir-mesh)

pub struct SdrGossipTransport {
    sdr_device: Arc<SdrDevice>,     // LimeSDR or bladeRF abstraction
    frequency: f64,                  // Center frequency in MHz
    bandwidth: f64,                  // Channel bandwidth in Hz
    sample_rate: u32,               // ADC/DAC sample rate (SPS)
    tx_chan: tokio::sync::mpsc::Sender<Bytes>,
    rx_abort: tokio::sync::oneshot::Sender<()>,
}

#[async_trait::async_trait]
impl GossipTransport for SdrGossipTransport {
    async fn broadcast(&self, payload: Bytes) -> Result<(), GossipError> {
        self.tx_chan.send(payload).await.map_err(...)
    }

    async fn recv(&self) -> Result<Bytes, GossipError> {
        // This is the receive side of the GossipSync dispatcher
        // The actual IQ→bytes decoding happens in a background task
        // that pushes decoded messages into a separate channel
        // This method reads from that channel
    }
}
```

### 5.2 TX Path (bytes → IQ → hardware)

```
GossipMessage (postcard bytes)
    → FCS frame builder (add header, CRC)
    → GFSK modulator (symbols → complex IQ samples)
    → Decimator to SDR sample rate (256 kSPS)
    → liblimesuite / libbladeRF transmit()
```

### 5.3 RX Path (hardware → IQ → bytes)

```
liblimesuite / libbladeRF receive() → raw IQ samples (f32)
    → AGC (automatic gain control)
    → Matched filter (GFSK)
    → Symbol timing recovery (Gardner or Mueller & Muller)
    → Phase tracking (PLL)
    → Demodulator (frequency discriminator)
    → FCS frame parser (strip header, verify CRC)
    → GossipMessage (postcard bytes)
```

### 5.4 Key Implementation Details

**FCS Framing (simplified AX.25-style):**
```
| PREAMBLE (0x55×8) | ADDR (1 byte) | CTRL (1 byte) | LENGTH (2 bytes) | DATA (N bytes) | FCS (2 bytes CRC-CCITT) |
```

**GFSK Modulation:**
- Pre-multiply message bits by Gaussian filter (BT=0.5)
- Integrate to get phase
- `IQ[n] = cos(phase[n]), -sin(phase[n])`
- Output at 256 kSPS

**GFSK Demodulation:**
- Product detector: `IQ[n] * conj(IQ[n-1])` → phase difference
- Low-pass filter → frequency deviation
- Threshold decision → bits
- FCS validation → accept or discard

## 6. Integration with Existing Mesh Stack

### 6.1 Gossip Layer

The trait in `crdt/sync.rs` can have more than one impl. The tree has one production impl, `IrohGossipTransport`. A second channel is a proposal:

```rust
// In meshd.rs, instead of:
let sync = Arc::new(GossipSync::new(IrohGossipTransport { ... }));

// We'd have:
let iroh_sync = Arc::new(GossipSync::new(IrohGossipTransport { ... }));
let sdr_sync = Arc::new(GossipSync::new(SdrGossipTransport::new(sdr_cfg)?));

// Both feed the same CRDT state
tokio::spawn(iroh_sync.run(merge_handler.clone()));
tokio::spawn(sdr_sync.run(merge_handler));
```

### 6.2 Underlay / Routing Layer

An SDR device is not an underlay route today. `OuterGuard` in `bin/meshd/transport.rs` is the nft mark that sends locally generated iroh packets through table 210. Babel owns peer routes on the 802.11s bridge. A virtual `sdr0` that babel could export is a separate proposal from a `GossipTransport`:

```
ip -6 route add <peer_ula>/128 dev sdr0 proto babel metric 9000
```

Where `sdr0` is a virtual interface created by the SDR transport (similar to how `br-mesh` is the 802.11s underlay).

### 6.3 TUN / Overlay Layer

The overlay TUN (`mjolnir0`) is unchanged. It routes IP packets based on the routing table, which includes both the 802.11s underlay and the SDR underlay routes.

## 7. OpenWrt Considerations

### 7.1 Cross-Compilation

LimeSuite and libbladeRF must be cross-compiled for the OpenWrt target architecture (typically `aarch64_cortex-a53` or `mipsel_mips24kc`):

```bash
# OpenWrt SDK approach:
mkdir ~/openwrt-sdk && cd ~/openwrt-sdk
# Download SDK for target platform
# Build LimeSuite as OpenWrt package:
cp -r ~/LimeSuite package/
# Make menuconfig → select LimeSuite package
make package/LimeSuite/compile V=s
```

### 7.2 Resource Constraints

| Platform | CPU | RAM | USB 3.0 | SDR Feasibility |
|---|---|---|---|---|
| mt7986 (Filco/Mt7981) | 1.3 GHz quad-core A53 | 512 MB | Yes (USB 3.0 port) | ✅ 9600 baud GFSK feasible |
| mt7621 (WRX50) | 880 MHz dual-core MIPSi | 256 MB | No (USB 2.0) | ✅ 9600 baud GFSK feasible |
| x86_64 laptop | Any | Any | Yes | ✅ Any modulation |

**Key concern:** GFSK modulation/demodulation at 9600 baud with 256 kSPS sampling requires ~25× processing per sample. On a Cortex-A53, this is feasible (single-core). On MIPS24K, it may be tight.

### 7.3 USB Device Access

```bash
# udev rules for non-root access (same as current mesh pattern)
# LimeSDR:
SUBSYSTEM=="usb", ATTR{idVendor}=="1d50", ATTR{idProduct}=="6108", MODE="0666"
# bladeRF:
SUBSYSTEM=="usb", ATTR{idVendor}=="2cf0", ATTR{idProduct}=="5272", MODE="0666"
```

## 8. MVP Build Plan

### Phase 1: SoapySDR Integration (1 week)

1. **Add `soapysdr` crate** to the workspace (117K downloads, Apache-2.0)
2. **Single `SdrGossipTransport`** impl using `soapysdr::Device` — works with ANY SoapySDR device
3. **Auto-detect TX/RX capability** — RX-only devices (RTL-SDR, Airspy) get `tx_stream: None`
4. **Device selection** via CLI: `--sdr-device "driver=rtlsdr,serial=00000001"` or `--sdr-device "driver=lms7"`
5. **No per-device code** — new hardware = install SoapySDR module, no code change

### Phase 2: GFSK Modem (2-3 weeks)

1. **FCS frame builder/parser** (AX.25-style framing)
2. **GFSK modulator** (Gaussian filter + phase integration)
3. **GFSK demodulator** (product detector + timing recovery + PLL)
4. **AGC** (automatic gain control loop)
5. **Unit tests** with known IQ patterns

### Phase 3: SdrGossipTransport Integration (1-2 weeks)

1. **Implement `GossipTransport` trait** for `SdrGossipTransport`
2. **Wire into meshd** as an optional second gossip channel
3. **CLI flag:** `--sdr-frequency 915 --sdr-device limesdr`
4. **Config file:** `~/.config/mjolnir/sdr.yaml`

### Phase 4: End-to-End Testing (1 week)

1. **Two laptops:** LimeSDR-USB → Ethernet (baseline) → SDR link
2. **Two OpenWrt nodes:** LimeSDR-USB → SDR link
3. **Measure:** throughput, latency, BER, range
4. **Tune:** sample rate, baud rate, AGC parameters

## 9. Risks and Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| OpenWrt cross-compilation fails | High | Start with laptop testing; OpenWrt is Phase 4 |
| Cortex-A53 CPU too slow for GFSK | Medium | Reduce sample rate to 128 kSPS; use SIMD (NEON) |
| USB 3.0 throughput bottleneck | Low | 9600 baud = 1.2 KB/s — well below USB 3.0 capacity |
| Frequency interference in ISM band | Medium | Frequency hopping or spread spectrum in Phase 2 |
| FPGA firmware mismatch (LimeSDR) | Low | Use factory firmware; user-replaceable FPGA is a feature |
| No mainline kernel driver needed | N/A | Both LimeSDR and bladeRF use libusb (userspace only) |

## 10. What Comes After MVP

| Phase | Feature | Effort |
|---|---|---|
| Phase 2 | Frequency hopping (FHSS) for interference resilience | 2-3 weeks |
| Phase 3 | OFDM modem (higher throughput, ~100 kbps) | 4-6 weeks |
| Phase 3 | Custom FPGA waveform on LimeSDR (CSS/LoRa) | 4-6 weeks |
| Phase 4 | Multi-hop SDR routing (Babeld over SDR underlay) | 2-3 weeks |
| Phase 4 | SoapySDR abstraction (multi-device support) | 2-3 weeks |
| Phase 5 | SDR over IP (SoapyRemote client) | 2-3 weeks |
| Phase 5 | AES-128 encryption on SDR link (before iroh layer) | 1-2 weeks |

## 11. SDR Driver Deep Dive: Why SoapySDR over Direct Drivers

### Why not liblimesuite / libbladeRF directly?

Building per-device FFI bindings (limesuite-sys, bladerf-sys) means:
- One `GossipTransport` impl per device (LimeSDRTransport, BladeRfTransport, RtlSdrTransport...)
- New hardware = new FFI crate + new impl
- More code to maintain, more crates in the workspace

### Why SoapySDR wins

- **One codebase** for 15+ devices — RTL-SDR, HackRF, LimeSDR, bladeRF, PlutoSDR, Airspy, SDRplay, USRP
- **Runtime module loading** — install a SoapySDR module, no code change
- **RX-only support** — automatic, no special case code
- **SoapyRemote** — built-in SDR-over-IP
- **`soapysdr` Rust crate** — 117K downloads, pre-generated bindings, safe wrappers
- **~200 lines** for generic `SdrGossipTransport` vs ~200 lines per device

### Direct driver approach still useful for...

- **Custom FPGA waveforms** on LimeSDR (user-replaceable FPGA → upload CSS/LoRa firmware)
- **Maximum performance** (SoapySDR adds one extra copy through its normalization layer)
- **Device-specific features** not exposed by SoapySDR (e.g., LimeSDR FPGA calibration)

For MVP: SoapySDR is the right choice. Direct drivers can be added later for specific hardware features.

### GNU Radio note

GNU Radio is NOT needed. It's a flow-graph editor with huge dependencies (100+ MB, boost, swig, python3, GTK). We just need raw IQ samples from the SDR — SoapySDR gives us that in ~50 lines of Rust. GNU Radio is useful only for prototyping modems, not production code.

## 12. Summary: MVP Scope

| Component | Status | Effort |
|---|---|---|
| LimeSDR-USB hardware | ✅ Proven, open source, USB 3.0 | $0 (buy hardware) |
| liblimesuite FFI bindings | 🔲 Need to write | 3 days |
| GFSK modem (TX + RX) | 🔲 Need to write | 1 week |
| SdrGossipTransport | 🔲 Need to write | 3 days |
| meshd integration | 🔲 Need to wire | 2 days |
| OpenWrt cross-compile | 🔲 Need to test | 2 days |
| **Total MVP** | | **~5 weeks** (SoapySDR + GFSK modem + meshd integration) |

**MVP Deliverable:** Two nodes (laptops first, OpenWrt second) connected via LimeSDR-USB at 915 MHz, running gossip messages over the SDR link at 9600 baud GFSK.
