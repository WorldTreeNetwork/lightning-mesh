# Hypothesis: DAC's RVQ Token Structure as PLC Substrate for mjolnir-mesh

## Summary

**Partially confirmed in framing, refuted in the DAC-specific claim.** Using a neural codec's RVQ token space as the target for a separately-trained causal predictor is a legitimate, published approach to generative PLC and slots cleanly into mjolnir-mesh's `Recover` trait + per-peer inference thread + tract scaffold. **However, DAC is the wrong codec for the role.** DAC's non-causal encoder imposes ~190 ms algorithmic delay and no streaming mode exists; obtaining tokens at runtime is impossible within a sub-100 ms budget. The correct substrate is a *causal* tokenizer — Mimi (Kyutai, Apache 2.0) is the field-consensus choice and is **already named as the reference codec in mjolnir-mesh's own `docs/architecture/neural-bridge-plc.md`**. DAC's role is at most an offline training-time quality-ceiling reference, not a runtime component.

## Evidence

### 1. mjolnir-mesh Repo: Trait Definitions, Scaffold, Threading

**`Recover` trait** (`crates/mjolnir-media/src/recover.rs:15–44`):

```rust
pub trait Recover: Send {
    fn decode(&mut self, packet: &[u8], out: &mut [i16]) -> Result<()>;
    fn decode_lost(&mut self, lookahead: Option<&[u8]>, out: &mut [i16]) -> Result<()>;
    fn supports_speculation(&self) -> bool { false }
}
```

- Output: caller-owned `&mut [i16]` (no allocation on inference path).
- Slice sized for one frame: 960 samples @ 48 kHz mono / 20 ms.
- `Send` for cross-thread use; blanket impl over `Box<dyn Recover>`.

**Type aliases** (`crates/mjolnir-audio/src/conceal.rs:38–43`):
```rust
pub type PlcBackend = dyn Recover + Send;
pub type PlcFactory = Arc<dyn Fn(&AudioConfig) -> Result<Box<PlcBackend>> + Send + Sync>;
```

The `PlcFactory` is the plug point for alternative neural backends.

**Concrete backends**: `OpusPlc` (libopus 1.5+ FARGAN via `decode_lost`) and `SilencePlc`.

**tract scaffold** (`crates/mjolnir-audio/src/plc_tract.rs`):
```rust
pub struct TractPlc {
    decoder: OpusDecoder,
    plan: TypedSimplePlan<TypedModel>,
    frame_samples: usize,
}
```

- Loads ONNX, runs tract optimizer, compiles to runnable plan — fails fast at construction.
- `decode` passes through to inner Opus decoder; `decode_lost` returns explicit error pending model selection.
- Commit 81d2453 names target class: "small-CNN / GRU-class neural PLC models (tPLCnet, PARCnet-IS2-style)."

**Threading model** (`crates/mjolnir-audio/src/mixer.rs`): per-peer tokio inference task owns `SelfHealingBuffer` + `Box<PlcBackend>`. Ticks at 20 ms, pulls one frame via `SelfHealingBuffer::pull`, pushes to `rtrb` SPSC ring. cpal callback drains ring only — never touches backend. Inference task killed via `AbortOnDrop`. Heavier backend (3–10 ms per `decode_lost`) has full 20 ms tick budget.

**Existing design doc** (`docs/architecture/neural-bridge-plc.md`): the project *already has* a v2 design for token-level generative PLC proposing a richer `StreamingRecover` trait:

```rust
pub trait StreamingRecover {
    fn observe(&mut self, frame: CodecFrame) -> AudioOutput;
    fn observe_dred(&mut self, frames: &[CodecFrame], position: FramePos) -> Vec<ReplayHint>;
    fn observe_anchor(&mut self, frames: &[CodecFrame], position: FramePos);
    fn generate(&mut self) -> AudioOutput;
    fn metadata_rx(&self) -> &Receiver<FrameMetadata>;
}
```

**The design doc names Mimi as reference codec (12.5 Hz, 8 codebooks, ~80 ms causal, Apache 2.0) and a Mamba/SSM or streaming Transformer as the token LM. DAC is not mentioned.**

**Prior synthesis** (`docs/research/audio-models-for-neural-plc/synthesis.md`): tract chosen over ort for "pure-Rust, no dynamic library, designed by Sonos for on-device real-time audio DSP." Frame budget 960 samples @ 48 kHz mono / 20 ms.

### 2. Token-Level PLC Literature

- **SoundSpring** (arXiv 2501.12696, IEEE JSAC 2025): neural codec at TX + bidirectional Transformer MLM at RX for joint entropy coding + PLC over RVQ tokens. Three token states: received, lost, RVQ-cascade-invalid. Outperforms traditional PLC under high loss. **Bidirectional (non-causal)** — needs a receive buffer.
- **Opus Deep PLC / FARGAN** (libopus 1.5+): predictor (pitch, Bark coeffs, V/UV) + generator (PCM from features). Token-level PLC paradigm in continuous-latent space. DRED is the redundancy variant (<32 kbps for 1 s of redundancy).
- **Glaris / Error-Resilient Semantic Communication** (arXiv 2512.08203, Dec 2024): dual-function entropy model; PLC reconstructs missing RVQ latents → decodes to PCM.
- **WaveNetEQ / WaveRNN-PLC** (Google, 2019–2022): AR sample generation conditioned on codec state; waveform space; causal.
- **AudioLM-style** (Google, 2023): hierarchical AR over semantic + acoustic tokens. PLC is the degenerate case: prediction during a gap = no-input forward pass. The neural-bridge-plc doc formalizes this: "Whether the next emitted frame is real or hallucinated becomes purely a question of which input source is authoritative."
- **tPLCnet** (Interspeech 2022, 3rd place, MIT): time-domain seq2one GRU on PCM features; no discrete tokens. Direct fit for `TractPlc::decode_lost` after TFLite→ONNX.
- **PARCnet-IS2** (IS2 2024, 416 K params, <11.6 ms on i5): hybrid linear predictor + FF-CNN; named in synthesis doc as music-PLC candidate.

