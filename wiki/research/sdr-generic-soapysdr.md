---
title: Generic SDR Abstraction with SoapySDR
created: 2026-11-05
updated: 2026-11-05
type: research
tags: [sdr, soapysdr, carrier, generic, rx-only, abstraction]
state: [FUTURE]
confidence: high
---

# Generic SDR Abstraction with SoapySDR

Nothing in this tree calls SoapySDR. This page is a proposal for one `GossipTransport` over `soapysdr::Device`.

**Goal:** One codebase that works with any SDR hardware — RTL-SDR ($30), LimeSDR ($250), bladeRF ($350), HackRF ($340), Airspy ($100), PlutoSDR ($250), USRP ($2,100) — including RX-only devices.

## 1. Why GNU Radio is NOT the answer

GNU Radio is a pain for our use case:

- **Huge dependency chain** — 100+ MB install, needs boost, swig, python3, GTK
- **Breaking changes** — Python 2→3 migration broke everything, build system changes every version
- **C++ boilerplate** — Custom blocks need cmake, swig bindings, gr_modtool scaffolding
- **Not Rust-friendly** — It's Python/C++, not a library you embed in Rust
- **Overkill** — GNU Radio is a flow-graph editor. We just need raw IQ samples

GNU Radio is useful for **prototyping** a modem, but the production code needs just:

```
Rust code → SDR library → raw IQ samples (f32) → our GFSK modem → bytes
```

The SDR driver layer is ~50 lines of Rust. The modem is ~500 lines. We build the modem, not the driver.

## 2. How SDRs are actually driven (simple way)

Both LimeSDR and bladeRF use **pure userspace USB libraries**. No kernel drivers. No GNU Radio.

**LimeSDR — 10 function calls:**
```
LMS_GetDeviceList() → LMS_Open() → LMS_SetSampleRate() → LMS_SetLOFrequency()
→ LMS_SetupStream() → LMS_StartStream() → LMS_RecvStream() / LMS_SendStream()
```

**bladeRF — 5-6 function calls:**
```
bladerf_open() → bladerf_set_frequency() → bladerf_set_sample_rate()
→ bladerf_sync_config() → bladerf_sync_rx() / bladerf_sync_tx()
```

Both talk USB directly via libusb-1.0. The API returns **complex IQ samples as float arrays** (or int16 for LimeSDR).

## 3. SoapySDR: The "USB driver for SDRs"

Just like `libusb` abstracts USB devices, **SoapySDR** abstracts SDR hardware. You write code against the SoapySDR API once, and it works with any supported device.

### 3.1 Supported devices (one API, all hardware)

| Hardware | SoapySDR Module | RX | TX | Duplex | Price |
|---|---|---|---|---|---|
| RTL-SDR v3/v4 | SoapyRTLSDR | ✅ | ❌ | RX-only | $30-40 |
| HackRF One | SoapyHackRF | ✅ | ✅ | Half | $340 |
| LimeSDR-USB | SoapyLMS7 | ✅ | ✅ | Full | $250 |
| bladeRF 2.0 | SoapySDRBladeRF | ✅ | ✅ | Full | $350 |
| PlutoSDR | SoapyPlutoSDR | ✅ | ✅ | Full | $250 |
| Airspy R2 | SoapyAirspy | ✅ | ❌ | RX-only | $100 |
| SDRplay RSP | SoapySDRPlay | ✅ | ❌ | RX-only | $130 |
| USRP B210 | SoapyUHD | ✅ | ✅ | Full | $2,100 |

### 3.2 The API is the same for all devices

```rust
// Same code works for RTL-SDR, LimeSDR, bladeRF, HackRF, etc.
let devices = soapysdr::enumerate()?;                          // Find all SDRs
let device = soapysdr::Device::from_args("driver=rtlsdr")?;    // or "driver=lms7" or "driver=bladerf"
device.set_sample_rate(0, 2.4e6)?;                              // Set sample rate
device.set_frequency(0, soapysdr::Direction::Rx, 915e6)?;      // Set frequency
device.set_gain(0, soapysdr::Direction::Rx, 30.0)?;            // Set gain
let rx_stream = device.setup_stream(soapysdr::Direction::Rx, "CF32")?;  // Complex float32
// Now read IQ samples — same code regardless of hardware
```

The `setupStream()` call takes a format string. `"CF32"` = complex float32. **Every device returns the same format**, even if the hardware uses 8-bit or 12-bit ADCs. SoapySDR normalizes it.

### 3.3 Plugin architecture

SoapySDR uses runtime-loadable modules. Hardware support is added through dynamically loaded libraries:

