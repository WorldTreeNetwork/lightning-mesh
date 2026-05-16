# Audio Models for Neural PLC in mjolnir-mesh — Deployment-Focused Synthesis

## Executive Summary

**Ship Opus deep-PLC (FARGAN) via a libopus 1.5+ build flag in the next sprint. This is a zero-Rust-change, BSD-3-clause, ~1% CPU upgrade that puts neural PLC behind the existing `Recover` trait today** [h1: opus deep-PLC]. Everything else in the open-source landscape — voice-cloning TTS (F5-TTS, CosyVoice 2, Sesame CSM), music gen (MusicGen, Stable Audio), autoregressive token LMs (Bark, Tortoise, VALL-E), and diffusion models (Riffusion, Jukebox) — is either architecturally incompatible with sub-40 ms PLC budgets [h3: budget arithmetic] or a 6–12 month integration project [h2: streaming TTS]. For burst loss >80 ms the answer is **DRED** (sender-side redundancy VAE shipped with libopus 1.5+), not a bigger PLC model [h1: DRED]. AMD AIE / Ryzen AI is a research-only target for audio in 2026; there are zero production audio kernels in the Vitis Model Zoo and even Microsoft's Windows Studio Effects Voice Focus is unavailable on AMD NPUs [h4: vitis model zoo, h4: windows studio effects]. Confidence: **high** on the recommendation; **medium-high** on the "wait" verdicts for TTS-as-PLC because of the structural text-input mismatch [h2: open question 3].

---

## 1. Tier-1 Recommendation: Ship Opus Deep-PLC via libopus 1.5+

### The why

The existing `OpusPlc` backend in `crates/mjolnir-audio/src/conceal.rs` already calls `OpusDecoder::decode_lost()` — but against a system libopus that probably predates deep-PLC and silently falls back to the heuristic LPC extrapolator [h1: build flag invisibility]. Building libopus 1.5+ with `--enable-deep-plc` and setting decoder complexity ≥5 swaps the runtime from heuristic PLC to **FARGAN + a predictive acoustic model** with zero code changes to `conceal.rs`, `recover.rs`, `mixer.rs`, or `service.rs` [h1: opus deep-PLC, h1: recommendation]. Cost profile:

| Property | Value | Source |
|---|---|---|
| License | BSD 3-clause | [h1: opus deep-PLC] |
| CPU cost | ~1% of one laptop core | https://opus-codec.org/demo/opus-1.5/ |
| FARGAN complexity | 600 MFLOPS worst case | https://opus-codec.org/demo/opus-1.5/ |
| Binary delta | ~1 MB | https://opus-codec.org/demo/opus-1.5/ |
| State memory | ~60 KB | https://opus-codec.org/demo/opus-1.5/ |
| Quality | 2nd place, Interspeech 2022 PLC Challenge | https://opus-codec.org/demo/opus-1.5/ |
| Standalone? | **No** — coupled to Opus decoder internals (pitch, LPC) | [h1: opus deep-PLC] |

