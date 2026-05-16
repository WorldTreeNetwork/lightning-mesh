# Hypothesis: H5 — The Rust Inference Story Is the Actual Binding Constraint

## Summary

**Confirmed with high confidence.** The Rust inference ecosystem is mature enough to be usable but imposes concrete, non-trivial constraints that will shape every model selection decision for the mjolnir-mesh PLC backend. `ort` (ONNX Runtime Rust bindings) is the strongest path for production deployment today: it is actively maintained, supports CPU/CUDA/CoreML/DirectML execution providers, and has documented audio model precedents. `candle` is a viable pure-Rust alternative for small CNN-class models but requires hand-porting or finding a pre-ported architecture. `burn` is too immature for this use case. The codebase's `Recover` trait and `cpal` callback threading model are compatible with stateful neural inference per-frame, but the `Vec<i16>` allocation on every call is a real-time hazard that must be addressed regardless of which inference backend is chosen. The 20 ms / 960-sample budget at 48 kHz is tight for any model larger than a small CNN on CPU — SoTA neural PLC models (LACE, Apollo) are likely too heavy without quantization.

---

## Evidence

### 1. Local codebase: real-time thread constraints

**The `Recover` trait** (`crates/mjolnir-media/src/recover.rs:15-44`) has this exact signature:

```rust
pub trait Recover: Send {
    type Output;
    fn decode(&mut self, packet: &[u8]) -> Result<Self::Output>;
    fn decode_lost(&mut self, lookahead: Option<&[u8]>) -> Result<Self::Output>;
    fn supports_speculation(&self) -> bool { false }
}
```

Key observations:
- `&mut self` — stateful, sequential access only. Per-frame neural inference that mutates model state (RNN/LSTM hidden state, WaveRNN carry) fits naturally. No concurrency complexity needed.
- `Result<Self::Output>` where `Output = Vec<i16>` — **allocates a new `Vec` on every concealed frame.** This is called from the `cpal` output callback (audio thread), which is a real-time context. Heap allocation from a real-time thread is a latency hazard, not a correctness error, but it will cause occasional glitches under memory pressure.
- `anyhow::Result` — inference errors propagate cleanly; the mixer logs them at `debug!` level and outputs silence, which is a reasonable degradation path (`mixer.rs:75`).

**The `cpal` callback path** (`crates/mjolnir-audio/src/mixer.rs:273-286`):

```rust
device.build_output_stream(
    stream_config,
    move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
        // ...
        mix_into(&handles, &mut mix);  // this calls fill_tail → buffer.pull() → recover.decode_lost()
    },
    ...
)
```

`fill_tail` → `SelfHealingBuffer::pull` → `Recover::decode_lost` is the hot path. The entire neural inference call must complete within the cpal callback period. At 48 kHz stereo with a 20 ms frame (`AudioConfig::default()` gives `frame_size() = 960`), that is **20 ms of wall clock** shared across all peers. With N peers, each peer's PLC budget is roughly 20 ms / N.

**The `SelfHealingBuffer`** (`crates/mjolnir-media/src/service.rs:73-170`) holds the `Recover` backend by value behind a generic parameter `R: Recover`. The backend lives inside a `Mutex<PeerSlot>` (`mixer.rs:35-38`). Mutex acquisition on the audio thread is another real-time hazard, already present regardless of inference backend.

**Frame budget arithmetic:**
- 48 kHz, 20 ms frames → 960 samples/frame/channel
- Mono (default config): 960 `i16` samples per pull
- Wall clock budget: 20 ms per frame total, shared across N peers
- For a single peer: ~20 ms available for decode + PLC; for 4 peers: ~5 ms each
- A small CNN (e.g. 1–2 conv layers, ~100k params, `ndarray` or ONNX CPU): typically 0.5–3 ms on modern x86
- A GRU/LSTM-based PLC (e.g. DTLN-style, ~1M params): 3–10 ms on CPU, feasible for 1–2 peers
- SoTA recurrent neural PLC (LACE, Apollo, ~10M+ params): 15–50 ms on CPU, **exceeds budget for even a single peer without an NPU or GPU offload path**

---

### 2. `ort` — ONNX Runtime Rust bindings

**Source:** `https://github.com/pykeio/ort` and `https://ort.pyke.io/`

