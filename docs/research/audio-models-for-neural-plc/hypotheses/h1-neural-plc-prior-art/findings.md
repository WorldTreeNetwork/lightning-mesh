# Hypothesis: H1 — Dedicated neural PLC (LACE/NoLACE + PLC Challenge winners) is the only "today" answer for the `Recover` trait; everything else is months-out or wrong-tool

## Summary

The hypothesis is **partially confirmed but significantly mis-framed**. LACE and NoLACE are **not PLC models at all** — they are codec enhancement postfilters that operate on successfully-received frames and cannot substitute for `decode_lost`. The correct neural PLC work in the Xiph/Opus ecosystem is **Opus's deep-PLC (FARGAN + predictive acoustic model)**, which is deployed and BSD-licensed in libopus 1.5+. That system is the genuine "today" answer for the `Recover::decode_lost` path, but it is inseparable from libopus and requires a system C library link rather than a standalone ONNX model. Among the open-field neural PLC models, **FRN (Full-band Recurrent Network)** provides an immediately deployable ONNX model with public weights — but is CC-BY-NC licensed, blocking commercial shipping. tPLCnet offers MIT-licensed weights in TFLite format. The PLC Challenge winner systems (BS-PLCNet / 1024K teams) have no public weights. The recommended "start here" choice for a Rust audio engineer today is to FFI into libopus 1.5+ with `--enable-deep-plc` for v1, reserving tPLCnet or a retrained FRN as the upgrade path if a standalone neural backend is needed.

## Evidence

### 1. LACE and NoLACE: Enhancement, NOT PLC

LACE (Linear Adaptive Coding Enhancer) and NoLACE (its nonlinear successor) are **Opus Speech Coding Enhancement (OSCE)** models. They operate as adaptive postfilters on the decoded signal to improve perceptual quality of *correctly received* frames at low bitrates (6–12 kb/s). The architecture is: Conv → CPool → Conv → TConv → GRU feature encoder feeding adaptive comb filters and temporal shaping modules (NoLACE adds three AdaShape iterations). They require pitch-lag side information from the Opus codec and process the decoded-but-intact signal.

Per the paper (arxiv.org/abs/2309.14521), they have "no mention of packet loss concealment capability" — they require the received signal as input. When a packet is lost there is no input signal, making them **inapplicable** to `Recover::decode_lost`.

Specs:
- LACE: 100–280 MFLOPS, ~900 K parameters, ~0.15% CPU, adds ~0.5 MB to binary
- NoLACE: 400–620 MFLOPS, ~1.8 M parameters, ~0.75% CPU, adds ~1.1 MB to binary
- Sample rate: 16 kHz (wideband Opus linear-predictive mode)
- Build flag: `--enable-osce`
- License: BSD 3-clause (part of libopus)
- Code: `github.com/xiph/opus` (main branch, `dnn/` directory)

**Verdict on LACE/NoLACE for `Recover`**: Wrong tool. They improve fidelity on received frames; they cannot synthesize missing ones.

### 2. Opus Deep PLC (FARGAN): The Real Xiph Neural PLC

Opus 1.5+ ships a dedicated deep PLC system, enabled via `--enable-deep-plc`. This is the actual neural concealment backend:

- **Predictive model**: Estimates Bark frequency cepstral coefficients (18 BFCCs), pitch period, and voice/unvoiced indicator from the last good frame's codec state.
- **Generative model (FARGAN)**: Framewise autoregressive GAN vocoder, generates speech in 2.5 ms sub-frames using pitch-based autoregressive feedback. Complexity: **600 MFLOPS worst-case**; runtime cost **~1% of one laptop CPU core**.
- **Memory footprint**: ~1 MB binary increase, ~60 KB state memory, ~1.4 MB constants.
- **DRED complement**: For bursts >~80 ms, DRED (Deep Redundancy) encodes up to 1 second of redundancy at <32 kb/s overhead in a backward-decoding RDO-VAE with ~1 M weights each for encoder/decoder (100 MFLOPS encoder, 50 MFLOPS decoder average). DRED is sender-side; it muxes redundant audio into existing Opus packets without breaking the base spec.
- **Quality**: The Opus 1.5 deep PLC system "placed 2nd in Microsoft's Audio Deep PLC Challenge (Interspeech 2022)". An open GitHub issue (github.com/xiph/opus/issues/306, Dec 2023, unresolved) reports subjective quality regression vs the older LPCNet-based PLC in some conditions.
- **Opus 1.6** (released December 15, 2025) extends FARGAN with a bandwidth extension (BWE) neural model that generates 8–20 kHz from wideband speech, bringing DRED+FARGAN to fullband quality without fullband encoding.
- **License**: BSD 3-clause (entire libopus codebase).
- **Standalone?** No. FARGAN + the predictive acoustic model are tightly coupled to the Opus decoder's internal codec state (pitch, LPC coefficients). They cannot be extracted as a standalone ONNX model without substantial re-engineering.

