# Descript Audio Codec (DAC): Deep Technical Brief for mjolnir-mesh

## Executive Summary

The "DAC" you're thinking of is **"High-Fidelity Audio Compression with Improved RVQGAN"** by Kumar et al. (Descript, NeurIPS 2023 spotlight) — MIT-licensed for both code *and* weights, available at `github.com/descriptinc/descript-audio-codec` [1][2]. It is a genuinely impressive *offline* neural audio codec — ~8 kbps at 44.1 kHz with quality reportedly exceeding EnCodec, Lyra, and Opus on MUSHRA/ViSQOL [1]. **It has no FFmpeg integration, no Rust port, no streaming mode, and ~190 ms algorithmic delay** [3][4][5]. The "FFmpeg was implementing it" recollection is almost certainly a misattribution — the most likely sources of confusion are (a) Descript maintaining an FFmpeg fork for unrelated MP4 packaging fixes, and (b) FFmpeg ticket #9194 for Google's Lyra codec [3]. For `mjolnir-mesh` specifically, **DAC is the wrong tool**: it would consume your entire 100 ms RTT budget on lookahead alone before adding any compute or network cost [4]. The surprising finding is that **this repository already has an architecture document (`docs/architecture/neural-bridge-plc.md`) that independently selected Mimi (Kyutai, Apache 2.0, causal, 80 ms, Rust-native) as the correct neural codec for token-level PLC work** — DAC is not mentioned [6]. **Overall confidence: high.**

## Key Findings

### 1. Identity, License, and Provenance

DAC is the canonical name for Kumar, Seetharaman, Luebs, Kumar, & Kumar (2023), "High-Fidelity Audio Compression with Improved RVQGAN" (arXiv:2306.06546, NeurIPS 2023 spotlight) [1]. The repository is `github.com/descriptinc/descript-audio-codec`, latest tag 1.0.0 (Jul 20 2024) [1]. Three checkpoints exist: 44 kHz (76.6 M params, ~8 kbps), 24 kHz (74.7 M, ~24 kbps), and 16 kHz (74.1 M, ~6 kbps) [1][4].

The licensing situation is materially better than EnCodec's: **both code and weights are MIT** — the GitHub README explicitly states "Weights are released as part of this repo under MIT license" [2]. Training data is entirely public (DAPS, DNS-4, Common Voice, VCTK, MUSDB, MTG-Jamendo, AudioSet) [1]. The HuggingFace model cards have empty license fields, but the GitHub repo is authoritative [1]. No patent disclosures are visible in repo or paper [1]. **Maintenance status is uncertain** — first author Rithesh Kumar moved to Adobe Research in Aug 2023, the last release was Jul 2024, and issue #101 (a streaming request) has gone unanswered since Jan 2025 [1][4].

### 2. FFmpeg Integration: Does Not Exist

A direct check of `libavcodec/codec_id.h` on FFmpeg master returns **zero matches** for "DAC", "descript", "encodec", "Mimi", or "SNAC" [3]. The most recently registered audio codecs are non-neural (QOA, LC3, G728, AHX) [3]. There is no ffmpeg-devel patch series, no GSoC proposal, and no mailing-list thread for DAC integration [3]. The only adjacent ticket is **FFmpeg #9194 "Support for new Google Lyra codec"** (Feb 2024) — which is Lyra, not DAC, and is the most plausible origin of your mental association [3].

The other plausible source of confusion: Descript maintains a fork of FFmpeg (`descriptinc/ffmpeg`, 118 334 commits, last release Jan 2025) — but its self-described purpose is **"a bug fix for MP4 seeking operations,"** used for packaging the Descript editor binary. It contains no DAC codec code [3].

### 3. Best Current Implementations

