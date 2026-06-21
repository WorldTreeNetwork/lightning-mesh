# mjolnir-audio

Real-time multi-peer voice for mjolnir-mesh. Opus capture, encode, decode,
mix, and play — with packet loss concealment that is willing to be more
than a stalling extrapolator.

This crate owns everything that happens to *audio samples*, from the
microphone to the speaker, on a mesh peer. The wire-level transport
(QUIC bidi streams, datagrams, MoQ broadcast) lives outside; consumers
feed decoded payloads in and pull mixed PCM out.

## What's in here

```
capture.rs    cpal input → Opus encoder → encoded frame stream
codec.rs      Opus encode/decode wrappers
conceal.rs    PLC backends: OpusPlc (CPU default, FARGAN-capable),
              SilencePlc (baseline)
plc_tract.rs  Scaffolding for ONNX-hosted neural PLC via Sonos `tract`
device.rs     cpal device selection
mixer.rs      Per-peer jitter-buffer-driven decode + sum → cpal output
lib.rs        AudioConfig (sample rate / channels / bitrate / frame ms)
```

## The mental model

Audio at the receiver is a steady pull from the speaker driver running on
a real-time callback. Once per frame, the mixer asks every peer's
[`SelfHealingBuffer`](../mjolnir-media) (from `mjolnir-media`) for its
next PCM frame, sums them, and hands the result back to `cpal`.

The buffer answers with PCM regardless of what's on the wire:

- If a packet arrived in time → real audio
- If it's late or lost → a PLC backend synthesises a fill frame
- If forward-error-correction data is sitting in the next-arrived packet
  → the lookahead is used to reconstruct the lost frame exactly

The PLC backend is the [`Recover`](../mjolnir-media/src/recover.rs) trait,
defined in `mjolnir-media` so it can be reused by future media types
(video, anything else with codec state). This crate provides the
audio-specific implementations.

## PLC backends

| Backend       | When it fires                       | Quality                                                          |
|---------------|-------------------------------------|------------------------------------------------------------------|
| `OpusPlc`     | Default. CPU-cheap.                 | Opus heuristic LPC; auto-upgrades to FARGAN neural PLC at libopus 1.5+ |
| `SilencePlc`  | Tests, dropout-audibility demos.    | Zeros on loss (worst-case reference)                              |
| `TractPlc`    | Scaffolding (`plc_tract.rs`).       | Future ONNX-hosted neural backends; not yet a production option   |

All backends implement the same `Recover` trait:

```rust
fn decode(&mut self, packet: &[u8], out: &mut [i16]) -> Result<()>;
fn decode_lost(&mut self, lookahead: Option<&[u8]>, out: &mut [i16]) -> Result<()>;
fn supports_speculation(&self) -> bool { false }
```

Output is written into a caller-owned slice — no heap allocation on the
audio thread. `lookahead` is non-destructive: when present, FEC-capable
backends use it to reconstruct the lost frame; the lookahead packet is
left in the buffer and decoded normally at its own scheduled slot.

The `Mixer` mints one backend per peer via a `PlcFactory`. To swap
implementations (e.g. for tests, for benchmarking, for a future neural
backend) build the mixer with a different factory:

```rust
let mixer = Mixer::with_factory(cfg, silence_plc_factory());
```

## Roadmap for the PLC lane

The audio concealment story is a cascade, not a single mechanism:

| Gap length    | Mechanism                                | Status                                |
|---------------|------------------------------------------|---------------------------------------|
| 0–20 ms       | Opus heuristic                           | In `OpusPlc::decode_lost(None, …)`    |
| 20–80 ms      | FARGAN neural deep-PLC                   | Auto-active at libopus 1.5+ link time |
| 80–1000 ms    | DRED sender-side redundancy              | Designed; planned                     |
| 200 ms – 30 s | Neural Bridge PLC (streaming speech LM)  | Designed; planned (v2 lane)           |

The detailed plan for libopus 1.5+ enablement, DRED, and the standalone
neural-PLC investigation is in
[`docs/research/audio-models-for-neural-plc/synthesis.md`](../../docs/research/audio-models-for-neural-plc/synthesis.md).

The neural bridge engine — a streaming speech LM trained with
fill-in-the-middle masking that reconciles four input streams (live,
DRED-past, future-anchor, self-hallucination) through a speculative
output buffer with entropy-adaptive depth — is described in
[`docs/architecture/neural-bridge-plc.md`](../../docs/architecture/neural-bridge-plc.md).

The trait surface in this crate (`PlcBackend`, `PlcFactory`, the boxed
`Recover` shape) is intentionally small so adding a new backend stays a
contained change.

## Why this lives in mjolnir-mesh

Mesh networks deliver packets over multiple paths and reroute when paths
fail. Voice over a mesh is the acid test of whether the rest of the
architecture is actually producing something a person can hear cleanly.
This crate is where the codec, the buffer, and the concealment model
meet — the place the user judges the network from.

See the top-level [`README.md`](../../README.md) for the broader mesh
vision and how this slots in.

## Dependencies of note

- [`opus`](https://crates.io/crates/opus) — libopus FFI. Build with
  libopus 1.5+ to activate FARGAN deep-PLC; older versions silently fall
  back to the heuristic.
- [`cpal`](https://crates.io/crates/cpal) — cross-platform audio I/O.
- [`rtrb`](https://crates.io/crates/rtrb) — single-producer single-consumer
  ring buffer for the audio-thread boundary.
- [`tract-onnx`](https://crates.io/crates/tract-onnx) — future neural PLC
  backend host (currently scaffolding only).
- [`mjolnir-media`](../mjolnir-media) — provides `Recover`, the jitter
  buffer, and `SelfHealingBuffer`.

## References

- [Self-healing jitter buffer design](../../docs/architecture/self-healing-jitter-buffer.md)
- [Neural bridge PLC design](../../docs/architecture/neural-bridge-plc.md)
- [Neural PLC research synthesis](../../docs/research/audio-models-for-neural-plc/synthesis.md)
- [`mjolnir-media`](../mjolnir-media) — the trait + buffer this crate plugs into