**No published paper uses DAC's RVQ codebooks specifically** (vs SoundStream / EnCodec / Mimi) as a target for causal concealment — consistent with DAC's non-causal nature.

### 3. Sketch and Fatal Flaws

**Theoretical case**: encode training corpus with DAC offline → train small causal AR model to predict next codes → at runtime predict missing codes → DAC decode to PCM. Encoder only needed at training time.

**Fatal Flaw 1 — Encoder is needed at runtime too.** The predictor's hidden state is over DAC token sequences, so every received frame must be re-encoded to update state. DAC's ~190 ms algorithmic delay means the encoder needs ~9 frames of lookahead at 20 ms ticks — never current.

**Fatal Flaw 2 — Compute.** DAC 44 kHz is ~74 M params, no published CPU RTF, likely >1× on commodity x86. Per-peer at 50 Hz requires dedicated GPU per peer.

**Fatal Flaw 3 — No causal mode exists.** Symmetric padding throughout; issue #101 unanswered; issue #39 confirms broken chunked inference. Causal retrain = new codec, not DAC.

**Fatal Flaw 4 — Token-rate mismatch.** DAC 16 kHz: 50 Hz; 44 kHz: 86 Hz. Mimi: 12.5 Hz. Mimi's slower rate trades for cheaper predictor compute.

**Alternative 1 — PCM/Opus-feature predictor, skip DAC entirely.** tPLCnet, FARGAN, PARCnet do this. Causal by design. tPLCnet plugs into `TractPlc::decode_lost` after TFLite→ONNX. **Synthesis doc near-term recommendation.**

**Alternative 2 — Mimi as causal tokenizer.** 79.3 M params, 12.5 Hz, 8 codebooks, ~80 ms causal, 8.1 G FLOPs (~101 MFLOPS per frame @ 12.5 Hz). **Correct medium-term approach per repo design doc.** Open question: per-frame CPU latency.

### 4. Verdict

**Can DAC tokens be obtained at runtime?** **No.** ~190 ms encoder delay + non-causal padding + no streaming + no causal fork. Even the narrower "training-target only" role fails because the predictor must track real-time token state, which requires running the encoder live.

**A causal tokenizer is required.** Mimi is the concrete candidate; the repo's own `neural-bridge-plc.md` already reached this conclusion independently. **DAC is irrelevant to the PLC substrate question.**

## Confidence

**Level**: high. Trait/scaffold/threading read directly from source; literature from 2024–2025 papers; repo design doc independently corroborates the Mimi recommendation. Medium-confidence elements: Mimi per-frame CPU cost (unpublished); tPLCnet GRU-reset fidelity through TFLite→ONNX (untested).

## Sources

- [1] /home/dorje/work/IdentiKey/mjolnir-mesh/crates/mjolnir-media/src/recover.rs:15–44
- [2] /home/dorje/work/IdentiKey/mjolnir-mesh/crates/mjolnir-audio/src/conceal.rs:38–43
- [3] /home/dorje/work/IdentiKey/mjolnir-mesh/crates/mjolnir-audio/src/plc_tract.rs
- [4] /home/dorje/work/IdentiKey/mjolnir-mesh/crates/mjolnir-audio/src/mixer.rs
- [5] /home/dorje/work/IdentiKey/mjolnir-mesh/docs/architecture/neural-bridge-plc.md
- [6] /home/dorje/work/IdentiKey/mjolnir-mesh/docs/research/audio-models-for-neural-plc/synthesis.md
- [7] /home/dorje/work/IdentiKey/mjolnir-mesh/docs/research/descript-audio-codec/hypotheses/h3-realtime-suitability/findings.md
- [8] https://arxiv.org/abs/2501.12696 — SoundSpring (IEEE JSAC 2025)
- [9] https://arxiv.org/pdf/2512.08203 — Glaris (Dec 2024)
- [10] https://huggingface.co/docs/transformers/en/model_doc/mimi
- [11] https://kyutai.org/Moshi.pdf
- [12] https://github.com/kyutai-labs/moshi
- [13] https://dl.acm.org/doi/fullHtml/10.1145/3561212.3561226 — AR PLC for networked music (Audio Mostly 2022)
- [14] https://www.isca-archive.org/interspeech_2024/muller24c_interspeech.pdf
- [15] https://github.com/descriptinc/descript-audio-codec/issues/101
- [16] https://github.com/descriptinc/descript-audio-codec/issues/39
- [17] https://github.com/breizhn/tPLCnet

## Open Questions

1. **Mimi encoder CPU latency per frame at 12.5 Hz on a single x86 core** — single most important number for committing to the Mimi-token predictor path.
2. **Whether the Moshi Rust backend exposes Mimi encoder/decoder as separable ONNX or tract-compatible modules.**
3. **tPLCnet GRU reset-gate survival through `tf2onnx`** — needs a 1-hour spike.
4. **Whether `TractPlc` can host a Mimi-token predictor at all** — Mamba/SSM or 50–300 M-param transformer likely exceeds tract's efficient op coverage; `ort` may be required.
5. **Opus 50 Hz vs Mimi 12.5 Hz token-rate reconciliation** — 4:1 downsampling or different concealment granularity needed.
