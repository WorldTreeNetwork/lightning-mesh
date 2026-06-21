# Hypothesis: DAC's frame size, lookahead, and per-frame compute make it unsuitable for sub-100ms RTT mesh audio

## Summary

**Hypothesis confirmed (high confidence).** Stock DAC is not strictly causal: its convolutions use symmetric (centered) padding throughout the encoder and decoder, giving it an algorithmic delay on the order of ~190 ms (per peer-reviewed Interspeech 2024 evaluation). The official repo has no streaming/causal mode, the only streaming feature request (#101, Jan 2025) has received no maintainer response, and a separate "chunked inference" issue (#39) confirms that naive frame-at-a-time encoding produces *different* codes than encoding the full waveform, with ~5 ms artefacts at chunk boundaries. DAC cannot drop-in replace Opus in a sub-100 ms RTT mesh path. A "streaming-DAC" variant does not exist in any reachable form on GitHub or arXiv as of May 2026.

## Evidence

### 1. Frame structure and bitrate (per variant)

Pulled directly from the official YAML configs in `descriptinc/descript-audio-codec/conf/final/`:

| Variant | sample_rate | encoder_rates | hop_length | frame rate (token Hz) | n_codebooks | codebook_size | effective bitrate |
|---|---|---|---|---|---|---|---|
| 44 kHz | 44 100 | [2, 4, 8, 8] | 512 (11.61 ms) | 86.13 Hz | 9 | 1024 | ~7 752 bps |
| 24 kHz | 24 000 | [2, 4, 5, 8] | 320 (13.33 ms) | 75.00 Hz | 32 | 1024 | ~24 000 bps |
| 16 kHz | 16 000 | [2, 4, 5, 8] | 320 (20.00 ms) | 50.00 Hz | 12 | 1024 | ~6 000 bps |

(`hop_length = prod(encoder_rates)`; `frame_rate = sample_rate / hop_length`.) DAC-JAX paper confirms 86 Hz token rate and 8-dim latent per codebook.

### 2. Causality — confirmed non-causal

Source-level inspection of `dac/model/dac.py` (and the underlying conv blocks) shows:

- All `Conv1d` layers use symmetric padding (`padding=math.ceil(stride/2)`, `padding=3`, `padding=1`). No `padding_mode='causal'`, no manual left-padding, no causal masking.
- Snake1d activations and residual units are symmetric.
- `ConvTranspose1d` in the decoder also uses centered output_padding — not causal upsampling.

This is corroborated externally by Müller et al., Interspeech 2024 "Speech quality evaluation of neural audio codecs", which states explicitly that DAC "achieves quality close to the original audio, though this comes at the price of extra complexity and significant codec delay (around 190 ms) due to the use of non-causal convolutional layers."

### 3. Algorithmic latency

For a strictly streamable codec the minimum encode→decode delay is `frame_size + lookahead`. For DAC:

- Per-frame quantum: 11.6 ms (44 k), 13.3 ms (24 k), 20 ms (16 k) at the *output of the bottleneck*.
- Receptive field of the stacked 4-rate encoder + RVQ + 4-rate decoder is ~512 samples *per side* at the bottleneck, but the cascaded convolutions plus mirrored decoder accumulate a much larger effective look-ahead in the raw-audio domain. The empirically measured/attributed value is **~190 ms** (Müller et al. 2024).
- Compare Opus: 20 ms frame + 6.5 ms encoder lookahead = **26.5 ms** algorithmic delay.

That is roughly a **7× algorithmic-delay penalty** before a single ms of compute or network is added. With a 100 ms RTT budget (50 ms one-way), DAC alone consumes the entire budget on lookahead.

### 4. Compute

Authoritative compute numbers are sparse — Descript never published Tables of CPU RTF. Best available data points:

- **DAC-JAX paper (arXiv 2405.11554)**: on RTX 2080, compressing a 4 640 ms hop takes 55.6 ms (JAX) / 64.3 ms (PyTorch). That is ≈ 0.012 RTF *on a desktop GPU* with full-context batching. JAX is faster than PyTorch for hops < 647 ms; beyond ~4.6 s PyTorch wins. No CPU benchmarks reported.
- **No published CPU/Apple-Silicon RTF numbers exist** for the official 44 kHz DAC (~74M-class params). Generic streaming codecs like StreamCodec (7M params) achieve "20× realtime on CPU"; DAC is roughly an order of magnitude larger, so realistic CPU RTF is best-case borderline and likely > 1 for the 44 kHz variant on commodity x86 without GPU acceleration.
- Parameter count: official Descript card does not state it; community ports (HF `parler-tts/dac_44khZ_8kbps`) advertise `latent_dim=1024`, `decoder_dim=1536`, which yields the commonly cited ~74M-parameter ballpark, but no primary Descript publication states the exact figure.

### 5. Streaming forks / community work

- **Issue #101 "Streaming DAC (feature request/question)"** — opened Jan 2025, *no maintainer response*, no labels, no linked work. Streaming is not on Descript's roadmap as of May 2026.
- **Issue #39 "Chunked inference result depends on chunk length"** — confirms that even non-real-time chunking is broken: encoded codes differ depending on chunk length, decode produces ~5 ms repeated artefacts at every chunk boundary, and the encode pads/overlaps while the decode does not. Exact symptom of non-causal convolutions plus mismatched padding semantics. No maintainer fix.
- No fork named `streaming-dac` / `causal-dac` / similar surfaces on GitHub.
- Adjacent ecosystem fills the gap *with different codecs*: StreamCodec (arXiv 2504.06561, 20 ms latency, 7M params, ~20× CPU realtime), FocalCodec-Stream (arXiv 2509.16195, 80 ms via causal distillation of WavLM), AudioDec, HILCodec, Mimi (Kyutai, 12.5 Hz, causal). All explicitly position themselves *against* DAC because DAC is not streamable.

### 6. Verdict

DAC cannot be used in a sub-100 ms RTT mesh audio path under any reasonable interpretation:

- The 44 kHz model's ~190 ms algorithmic delay alone exceeds the entire 100 ms RTT budget.
- Even if the receptive field were tractable, no causal/streaming reference implementation exists.
- "Chunked inference" mode is broken at frame boundaries (issue #39), so streaming cannot be bolted on by truncating context.
- Compute on CPU for the 44 kHz variant is unbenchmarked but, given the model size and the absence of any community real-time deployment, is best treated as "unknown but likely > 1× RTF on commodity CPU."

The only condition under which DAC *could* plausibly enter a real-time path is: (a) re-train a strictly causal variant from scratch with rewritten encoder rates and conv padding, (b) accept a higher floor on quality due to the causal constraint, (c) target the 16 kHz variant (20 ms hop) on a GPU. None of (a)/(b)/(c) is on Descript's roadmap or in any public fork — effectively building a new codec.

For mjolnir-mesh specifically: keep Opus as the transport codec, and treat DAC at most as an offline reference for *quality ceiling* of neural codecs, not as a candidate for the in-flight path. If neural compression is later required, the right starting points are causal/streamable codecs (StreamCodec, FocalCodec-Stream, Mimi, AudioDec) — not DAC.

## Confidence

**Level**: high

The non-causal architecture is verified directly from the source repo's config and conv padding; the ~190 ms delay is from a peer-reviewed Interspeech 2024 evaluation; the absence of streaming is verified by an unanswered official feature request plus zero matching forks on GitHub. The only "medium-confidence" element is the exact parameter count of the 44 kHz model.

## Sources

- [1] https://arxiv.org/abs/2306.06546 — Kumar et al., "High-Fidelity Audio Compression with Improved RVQGAN" (DAC paper, 2023).
- [2] https://github.com/descriptinc/descript-audio-codec/blob/main/conf/final/44khz.yml
- [3] https://github.com/descriptinc/descript-audio-codec/blob/main/conf/final/24khz.yml
- [4] https://github.com/descriptinc/descript-audio-codec/blob/main/conf/final/16khz.yml
- [5] https://github.com/descriptinc/descript-audio-codec/blob/main/dac/model/dac.py
- [6] https://github.com/descriptinc/descript-audio-codec/issues/101
- [7] https://github.com/descriptinc/descript-audio-codec/issues/39
- [8] https://www.isca-archive.org/interspeech_2024/muller24c_interspeech.pdf — "DAC ... significant codec delay (around 190 ms) due to the use of non-causal convolutional layers."
- [9] https://arxiv.org/html/2405.11554v1 — DAC-JAX paper.
- [10] https://huggingface.co/parler-tts/dac_44khZ_8kbps/raw/main/config.json
- [11] https://huggingface.co/docs/transformers/en/model_doc/dac
- [12] https://arxiv.org/abs/2504.06561 — StreamCodec.
- [13] https://arxiv.org/abs/2509.16195 — FocalCodec-Stream.

## Open Questions

- Exact parameter count of DAC 44 kHz.
- CPU RTF on x86 and Apple Silicon for DAC 16 kHz.
- Whether issue #39's chunk-boundary artefacts could be eliminated by overlap-add at decode time.
- DAC as latent target for *causal* PLC (PLC is causal by definition — encoder's non-causality is fatal in the real-time path but not at training time).

## Sub-Hypotheses (noted, not spawned)

- **dac-causal-retrain-feasibility**
- **dac-as-plc-latent-space**
- **16khz-dac-cpu-rtf** (empirical measurement)