**Rust FFI path**: Existing Rust crates (`opus` on crates.io via SpaceManiac/opus-rs, `opusic-sys`) link against system libopus via pkg-config. `unsafe-libopus` is a c2rust transpilation of libopus **1.3.1** — it predates deep PLC. To get deep PLC via Rust, the project must either link to a system libopus ≥1.5 built with `--enable-deep-plc`, or bundle and build libopus from source in `build.rs`.

The existing `OpusPlc` backend in `/home/dorje/work/IdentiKey/mjolnir-mesh/crates/mjolnir-audio/src/conceal.rs` already uses `OpusDecoder::decode_lost()` at line 75 — this is **already Opus's classical PLC** (heuristic, not neural). Upgrading to neural deep PLC requires building against libopus 1.5+ with `--enable-deep-plc` and setting decoder complexity ≥5 at runtime, with **no Rust code changes** to the existing `OpusPlc` struct.

### 3. Interspeech 2022 PLC Challenge / ICASSP 2024 PLC Challenge

**Interspeech 2022**: The winning submission was "End-to-End Multi-Loss Training for Low Delay Packet Loss Concealment" (Nan Li et al.), a multi-loss architecture with signal, perceptual, and ASR loss components. PLCNet (Liu et al., GAN-based, semi-supervised) ranked 3rd. Challenge infrastructure is at `github.com/microsoft/PLC-Challenge` (MIT license) — but this is the **evaluation harness and dataset tooling**, not a deployable model. No winning-team weights are publicly available.

**ICASSP 2024**: Co-winners 1024K and NWPU & ByteAudio (P.804 overall quality 3.44–3.49). **No architectural details or public weights** are disclosed in the challenge paper (arxiv.org/html/2402.16927v1). BS-PLCNet (GCRN for 0–8 kHz + GRU for 8–24 kHz, multi-task with F0 and linguistic objectives) achieved co-first place but has no published code repository with weights as of investigation date. The project page (`zzhdzdz.github.io/BS-PLCNet`) is a demo page only.

**Verdict**: Challenge winner architectures are not deployable today. The challenge infrastructure and evaluation tools are MIT-licensed but contain no trained weights.

### 4. FRN (Full-band Recurrent Network)

- **Architecture**: Full-band recurrent network, 48 kHz, blindly conceals without loss mask knowledge, uses autoregressive feedback on previous output frames.
- **Code**: `github.com/Crystalsound/FRN`
- **Weights**: ONNX model provided in repo at `lightning_logs/best_model.onnx`. Can be run with `inference_onnx.py`.
- **License**: **CC-BY-NC 4.0** — non-commercial only. **Cannot be shipped in a commercial product.**
- **Deployability**: ONNX weights available today → `ort` crate in Rust can load and run them. But the NC license is a hard blocker for shipping.

### 5. tPLCnet

- **Architecture**: Sequence-to-one (seq2one), time-domain, short temporal context buffer, GRU-based (inferred from paper class), predicts one lost frame from context. Only runs on-demand during loss events, not continuously.
- **Code + weights**: `github.com/breizhn/tPLCnet` — models in TFLite (`.tflite`) format, not ONNX.
- **License**: MIT — shippable commercially.
- **Deployability**: Requires either TFLite runtime (C library FFI) or conversion to ONNX via `tf2onnx`. Conversion is standard but adds a one-time engineering step. tflite2onnx tooling exists and is routine.
- **Quality**: PLC-MOS improvement of 1.07 over zero-fill baseline (Interspeech 2022, competitive 3rd place).
- **Training data**: 64 hours open-source speech + Microsoft challenge loss traces.
- **Parameter count and MFLOPS**: Not disclosed in the abstract; paper PDF needed.

### 6. What Does the `Recover` Trait Need for Neural PLC State Management?

