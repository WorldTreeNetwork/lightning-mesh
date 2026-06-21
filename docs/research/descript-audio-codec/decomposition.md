# Decomposition: Descript Audio Codec (DAC) — Technical Investigation for Real-Time Mesh Audio over QUIC

## Understanding

The user wants a deep technical brief on Descript's open-source neural audio codec (DAC) to decide whether/how it fits into `mjolnir-mesh` — a Rust real-time mesh audio system currently running libopus 1.5 over QUIC datagrams, with an in-flight neural PLC pipeline. A good answer enumerates DAC's identity (paper, repo, license), its quality/latency/compute envelope, FFmpeg and Rust integration maturity, how it compares to Opus / EnCodec / SoundStream / Lyra / Mimi / SNAC for **streaming low-latency lossy-network** use, and where it would either help or break the existing per-peer inference + SPSC ring + out-slice trait architecture.

## Sub-Questions

1. **Identity & provenance**: Which "Descript Audio Codec" is canonical (paper, authors, repo, version history, license), and what is the official model card (sample rates, bitrates, codebook structure, parameter count)?
2. **Integration & ecosystem**: What is the state of FFmpeg support, Rust bindings/ports, ONNX/GGUF/Candle/Burn exports, and other production-grade reimplementations?
3. **Real-time suitability**: What are DAC's actual latency, lookahead, frame size, and compute (CPU/GPU) characteristics — and is it usable in a streaming sub-100ms loop, or is it inherently offline/file-oriented?
4. **Comparative quality & resilience**: How does DAC compare to Opus 1.5 (with built-in neural PLC), EnCodec, SoundStream, Lyra v2, Mimi (Kyutai/Moshi), and SNAC on (a) MUSHRA/ViSQOL quality at matched bitrate, (b) behavior under packet loss, and (c) suitability for neural PLC backbones?
5. **Architectural fit with mjolnir-mesh**: Could DAC plug into the existing per-peer inference thread + SPSC ring + out-slice trait scaffold as either a codec OR a PLC backbone, and what would integration cost / risk look like vs continuing with Opus + a separate neural PLC model?

## Selected Hypotheses (top 5)

1. **H3 — Real-time suitability** (web + analysis, heaviest): Frame size, lookahead, causality, per-frame compute.
2. **H4 — Competitive landscape** (web): DAC vs Lyra v2 / Mimi / SNAC for streaming.
3. **H5 — RVQ tokens as PLC substrate** (codebase + web + analysis): Fit with existing out-slice trait & per-peer inference thread.
4. **H2 — Integration ecosystem** (web): FFmpeg / Rust / ONNX / tract / Candle / Burn ports.
5. **H1 — Canonical identity & license** (web): Paper, repo, weights license.

## Investigation Notes

- H3 is the decisive branch; the rest hinge on its answer.
- H5 should read repo code (`src/` for out-slice trait, SPSC ring, tract scaffold; `docs/research/` for prior neural PLC artifacts).
- H1, H2, H4 are pure external/web branches.
- All investigators cite primary sources (arXiv, GitHub, FFmpeg mailing list / tracker).