- Maintained by `pykeio` (Rowan Nulla), not Microsoft. Microsoft maintains the upstream C++ library; `ort` is an independent Rust binding that dynamically links against it.
- Latest version as of search: **2.0.0-rc.12** (still release-candidate after a long RC cycle — the 2.x API has been in RC for over a year, but it is widely used in production). The 1.x series (`ort` = `1.16.x`) is stable and shipped.
- Execution providers: **CPU** (always available), **CUDA**, **TensorRT**, **DirectML**, **CoreML**, **ROCm**, **OpenVINO**, **VitisAI** (AMD AI Engine — directly relevant to the project's AIE aspirations mentioned in `conceal.rs:18`). EP selection is runtime-configurable.
- Allocation behavior: ONNX Runtime does its own arena allocation internally; the Rust binding gives you `Tensor` views. The `Vec<i16>` allocation in `decode_lost` would be the caller's responsibility, not ORT's inner loop. You cannot eliminate it without changing `Output` to a borrowed type, which would require a lifetime parameter on `Recover` — a significant trait API change.
- **Audio precedent:** `sbv2-api` (Style-BERT-VITS2 TTS via ort) and a Qwen3 TTS project where the vocoder runs in ONNX Runtime. Silero VAD (voice activity detection) is a widely-cited `ort` audio example with a documented 1.93x speedup over Python. These are real-time-ish but not hard-real-time 20 ms budget cases.
- **ONNX export reality check:** Many SoTA audio models (Voicebox, SoundStorm, DAC-based vocoders) use custom PyTorch ops (`torch.stft`, `einops` patterns, custom CUDA kernels) that do not export cleanly to ONNX. Simpler architectures (Silero VAD, DTLN, small Conv-TasNet variants) export routinely.

---

### 3. `candle` — Hugging Face pure-Rust ML framework

**Source:** `https://github.com/huggingface/candle`

- Maintained by Hugging Face. Apache 2.0 license. Actively developed.
- Pure Rust: no C++ runtime linked, no dynamic library dependency. This is a significant operational advantage for the mjolnir-mesh deployment story.
- **Shipped audio models:** Whisper (ASR), EnCodec (audio codec/compression), MetaVoice-1B (TTS), Parler-TTS. CNN-class architectures (ResNet, EfficientNet, ConvNeXt) are present for vision — the same primitives would work for a small audio CNN.
- **GGUF quantization:** Supported. 4-bit/8-bit quantized GGUF models load natively. This is the primary path for running a model that would otherwise be too heavy for a 5–20 ms CPU budget.
- **No pre-ported neural PLC model exists in candle's examples.** DTLN, LACE, Apollo, etc. are not there. You would need to either (a) port the architecture manually in Rust using candle ops, or (b) use ONNX export + ort instead.
- **Suitability for a small CNN PLC:** High, if you write the model in candle. `candle::Tensor` operations (conv1d, relu, linear) are available and work on CPU via the `ndarray` backend without GPU.
- **Real-time suitability:** Unknown. candle's allocation profile inside a tight inference call has not been publicly characterized for hard-real-time contexts. The Qwen3-ASR Rust example (`alan890104/qwen3-asr-rs`) shows full-sentence ASR, not 20 ms frame-by-frame concealment.

---

### 4. `burn` — tracel-ai deep learning framework

**Source:** `https://github.com/tracel-ai/burn`, `https://docs.rs/crate/burn/latest`

- Latest: **0.20.1** (Jan 2026), with 0.21.0-pre.2 in preview (March 2026). Versioned below 1.0 — pre-stable API.
- Backends: `wgpu` (WebGPU/GPU), `ndarray` (CPU), `libtorch`, `candle` (burn can use candle as a backend). The `wgpu` backend is the flagship.
- **Audio support:** No dedicated audio model examples found. The project is primarily oriented toward training and vision/LLM inference.
- **Real-time suitability:** Low confidence. `wgpu` inference involves GPU command submission latency that is incompatible with a 5–20 ms hard-real-time audio callback. `ndarray` CPU backend is plausible but untested in this context.
- **Verdict:** Not suitable for this use case today. The API is still unstable, there are no audio inference precedents, and the wgpu-first design philosophy adds latency overhead not appropriate for per-frame audio PLC.

---

### 5. GGML / whisper.cpp ecosystem

**Source:** `https://github.com/ggml-org/whisper.cpp`, `https://github.com/tazz4843/whisper-rs`, `https://github.com/operator-kit/whisper-cpp-plus-rs`

- `whisper-rs` provides safe Rust FFI bindings to whisper.cpp. `whisper-cpp-plus-rs` adds real-time PCM streaming via `WhisperStreamPcm` and tokio integration.
- GGML as a general inference library: the ggml C library underpins llama.cpp and whisper.cpp. No generic "run any audio model in ggml from Rust" path exists — you must use the specific C++ wrapper for a supported model.
- **Applicability to PLC:** Whisper is an ASR model operating on 30-second windows — completely wrong granularity for 20 ms PLC. There is no GGML-native PLC model. The whisper.cpp Rust path is useful for VAD/ASR but not for packet loss concealment.
- A Qwen3 TTS Rust project uses GGUF weights for the LLM stage and ONNX Runtime for the vocoder — confirming the hybrid pattern (GGUF + ort) is practical.

---

### 6. Direct C FFI / other runtimes

- **TensorRT (NVIDIA):** No maintained Rust wrapper that handles the full engine-build + inference lifecycle. Would require unsafe FFI. Only relevant if CUDA is the deployment target.
- **OpenVINO:** Available as an `ort` execution provider, which is the better path than raw FFI.
- **mlx (Apple Silicon):** Rust bindings exist (`mlx-rs`) but are experimental. Relevant only for macOS/Apple Silicon deployment, not a cross-platform story.
- **tract:** (`https://github.com/sonos/tract`) — pure Rust ONNX/NNEF inference, maintained by Sonos. Not mentioned in the search results but highly relevant: it is real-time audio oriented (Sonos uses it for on-device DSP models), has no dynamic library dependency, and has a smaller binary footprint than ort. **This is an under-investigated path and flagged as an open question.**

---

### 7. Trait compatibility with stateful neural inference

The `Recover` trait's `&mut self` on `decode_lost` is correct for stateful models: an RNN's hidden state, a WaveNet's autoregressive buffer, or a CNN's causal conv cache all need mutable access between frames. The factory pattern (`PlcFactory = Arc<dyn Fn(&AudioConfig) -> Result<Box<PlcBackend>> + Send + Sync>`, `conceal.rs:36-37`) means each peer gets its own model instance, which is the right design for stateful per-stream concealment.

The single non-trivial API constraint is the `Vec<i16>` output: any neural backend must allocate a fresh `Vec` per concealment call. At 48 kHz mono, 960 samples × 2 bytes = ~1.9 KB per allocation. Under packet loss bursts this could be several allocations per 20 ms tick. This is acceptable under a non-real-time-class allocator (jemalloc, system malloc on Linux) in practice, but it is a design smell for a strict real-time system.

---

## Confidence

**Level**: high

Multiple independent sources agree on the ecosystem state: GitHub repositories with verifiable commit histories, crates.io version data, and the local codebase provides ground truth for the real-time threading and trait constraints. The frame budget arithmetic is deterministic. The one area of genuine uncertainty is per-model inference latency on the audio thread, which requires benchmarking.

---

## Sources

- [1] **url**: `https://github.com/pykeio/ort` — "Fast ML inference for ONNX models in Rust; v2.0.0-rc.12; execution providers include CPU, CUDA, TensorRT, CoreML, DirectML, VitisAI, OpenVINO"
- [2] **url**: `https://ort.pyke.io/` — "Official ort documentation; audio examples include sbv2-api (Style-BERT-VITS2 TTS) and Silero VAD; 1.93x speedup vs Python on VAD benchmark"
- [3] **url**: `https://github.com/huggingface/candle` — "Pure-Rust ML framework; ships Whisper, EnCodec, MetaVoice-1B, Parler-TTS; GGUF quantization supported; Apache 2.0"
- [4] **url**: `https://github.com/tracel-ai/burn` — "burn v0.20.1 (Jan 2026); wgpu + ndarray + libtorch backends; no audio model examples; pre-1.0 API"
- [5] **url**: `https://github.com/ggml-org/whisper.cpp` — "whisper.cpp: ggml-based ASR; 30s window granularity; not applicable to 20ms PLC"
- [6] **url**: `https://github.com/tazz4843/whisper-rs` — "Safe Rust FFI bindings to whisper.cpp; real-time PCM streaming via WhisperStreamPcm"
- [7] **url**: `https://github.com/operator-kit/whisper-cpp-plus-rs` — "Enhanced whisper.cpp Rust bindings with VAD and tokio integration"
- [8] **url**: `https://calmops.com/programming/rust/real-time-ml-model-development-with-rust-and-onnx-runtime/` — "Real-time ML with Rust and ONNX Runtime; covers threading and latency considerations"
- [9] **file**: `/home/dorje/work/IdentiKey/mjolnir-mesh/crates/mjolnir-media/src/recover.rs:15-44` — "`Recover` trait definition; `&mut self`, `Output = Vec<i16>`, `anyhow::Result`"
- [10] **file**: `/home/dorje/work/IdentiKey/mjolnir-mesh/crates/mjolnir-audio/src/conceal.rs:1-48` — "PlcBackend type alias, PlcFactory pattern, future-backend comment referencing 'neural PLC on CPU, AIE-resident cascade via parakeet-aie'"
- [11] **file**: `/home/dorje/work/IdentiKey/mjolnir-mesh/crates/mjolnir-audio/src/mixer.rs:273-286` — "`cpal` output callback; inference path called from audio thread with no async yield"
- [12] **file**: `/home/dorje/work/IdentiKey/mjolnir-mesh/crates/mjolnir-audio/src/lib.rs:23-48` — "`AudioConfig::default()` gives 48 kHz, mono, 20 ms frames → `frame_size() = 960`"
- [13] **file**: `/home/dorje/work/IdentiKey/mjolnir-mesh/Cargo.toml` — "No `ort`, `candle`, or `burn` in workspace dependencies; no inference crate present yet"

---

## Open Questions

1. **`tract` (Sonos) is uncharacterized here.** `tract` (`github.com/sonos/tract`) is a pure-Rust ONNX/NNEF runtime used by Sonos for on-device real-time audio DSP inference. It may be better suited than `ort` for the no-dynamic-library, hard-real-time-audio-thread constraint. Its latency profile for small models on the audio thread is unknown and warrants a dedicated investigation.

2. **Actual inference latency of candidate PLC model architectures on CPU.** The budget analysis above is order-of-magnitude reasoning. The specific question is: can a DTLN-style or Conv-TasNet-small model run its `decode_lost` path end-to-end in under 3–5 ms on a single x86 core? This requires benchmarking, not analysis.

3. **`Recover` trait allocation contract.** The `Output = Vec<i16>` forces a heap allocation on every concealment call from the audio thread. Changing to `Output = SmallVec<[i16; 960]>` (stack allocation for mono 20 ms frames) or passing in a pre-allocated output buffer would eliminate this. Is the trait API frozen or can it be evolved before the neural backend is wired in? The current Git history shows recent churn in this area (`recover.rs` is listed as modified in the working tree).

4. **ONNX export feasibility for the specific chosen model.** Many SoTA neural PLC models (LACE, Apollo, DeepFilterNet variants) use custom PyTorch operations. The ONNX export path must be validated against the exact model architecture before committing to `ort`. A model that fails ONNX export forces either the candle port path or a Python sidecar.

5. **Multi-peer CPU budget partitioning.** The cpal callback mixes N peers sequentially under a `Mutex<PeerSlot>`. With N > 2 peers and a 3 ms neural inference path per peer, the callback will exceed 20 ms. Whether neural PLC should be run on the audio thread at all (vs. a dedicated thread pool with a fallback to Opus PLC on underrun) is an architectural question not resolved by the current code.

6. **VitisAI / AMD AIE execution provider.** The code comment in `conceal.rs:18` mentions "parakeet-aie" as a future NPU path. The `ort` VitisAI EP supports AMD's AI Engine (Ryzen AI / Versal). Whether the target hardware platform includes an AMD AI Engine — and whether the chosen model fits in AIE SRAM — is unknown.

---

## Sub-Hypotheses

- **[h5a-tract-real-time]**: `tract` (Sonos's pure-Rust ONNX runtime) may be a better fit than `ort` for hard-real-time audio-thread inference in mjolnir-mesh because it has no dynamic library dependency, was designed for on-device DSP workloads, and Sonos has shipped it in real-time audio products — but its per-frame latency, threading model, and audio model coverage have not been verified against the specific 20 ms / 960-sample constraint.

- **[h5b-plc-model-onnx-exportability]**: The practical bottleneck may not be the Rust runtime but whether the best-available small neural PLC model (DTLN, DeepFilterNet-small, a custom Conv1D model) exports to ONNX without custom ops — this determines whether `ort` is usable or whether a candle port or Python sidecar becomes mandatory, and it cannot be resolved without testing each candidate model's `torch.onnx.export()` output.

- **[h5c-audio-thread-offload-architecture]**: The current cpal callback architecture calls `Recover::decode_lost` synchronously on the real-time audio thread; for any neural backend heavier than a trivial CNN, a producer-consumer architecture (inference on a dedicated thread, output ring-buffer consumed by cpal) may be required — this is a structural change to how `SelfHealingBuffer` and `PlcFactory` are used and should be evaluated before any neural backend is selected.