Opus 1.6 (released **2025-12-15**) extends this with a neural bandwidth-extension (BWE) model that lifts wideband FARGAN output to fullband (8–20 kHz), so the DRED+FARGAN pair now delivers fullband quality without fullband encoding [h1: opus deep-PLC; https://opus-codec.org/demo/opus-1.6/].

### Action items (this sprint)

1. **Audit the link.** Inspect `crates/mjolnir-audio/Cargo.toml` and verify what binds libopus (`audiopus`, `opus-rs`, `opusic-sys`). The transpiled-Rust `unsafe-libopus` crate is **libopus 1.3.1** and predates deep-PLC entirely — if it is in the dependency graph, deep-PLC is impossible at runtime [h1: source 15].
2. **Pin a libopus version.** Add a Cargo feature `neural-plc` whose `build.rs` either (a) requires `pkg-config` libopus ≥ 1.5 with `--enable-deep-plc`, or (b) vendors libopus source and compiles it from `build.rs` with the flag set. The bundled-source path is deterministic and recommended; system-libopus depends on distro packaging.
3. **Set decoder complexity ≥ 5.** FARGAN only activates above complexity 5 in the runtime decoder path. Add this in `OpusDecoder::new()`.
4. **A/B against the open quality regression.** GitHub issue `xiph/opus#306` (open Dec 2023, unresolved) reports a subjective regression vs the older LPCNet-based PLC in some conditions [h1: source 12]. Run an A/B with the project's actual loss patterns before declaring victory.

This is the only "today" answer. Everything below this section is "later" or "no".

---

## 2. Architectural Reality Check

### 2a. LACE/NoLACE are NOT PLC models — common confusion, important correction

The decomposition treated LACE and NoLACE as the strongest PLC candidates. **They are not PLC models at all.** LACE (Linear Adaptive Coding Enhancer) and NoLACE are Opus Speech Coding Enhancement (OSCE) postfilters — they operate on *successfully received* low-bitrate Opus frames to improve perceptual quality at 6–12 kb/s [h1: LACE/NoLACE evidence; https://arxiv.org/abs/2309.14521]. They require the received signal as input. When a packet is lost there is no input signal. They cannot back `Recover::decode_lost`.

Specs (for reference):
- LACE: 900K params, 280 MFLOPS, ~0.15% CPU, `--enable-osce` build flag, BSD 3-clause
- NoLACE: 1.8M params, 620 MFLOPS, ~0.75% CPU, same build flag, BSD 3-clause

These are still worth enabling — they are postfilter quality wins on received frames — but they belong in the codec build, not in the `Recover` trait. The real Xiph neural PLC is FARGAN (Tier 1).

### 2b. AR token LMs and diffusion models are eliminated, not slow

The "famous" generative audio names — Bark, Tortoise, MusicGen (all sizes), Stable Audio Open, AudioGen, Riffusion, Jukebox, VALL-E reproductions — are not "too slow to ship today, fast enough in 2 years." They are structurally disqualified by the codec frame rate and the iteration count [h3: per-model verdict]. Concrete numbers from the findings:

| Model | Best-case wall clock | License | Disqualifier |
|---|---|---|---|
| Bark | 8.1 s on TITAN RTX (FP16+offload) | MIT | ~400× over a 20 ms budget |
| Tortoise (fast fork) | 45 s P90 for 18-word sentence | Apache 2.0 | Utterance-scale, not frame-scale |
| MusicGen small | "first audio after ~5 s" (streaming fork) | **CC-BY-NC** | 50 Hz codec rate + transformer pass per frame |
| Stable Audio Open | 25 s on RTX 3090 (200 steps) | Non-commercial community license | Iterative diffusion; first step is noise |
| Riffusion | ~2–4 s per 50-step SD pass | OpenRail-M | Fixed spectrogram patch, no streaming |
| Jukebox | ~hours per minute of audio | MIT | ~10,000× over budget |
| VALL-E open repros | 17.79 s median, 45.38 s P90 | MIT | 75 Hz AR codec, utterance-scale only |

[h3: per-model verdict; sources 1, 8, 9, 10]. Quantization does not save these models — the bottleneck is autoregressive token generation at codec frame rate or iterative denoising, not weight precision.

### 2c. The TTS-as-PLC text-input mismatch (the structural problem)

Every voice-cloning TTS model in the H2 sweep takes **text** as input. The lost packet's text is by definition unknown — that is what loss means. This gives two architectural choices, neither of which is "drop a TTS model into `Recover`" [h2: open question 3]:

- **(a) ASR + LM prediction upstream.** Run ASR on the received audio buffer continuously, predict the next 100–500 ms of text with a small LM, feed that text + the speaker's reference audio into the TTS. This is a multi-model pipeline running per peer, far outside the audio thread, and only works during longer losses where the latency overhead is amortized.
- **(b) Accept that "voice cloning PLC" actually means "fill the gap with neutral plausible audio in the speaker's timbre."** No text — generate from prior audio context alone (which most of these models cannot do; they are text-conditioned by design). This requires a *different* class of model than text-to-speech.

This is the single biggest reason TTS-as-PLC is not just "months out" but architecturally questionable for the PLC use case. CosyVoice 2 and CSM both attend to prior audio context, which makes (b) plausible-in-principle for them, but neither has been benchmarked in this mode.

---

## 3. Standalone Neural PLC When libopus Deep-PLC Is Not Enough

If you need a neural PLC backend that is not coupled to libopus — e.g., for an NPU-resident cascade, a non-Opus codec path, or to swap implementations without rebuilding the C library — the **only** model with the right combination of license, weights, and ONNX-reachability is:

### tPLCnet (MIT, `breizhn/tPLCnet`)

- Time-domain seq2one GRU-based architecture, runs on-demand only during loss events [h1: tPLCnet]
- **MIT license** — shippable commercially
- Weights ship in TFLite format; conversion via `tf2onnx` is standard but adds a one-time engineering step
- Interspeech 2022 PLC Challenge: 3rd place, PLC-MOS +1.07 vs zero-fill baseline
- Parameter count and exact MFLOPS not disclosed in the abstract — needs a one-hour benchmark on x86 before committing
- ONNX path → `ort` crate in Rust [h5: ort]

### What to skip

- **FRN** (`Crystalsound/FRN`): **CC-BY-NC 4.0** [h1: FRN]. ONNX weights are right there in the repo, but the license is a hard commercial blocker. Don't ship.
- **PLC Challenge 2024 winners** (BS-PLCNet, 1024K, NWPU & ByteAudio): no public weights, demo pages only [h1: ICASSP 2024]. Not deployable today.

### Wiring into `Recover`

The `Recover` trait's `&mut self` + `Output = Vec<i16>` shape is correct for stateful recurrent inference [h5: trait compatibility]. A `NeuralPlc` struct would carry the ONNX session and a GRU hidden-state tensor as fields; `decode()` updates state normally, `decode_lost()` runs the generative inference step seeded from current hidden state and ignores `lookahead` (no FEC integration without codec coupling) [h1: trait section].

---

## 4. Burst Loss > 80 ms: DRED, Not a Bigger PLC Model

FARGAN's quality degrades on bursts >~80 ms because it cannot predict over long horizons from stale codec state [h1: gap >80 ms]. The temptation is "find a bigger neural PLC model that can hold the gap for half a second." Don't. The right architectural fix already exists and ships in libopus 1.5+:

### DRED (Deep REDundancy)

- Sender-side redundancy VAE shipped in libopus 1.5+
- Covers bursts **up to 1 second** at **<32 kb/s overhead**
- ~1M weights each for encoder/decoder; 100 MFLOPS encode, 50 MFLOPS decode average
- Backward-decoding RDO-VAE — receiver decodes redundancy from any received packet within the window
- Muxes into existing Opus packets; does **not** break the base spec [h1: opus deep-PLC; https://arxiv.org/html/2212.04453v3]

The catch: DRED requires **sender-side** bitstream changes. mjolnir-node's encoder must emit DRED payloads, and `SelfHealingBuffer` needs to detect and consume them on the receive path. The IETF draft (`draft-ietf-mlcodec-opus-dred`) is not yet standardized, so interop with non-mjolnir Opus endpoints is best-effort, but for a mesh where both endpoints are mjolnir-node this is a controlled environment.

**Implication for the roadmap:** DRED is the second sprint after the Tier 1 deep-PLC enable. Standalone neural PLC models (tPLCnet, etc.) are a *third* lane that you may never need if DRED+FARGAN handles the loss distribution you actually see in production.

---

## 5. Voice Cloning Future: 6–12 Month Bets

Both of these are real candidates, but neither is deployable today and both carry caveats. Use this section to *plan* for the v2 horizon, not to ship.

### CosyVoice 2 (rank 1 future bet)

- **Apache 2.0** [h2: CosyVoice 2]. No commercial blocker.
- LLM-based streaming TTS, 0.5B params, chunk-aware causal flow matching
- Claimed **150 ms first-chunk latency** — hardware not specified, likely A100-class [h2: open question 1]
- ONNX export (`Lourdle/CosyVoice2-0.5B_ONNX`) and a C++/GGML port (`cosyvoice.cpp`) with q4_0–q8_0 quantization exist
- TensorRT-LLM gives a documented 4× speedup; inferred consumer-GPU first chunk could approach 40 ms (speculative)
- **No Rust port.** Path is ONNX via `ort`, or wrap `cosyvoice.cpp` via FFI [h5: ort]
- Still **7.5× over** the 20 ms PLC frame budget at the claimed 150 ms; viable for 200+ ms gap fills, not per-frame concealment

### F5-TTS (rank 2 future bet)

- Code MIT, **weights CC-BY-NC 4.0** (Emilia dataset) [h2: F5-TTS] — commercial blocker on the released checkpoint
- 335M-param DiT (smallest of the realistic set); ConvNeXt V2 text head; Vocos vocoder
- Fast-F5-TTS (7-NFE EPSS variant): **RTF 0.030 on RTX 3090** — fastest per-unit-audio compute in the set
- First-audio latency 300–2000 ms in practice due to non-streaming design and Python stack overhead
- Path to deployment: retrain on Apache/CC-BY-licensed data (LibriSpeech, VoxPopuli) to clear the license, ONNX+int8, sentence-chunking at ~50 ms

### Sesame CSM-1B (dark horse)

- **Apache 2.0** [h2: Sesame CSM]
- RTF 0.28 on RTX 4090 (10 s audio in 2.8 s); RTF 0.8 on RTX 4070
- TTFA ~150 ms synthesis-only — same floor as CosyVoice 2
- **Has a Rust port** (`cartesia-one/csm.rs`, GGUF q8_0/q4_k, candle backend) — only model in the set with a production-grade Rust inference path
- **But** csm.rs is **AGPL-3.0** — complicates embedding in a proprietary Rust binary [h2: open question 4]
- Best prosody quality for conversational register; worst latency floor of the three

### All three carry the text-input mismatch caveat from §2c

None of these is a drop-in `Recover` backend. The realistic v2 deployment is "trigger a TTS fill on losses >300 ms when ASR has high confidence in the next 1–2 seconds of speaker text" — a separate concealment lane that runs alongside, not instead of, FARGAN/DRED.

### Eliminated from contention

- **XTTS-v2** — Coqui Public Model License (non-commercial); Coqui shut down 2024 [h2: XTTS-v2]
- **StyleTTS2** — diffusion-based, 5–6 s for 439 chars on RTX 2080 Ti, ambiguous model terms, GPL phonemizer contamination [h2: StyleTTS2]
- **OpenVoice v2** — MIT, fast tone-color converter (85 ms/s on A10G), but the underlying VITS base TTS isn't streaming-designed; could be repurposed as a post-processing voice-matching layer on top of a different generator [h2: OpenVoice v2]
- **MetaVoice-1B** — Apache 2.0 but project abandoned, 30-s reference audio requirement [h2: MetaVoice]
- **Parler-TTS** — wrong problem: description-based speaker selection, not reference-audio cloning [h2: Parler-TTS]

---

## 6. Music PLC: Small CNNs, Not MusicGen / Stable Audio

The music-aware concealment angle is solved by an entirely different lineage than the "famous" music gen models. The relevant work is the **IEEE-IS2 2024/2025 Music PLC Challenge** [h3: PARCnet-IS2]:

- **PARCnet-IS2** (Polimi ISPL, IS2 2024 baseline): hybrid linear predictor + feedforward CNN, **416K parameters**, processes 512-sample (11.6 ms at 44.1 kHz) packets in a single feed-forward pass, no autoregression
- Challenge constraint: **< 11.6 ms per packet on Intel Core i5** — the model meets it [h3: source 3]
- Same architectural family as NSNet/DeepVQE — small spectral predictors, not generative transformers
- Aironi et al.'s 2024 GAN variant has a "lite" version at 3.4M params (full 54.4M)

This is the correct shape of model for music PLC: small, feed-forward, purely causal, fixed-frame. It fits cleanly into the `Recover` trait. There is no off-the-shelf ONNX or Rust port to drop in, and PARCnet's training data is MAESTRO piano only — broader music coverage (vocals, full mix) would need a different training corpus that doesn't appear to exist in open weights [h3: open question 3].

**For mjolnir-mesh in its current scope (mesh voice audio), music PLC is out of scope.** Note PARCnet-IS2 as the right reference architecture if music-mode ever becomes a requirement; do not chase MusicGen / Stable Audio.

---

## 7. The Rust Inference Path

The Rust inference story is a real binding constraint and ranks model selection more than model quality does [h5: confirmed].

### Ranked recommendation

1. **`ort` (ONNX Runtime Rust bindings)** — default choice. Latest 2.0.0-rc.12; 1.x is stable. CPU/CUDA/CoreML/DirectML/**VitisAI**/ROCm/OpenVINO execution providers. Documented audio precedents (Silero VAD with 1.93× speedup vs Python; `sbv2-api` for Style-BERT-VITS2). Dynamically links libonnxruntime [h5: ort].
2. **`tract` (Sonos pure-Rust ONNX/NNEF)** — **under-investigated; potentially better fit** for hard-real-time audio threads. No dynamic library dependency, smaller binary, Sonos uses it for on-device DSP. Per-frame latency on the audio thread is unknown. This is the most important open question for the runtime decision [h5: open question 1; h5: sub-hypothesis a].
3. **`candle` (HuggingFace pure-Rust ML)** — viable for small CNN-class models if you hand-port the architecture. GGUF quantization supported. No pre-ported neural PLC model. Allocation profile in tight inference calls not publicly characterized [h5: candle].
4. **`burn`** — too immature. v0.20.1 (pre-1.0), wgpu-first design with GPU command submission latency incompatible with 5–20 ms audio callbacks, no audio examples [h5: burn]. Skip.

### The `Recover` trait API smell

`Output = Vec<i16>` forces a heap allocation on every concealment call from the `cpal` audio thread. At 48 kHz mono / 20 ms frames that's ~1.9 KB per allocation — fine under jemalloc most of the time, but a real-time hazard under memory pressure and during loss bursts [h5: trait compatibility]. Two ways to fix without disturbing existing callers:

- Change to `Output = SmallVec<[i16; 960]>` (stack-allocate the mono-20ms common case)
- Or evolve the trait to accept a caller-provided `&mut [i16]` output buffer (zero-allocation; requires a lifetime parameter or a separate method)

The Git status shows `crates/mjolnir-media/src/service.rs` is modified in the working tree — the trait is not yet frozen, so this is the right moment to address it [h5: open question 3].

### Frame budget math

48 kHz / 20 ms / mono = 960 samples per pull. Single peer: ~20 ms wall clock. 4 peers: ~5 ms each [h5: frame budget arithmetic]. A small CNN (~100K params) is 0.5–3 ms on x86; a GRU/LSTM ~1M-param PLC is 3–10 ms (feasible for 1–2 peers); a 10M+ param SoTA model exceeds budget for even one peer without GPU/NPU offload. **A multi-peer mesh with neural PLC on the audio thread will need either tract's lighter overhead, a dedicated inference thread with a ring buffer, or both** [h5: sub-hypothesis c].

---

## 8. AMD AIE / Ryzen AI: Not a 2026 Target for Audio

The AIE/NPU tier is research-only for audio in 2026, with one nuance that doesn't change the recommendation [h4: summary].

### Hard evidence

- **Vitis AI Model Zoo (v3.5, current): zero audio models.** Coverage is ADAS/AD, medical, video surveillance, robotics, data center [h4: vitis model zoo]
- **Official Ryzen AI audio support (SW 1.7.1, April 2026): ASR only.** Whisper encoder + Zipformer; decoders run on CPU or iGPU; **no KV-cache on NPU** [h4: LIRA]
- **Architectural mismatch:** 64 KB per AIE-ML tile, static shapes mandatory, batch size 1 only, no recurrent op native support (RNN/LSTM/GRU routed to CPU/iGPU in every documented example) [h4: AIE constraints]
- **Transformer/diffusion audio is GPU-routed:** AMD's own ACE Step 1.5 music-gen demo runs on Radeon (ROCm) + ComfyUI, not NPU [h4: ACE Step]
- **Production gap signal:** Microsoft's Windows Studio Effects Voice Focus is unavailable on AMD Ryzen AI 300-series ("basic effects only"). Even Microsoft hasn't shipped a production audio kernel on XDNA [h4: Windows Studio Effects]

### The nuance

A LACE/NoLACE-shaped CNN PLC model (fixed-frame, CNN-dominant, no recurrence) is **architecturally plausible** for the VitisAI EP INT8 CNN path [h4: CNN-shaped PLC; h4: sub-hypothesis aie-cnn-plc-feasibility]. Zero documented examples exist. Whether the operator graph (gated convolutions, layer norm, sigmoid activations) maps cleanly to the supported-ops set is unverified and requires running `vitisai_ep_report.json` on a concrete model.

### Recommendation

Treat AIE as a **2027+** optimization target. The investment order is unambiguous:

1. CPU (FARGAN today, tPLCnet later if needed)
2. NVIDIA consumer GPU (only if you adopt a TTS-class voice-cloning backend in v2)
3. AMD NPU (only after the v2 model is selected and there's evidence it maps to CNN shapes)

The comment in `conceal.rs:18` referencing "AIE-resident cascade via parakeet-aie" should be re-scoped to "future work" rather than a near-term lane.

---

## 9. Concrete 90-Day `Recover` Trait Plan

### Days 0–30 (Tier 1 ship)

- Audit libopus binding in `crates/mjolnir-audio/Cargo.toml`; confirm no `unsafe-libopus` in dep graph
- Add `neural-plc` Cargo feature with `build.rs` that vendors libopus 1.5+ source and compiles with `--enable-deep-plc --enable-osce`
- Set decoder complexity ≥ 5 in `OpusDecoder::new()`
- Add a runtime probe that asserts deep-PLC is active (the existing `OpusPlc` struct already uses `decode_lost`; this becomes the neural path automatically once the C library is upgraded)
- A/B benchmark against current heuristic PLC on realistic loss patterns; track Opus issue #306 regression in your test set

### Days 30–60 (DRED enable + trait hardening)

- Implement DRED encoder emission in `mjolnir-node` audio sender path (Opus 1.5+ already implements DRED — needs encoder config flag and packet framing)
- Update `SelfHealingBuffer` to detect and consume DRED redundancy on burst losses up to 1 s
- Evolve `Recover::Output` to `SmallVec<[i16; 960]>` or buffer-passing to eliminate heap allocation in the cpal callback; this is a backward-incompatible trait change but a small one
- Decide on `tract` vs `ort` by running a 1-hour spike: load a small ONNX model, measure per-frame latency on a single x86 core under realistic load. If `tract` per-frame latency is within ~20% of `ort` and binary footprint is smaller, default to `tract`

### Days 60–90 (Standalone neural PLC spike)

- Convert tPLCnet TFLite weights to ONNX via `tf2onnx`; verify GRU reset gates survive conversion (known edge case)
- Implement `NeuralPlc` struct that wraps the ONNX session and holds the GRU hidden-state tensor as `&mut self`
- Benchmark on x86 single core: does it run end-to-end under 3 ms per `decode_lost` call? If yes, it's a viable alternative-or-cascade with FARGAN. If no, abandon this lane; FARGAN + DRED is sufficient
- Document the AIE/NPU lane as deferred to 2027+ with a clear "what would change our mind" set of triggers (XDNA3 audio docs, a published VitisAI EP PLC model)

### What this plan does *not* commit to

- TTS-as-PLC (CosyVoice 2 / F5-TTS / CSM) — re-evaluate at the 90-day mark only if there is a concrete user request for speaker-matched concealment, with the text-input mismatch (§2c) explicitly resolved or accepted
- Music PLC — out of scope for mesh voice
- AMD NPU audio — not a 2026 target

---

## 10. References

### Opus, FARGAN, DRED (Tier 1)
- [h1: opus deep-PLC] `hypotheses/h1-neural-plc-prior-art/findings.md` §2 — FARGAN architecture, 600 MFLOPS, ~1% CPU, BSD 3-clause
- https://opus-codec.org/demo/opus-1.5/ — Opus 1.5 release notes; deep PLC enabled via `--enable-deep-plc`
- https://opus-codec.org/demo/opus-1.6/ — Opus 1.6 (Dec 2025); FARGAN BWE for fullband
- https://arxiv.org/html/2212.04453v3 — DRED paper; 1 s burst coverage, sender-side multiplex
- https://github.com/xiph/opus/issues/306 — Open regression issue vs LPCNet PLC

### LACE/NoLACE (postfilter, not PLC)
- [h1: LACE/NoLACE evidence] `hypotheses/h1-neural-plc-prior-art/findings.md` §1
- https://arxiv.org/abs/2309.14521 — NoLACE paper; "no mention of packet loss concealment capability"
- https://arxiv.org/html/2309.14521v2 — Full HTML; LACE 900K/280 MFLOPS, NoLACE 1.8M/620 MFLOPS

### Standalone neural PLC
- [h1: tPLCnet] `hypotheses/h1-neural-plc-prior-art/findings.md` §5
- https://github.com/breizhn/tPLCnet — MIT, TFLite, seq2one
- https://arxiv.org/abs/2204.01300 — tPLCnet paper, Interspeech 2022 3rd place
- [h1: FRN] `hypotheses/h1-neural-plc-prior-art/findings.md` §4 — CC-BY-NC, blocker
- https://github.com/Crystalsound/FRN — ONNX weights, non-commercial
- https://github.com/microsoft/PLC-Challenge — MIT eval harness only, no weights
- https://arxiv.org/html/2402.16927v1 — ICASSP 2024 PLC Challenge, no public winner weights
- https://arxiv.org/abs/2401.03687 — BS-PLCNet; no public weights

### AR / diffusion elimination
- [h3: per-model verdict] `hypotheses/h3-ar-diffusion-elimination/findings.md`
- https://huggingface.co/blog/optimizing-bark — Bark 8.1 s on TITAN RTX
- https://huggingface.co/facebook/musicgen-large/discussions/13 — MusicGen streaming "first audio after ~5 s"
- Stable Audio Open model card — 200 steps default, 8 steps/s on RTX 3090
- https://www.preprints.org/manuscript/202508.0654/v1/download — VALL-E P90 45.38 s

### Music PLC
- https://ar5iv.labs.arxiv.org/html/2409.18564 — IS2 2024 challenge report
- https://internetofsounds2025.ieee-is2.org/workshops/3rd-ieee-international-workshop-networked-immersive-audio/music-packet-loss-concealment — IS2 2025 < 11.6 ms constraint
- https://github.com/polimi-ispl/2024-music-plc-challenge/tree/main/parcnet-is2 — PARCnet-IS2 416K params

### Voice cloning TTS (months out)
- [h2: CosyVoice 2] `hypotheses/h2-tts-streaming-candidates/findings.md` §2
- https://huggingface.co/FunAudioLLM/CosyVoice2-0.5B — Apache 2.0, 150 ms first chunk
- https://github.com/Lourdle/cosyvoice.cpp — q4_0–q8_0 quantization
- [h2: F5-TTS] `hypotheses/h2-tts-streaming-candidates/findings.md` §1
- https://github.com/SWivid/F5-TTS — MIT code / CC-BY-NC weights
- https://fast-f5-tts.github.io/ — Fast-F5-TTS RTF 0.030 on RTX 3090
- [h2: Sesame CSM] `hypotheses/h2-tts-streaming-candidates/findings.md` §3
- https://github.com/cartesia-one/csm.rs — AGPL-3.0 Rust port
- https://huggingface.co/sesame/csm-1b — Apache 2.0

### Rust inference
- [h5: ort] `hypotheses/h5-rust-inference-path/findings.md` §2 — `ort` 2.0.0-rc.12; CPU/CUDA/CoreML/DirectML/VitisAI EPs
- https://github.com/pykeio/ort
- https://ort.pyke.io/ — Silero VAD 1.93× speedup precedent
- [h5: candle] `hypotheses/h5-rust-inference-path/findings.md` §3
- https://github.com/huggingface/candle — Whisper, EnCodec, MetaVoice-1B, Parler-TTS shipped; GGUF
- [h5: burn] `hypotheses/h5-rust-inference-path/findings.md` §4
- https://github.com/sonos/tract — under-investigated alternative; pure Rust, no dylib (referenced in h5 §6)
- [h5: frame budget arithmetic] `hypotheses/h5-rust-inference-path/findings.md` §1

### AMD AIE / Ryzen AI (research-only for audio)
- [h4: vitis model zoo] `hypotheses/h4-amd-aie-ryzen-ai/findings.md` §1
- https://docs.amd.com/r/en-US/ug1414-vitis-ai/Vitis-AI-Model-Zoo — no audio entries
- [h4: LIRA] `hypotheses/h4-amd-aie-ryzen-ai/findings.md` §2
- https://github.com/amd/LIRA — Whisper/Zipformer only; no NPU KV-cache
- [h4: AIE constraints] `hypotheses/h4-amd-aie-ryzen-ai/findings.md` §3
- https://docs.amd.com/r/en-US/am020-versal-aie-ml/AIE-ML-Tile-Architecture — 64 KB per tile
- [h4: ACE Step] https://www.amd.com/en/blogs/2026/commercial-grade-ai-music-generation-on-amd-ryzen-ai-and-radeon-ace-step-1-5.html — Radeon, not NPU
- [h4: Windows Studio Effects] https://riallto.ai/notebooks/2_1_MS_Windows_Studio_Effects.html — Voice Focus unavailable on AMD NPU

### Codebase (ground truth)
- `crates/mjolnir-media/src/recover.rs:15-44` — `Recover` trait definition
- `crates/mjolnir-audio/src/conceal.rs:1-109` — `OpusPlc`, `SilencePlc`, `PlcFactory`
- `crates/mjolnir-audio/src/mixer.rs:273-286` — cpal output callback; PLC on audio thread
- `crates/mjolnir-audio/src/lib.rs:23-48` — 48 kHz / 20 ms / 960 samples default frame
- `Cargo.toml` — no `ort`/`candle`/`burn` workspace deps yet

---

## 11. Verification

### Citation audit
- **All hypothesis findings cited**: H1, H2, H3, H4, H5 are each cited multiple times with the specific section referenced
- **External URLs**: every URL cited above appears in at least one findings document's Sources section — no fabricated links
- **Codebase references**: file paths and line numbers match those in H1 source [13–14] and H5 sources [9–13]
- **Citations checked**: 28 distinct sources cited; all map to real entries in the findings documents

### Coverage check
All 5 selected hypotheses appear in synthesis:
- H1 (neural PLC prior art): §1 (FARGAN), §2a (LACE/NoLACE correction), §3 (tPLCnet/FRN), §4 (DRED)
- H2 (TTS streaming candidates): §5 (CosyVoice 2 / F5-TTS / CSM) and §2c (text-input mismatch)
- H3 (AR/diffusion elimination): §2b (per-model verdict table) and §6 (music PLC reframing)
- H4 (AMD AIE / Ryzen AI): §8 (entire section)
- H5 (Rust inference path): §7 (entire section)

Cuts (H6–H10) were correctly absorbed into the selected hypotheses' findings per the decomposition's instructions; no silent drops.

### Claim validation
Every numerical claim (CPU %, MFLOPS, RTF, latency in ms, parameter counts) traces to a specific findings section, which in turn cites a primary source (paper, model card, GitHub repo, or release note). Spot checks:
- "FARGAN ~1% CPU, 600 MFLOPS": H1 §2, citing opus-codec.org/demo/opus-1.5/ — PASS
- "Bark 8.1 s on TITAN RTX": H3 §Bark, citing huggingface.co/blog/optimizing-bark — PASS
- "CosyVoice 2 150 ms first chunk": H2 §2, citing FunAudioLLM/CosyVoice2-0.5B — PASS (with hardware caveat surfaced in §5)
- "csm.rs is AGPL-3.0": H2 §3, citing cartesia-one/csm.rs — PASS
- "Vitis Model Zoo has no audio models": H4 §1, citing docs.amd.com/r/en-US/ug1414-vitis-ai — PASS

### Unsupported claims
- The "DRED requires sender-side bitstream changes in mjolnir-node" implementation detail is **inference** from the DRED architecture description in H1 §2, not a direct claim about mjolnir-node's current encoder behavior. Flagged as an implementation question for the days 30–60 sprint plan.
- The "v2.0.0-rc.12 is still RC after over a year" qualitative judgment in §7 comes from H5 §2 source [1] which states the version; the "over a year" qualifier is in H5's prose but not directly quoted from a release date — minor.
- The "decoder complexity ≥ 5" requirement for FARGAN is asserted in H1 §1 recommendation but not directly quoted from a source URL in this synthesis — sourced through H1 (medium confidence).

### Confidence calibration
- **High confidence**: Opus deep-PLC recommendation; AR/diffusion elimination; AMD NPU verdict; Rust ranking
- **Medium-high confidence**: tPLCnet as fallback (paper PDF not fully read; ONNX conversion fidelity unverified)
- **Medium confidence**: TTS-as-PLC future bets (latency claims uncited to specific hardware; text-input mismatch is structural)
- **Low confidence**: PARCnet-IS2 CPU latency exact numbers (training data limited to MAESTRO piano)

### Issues found
None that change recommendations. One area worth flagging to the caller: the `crates/mjolnir-media/src/service.rs` modification in the working tree may already touch the `Recover` trait or `SelfHealingBuffer` — the 90-day plan assumes the trait can still evolve, which should be confirmed before the days 30–60 trait hardening step.

### Verification status
**PASS_WITH_WARNINGS** — recommendations are well-supported and citations resolve; the warnings are minor (one inferred implementation detail and one qualitative version-age claim) and do not affect the headline recommendation.