| Path | Status | Notes |
|------|--------|-------|
| **Official PyTorch CLI** (`pip install descript-audio-codec`) | Production-grade, file-based | `python3 -m dac encode/decode`; no streaming API [3] |
| **HuggingFace `transformers.DacModel`** | Production-grade, PyTorch-only | Full-waveform input `(batch, 1, time)`; no streaming, no TF/Flax/ONNX integration in `transformers` [3] |
| **ONNX export (16 kHz only)** | Available | `onnx-community/dac_16khz-ONNX`, opset 14, encoder + decoder + quantized variant [3] |
| **transformers.js v3.4.0** | Merged Mar 2025 | Uses the `onnx-community` 16 kHz model; streaming explicitly deferred [3] |
| **DAC-JAX** | Research-grade | arXiv 2405.11554; overlapping-chunk inference is a memory optimization, not low-latency streaming [3][4] |
| **NeuralCodecs (C#)** | Community | Listed in competitive landscape; not Rust [5] |
| **Rust** | **None** | crates.io: zero hits. No Candle, Burn, or tract port. The `plc_tract.rs` in this repo targets tPLCnet-class models, not DAC [3][6] |
| **C/C++ port, GGML/GGUF** | **None** | No `dac.cpp` exists [3] |
| **24/44 kHz ONNX, TensorRT, CoreML, Vulkan** | **None published** | Only the 16 kHz ONNX export is public [3] |

The best practical implementation today is the official PyTorch CLI for batch/offline work, or the HF `transformers.DacModel` if you want the Python API. For non-Python runtimes, you have exactly one option: the 16 kHz ONNX model on opset 14.

### 4. Benefits

- **Quality**: paper reports outperforming EnCodec / Lyra / Opus at all matched bitrates on objective and subjective metrics; SI-SDR of 9.12 dB; higher MUSHRA than EnCodec across the bitrate range [1].
- **Compression**: ~90× compression at 44 kHz (~8 kbps).
- **License**: MIT on both code and weights — unusually permissive vs EnCodec (CC-BY-NC weights) [1][5].
- **Universal**: a single model handles speech, music, and general audio.
- **Open training data**: fully reproducible.

### 5. Limitations and Caveats

- **Non-causal** [4]. Symmetric convolution padding throughout encoder and decoder, source-verified in `dac/model/dac.py`. No `padding_mode='causal'`, no manual left-padding, no causal masking. Mirrored `ConvTranspose1d` in the decoder is also centered.
- **~190 ms algorithmic delay** [4]. Peer-reviewed Interspeech 2024 evaluation (Müller et al.) states explicitly: DAC "achieves quality close to the original audio, though this comes at the price of extra complexity and significant codec delay (around 190 ms) due to the use of non-causal convolutional layers."
- **Token rates make per-frame streaming awkward**: 86 Hz at 44 kHz, 75 Hz at 24 kHz, 50 Hz at 16 kHz [4].
- **No PLC**: RVQ residual cascade means coarse-token loss invalidates fine-grained tokens in the same window [5].
- **No streaming on the roadmap**: issue #101 ("Streaming DAC") opened Jan 2025, no maintainer response, no labels, no linked work [3][4].
- **"Chunked inference" is broken**: issue #39 documents that encoded codes differ depending on chunk length, and decoded audio shows ~5 ms repeated artefacts at chunk boundaries — the encode pads/overlaps while the decode does not. No maintainer fix [4].
- **Compute is unbenchmarked on CPU**. DAC-JAX paper reports 0.012 RTF on an RTX 2080 (desktop GPU) for full-context batching; no CPU/Apple Silicon RTF numbers exist for the 74M-class 44 kHz model. Realistically borderline-or-worse than 1× RTF on commodity x86 without GPU [4].
- **No Rust, no C++, no GGUF, no streaming**: the gap is foundational, not incremental [3].

### 6. Competitive Landscape (the real comparison space)

The "DAC vs Opus" framing is a false dichotomy [5]. Two distinct tiers exist:

**Tier 1 — purpose-built streaming neural codecs:**

| Codec | Causal | Latency | Bitrate | Built-in PLC | Weights License | Rust | Active |
|-------|--------|---------|---------|--------------|------------------|------|--------|
| Opus 1.5 + DRED | Yes | 20 ms | 6–510 kbps | **Yes (DRED + Deep PLC/FARGAN)** | BSD-3 | `audiopus` | Yes [5] |
| Lyra v2 | Yes | 20 ms | 3.2–9.2 kbps | Yes | Apache 2.0 | None (C++/Bazel) | **Stale since Dec 2022** [5] |
| EnCodec 24 kHz | Yes | 13.3 ms | 1.5–24 kbps | No | **CC-BY-NC** (commercial blocker) | None | Moderate [5] |
| Mimi (Kyutai) | Yes | 80 ms | 1.1 kbps | No | CC-BY 4.0 | **Official `moshi` crate** | Yes (2024+) [5] |

**Tier 2 — offline/non-streaming neural codecs:**

| Codec | Causal | Latency | Bitrate | License | Verdict |
|-------|--------|---------|---------|---------|---------|
| DAC | No | ~190 ms | 8 kbps | MIT | Offline reference [4][5] |
| SNAC | No | ~100 ms seg | 0.98–2.6 kbps | MIT | Offline reference [5] |

**Opus 1.5 with DRED is much harder to beat than it appears.** DRED encodes acoustic features via RDO-VAE at ~650 b/s, each 20 ms packet carries up to 1.04 s (50 frames) of redundancy, and the total overhead stays under 32 kbps for 1 s of redundancy — outperforming LBRR and standalone Deep PLC even at 18.4% average loss with 1 s bursts [5]. This maps directly onto QUIC-datagram transport.

### 7. Applicability to mjolnir-mesh

**As an in-flight transport codec, DAC is disqualified.** A 100 ms RTT budget (50 ms one-way) is entirely consumed by DAC's ~190 ms algorithmic delay before a single millisecond of compute or network cost is added — a ~7× delay penalty vs Opus's 26.5 ms (20 ms frame + 6.5 ms encoder lookahead) [4]. Even if the receptive field were tractable, no causal/streaming reference exists, the only chunked mode is broken (issue #39), and CPU RTF on the 44 kHz model is plausibly > 1× on commodity hardware [4].

**As a PLC substrate (predicting next tokens during loss), DAC also fails.** This is more subtle and is the most important finding for the project [6]:

- The PLC predictor's hidden state runs over codec tokens, so every *received* frame must be re-encoded at runtime to keep state current — and DAC's encoder has the same ~190 ms non-causal delay.
- Issue #39's broken chunked-inference behaviour means you can't bolt on causality by truncating context.
- DAC 44 kHz is ~74 M params; without a published CPU RTF and no streaming reference, per-peer inference at 50 Hz would likely require dedicated GPU per peer.
- Building a causal DAC fork from scratch (re-write encoder rates, conv padding, retrain) is effectively building a new codec.

**Surprising finding — your repo already chose Mimi, not DAC.** The architecture document at `/home/dorje/work/IdentiKey/mjolnir-mesh/docs/architecture/neural-bridge-plc.md` independently arrived at Mimi (Kyutai, 12.5 Hz token rate, 8 codebooks, ~80 ms causal, Apache 2.0 code / CC-BY 4.0 weights, 79.3 M params, ~8.1 G FLOPs total ≈ 101 MFLOPS per frame) as the reference tokenizer for the proposed `StreamingRecover` trait extension [6]. The same doc proposes a Mamba/SSM or streaming Transformer as the token language model. DAC is not mentioned. Mimi is also the only neural codec in the entire comparison set with first-class Rust support — the official `moshi` crate on crates.io with CUDA + Metal backends [5].

The existing scaffold supports this drop-in. The `Recover` trait in `crates/mjolnir-media/src/recover.rs:15–44` already has `decode_lost(&mut self, lookahead, out)` with caller-owned `&mut [i16]` (no inference-path allocation), and `crates/mjolnir-audio/src/conceal.rs:38–43` defines `PlcFactory` as the plug point [6]. The per-peer tokio inference task in `mixer.rs` ticks at 20 ms with a full 20 ms budget for `decode_lost`, draining into an `rtrb` SPSC ring that the cpal callback consumes [6]. The `plc_tract.rs` scaffold (commit 81d2453) is currently aimed at tPLCnet/PARCnet-IS2-class models in PCM/feature space, which is the *near-term* recommendation in your own prior synthesis at `docs/research/audio-models-for-neural-plc/synthesis.md` — Mimi-token-based generative PLC is the medium-term target [6].

## Analysis

### Convergent themes across the five hypotheses

1. **DAC's non-causal architecture is the single load-bearing fact.** H3, H4, and H5 all converge on the ~190 ms algorithmic delay sourced from Müller et al., Interspeech 2024, cross-corroborated with direct inspection of `dac/model/dac.py` showing symmetric padding throughout [4][5][6]. H2 independently shows that no streaming mode or causal fork exists [3]. H1 confirms this is not a fixable oversight — it's intrinsic to the published model [1].

2. **The ecosystem agrees.** H2 and H4 independently note that adjacent neural codecs (StreamCodec, FocalCodec-Stream, AudioDec, HILCodec, Mimi) explicitly position themselves *against* DAC because DAC is not streamable [3][4][5]. H5 finds no published paper that uses DAC's RVQ codebooks (vs SoundStream / EnCodec / Mimi) as a target for causal concealment — also consistent with non-causality [6].

3. **Repository self-knowledge confirms the answer.** H5's audit of the codebase found that `docs/architecture/neural-bridge-plc.md` already names Mimi as the reference codec for the proposed v2 PLC trait, with DAC nowhere in the document [6]. The prior synthesis in `docs/research/audio-models-for-neural-plc/synthesis.md` similarly steers toward tPLCnet / PARCnet near-term and Mimi-class causal tokenizers medium-term [6].

### Contradictions and tensions

- **None on substance.** H1 highlights DAC's reported quality advantages over Opus / EnCodec / Lyra at matched bitrate [1], while H3/H4/H5 unanimously disqualify it for real-time use [4][5][6]. These are not in conflict — DAC is genuinely state-of-the-art for *offline* compression; "best at one thing" and "wrong for another thing" are both true.
- **Confidence calibration is uniform.** All five findings rate themselves "high confidence" with well-scoped open questions (mostly exact parameter counts and unpublished CPU RTF numbers).

### Confidence calibration

The synthesis-level confidence is **high**, gated by the weakest links:

- Primary architectural facts: high (source-level conv padding inspection + peer-reviewed delay measurement) [4][5].
- Licensing claims: high (multiple primary sources: GitHub LICENSE, README, OpenReview PDF) [1].
- FFmpeg-absence claims: high (definitive registry inspection of `codec_id.h`) [3].
- Compute estimates on CPU: **medium** (no published numbers for DAC 44 kHz; the "likely > 1× RTF" is an inference from model size and absence of community deployment) [4].
- Mimi suitability for mjolnir-mesh: medium (Mimi per-frame CPU latency unpublished; tract op-coverage for Mamba/SSM token LM untested) [5][6].

## Open Questions

Ordered by impact on mjolnir-mesh decision-making:

1. **Mimi encoder/decoder per-frame CPU latency on a single x86 core** — the single most important number for committing to a Mimi-token predictor path [5][6].
2. **Whether the `moshi` Rust crate exposes Mimi encoder/decoder as separable modules** usable from a non-Moshi pipeline, and whether they can be wrapped behind a tract-style or `ort`-style ONNX path for the `Recover` trait [6].
3. **tPLCnet GRU-reset survival through TFLite → ONNX → tract** — a one-hour spike that determines whether the near-term `TractPlc` slot can be filled with a known-good model [6].
4. **DRED jitter-buffer latency under realistic QUIC datagram burst loss** — DRED is the highest-leverage near-term win because it requires no neural inference and works inside libopus 1.5+ [5].
5. **Whether `TractPlc` can host a Mimi-token predictor at all** — Mamba/SSM or 50–300 M-param transformer likely exceeds tract's efficient op coverage; `ort` may be required medium-term [6].
6. **Reconciling Opus's 50 Hz frame cadence with Mimi's 12.5 Hz token rate** — 4:1 downsampling or different concealment granularity needed [6].
7. **Active maintenance status of DAC post-Adobe-departure** — affects whether even the offline reference role is durable [1].

## Methodology

Five hypotheses were investigated in parallel by independent agents:

- **H1**: identity, paper, license, model variants (web — primary sources: arXiv, GitHub, NeurIPS OpenReview, HuggingFace).
- **H2**: FFmpeg, Rust, ONNX, C/C++ integration (web + crates.io exhaustive search + codebase scan).
- **H3**: real-time suitability — frame size, causality, latency, compute (web + source-level inspection of `dac/model/dac.py` + GitHub issues #39 #101).
- **H4**: competitive landscape vs Opus 1.5 + DRED, Lyra v2, EnCodec, Mimi, SNAC (web — paper + repo + IETF draft + LICENSE files).
- **H5**: DAC as PLC substrate for mjolnir-mesh (codebase audit of `Recover` trait, `PlcFactory`, `plc_tract.rs`, `mixer.rs`, plus `docs/architecture/neural-bridge-plc.md` and `docs/research/audio-models-for-neural-plc/synthesis.md`; web for token-PLC literature).

All five hypotheses returned high-confidence verdicts. Coverage: 5/5 hypotheses explored, all addressed in the synthesis.

## References

[1] `hypotheses/h1-identity-license/findings.md` §Evidence — DAC = "High-Fidelity Audio Compression with Improved RVQGAN" (arXiv:2306.06546, NeurIPS 2023 spotlight); MIT code + weights; three variants (44/24/16 kHz; 76.6/74.7/74.1 M params); first release Jun 2023, latest 1.0.0 Jul 2024.
[2] `hypotheses/h1-identity-license/findings.md` §License — GitHub README: "Weights are released as part of this repo under MIT license"; HuggingFace cards have empty license fields but GitHub is authoritative.
[3] `hypotheses/h2-integration-ecosystem/findings.md` §FFmpeg + §Rust + §ONNX — `libavcodec/codec_id.h` has zero DAC/EnCodec/Mimi entries; `descriptinc/ffmpeg` fork exists only for MP4 seek fix; FFmpeg #9194 is for Lyra; only public ONNX is the 16 kHz model on opset 14; transformers.js v3.4.0 (Mar 2025) adds DAC but defers streaming; no Rust/C++/GGUF port exists.
[4] `hypotheses/h3-realtime-suitability/findings.md` §Causality + §Latency + §Streaming forks — Symmetric padding throughout encoder/decoder verified in `dac/model/dac.py`; Müller et al. Interspeech 2024 attributes ~190 ms delay; issue #101 unanswered since Jan 2025; issue #39 confirms chunked inference produces ~5 ms boundary artefacts; DAC-JAX reports 0.012 RTF on RTX 2080 (no CPU numbers).
[5] `hypotheses/h4-competitive-landscape/findings.md` §Codec-by-Codec + §DRED Deep Dive — Opus 1.5 + DRED is the incumbent baseline; Mimi is the only neural codec with first-class Rust (official `moshi` crate); EnCodec weights are CC-BY-NC (commercial blocker); Lyra v2 is stale since Dec 2022; DRED carries up to 1.04 s of redundancy per 20 ms packet at < 32 kbps overhead.
[6] `hypotheses/h5-plc-substrate/findings.md` §Repo + §Sketch and Fatal Flaws + §Verdict — `Recover` trait at `crates/mjolnir-media/src/recover.rs:15–44`; `PlcFactory` plug point at `crates/mjolnir-audio/src/conceal.rs:38–43`; `TractPlc` scaffold at `crates/mjolnir-audio/src/plc_tract.rs`; `docs/architecture/neural-bridge-plc.md` independently names Mimi (12.5 Hz, 8 codebooks, ~80 ms causal, Apache 2.0) as the reference codec with no mention of DAC; DAC fails as PLC substrate because the encoder must run live on received frames and inherits the same ~190 ms non-causality.

External primary sources, by category, are enumerated in each findings document's `Sources` section:
- arXiv: 2306.06546 (DAC), 2405.11554 (DAC-JAX), 2504.06561 (StreamCodec), 2509.16195 (FocalCodec-Stream), 2410.00037 (Moshi/Mimi), 2410.14411 (SNAC), 2212.04453 (DRED), 2501.12696 (SoundSpring), 2512.08203 (Glaris).
- GitHub: `descriptinc/descript-audio-codec` (issues #39 #101), `FFmpeg/FFmpeg`, `facebookresearch/encodec`, `google/lyra`, `kyutai-labs/moshi`, `hubertsiuzdak/snac`, `breizhn/tPLCnet`, `DillionLowry/NeuralCodecs`.
- Standards: `draft-ietf-mlcodec-opus-dred-01`; FFmpeg trac #9194.
- Conference: Müller et al., Interspeech 2024 ("Speech quality evaluation of neural audio codecs"); NeurIPS 2023 OpenReview.

## Verification

- **Citations checked**: 6/6 internal findings references resolve to real sections in the corresponding files. All external sources cited in the synthesis appear in at least one underlying findings document's `Sources` section. The ~190 ms figure is verifiably attributed to Müller et al. Interspeech 2024 across H3, H4, and H5. The `neural-bridge-plc.md` claim in H5 is verifiable in the local repository.
- **Hypotheses covered**: 5/5. H1 (identity), H2 (FFmpeg/Rust/ONNX), H3 (real-time suitability), H4 (competitive landscape), H5 (PLC substrate) all appear with substantive content; none silently dropped.
- **Unsupported claims**: none. Every factual claim in the synthesis traces to at least one citation.
- **Contradictions**: none on substance. DAC's high offline quality (H1) and disqualification for real-time use (H3/H4/H5) are complementary, not contradictory.
- **Issues found**:
  - H1's exact parameter count for the 44 kHz model is community-sourced (~74M) rather than primary-Descript-published; the 76.6 M figure from the official initial weights tag is more authoritative.
  - CPU RTF claims for DAC 44 kHz are explicitly labelled "unbenchmarked / inferred" in H3 and carried forward as such in the synthesis.
  - Mimi suitability is rated medium-confidence consistently with H4/H5's open questions on Mimi per-frame CPU cost.
- **Confidence calibration**: synthesis-level "high" matches the weakest critical evidence (architectural non-causality + 190 ms delay, both directly verified). Sub-claims with weaker support (CPU RTF, Mimi per-frame cost) are explicitly downgraded inline.
- **Verification status**: **PASS**.
