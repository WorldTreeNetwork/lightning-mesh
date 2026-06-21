# mjolnir-media

Transport-agnostic, codec-agnostic media primitives shared across the
mjolnir-mesh real-time stack. This is the layer between "wire bytes
arrived" and "PCM is ready for the speaker." It does not know what
transport delivered the bytes, what codec produced them, or what device
will play them.

What it owns:

- A sequence-keyed jitter buffer with adaptive depth.
- The `Recover` trait — the decode-and-conceal seam every media type
  plugs into.
- `SelfHealingBuffer` — a small service that composes jitter + recover
  into a single "always emits a fresh frame on schedule" pull source.

## What's in here

```
jitter.rs     JitterBuffer<T>: sequence-keyed reorder/dejitter ring
recover.rs    The Recover trait (decode + conceal in one shape)
service.rs    SelfHealingBuffer: jitter + Recover composed into a
              "Redis-style" served-data-structure service
lib.rs        Re-exports
```

## The mental model

A jitter buffer is *not* a dumb FIFO. It is a long-running service that
holds a piece of state (the buffer) and maintains its invariants under
churn. Clients submit ops; the buffer answers reads. The whole framing
is laid out in
[`docs/architecture/self-healing-jitter-buffer.md`](../../docs/architecture/self-healing-jitter-buffer.md);
this crate is the implementation of that frame.

```
Client                                    SelfHealingBuffer
─────────                                 ─────────────────────────────
push_packet(seq, arrival_ts, payload) ─►  decode, place in ring, update loss stats
pop_frame(playback_ts)                ─►  return PCM (real | FEC | concealed)
state()                               ─►  depth, recent loss rate
```

Two minimal commands carry the load. The consumer never has to ask "was
this frame real or concealed?" — it just consumes a clean stream. The
service handles reordering, adaptive depth, and concealment internally.

## The `Recover` trait

```rust
pub trait Recover: Send {
    fn decode(&mut self, packet: &[u8], out: &mut [i16]) -> Result<()>;
    fn decode_lost(&mut self, lookahead: Option<&[u8]>, out: &mut [i16]) -> Result<()>;
    fn supports_speculation(&self) -> bool { false }
}
```

Two concrete responsibilities (decode a received packet; synthesise a
fill on loss) live on one trait because codec-native PLC depends on
state that the same backend's `decode` populates. Splitting them would
force expensive state mirroring.

Three properties worth flagging:

- **Caller-owned output slice.** Every method writes into a
  caller-provided `&mut [i16]`. Backends must not allocate on the
  inference path. The slice is sized for one frame of PCM at the
  configured rate × channels × frame duration.
- **Non-destructive FEC hint.** `decode_lost` takes an optional
  `lookahead` — the next-in-sequence packet, when it has already
  arrived. Codecs with in-band FEC (Opus's bidirectional FEC; redundant
  video slices) can reconstruct the lost frame from it. The hint is
  non-destructive: the lookahead is left in the buffer and returned by
  the next `decode` call.
- **Speculation hint.** `supports_speculation` lets backends that
  predict for free (an NPU-resident cascade running every cycle anyway)
  advertise that fact, so the service can speculate ahead and discard
  the prediction on successful arrival. Defaults to `false`.

The trait is media-generic. The audio impls live in
[`mjolnir-audio::conceal`](../mjolnir-audio/src/conceal.rs); a future
video crate would implement the same trait against H.264/H.265/AV1.

## `SelfHealingBuffer`

The composition you actually consume. Wraps a `JitterBuffer<T>` and any
`Recover` backend. Drains packets to the recover on push; emits a fresh
frame on every pull whether the wire delivered one or not. Returns a
`PullStatus` and `BufferStats` for the caller's telemetry.

Typical use (from `mjolnir-audio::Mixer`):

```rust
let backend: Box<dyn Recover + Send> = plc_factory(&audio_config)?;
let buffer = SelfHealingBuffer::new(audio_config, backend);

// On packet arrival:
buffer.push_packet(seq, arrival_ts, payload);

// Once per audio-thread tick:
let mut frame = [0i16; FRAME_SAMPLES];
let status = buffer.pop_frame(playback_ts, &mut frame);
```

The backend is owned by the buffer. One buffer per peer (per stream).

## Why a separate crate?

Three reasons the jitter buffer and the `Recover` trait don't live in
`mjolnir-audio`:

1. **Reusable for non-audio media.** Video, screen-share, and any future
   codec stream wants the same reorder + conceal shape.
2. **Transport-agnostic.** This crate has no dependency on `cpal`,
   `opus`, `iroh`, or any specific wire format. It's pure primitives.
3. **The trait is the contract.** Backends — including future neural
   ones in different crates or NPU-hosted implementations — plug in
   against a stable shape defined here.

The dependency list reflects this. Just `anyhow` and `bytes`. Nothing
else.

## Roadmap

The current `Recover` trait covers the synchronous decode + per-loss
concealment shape. The
[neural bridge PLC design](../../docs/architecture/neural-bridge-plc.md)
proposes a richer `StreamingRecover` shape that surfaces four input
streams (live, DRED-revealed past, future anchor, self-hallucination)
plus a metadata sidechannel (entropy, confidence, glitch level,
provenance). When that lands it will be an additional trait alongside
`Recover`, not a replacement — most peers will keep the simple shape.

## References

- [Self-healing jitter buffer (design)](../../docs/architecture/self-healing-jitter-buffer.md)
- [Neural bridge PLC (extends this crate's trait surface)](../../docs/architecture/neural-bridge-plc.md)
- [`mjolnir-audio`](../mjolnir-audio) — the consumer that provides the
  audio-side `Recover` impls
- Top-level [`README.md`](../../README.md) — the broader mesh vision