```
┌─────────────────────────────────────┐
│  Application (lightning-meshd)      │
│  Uses SoapySDR C API                │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│  SoapySDR core library              │
│  (libsoapysdr0.8)                   │
│  C API + module loader              │
└──────┬──────────┬──────────┬────────┘
       │          │          │
  ┌────▼───┐ ┌───▼────┐ ┌───▼──────┐
  │SoapyRTL│ │SoapyLMS│ │SoapyHack │
  │SDR     │ │7       │ │RF        │
  │module  │ │module  │ │module    │
  └────────┘ └────────┘ └──────────┘
```

New SDR support = install a SoapySDR module, no code changes.

### 3.4 System dependencies

On Ubuntu:
```bash
sudo apt install libsoapysdr-dev
sudo apt install soapysdr-module-rtlsdr soapysdr-module-lms7 soapysdr-module-hackrf
```

On macOS:
```bash
brew install soapy-sdr
brew install soapyrtlsdr soapyhackrf soapybladerf
```

No compile-time dependency on specific hardware. Drivers are loaded dynamically at runtime.

## 4. Rust bindings: `soapysdr` crate

The [`soapysdr`](https://crates.io/crates/soapysdr) crate wraps SoapySDR 0.8.x:

- **117K downloads**, 105 stars on GitHub
- Pre-generated bindings (no clang at build time)
- Safe Rust wrappers around the C API
- MSRV: Rust 1.85
- License: Apache-2.0 / BSL-1.0

**Caveat:** Many SoapySDR driver modules have error handling and thread safety bugs. The crate provides safe Rust wrappers assuming the drivers meet the SoapySDR core API contract. For atypical usage, wrap stream calls in `tokio::spawn_blocking` for safety.

## 5. RX-only devices: how they work in the mesh

RX-only devices (RTL-SDR, Airspy, SDRplay) just don't have a TX stream. The API handles this at runtime:

```rust
pub struct SdrGossipTransport {
    device: soapysdr::Device,
    rx_stream: Option<soapysdr::RxStream>,
    tx_stream: Option<soapysdr::TxStream>,  // None for RX-only devices
    frequency: f64,
}

impl SdrGossipTransport {
    pub fn new(device: soapysdr::Device) -> Result<Self> {
        let has_tx = device.supports_tx();
        let has_rx = device.supports_rx();  // Always true for valid devices
        
        Ok(Self {
            device,
            rx_stream: if has_rx { Some(setup_rx(&device)?) } else { None },
            tx_stream: if has_tx { Some(setup_tx(&device)?) } else { None },
            frequency: 915e6,
        })
    }
}
```

### 5.1 RX-only mesh behavior

A node with only an RX-only SDR:

1. **Receives** gossip messages from other nodes on the SDR frequency
2. **Forwards** them via another carrier (WiFi/ethernet mesh, or another SDR node with TX)
3. **Transmits** via a different carrier (e.g., another node's SDR, or iroh-gossip over WiFi)

This is useful as a **low-cost wide-area receiver** — a node with a $30 RTL-SDR and a directional antenna can receive mesh traffic from 10+ km away and forward it over WiFi to the rest of the mesh.

## 6. Practical deployment scenarios

### Scenario A: Two full-duplex nodes (LimeSDR-USB)

Both nodes TX+RX on SDR frequency. Full bidirectional mesh over SDR link. Like a point-to-point radio bridge.

**Cost:** 2 × $250 = $500

### Scenario B: One TX + many RX (1× LimeSDR + 3× RTL-SDR)

LimeSDR node broadcasts mesh gossip on SDR frequency. RTL-SDR nodes receive and forward via WiFi/ethernet. RTL-SDR nodes reply via WiFi back to the LimeSDR node.

**Cost:** $250 + 3 × $30 = $340 for 4-node mesh

### Scenario C: Half-duplex (HackRF One)

TX and RX on same frequency, not simultaneous. Time-slice the link: TX for 500ms, RX for 500ms. Lower throughput but works with wider frequency range (1 MHz – 6 GHz).

**Cost:** $340 per node

### Scenario D: SDR-over-IP (SoapyRemote)

SDR plugged into a Raspberry Pi. Other mesh nodes access it over Ethernet via SoapyRemote. Same SoapySDR API, just `driver=remote,remote-host=192.168.1.50`.

**Cost:** $30 RTL-SDR on Pi + $0 network hop

### Scenario E: Mixed fleet (realistic production)

| Node | Hardware | Role | Cost |
|---|---|---|---|
| Gateway | LimeSDR-USB | TX+RX SDR backhaul | $250 |
| Outpost 1 | RTL-SDR v4 | Wide-area RX, WiFi forward | $40 |
| Outpost 2 | RTL-SDR v4 | Wide-area RX, WiFi forward | $40 |
| Mobile | HackRF One | TX+RX in the field | $340 |
| Lab | Airspy R2 | High-sensitivity RX monitoring | $100 |

**Total:** $770 for 5-node mesh with diverse capabilities.

## 7. Architecture: SoapySDR in the mesh stack

```
┌─────────────────────────────────────────────┐
│  SdrGossipTransport (generic via SoapySDR)  │
│  - soapysdr::Device abstraction             │
│  - Detects TX/RX capability at runtime      │
│  - Normalized CF32 IQ samples               │
└────────────────┬────────────────────────────┘
                 │
    ┌────────────┼────────────┐
    │            │            │
  TX path      RX path    Config
    │            │            │
    ▼            ▼            ▼
  GFSK       GFSK       set_freq/
  modem      modem      gain/srate
    │            │
    ▼            ▼
  IQ→bytes   bytes→IQ
```

The `SdrGossipTransport` is a single `GossipTransport` implementation that works with any SoapySDR device. No per-device transport classes needed.

### 7.1 TX path (bytes → IQ → SoapySDR → RF)

```
GossipMessage (postcard bytes)
    → FCS frame builder (add header, CRC)
    → GFSK modulator (symbols → complex IQ samples)
    → Decimator to SoapySDR sample rate
    → soapysdr::TxStream::write_stream()
```

### 7.2 RX path (SoapySDR → IQ → bytes)

```
soapysdr::RxStream::read_stream() → complex float32 IQ samples
    → AGC (automatic gain control)
    → Matched filter (GFSK)
    → Symbol timing recovery (Gardner or Mueller & Muller)
    → Phase tracking (PLL)
    → Demodulator (frequency discriminator)
    → FCS frame parser (strip header, verify CRC)
    → GossipMessage (postcard bytes)
```

## 8. SoapySDR device discovery and selection

```rust
// Discover all connected SDRs
let devices = soapysdr::enumerate()?;
for dev_info in devices {
    println!("Driver: {}, Serial: {}, Info: {}",
        dev_info.driver,
        dev_info.label,
        dev_info.version
    );
}

// Open a specific device
let device = soapysdr::Device::from_args("driver=rtlsdr,serial=00000001")?;

// Open first available device
let device = soapysdr::Device::from_args("")?;

// Probe device capabilities
let hw_caps = device.get_hw_caps();
let rx_chans = device.list_rx_channels();
let tx_chans = device.list_tx_channels();
let freq_range = device.get_frequency_range(soapysdr::Direction::Rx, 0);
let sr_range = device.get_sample_rate_range(soapysdr::Direction::Rx, 0);
let gain_range = device.get_gain_range(soapysdr::Direction::Rx, 0);
```

## 9. SoapySDR stream API

### 9.1 Setup stream

```rust
// Create RX stream with complex float32 samples
let rx_stream = device.setup_stream(
    soapysdr::Direction::Rx,
    "CF32",  // Complex float32 (8 bytes per element)
    &[],     // Auto-select channel
    &Default::default(),
)?;
```

Format strings:
- `"CF32"` — complex float32 (8 bytes per element)
- `"CS16"` — complex int16 (4 bytes per element)
- `"CS8"` — complex int8 (2 bytes per element)

### 9.2 Read samples

```rust
let mut buffer = vec![Complex::<f32>::ZERO; 4096];
let timeout = std::time::Duration::from_millis(100);
let status = rx_stream.read(&mut buffer, None, timeout)?;
// status.num_samples = number of samples received
// status.flags = timestamp, overflow, error flags
// status.time_ns = timestamp in nanoseconds
```

### 9.3 Write samples

```rust
let tx_buffer: Vec<Complex<f32>> = generate_gfsk_samples();
let timeout = std::time::Duration::from_millis(100);
let status = tx_stream.write(&tx_buffer, None, timeout)?;
// status.num_samples = number of samples sent
```

### 9.4 Stream status

```rust
let status = stream.get_status()?;
println!("Samples sent: {}, Dropped: {}",
    status.num_transfers,
    status.num_overruns
);
```

## 10. SoapyRemote: SDR-over-IP

SoapyRemote is a built-in SoapySDR module that turns any SDR into a network peripheral:

```bash
# On the machine with the SDR plugged in:
SoapySDRServer --mode=server --port=8080

# On the mesh node accessing it remotely:
let device = soapysdr::Device::from_args(
    "driver=remote,remote-host=192.168.1.50,remote-port=8080"
)?;
```

Same API, zero code changes. The SDR can be on a different subnet, different OS, different architecture.

## 11. Comparison: SoapySDR vs direct driver approach

| | SoapySDR | Direct driver (liblimesuite/libbladeRF) |
|---|---|---|
| Hardware support | 15+ devices | 1 device per impl |
| New hardware | Install module, no code change | Write new FFI bindings |
| Code size | ~200 lines SdrGossipTransport | ~200 lines per device |
| Dependencies | libsoapysdr + modules | liblimesuite + libbladeRF + ... |
| OpenWrt | Cross-compile libsoapysdr once | Cross-compile each library |
| RX-only support | Automatic (no TX stream) | Must check and handle |
| SoapyRemote | Built-in SDR-over-IP | Must implement separately |
| Rust bindings | `soapysdr` crate (117K dl) | `limesuite-sys`, `bladerf-sys` |
| Thread safety | Varies by module (wrap in spawn_blocking) | Varies by library |
| MVP complexity | Lower (one impl for all) | Higher (one impl per device) |

**Recommendation:** SoapySDR for MVP. One codebase, one `GossipTransport` implementation, works with all devices including RX-only. The thread safety caveat is manageable with `tokio::spawn_blocking`.

## 12. Updated MVP plan with SoapySDR

### Phase 1: SoapySDR integration (1 week)

1. Add `soapysdr` crate to workspace
2. Add `SdrGossipTransport` that wraps `soapysdr::Device`
3. Implement `GossipTransport` trait
4. Auto-detect TX/RX capability
5. Support device selection via CLI: `--sdr-device "driver=rtlsdr,serial=00000001"` or `--sdr-device "driver=lms7"`
6. Config file: `~/.config/mjolnir/sdr.yaml`

### Phase 2: GFSK modem (2 weeks)

Same as before — FCS framing, GFSK modulator/demodulator, AGC. Works on top of any SoapySDR device.

### Phase 3: meshd integration (1 week)

Wire `SdrGossipTransport` into meshd as optional second gossip channel. Both SDR and iroh gossip feed the same CRDT state.

### Phase 4: SoapyRemote + testing (1 week)

Test SDR-over-IP. End-to-end two-node testing. OpenWrt deployment.

**Total: ~5 weeks** (slightly longer than direct driver approach due to SoapySDR dependency, but much broader hardware support from day one).

## 13. OpenWrt considerations with SoapySDR

### Cross-compilation

```bash
# OpenWrt SDK approach:
mkdir ~/openwrt-sdk && cd ~/openwrt-sdk
# Download SDK for target platform
cp -r ~/SoapySDR package/
# Build as OpenWrt package:
make package/SoapySDR/compile V=s
# Then add device modules:
cp -r ~/SoapyRTLSDR package/
cp -r ~/SoapyLMS7 package/
make package/SoapyRTLSDR/compile V=s
make package/SoapyLMS7/compile V=s
```

### udev rules

```bash
# SoapySDR uses libusb, same rules as direct drivers
SUBSYSTEM=="usb", ATTR{idVendor}=="0bda", ATTR{idProduct}=="2838", MODE="0666"  # RTL-SDR
SUBSYSTEM=="usb", ATTR{idVendor}=="1d50", ATTR{idProduct}=="6108", MODE="0666"  # LimeSDR
SUBSYSTEM=="usb", ATTR{idVendor}=="2cf0", ATTR{idProduct}=="5272", MODE="0666"  # bladeRF
```

### Resource constraints

SoapySDR is lightweight — the core library is ~2 MB. Device modules add ~1-5 MB each. On a Cortex-A53 with 512 MB RAM, this is negligible. The GFSK modem is the CPU bottleneck, not SoapySDR.

## 14. Summary

| Aspect | Detail |
|---|---|
| **Library** | SoapySDR 0.8.x (C++ core, C API) |
| **Rust bindings** | `soapysdr` crate (117K downloads, Apache-2.0) |
| **Hardware support** | RTL-SDR, HackRF, LimeSDR, bladeRF, PlutoSDR, Airspy, SDRplay, USRP, and more |
| **RX-only support** | Automatic — no TX stream, same code |
| **SDR-over-IP** | SoapyRemote built-in |
| **Dependencies** | `libsoapysdr0.8` + per-device modules (loaded at runtime) |
| **Code size** | ~200 lines for generic SdrGossipTransport |
| **New hardware** | Install module, no code change |
| **MVP effort** | ~5 weeks (SoapySDR + GFSK modem + meshd integration) |
| **Cost range** | $30 (RTL-SDR RX-only) to $2,100 (USRP B210) |

**Bottom line:** SoapySDR gives us one `GossipTransport` implementation that works with any SDR, including RX-only devices. No per-device code. No GNU Radio. Just `soapysdr` crate + our GFSK modem.
