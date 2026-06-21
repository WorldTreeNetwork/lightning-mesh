# Hypothesis: For Low-Latency Lossy Networks, the Relevant Competitors to DAC Are Lyra v2, Mimi, and SNAC — Not Opus

## Summary

**Strongly confirmed with important nuance.** DAC is fundamentally a non-causal, non-streaming, offline-quality codec with ~190 ms algorithmic latency and no built-in packet loss handling — it is architecturally disqualified from real-time transport. The real comparison space has two tiers: (1) purpose-built streaming neural codecs (Mimi, EnCodec 24 kHz, Lyra v2) for the encoder/transport role, and (2) Opus 1.5 with DRED as the incumbent baseline that is vastly harder to beat than it appears. SNAC is also non-causal and shares DAC's streaming disqualification. The "DAC vs Opus" framing is therefore a false dichotomy.

## Codec-by-Codec Analysis

### Opus 1.5+ (libopus, with DRED and Deep PLC)
- **License**: BSD 3-clause; no proprietary weights.
- **Causal/latency**: 20 ms frame (configurable to 2.5 ms). Fully causal. Deep PLC in-decoder = zero added encode latency.
- **Bitrate**: 6–510 kbps (+ DRED overhead ~12–32 kbps).
- **Sample rates**: 8/12/16/24/48 kHz.
- **Compute**: μs/frame on any CPU; no GPU.
- **PLC**: **Deep PLC + DRED built-in** (1.5). DRED: RDO-VAE encodes acoustic features @ ~650 b/s, each 20 ms packet carries up to 1.04 s of redundancy; outperforms LBRR and standalone Deep PLC even at 18.4% loss with 1 s bursts.
- **Rust**: `audiopus` crate.
- **Verdict**: Tier 1 for both transport and PLC.

### DAC
- **License**: MIT (code + weights).
- **Causal/latency**: Non-causal, ~190 ms.
- **Bitrate**: ~8 kbps (44.1k).
- **Sample rates**: 16/24/44.1 kHz.
- **PLC**: None; RVQ state dependency → catastrophic on loss.
- **Rust**: None official; community C# (`NeuralCodecs`).
- **Verdict**: Disqualified for transport; offline reference only.

### EnCodec (Meta)
- **License**: Code MIT; **weights CC-BY-NC 4.0** — commercial blocker.
- **Causal/latency**: 24 kHz model causal; 13.3 ms warm-up; 48 kHz stereo non-causal (1 s latency).
- **Bitrate**: 1.5/3/6/12/24 kbps.
- **Sample rates**: 24 kHz (causal) or 48 kHz (non-causal stereo).
- **Compute**: ~10× realtime on commodity CPU (~50 GFLOPS); no GPU required.
- **PLC**: None.
- **Rust/ONNX**: None official.
- **Verdict**: Technically excellent (causal, 13 ms, fast CPU) but **commercially blocked**.

### SoundStream (Google)
- **License**: No public weights.
- **Verdict**: Not deployable directly; functionally subsumed by Lyra v2.

### Lyra v2
- **License**: Apache 2.0 (code + TFLite weights).
- **Causal/latency**: 20 ms frame, fully causal.
- **Bitrate**: 3.2 / 6.0 / 9.2 kbps.
- **Sample rate**: 16 kHz (speech only — explicitly not a general audio codec; "isn't a replacement of Opus in any way").
- **Compute**: 0.57 ms encode+decode per 20 ms frame on Pixel 6 Pro = 35× realtime.
- **PLC**: **Built-in `DecodePacketLoss()` API**.
- **Rust**: None (Bazel + C++); WASM ports exist.
- **Maintenance**: Last release v1.3.2, **December 2022** — effectively abandoned.
- **Verdict**: Best property combo (causal/20 ms/built-in PLC/Apache) but speech-only and stale.

### Mimi (Kyutai / Moshi)
- **License**: Code MIT (Python) + Apache 2.0 (Rust); weights **CC-BY 4.0** (commercial OK with attribution).
- **Causal/latency**: 80 ms frame, 12.5 Hz token rate. Fully causal streaming in both directions. Moshi theoretical end-to-end 160 ms.
- **Bitrate**: 1.1 kbps (16 RVQ codebooks).
- **Sample rate**: 24 kHz.
- **Compute**: GPU-optimized (CUDAGraph). H100: 400 concurrent streams. CPU RTF unpublished.
- **PLC**: None.
- **Rust**: **Official `moshi` crate on crates.io with CUDA + Metal support.**
- **Verdict**: Viable for speech with caveats (80 ms per-packet exposure is large); the *only* neural codec in this list with first-class Rust.