From `/home/dorje/work/IdentiKey/mjolnir-mesh/crates/mjolnir-audio/src/conceal.rs` lines 63–79: the trait implementation holds a stateful `OpusDecoder` across both `decode` and `decode_lost` calls. Neural PLC models (all of the above) are **recurrent and stateful** — they maintain GRU/LSTM hidden state across frames. The trait design is already correct: `&mut self` on both `decode` and `decode_lost` means per-peer state is held in the implementing struct, which is exactly what's needed. A `NeuralPlc` struct would carry the model's recurrent hidden state (a tensor) and update it on every `decode` call, then use it to seed `decode_lost` synthesis. The `lookahead: Option<&[u8]>` hint in `decode_lost` is currently unused by neural backends (they can't decode FEC without codec integration), so it should be ignored and synthesis driven purely from internal recurrent state.

The existing `PlcFactory` / `PlcBackend` / `default_plc_factory()` indirection in `conceal.rs` means swapping in a neural backend requires only writing a new struct that implements `Recover<Output = Vec<i16>>` and changing the factory — no changes to `Mixer`, `SelfHealingBuffer`, or `room.rs`.

### 7. Gap >80 ms: Does Anything Beat Opus Built-in PLC?

For **occasional isolated loss (≤80 ms / ≤4 frames at 20 ms)**: Opus deep PLC (FARGAN) is competitive — it placed 2nd in the Interspeech 2022 challenge. A standalone neural model like tPLCnet or FRN would provide marginal improvement at the cost of an extra inference stack.

For **burst loss >80 ms**: Opus's FARGAN degrades because it cannot predict over long horizons from stale codec state. DRED (sender-side redundancy VAE) is the correct solution here — it covers bursts up to 1 second — but it requires changes on the **sender** side too and the IETF draft (draft-ietf-mlcodec-opus-dred) is not yet standardized. No standalone neural PLC model has demonstrated reliable quality at >500 ms gaps; this is an inherently hard problem.

## Confidence

**Level**: medium-high

Multiple independent sources (arxiv papers, official Opus releases, GitHub repos, challenge proceedings) agree on the key facts. The LACE/NoLACE misclassification finding is high-confidence (the paper is explicit). The deep-PLC assessment is high-confidence. Uncertainty remains around: exact parameter counts for tPLCnet (paper PDF inaccessible), the precise quality gap between deep-PLC and standalone neural models at specific loss rates (the CouthIT evaluation shows only plots, not tables), and whether any 2025-era standalone model exists with a permissive license and ONNX weights that clearly beats Opus deep PLC.

## Sources

- [1] **url**: https://arxiv.org/abs/2309.14521 — NoLACE paper: architecture is OSCE postfilter, not PLC; "no mention of packet loss concealment capability"; 1.8 M params, 620 MFLOPS
- [2] **url**: https://arxiv.org/html/2309.14521v2 — Full HTML text; extracted hyperparameter table: LACE 900 K / 280 MFLOPS, NoLACE 1.8 M / 620 MFLOPS; confirms PLC absence
- [3] **url**: https://opus-codec.org/demo/opus-1.5/ — Opus 1.5 release notes: deep PLC (~1% CPU, ~1 MB binary), FARGAN 600 MFLOPS, BSD 3-clause license
- [4] **url**: https://opus-codec.org/demo/opus-1.6/ — Opus 1.6 (Dec 2025): BWE extends FARGAN for fullband deep PLC; current stable release
- [5] **url**: https://arxiv.org/html/2212.04453v3 — DRED paper: 1 M encoder + 1 M decoder weights, 100 MFLOPS encode + 50 avg decode, covers bursts to 1 s, sender-side multiplexed
- [6] **url**: https://www.couthit.com/opus-deep-plc/ — Evaluation of Opus 1.5.2 deep PLC: FARGAN architecture, PESQ/ViSQOL/PLCMOS evaluation (plots only), 24 kbps wideband test conditions
- [7] **url**: https://arxiv.org/html/2402.16927v1 — ICASSP 2024 PLC Grand Challenge: co-winners 1024K and NWPU & ByteAudio (P.804 = 3.44–3.49), no architectures or weights disclosed
- [8] **url**: https://github.com/microsoft/PLC-Challenge — MIT license; evaluation harness + dataset tooling only; no trained model weights
- [9] **url**: https://github.com/Crystalsound/FRN — FRN: ONNX weights at `lightning_logs/best_model.onnx`, 48 kHz, **CC-BY-NC 4.0** (non-commercial only)
- [10] **url**: https://github.com/breizhn/tPLCnet — tPLCnet: MIT license, TFLite model weights in `models/`, time-domain seq2one, PLC-MOS +1.07 vs zero-fill
- [11] **url**: https://arxiv.org/abs/2204.01300 — tPLCnet paper: seq2one, 64 h training data, 3rd place Interspeech 2022 challenge
- [12] **url**: https://github.com/xiph/opus/issues/306 — Open issue Dec 2023 (unresolved): subjective quality regression in deep-plc vs older LPCNet PLC
- [13] **file**: `/home/dorje/work/IdentiKey/mjolnir-mesh/crates/mjolnir-audio/src/conceal.rs:1–109` — Existing `OpusPlc` and `SilencePlc` implementations; `PlcFactory` abstraction; comment "Future backends (neural PLC on CPU, AIE-resident cascade)" confirms the upgrade slot
- [14] **file**: `/home/dorje/work/IdentiKey/mjolnir-mesh/crates/mjolnir-media/src/recover.rs:1–63` — `Recover` trait definition; `decode_lost(lookahead: Option<&[u8]>)` signature; `&mut self` on both methods (correct for stateful recurrent models)
- [15] **url**: https://lib.rs/crates/unsafe-libopus — unsafe-libopus is libopus 1.3.1 (predates deep PLC); cannot be used for neural features
- [16] **url**: https://arxiv.org/abs/2401.03687 — BS-PLCNet: GCRN (0–8 kHz) + GRU (8–24 kHz), co-first ICASSP 2024; no public weights
- [17] **url**: https://www.isca-archive.org/interspeech_2022/diener22_interspeech.html — Interspeech 2022 PLC Challenge overview; 1st place: Li et al. multi-loss, 3rd: PLCNet (Liu et al., PLCMOS 3.829)
- [18] **url**: https://zzhdzdz.github.io/BS-PLCNet/ — BS-PLCNet project page: demo only, no code repository linked, no weights available

## Open Questions

1. **Exact quality delta at >80 ms gaps**: The CouthIT evaluation shows only plots; no published table comparing Opus deep PLC vs tPLCnet or FRN at 4, 8, 16, and 32 consecutive frame losses. A 30-minute benchmark using the ICASSP 2024 test set (with PLCMOS or P.804) would resolve this and determine whether a standalone neural model is worth the engineering overhead for v1.

2. **tPLCnet parameter count and MFLOPS**: The arXiv abstract does not disclose this; the full PDF is needed. Specifically: is the GRU hidden state large enough to dominate over ONNX overhead when running at 50 frames/second per peer?

3. **ONNX conversion fidelity for tPLCnet**: The weights are TFLite; conversion via `tf2onnx` is standard but has known edge cases with GRU reset gates. A one-hour conversion + smoke test would confirm whether the `ort` crate path is viable.

4. **`--enable-deep-plc` via Rust build.rs**: No existing published crate wraps libopus 1.5+ with deep-plc enabled. The project's existing `OpusDecoder` in `mjolnir-audio` likely links via `audiopus` or `opus-rs` against whatever system libopus is installed. Whether that system library was compiled with `--enable-deep-plc` is an invisible runtime condition. A Cargo feature + `build.rs` that bundles and compiles libopus 1.5 from source with the flag would make this deterministic.

5. **FRN commercial licensing**: The CC-BY-NC blocker on FRN is firm unless the authors grant a commercial exception (common in academic code if emailed). If the model architecture is reimplemented and retrained on permissively licensed speech data, the license issue vanishes. The training dataset (DNS, LibriSpeech, VCTK) is permissive.

6. **Unresolved quality regression (issue #306)**: Opus's deep PLC has a reported (but unquantified) subjective quality regression vs LPCNet PLC in some conditions. This needs a concrete A/B test with the project's actual packet loss patterns before committing to Opus deep PLC as the v1 baseline.

## Recommendation

**Start with**: Upgrade the system libopus build to 1.5+ with `--enable-deep-plc`, wire `decoder complexity ≥5` in `OpusDecoder::new()`, and verify that `OpusDecoder::decode_lost()` calls the neural FARGAN path. This requires **zero changes to `conceal.rs`, `recover.rs`, or `mixer.rs`** — only the C library build changes. This gives neural PLC today at BSD 3-clause license, ~1% CPU cost, and a clear upgrade to DRED for burst loss later.

**If a standalone neural backend is required** (e.g., for an NPU or a non-Opus audio path): prototype with tPLCnet (MIT, TFLite weights available, convert to ONNX with tf2onnx). Implement as a new `NeuralPlc` struct holding the ONNX session and a GRU hidden-state tensor as `&mut self` fields; `decode()` updates state normally, `decode_lost()` runs the generative inference step seeded from current hidden state and ignores the `lookahead` hint.

**Do not use** LACE/NoLACE for concealment — they are the wrong tool. **Do not use** FRN without a license exception. **Do not wait** for BS-PLCNet weights — they are not available.

## Sub-Hypotheses

- **[deep-plc-build-rs]**: Whether the existing `mjolnir-audio` Opus link deterministically enables deep-PLC or silently falls back to heuristic PLC — this cannot be resolved from the current codebase read alone; it requires inspecting the Cargo.toml dependencies and checking the system libopus version + compile flags at runtime.
- **[dred-sender-path]**: DRED requires sender-side bitstream changes. Whether mjolnir-node's Opus encoder currently emits DRED payloads, and what jitter-buffer integration changes `SelfHealingBuffer` would need, is a codebase question worth a dedicated investigation if burst >80 ms loss is a first-class requirement.
