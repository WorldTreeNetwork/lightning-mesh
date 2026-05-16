# Decomposition: Open-source voice/speech/music generation models for neural PLC backend in mjolnir-mesh

## Understanding

The caller needs a deployment-focused survey of open-source audio generation models (voice cloning, speech synthesis, music gen) evaluated against three hardware tiers (CPU, consumer NVIDIA, AMD AIE/NPU), with specific emphasis on which models can plausibly back a `Recover` trait for neural packet loss concealment in a real-time Rust mesh audio system. A "good answer" produces a tiered shortlist with concrete latency numbers, quantization status, license, streaming capability, and explicit "today / months out / research-only" classification — plus a clear callout of dedicated neural PLC research (LACE/NoLACE/PLC Challenge) which is the closest-fit prior art.

## Sub-Questions

1. **Neural PLC prior art**: What dedicated neural PLC models exist (LACE, NoLACE, PLC Challenge 2022/2024 winners, Microsoft research), what are their architectures/sizes, and which already ship with permissive licenses and reference implementations that could drop into the `Recover` trait?
2. **TTS/voice-cloning models**: Of the current SoTA open TTS/voice-cloning models (XTTS, F5-TTS, StyleTTS2, OpenVoice, CosyVoice, Parler-TTS, Bark, Sesame-CSM, MetaVoice, Tortoise, VALL-E reproductions), which have (a) streaming-friendly architectures, (b) license terms compatible with shipping, and (c) realistic CPU / consumer-GPU latency budgets that fit a PLC gap (tens to low-hundreds of ms)?
3. **Music generation models**: For music-aware concealment (MusicGen, Stable Audio Open, AudioGen, Riffusion, Jukebox-derived), is *any* of them fast enough and streaming-capable enough to be relevant to gap-fill at PLC timescales, or is this category strictly offline/batch and therefore out of scope for v1?
4. **Quantization & runtime maturity**: What is the actual current state of int8/int4/GGUF/GPTQ/AWQ/ONNX/CoreML/TensorRT support across these model families, and which ones have working Rust-callable inference paths (ONNX Runtime, candle, burn, llama.cpp-style ggml, direct FFI)?
5. **AMD AIE / Ryzen AI / Versal NPU support**: What is the realistic state of open-source audio-model deployment on AMD's AIE tile arrays today — Vitis AI, Ryzen AI SW, ONNX EP for NPU, IREE/MLIR paths — and which model architectures (CNN-based PLC vs. transformer LM vs. diffusion) actually map well to AIE memory/dataflow constraints?

## All Candidate Hypotheses