### SNAC
- **License**: MIT (code + weights).
- **Causal/latency**: Non-causal; multi-scale hierarchy; ~100 ms minimum segment.
- **Bitrate**: 0.98 (24k) / 1.9 (32k) / 2.6 (44k) kbps.
- **PLC**: None; coarse-token loss invalidates fine-grained tokens in window.
- **Rust/ONNX**: None official.
- **Verdict**: Disqualified for transport; interesting offline.

## Comparative Table

| Codec | Causal? | Latency | Bitrate | Built-in PLC | Weights license | Rust port | Active? |
|-------|---------|---------|---------|--------------|------------------|-----------|---------|
| Opus 1.5 + DRED | Yes | 20 ms | 6–510 kbps | **Yes (DRED+DeepPLC)** | BSD-3 (no weights needed) | `audiopus` | Yes |
| Lyra v2 | Yes | 20 ms | 3.2–9.2 kbps | Yes | Apache 2.0 | No (C++/Bazel) | **No (stale 2022)** |
| EnCodec 24 kHz | Yes | 13.3 ms | 1.5–24 kbps | No | **CC-BY-NC (NC)** | C# only | Moderate |
| Mimi (Kyutai) | Yes | 80 ms | 1.1 kbps | No | CC-BY 4.0 | **Yes (official)** | Yes (2024+) |
| DAC | No | ~190 ms | 8 kbps | No | MIT | No | Moderate |
| SNAC | No | ~100 ms seg. | 0.98–2.6 kbps | No | MIT | No | Moderate |
| SoundStream | Yes (paper) | 20 ms (paper) | 3–18 kbps (paper) | Unknown | No public weights | No | N/A |

## DRED Deep Dive

- RDO-VAE encodes acoustic features → ~650 b/s.
- Each 20 ms Opus packet appends a DRED extension covering up to 1.04 s (50 frames) of prior audio.
- Late-arriving packet reconstructs missing audio — no separate FEC channel or NACK.
- Total overhead < 32 kbps for 1 s of redundancy (~1.5× primary stream).
- At 18.4% average loss with 1 s bursts: outperforms LBRR and standalone Deep PLC.
- Cost: jitter buffer depth grows to exploit the redundancy. "Optimal trade-off between loss robustness and jitter buffer delay is still an open question."
- **Directly applicable to QUIC datagram transport** in mjolnir-mesh; works with the existing NetEq-style jitter buffer pattern.

## Confidence

**Level**: high. All architectural facts confirmed against multiple independent sources (paper + repo + IETF draft + LICENSE files + release pages).

## Sources

- [1] https://opus-codec.org/release/stable/2024/03/04/libopus-1_5.html
- [2] https://arxiv.org/html/2212.04453v3 — DRED paper.
- [3] https://datatracker.ietf.org/doc/html/draft-ietf-mlcodec-opus-dred-01
- [4] https://github.com/descriptinc/descript-audio-codec
- [5] https://github.com/descriptinc/descript-audio-codec/issues/101
- [6] https://arxiv.org/html/2405.11554v1
- [7] https://arxiv.org/html/2504.06561v1 — StreamCodec.
- [8] https://github.com/facebookresearch/encodec
- [9] https://huggingface.co/facebook/encodec_24khz/raw/main/README.md
- [10] https://opensource.googleblog.com/2022/09/lyra-v2-a-better-faster-and-more-versatile-speech-codec.html
- [11] https://github.com/google/lyra
- [12] https://github.com/kyutai-labs/moshi
- [13] https://crates.io/crates/moshi
- [14] https://arxiv.org/html/2410.00037v2 — Moshi paper.
- [15] https://github.com/hubertsiuzdak/snac
- [16] https://arxiv.org/html/2410.14411v1 — SNAC paper.
- [17] https://arxiv.org/html/2406.08900v1 — Neural codec error resilience.
- [18] https://www.isca-archive.org/interspeech_2024/muller24c_interspeech.html
- [19] https://github.com/DillionLowry/NeuralCodecs
- [20] https://lib.rs/crates/moshi-cli

## Open Questions

1. Mimi CPU RTF without GPU (matters for commodity mesh nodes).
2. EnCodec commercial license path or retraining feasibility.
3. DRED jitter-buffer latency under realistic QUIC datagram burst loss.
4. Whether Google has a Lyra v2 successor.
5. SNAC explicit causality confirmation.
6. Packet loss tolerance benchmarks for Mimi / EnCodec absent external PLC.
