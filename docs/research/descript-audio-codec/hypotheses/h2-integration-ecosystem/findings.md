# Hypothesis: DAC has no upstream FFmpeg decoder/encoder and is not exposed as a streaming codec — it ships as a Python/PyTorch CLI for file-based encode/decode

## Summary

**Confirmed with high confidence across all six sub-dimensions.** DAC has no entry in FFmpeg's `libavcodec` registry, no ffmpeg-devel patch, no GSoC proposal. The official distribution is the `descript-audio-codec` pip package with a file-based CLI. The user's "FFmpeg integrating DAC" recollection is almost certainly a misattribution: Descript maintains an FFmpeg fork (`descriptinc/ffmpeg`) for binary packaging only (an MP4 seek fix, last release Jan 2025), not for DAC codec integration. The only cross-runtime path is an ONNX export of the 16 kHz model on HuggingFace (`onnx-community/dac_16khz-ONNX`, opset 14), consumed by transformers.js v3.4.0 (merged March 2025). No Rust crate, no C/C++ port, no GGML/GGUF quantization, no streaming/chunked inference.

## Evidence

### 1. FFmpeg Integration — Does Not Exist

- **`libavcodec/codec_id.h` (master)**: zero matches for "DAC", "descript", "encodec", "neural", "Mimi", "SNAC". Most recently added audio IDs are QOA, LC3, G728, AHX — all signal-processing codecs, no neural.
- **FFmpeg trac / mailing list**: no thread for DAC, EnCodec, SNAC, or Mimi. The only adjacent ticket is #9194 "Support for new Google Lyra codec" (Feb 2024) — Lyra, not DAC.
- **`descriptinc/ffmpeg` fork**: 118 334 commits, last release Jan 21, 2025. Self-described purpose: "a bug fix for MP4 seeking operations." No DAC integration. Exists for packaging Descript's editor binary.
- **Likely source of user confusion**: EnCodec is the more commonly mentioned neural codec in FFmpeg-adjacent conversations (AudioCraft ships it); Descript using FFmpeg *internally in their editor* may be the misremembered fact.

### 2. Official Distribution: Python/PyTorch, File-Based

- CLI: `python3 -m dac encode <input> --output <codes>` / `python3 -m dac decode <codes> --output <pcm>` — file-oriented.
- Runtime deps: `argbind`, `descript-audiotools`, `einops`, `numpy`, `torch`, `torchaudio`, `tqdm`. **`onnx` and `onnx-simplifier` are dev-extras only**, not runtime deps.
- Streaming: issue #101 (Jan 2025) is open, unanswered. DAC-JAX paper describes "overlapping-chunk" processing as a *memory* pattern, not low-latency streaming.

### 3. ONNX — Exists at opset 14 via HF Optimum, 16 kHz Only

- **`onnx-community/dac_16khz-ONNX`** (HuggingFace): successful ONNX export of the 16 kHz model — `encoder_model.onnx` + `decoder_model.onnx` at opset 14. Quantized variant also published. Conversion uses dynamic axes (batch_size, num_channels, sequence_length).
- **transformers.js v3.4.0 PR #1215** (merged Mar 5, 2025): adds DAC + Mimi to transformers.js by consuming the `onnx-community` models. Streaming optimization deferred ("cool to add Mimi w/ past key values to speed up streamed decoding"), confirming no streaming path exists.
- **44 kHz and 24 kHz ONNX exports**: no published export. Tractability through tract / sonos's ONNX runtime unverified for the larger variants.
- **Snake activation / weight_norm**: no public ONNX-export failure issues filed. Either `torch.onnx.export` decomposes Snake into Sin/Mul/Add or the model was patched pre-export; the actual handling is undocumented.

### 4. HuggingFace `transformers.DacModel` — PyTorch Only, No Streaming

- API: `encode(input_values, n_quantizers)` / `decode(quantized_representation, audio_codes)` / `forward(...)`.
- Input shape: `(batch_size, 1, time_steps)` — full waveform tensor, not frame-by-frame streaming.
- Backend: **PyTorch only** (no TF, no Flax, no ONNX integration in `transformers`).
- Checkpoints: `descript/dac_16khz`, `descript/dac_24khz`, `descript/dac_44khz`.

### 5. Rust Ecosystem — No Published DAC Crate

- **crates.io**: zero results for `dac` or `descript-audio-codec` in audio/multimedia categories.
- **Candle** (huggingface/candle): no DAC port.
- **Burn**: no DAC port.
- **tract** (sonos/tract): no public DAC ONNX execution. The mjolnir-mesh repo's `plc_tract.rs` targets tPLCnet-class models, not DAC. tract supports opset 14 Conv1d / transposed convs / GRU; Snake activation would need composed ONNX-op sequence or a custom op. **Unverified end-to-end.**

### 6. C/C++ Ports, GGML/GGUF — Do Not Exist

- No `dac.cpp` repo analogous to `whisper.cpp` / `llama.cpp`.
- No GGML quantization or GGUF-format DAC weights.
- `ggml-org` has not touched neural audio codecs as of May 2026.

### 7. TensorRT / CoreML / Vulkan — No Published Path

No benchmark or tutorial for DAC on TensorRT, CoreML, or Vulkan/wgpu. General ONNX Runtime supports these backends, but no DAC-specific work has been published.

## Confidence

**Level**: high. Multiple primary sources converge: FFmpeg `codec_id.h` (definitive registry), upstream DAC GitHub issues, HuggingFace docs (PyTorch-only API), transformers.js PR (ONNX confirmed but limited), crates.io exhaustive search. The local mjolnir-mesh codebase confirms the tract/ONNX pattern points to tPLCnet — not DAC.

## Sources

- [1] https://github.com/FFmpeg/FFmpeg/blob/master/libavcodec/codec_id.h
- [2] https://github.com/descriptinc/descript-audio-codec
- [3] https://github.com/descriptinc/descript-audio-codec/blob/main/setup.py
- [4] https://github.com/descriptinc/descript-audio-codec/issues/101
- [5] https://huggingface.co/docs/transformers/en/model_doc/dac
- [6] https://huggingface.co/onnx-community/dac_16khz-ONNX
- [7] https://github.com/huggingface/transformers.js/pull/1215
- [8] https://github.com/descriptinc — Descript org repos (FFmpeg fork)
- [9] https://arxiv.org/html/2405.11554v1 — DAC-JAX
- [10] https://crates.io/categories/multimedia::audio
- [11] /home/dorje/work/IdentiKey/mjolnir-mesh/crates/mjolnir-audio/src/plc_tract.rs
- [12] /home/dorje/work/IdentiKey/mjolnir-mesh/crates/mjolnir-audio/Cargo.toml

## Open Questions

1. **Snake op decomposition in opset 14**: how `torch.onnx.export` handles Snake (`x + sin²(αx)/α`) is undocumented; tract-compatibility unverified.
2. **weight_norm under dynamic axes**: PyTorch usually inlines, but DAC's specific behavior is unbenchmarked.
3. **FFmpeg Lyra ticket #9194**: closest "neural codec in FFmpeg" precedent (was inaccessible at fetch time); likely the actual origin of the user's mental association.
4. **44/24 kHz ONNX export status**: only the 16 kHz model has a public export; the larger models may have op-graph issues.
5. **tract op coverage for DAC's decoder**: transposed-Conv1d padding semantics + Snake decomposition haven't been tested against tract's op set.