### H1: Dedicated neural PLC (LACE/NoLACE + PLC Challenge winners) is the only "today" answer for the `Recover` trait; everything else is months-out or wrong-tool
- **Plausibility**: high | **Info Value**: high | **Type**: web
- **Rationale**: LACE/NoLACE are explicitly Opus extensions designed for sub-10ms-frame neural PLC at CPU-deployable sizes (Xiph/Mozilla/IETF work). The Interspeech 2022/2024 PLC Challenge produced reference implementations sized for real-time. If true, this dramatically narrows v1 scope and de-risks the upgrade path from the existing Opus PLC backend.
- **If true**: We have a clear v1 target (LACE or a PLC Challenge winner), and TTS/music become v2+ enhancements rather than the backbone.
- **If false** (e.g., licenses are restrictive, or models don't actually beat Opus PLC at >80ms gaps): We're forced into the larger/slower TTS-as-PLC category much earlier, raising the cost floor significantly.
- **Effort**: medium

### H2: F5-TTS and CosyVoice 2 (2024-2025 flow-matching / streaming-AR generation) are the most plausible "months-out" voice-cloning backends because they were explicitly designed with streaming inference in mind
- **Plausibility**: high | **Info Value**: high | **Type**: web
- **Rationale**: F5-TTS uses flow matching with relatively low NFE and has community quantization activity; CosyVoice 2 explicitly advertises streaming/chunked generation and per-speaker conditioning. Both have Apache/MIT-leaning licenses (needs verification). These are the strongest candidates for "voice-cloning PLC that prefers the actual speaker's voice."
- **If true**: We have a concrete months-out target with a defensible license story and a real streaming path on consumer GPU; CPU may require aggressive int8/int4.
- **If false** (licenses are non-commercial, or streaming claims don't hold up at PLC latencies): We fall back to StyleTTS2 (which is non-AR and fast but has trickier voice-cloning) or accept that voice-cloning PLC is research-only.
- **Effort**: medium

### H3: Most "famous" generative audio models (Bark, Tortoise, MusicGen, Stable Audio, AudioGen, Jukebox, VALL-E reproductions) are architecturally incompatible with PLC latency budgets regardless of quantization
- **Plausibility**: high | **Info Value**: high | **Type**: analysis
- **Rationale**: Autoregressive token LMs over neural codec tokens (Bark, MusicGen, VALL-E family) generate at codec frame rate (~50Hz or slower) with large transformer forward passes per token; even heavily quantized they don't hit <40ms first-audio on CPU. Diffusion models (Stable Audio, Riffusion) need many denoising steps. Eliminating these up front saves the investigator agents enormous time.
- **If true**: The shortlist collapses to (a) dedicated neural PLC, (b) flow-matching/non-AR TTS, (c) small GAN vocoders — and music-aware concealment is deferred indefinitely.
- **If false** (some AR model has a genuinely streaming low-latency mode, e.g., MusicGen-stream or Bark-streaming forks): One of the music/speech families enters the months-out tier and changes the architecture conversation.
- **Effort**: light

### H4: The AMD AIE / Ryzen AI path is research-only for audio in 2026 — no production-grade audio model ships with AIE kernels, and the realistic NPU story is "ONNX Runtime VitisAI EP for CNN-shaped PLC models, nothing for transformer/diffusion audio gen"
- **Plausibility**: medium-high | **Info Value**: high | **Type**: web
- **Rationale**: AMD's Ryzen AI SW stack and Vitis AI have been LLM-focused (Llama, Phi) with limited audio coverage. AIE tile dataflow favors CNN/conv-heavy models with predictable shapes — which is exactly what LACE/NoLACE are, and exactly what large AR transformers and diffusion U-Nets are not. This is a high-info-value hypothesis because it constrains the entire AIE tier independent of model choice.
- **If true**: AIE tier = LACE/NoLACE-class CNN PLC only; transformer-based TTS on AIE is months-to-years out and not a v1 concern.
- **If false** (Ryzen AI has matured to handle transformer audio gen, or there's a Vitis AI audio model zoo we're unaware of): The AIE tier opens up significantly and may even leapfrog CPU.
- **Effort**: medium-heavy (information is scattered across AMD docs, GitHub, and forum posts)

### H5: The Rust inference story is the actual binding constraint, not the model — most candidates lack a usable Rust path and would force either a Python sidecar, ONNX Runtime FFI, or a candle/burn port
- **Plausibility**: high | **Info Value**: high | **Type**: hybrid
- **Rationale**: mjolnir-mesh is Rust. The `Recover` trait must be called from real-time audio threads. ONNX Runtime has Rust bindings (`ort`) and broad model coverage; candle has growing audio support; burn is less mature. Many SoTA TTS models use custom PyTorch ops, vocoders, or tokenizer pipelines that don't ONNX-export cleanly. This hypothesis reframes the entire ranking: model quality matters less than whether you can call it from Rust without a Python subprocess.
- **If true**: Ranking criterion shifts to "ONNX-exportable + permissive license + small enough" — which strongly favors LACE/NoLACE, small GAN vocoders, and StyleTTS2 over CosyVoice/F5-TTS/Bark.
- **If false** (e.g., a Python sidecar over UDS is acceptable, or candle/burn coverage is better than expected): The full SoTA TTS catalog becomes accessible and quality wins out.
- **Effort**: medium

### H6: Streaming-capable neural vocoders (HiFi-GAN, BigVGAN, Vocos, WaveRNN-derived) are an under-discussed sweet spot — small, fast, CPU-friendly, and could be combined with a separate predictor for PLC
- **Plausibility**: medium | **Info Value**: medium-high | **Type**: web
- **Rationale**: Vocoders are typically <50M params, run real-time on CPU, have ONNX exports, and permissive licenses. They aren't standalone PLC, but they're a building block — pair a small autoregressive feature predictor with a streaming vocoder and you have a viable PLC architecture distinct from "drop in someone else's TTS." This may also be the actual architecture under the hood of LACE/NoLACE.

### H7: Per-speaker conditioning ("make PLC sound like the actual speaker") is a strictly harder problem than gap-filling and may require speaker-embedding extraction (ECAPA-TDNN, WavLM, etc.) as a separate pipeline stage

### H8: License analysis will eliminate at least half the headline models because of non-commercial or unclear training-data licensing, and this is a more decisive filter than performance

### H9: Music-aware concealment is genuinely viable today via tiny waveform-domain models (not via MusicGen/Stable Audio) — something like a small Demucs-derived or NSNet-style network trained for music PLC

### H10: Sesame-CSM and other 2025-era "conversational speech models" may already be optimized for low-latency streaming and could be the dark-horse winner over F5-TTS / CosyVoice

## Selected Hypotheses (top 5)

Hypotheses selected for investigation, in priority order:

1. **H1: Dedicated neural PLC (LACE/NoLACE + PLC Challenge winners) is the "today" tier** → investigation_type: web
2. **H4: AMD AIE/Ryzen AI is research-only for audio in 2026 except for CNN-shaped PLC** → investigation_type: web
3. **H5: Rust inference story (ONNX Runtime / candle / burn / FFI) is the binding constraint** → investigation_type: hybrid
4. **H2: F5-TTS and CosyVoice 2 are the strongest months-out voice-cloning candidates** → investigation_type: web
5. **H3: Most headline generative audio models are architecturally incompatible with PLC latency budgets** → investigation_type: analysis

## Cuts

H6 (streaming vocoders) and H7 (per-speaker conditioning architecture) are likely subsumed inside H1's investigation of NoLACE internals and H2's investigation of CosyVoice's reference-audio path — investigators should surface these findings within those branches rather than spending a separate slot. H8 (license filter) is similarly cross-cutting: every selected hypothesis should produce a license verdict per model, so a standalone branch is redundant. H9 (small music-PLC models) is low-plausibility and the caller's framing already implies music-aware concealment is exploratory; if the H1/H3 investigations surface anything, it will be captured there. H10 (Sesame-CSM) is interesting but narrow and folds naturally into H2's "current SoTA streaming TTS" sweep.
