# Hypothesis: H4 — The AMD AIE / Ryzen AI Path Is Research-Only for Audio in 2026 — No Production-Grade Audio Model Ships with AIE Kernels, and the Realistic NPU Story Is "ONNX Runtime VitisAI EP for CNN-Shaped PLC Models, Nothing for Transformer/Diffusion Audio Gen"

## Summary

This hypothesis is **substantially confirmed with one important nuance**. No production-grade audio enhancement model (PLC, speech enhancement, audio codec post-processing) has been officially shipped with AIE/NPU kernels by AMD as of April 2026. The NPU audio story is entirely ASR (Whisper encoder + Zipformer), delivered through a hybrid NPU+CPU/GPU split that explicitly excludes recurrent decoders from the NPU. The "CNN-shaped PLC via VitisAI EP" path is plausible in principle but has no publicly documented production example, no model zoo entry, and requires manual quantization work. The transformer and diffusion audio generation path definitively runs on Radeon GPU (ROCm/DirectML), not the NPU.

## Evidence

### 1. Vitis AI Model Zoo: No Audio Models

The Vitis AI Model Zoo (through v3.5, the current release) covers: ADAS/AD, medical, video surveillance, robotics, and data center inference. The official documentation ([UG1414 v3.5](https://docs.amd.com/r/en-US/ug1414-vitis-ai/Vitis-AI-Model-Zoo)) explicitly enumerates "ADAS/AD, medical, video surveillance, robotics, data center, and so on" as the application domains. Audio, speech enhancement, PLC, and codec models are entirely absent. The GitHub model-list directory (`Xilinx/Vitis-AI/model_zoo/model-list`) contains only YAML files for vision and NLP inference tasks.

### 2. Ryzen AI Official Audio Support: ASR Only (Whisper Encoder + Zipformer)

As of Ryzen AI Software 1.7.1 (April 2026), the only production audio models on the NPU are:
- **Whisper base, small, medium** — encoder runs on NPU in BF16; decoder runs on CPU (KV caching explicitly unsupported on NPU)
- **Zipformer** — streaming ASR, low-latency transcription
- **Parakeet-TDT** (demo/prototype) — Conformer encoder on NPU (BF16), LSTM decoder on iGPU (DirectML), mel filterbank on CPU

Source: [LIRA GitHub](https://github.com/amd/LIRA): "KV caching is not supported on NPU. If you use `--use-kv-cache` with `--device npu`, only the encoder runs on NPU; the decoder runs on CPU." Models require `--static` flag (static shapes only). The Parakeet-TDT demo explicitly states "currently tested on Strix NPUs; Strix Halo may have compatibility issues" and is categorized in `RyzenAI-SW/Demos/` (not a production release).

### 3. AIE Tile Architecture Constraints — Confirmed Hostile to Dynamic/Recurrent Audio

AIE-ML tile local memory: **64 KB per compute tile**, organized as four 16 KB banks. Memory tiles provide 512 KB. Source: [AMD AM020 — AIE-ML Tile Architecture](https://docs.amd.com/r/en-US/am020-versal-aie-ml/AIE-ML-Tile-Architecture).

XDNA1 (Phoenix/Hawk Point): 4×5 or 4×4 grid. XDNA2 (Strix/Strix Halo): 8×4 grid with 512 KB memory tiles. Source: [AMD XDNA Wikipedia](https://en.wikipedia.org/wiki/AMD_XDNA).

Confirmed hard constraints:
- **Static shapes mandatory**: Dynamic shape fixing utilities exist to convert models prior to compilation; the NPU cannot execute dynamic shapes natively
- **Batch size = 1 only**: Persistent across all releases
- **No recurrent op native support**: RNNs, LSTMs, GRUs are explicitly routed to CPU or iGPU in all documented examples (Parakeet's LSTM decoder → iGPU, Whisper's decoder → CPU)
- **Cascade chain limit**: K-dimension constrained by cascade chain (38 columns on VEK280), introducing padding overhead for non-aligned tensor dimensions
- **No KV-cache on NPU**: Rules out autoregressive transformer decoders entirely

The VitisAI EP documentation confirms: "Models dominated by convolutions (CNNs) perform better with vectorized data, while models dominated by GEMMs (Transformers) perform better with unvectorized data." Even the transformer-friendly path requires falling back to CPU for unvectorizable subgraphs.

### 4. CNN-Shaped PLC Is Architecturally Plausible, But Zero Documented Examples Exist

LACE and NoLACE are CNN-dominant models with predictable tensor shapes and no dynamic sequence dependence (they operate on fixed-size frames). The VitisAI EP supports INT8-quantized CNNs natively, and AMD Quark provides the quantization toolchain. The CRONet paper ([arXiv:2604.14700](https://arxiv.org/html/2604.14700)) demonstrates a hybrid CNN-RNN fitting 73% of a VEK280 AIE-ML array's 304 engines on-chip — proving CNN-shaped models can fit the tile memory budget.

However: zero AMD docs, AMD GitHub examples, or third-party reports describe any audio CNN (PLC, enhancement, codec post-processing) deployed on Ryzen AI NPU through VitisAI EP. The NoLACE paper ([arXiv:2309.14521](https://arxiv.org/abs/2309.14521)) is purely academic, with no NPU deployment path described.

### 5. MLIR-AIE and IREE-AMD-AIE: Research Infrastructure, Not Production Audio

- **mlir-aie** ([Xilinx/mlir-aie](https://github.com/Xilinx/mlir-aie)): Described by AMD as "primarily intended to support tool builders...an open-source research project from the Research and Advanced Development group (RAD)." Not a production deployment path.
- **iree-amd-aie** ([nod-ai/iree-amd-aie](https://github.com/nod-ai/iree-amd-aie)): Self-described as "early-phase." 1,008 commits, 130 stars, 97 open issues. No audio inference examples documented. No production deployments.
- **ARIES** ([FPGA 2025](https://dl.acm.org/doi/10.1145/3706628.3708870)): Academic compiler achieving 22.58x speedup vs. Riallto on ResNet. No audio workloads.
- **AIE4ML** ([arXiv:2512.15946](https://arxiv.org/html/2512.15946v2)): Supports linear layers and MLP-Mixers; no recurrent layer support; "will extend to support additional operators" as future work; no audio claims.

### 6. Audio Generation (Transformer/Diffusion): Definitively GPU, Not NPU

AMD's own blog post on ACE Step 1.5 (commercial-grade AI music generation, May 2026) routes entirely through **AMD Radeon graphics (ROCm) and ComfyUI** — not the NPU. Source: [AMD blog](https://www.amd.com/en/blogs/2026/commercial-grade-ai-music-generation-on-amd-ryzen-ai-and-radeon-ace-step-1-5.html). AMD Noise Suppression runs on the Radeon GPU (RDNA2+), not the NPU. Windows Studio Effects Voice Focus runs on the NPU on Qualcomm Copilot+ PCs; AMD Ryzen AI 300-series gets only "basic effects" with no NPU Voice Focus. Source: Windows Studio Effects community reports.

### 7. Windows Studio Effects Gap on AMD NPU

An important production signal: Microsoft's Windows Studio Effects Voice Focus (audio noise suppression on NPU) is unavailable on AMD Ryzen AI 300-series. "Devices powered by AMD's Ryzen AI 300 series can only access the basic effects." This suggests AMD's own NPU audio story is incomplete even for Microsoft's own audio enhancement pipeline — a strong indicator that no vendor has shipped a production audio enhancement kernel stack on XDNA.

### 8. Quantization Toolchain Is Ready; Deployment Is Not

Quark and the VitisAI Quantizer can produce INT8 ONNX models from CNN-shaped audio models. The Optimum/Hugging Face quantization pipeline for Ryzen AI is documented. Brevitas integration exists for experimental quantization. Source: [Ryzen AI quantization docs](https://ryzenai.docs.amd.com/en/latest/model_quantization.html). The toolchain gap is not quantization — it is the absence of any audio model target in AMD's official examples, the operator support gaps for audio-specific ops, and the static shape requirement conflicting with variable-length audio inference.

## Confidence

**Level**: high

Multiple independent sources converge on the same conclusion: AMD's official audio NPU story is ASR-only through mid-2026, the model zoo has zero audio entries, the MLIR/IREE paths are research-grade, transformer/diffusion audio is definitively GPU-routed, and the Windows Studio Effects gap confirms the NPU audio stack is incomplete even from Microsoft's perspective.

## Sources

- [1] **url**: https://github.com/amd/LIRA — "KV caching is not supported on NPU. Only encoder runs on NPU; decoder runs on CPU." Static shapes required. Whisper base/small/medium and Zipformer only.
- [2] **url**: https://github.com/amd/RyzenAI-SW/tree/main/Demos/ASR/Parakeet-TDT — NPU=Conformer encoder (BF16), iGPU=LSTM decoder (DirectML), CPU=mel features. Static 15s chunks. "Tested on Strix NPUs only."
- [3] **url**: https://docs.amd.com/r/en-US/ug1414-vitis-ai/Vitis-AI-Model-Zoo — Vitis AI Model Zoo covers ADAS/AD, medical, video surveillance, robotics, data center. No audio models.
- [4] **url**: https://docs.amd.com/r/en-US/am020-versal-aie-ml/AIE-ML-Tile-Architecture — 64 KB local data memory per AIE-ML tile, four banks. Memory tiles: 512 KB.
- [5] **url**: https://arxiv.org/html/2604.14700 — CRONet on Versal AIE-ML: 64 KB/tile confirmed, cascade chain limit, 223/304 engines utilized. Demonstrates CNN-RNN fits on-chip. No audio context.
- [6] **url**: https://arxiv.org/html/2512.15946v2 — AIE4ML: linear layers and MLP-Mixers only. No recurrent support. No audio claims. Future work for additional operators.
- [7] **url**: https://github.com/nod-ai/iree-amd-aie — Self-described "early-phase." 130 stars. No audio inference examples.
- [8] **url**: https://github.com/Xilinx/mlir-aie — AMD RAD research project; "primarily intended to support tool builders." Not production audio path.
- [9] **url**: https://ryzenai.docs.amd.com/en/latest/relnotes.html — Release 1.7.1 (Apr 2026): audio support = Whisper.cpp ASR only. Batch size = 1 persistent constraint. No recurrent, no audio codec/enhancement.
- [10] **url**: https://www.amd.com/en/blogs/2026/commercial-grade-ai-music-generation-on-amd-ryzen-ai-and-radeon-ace-step-1-5.html — ACE Step 1.5 music generation: Radeon GPU (ROCm) + ComfyUI. Not NPU.
- [11] **url**: https://onnxruntime.ai/docs/execution-providers/Vitis-AI-ExecutionProvider.html — VitisAI EP: INT8 or BF16 quantization required. CNNs = vectorized path. Transformers = unvectorized path. Operator support list referenced but not enumerated publicly.
- [12] **url**: https://arxiv.org/abs/2309.14521 — NoLACE paper: CNN-dominant Opus codec enhancer. No NPU deployment path described. Academic only.
- [13] **url**: https://riallto.ai/notebooks/2_1_MS_Windows_Studio_Effects.html — Windows Studio Effects Voice Focus on NPU; AMD Ryzen AI 300-series gets "basic effects only" — no NPU Voice Focus.
- [14] **url**: https://en.wikipedia.org/wiki/AMD_XDNA — XDNA1: 4×5/4×4 grid, 64 KB/tile. XDNA2: 8×4 grid, 512 KB memory tiles.
- [15] **url**: https://dl.acm.org/doi/10.1145/3706628.3708870 — ARIES FPGA 2025: MLIR-based compiler achieving 22.58x over Riallto on ResNet. Research context. No audio.

## Open Questions

1. **VitisAI EP operator support matrix for audio ops**: The full list of ONNX operators supported on NPU is not publicly enumerated as a flat table. It is unclear whether `Conv1d`-equivalent operations (which LACE/NoLACE depend on) are supported or fall back to CPU. The `vitisai_ep_report.json` mechanism would reveal this for a specific model, but no one appears to have run it on a PLC-shaped model publicly.

2. **Static-shape compatibility for frame-based PLC**: LACE/NoLACE operate on fixed-frame audio (typically 10ms at 16 kHz = 160 samples). This is a static shape — in principle compatible with the NPU's static-shapes-only requirement. Whether the full operator graph (gated convolutions, layer norm, sigmoid activations) maps cleanly to the VitisAI EP's supported-ops set is unverified.

3. **XDNA3 / Ryzen AI 400 audio roadmap**: AMD released the Ryzen AI 400 series at CES 2026 with XDNA3. No public documentation yet describes whether XDNA3 expands audio model support or addresses the Windows Studio Effects gap.

4. **Third-party deployment (WebRTC stack vendors)**: It is possible that a vendor (e.g., Cisco, Zoom, Teams) has privately deployed a CNN PLC model through VitisAI EP using AMD's OEM partnership track, which would not appear in public docs. No public evidence of this exists.

5. **parakeet-aie (user's own project)**: The search found no public repository at `github.com/duke/parakeet-aie` or equivalent. The closest match is AMD's own `RyzenAI-SW/Demos/ASR/Parakeet-TDT`, which is a demo prototype using the Parakeet-TDT model's Conformer encoder on NPU. If the user's project is internal or private, its AIE runtime cannot be assessed from public sources.

## Sub-Hypotheses

- **[aie-cnn-plc-feasibility]**: The VitisAI EP INT8 CNN path may technically support LACE/NoLACE-shaped models (fixed frames, Conv1d-equivalent ops) without modification — this requires running the actual model through the VitisAI EP partitioner and inspecting the `vitisai_ep_report.json` to see what fraction of ops land on NPU vs. CPU. Cannot be resolved without a test run.
- **[xdna3-audio-expansion]**: Whether XDNA3 (Ryzen AI 400, CES 2026) expands the audio NPU story — specifically whether AMD has addressed the Windows Studio Effects Voice Focus gap — is unresolvable from current public documentation; AMD has not published XDNA3 audio-specific capability docs as of May 2026.
