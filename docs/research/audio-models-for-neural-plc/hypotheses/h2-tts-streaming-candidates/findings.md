# Hypothesis: H2 — F5-TTS, CosyVoice 2, and Sesame-CSM as the Most Plausible "Months-Out" Voice-Cloning PLC Backends

## Summary

F5-TTS and CosyVoice 2 are the strongest near-term candidates for speaker-matched voice-cloning in a PLC pipeline, but neither is deployable today within the strict <30ms wall-clock budget for a 20ms audio frame. CosyVoice 2's 150ms first-chunk streaming latency and F5-TTS's 300-500ms first-audio latency on RTX 4090 place both firmly in "months-out" territory — reachable with architectural trimming, quantization, and smaller output windows, but not off-the-shelf. Sesame-CSM is a compelling dark horse for prosody quality, but its autoregressive architecture (RTF ~0.28 on RTX 4090, ~0.8 on RTX 4070) is further from real-time than flow-matching peers. The remaining five models range from "license-blocked" (XTTS-v2) to "wrong problem" (Parler-TTS, StyleTTS2, OpenVoice) to "abandoned" (MetaVoice).

---

## Evidence

### 1. F5-TTS

**Architecture.** Fully non-autoregressive. Core is a 335.8M-parameter Diffusion Transformer (DiT) — 22 layers, 16 heads, 1024 embedding dim, 2048 FFN dim. Text is processed by a ConvNeXt V2 head (4 layers, 512 dim). Vocos is the mel-to-waveform vocoder (separate, ~13M parameters). No dedicated speaker encoder: conditioning is done by concatenating the reference mel spectrogram + its transcription as a prefix to the generation sequence (in-context flow matching). Voice cloning is thus inference-time, requiring only a WAV + transcript, no precomputed embedding.

**License.** Code: MIT. Pre-trained model weights: CC-BY-NC-4.0 (due to Emilia training dataset). Non-commercial use only for the released checkpoint. Fine-tuned checkpoints on other data could be differently licensed.
Sources: [GitHub LICENSE](https://github.com/SWivid/F5-TTS/blob/main/LICENSE), [README](https://github.com/SWivid/F5-TTS).

**Streaming.** Not natively supported. F5-TTS is batch-by-sentence: the model must process a complete text chunk before any audio is returned. Community workarounds split long text into sentence chunks and stream each batch output, but first-packet latency under this scheme reaches 2 seconds in practice (GitHub issue #1225, user with nfe_step=64). With 16 NFE on RTX 3090, RTF is 0.15 — meaning 10s of audio in 1.5s, but the *latency to first byte* for a short chunk (~500ms of audio) is still 75ms+ on ideal hardware, much more in practice due to Python overhead.

**Quantization / ONNX.** Community ONNX export exists: `DakeQQ/F5-TTS-ONNX` (GitHub) supports fp32 and fp16, with OpenVINO acceleration for CPU (5–20% improvement reported). No GGUF port. CPU-only fork exists (`Raxephion/F5TTS-CPU_ONLY-WebUI`). No native int8 export documented.

**GPU latency (NVIDIA consumer).** RTF 0.15 at 16 NFE, RTF 0.31 at 32 NFE, benchmarked on RTX 3090 (10s speech segment). Fast-F5-TTS (EPSS pruning, 7 steps) achieves RTF 0.030 on RTX 3090 — 4× speedup over baseline 32-step inference. First-audio latency on RTX 4090: reported at 300–500ms.

**CPU latency.** Not benchmarked by the paper. Apple M3 Max: ~4s for a full utterance (MLX port). For a PLC scenario targeting 100–500ms of audio output, CPU is impractical without severe quantization + step reduction.

**Rust path.** ONNX export exists; no Rust-native candle port documented.

**PLC verdict.** Months-out. RTF of 0.030 (Fast variant, 7 NFE) on RTX 3090 means 20ms of audio takes ~0.6ms of compute — theoretically viable. But: (1) model weights are CC-BY-NC, (2) no streaming to sub-frame granularity, (3) real-world first-chunk latency 300-2000ms due to non-streaming design and Python stack. Path to deployment requires: commercial license or retraining on permissive data, ONNX+int8 pipeline, sentence-chunking at shortest viable unit (~50–100ms), and tight C++/Rust wrapper.

---

### 2. CosyVoice 2

**Architecture.** LLM-based streaming TTS. Total 0.5B parameters (CosyVoice2-0.5B checkpoint). Architecture: text LM (LLM backbone generating semantic speech tokens at 25Hz) + chunk-aware causal flow matching acoustic model + codec decoder. Finite-scalar quantization improves codebook utilization. Pre-trained LLMs can be used directly as backbone. Voice cloning: zero-shot in-context from reference WAV + reference text (no precomputed embedding required at inference time; cross-lingual cloning supported).

**License.** Apache 2.0. Verified on GitHub `FunAudioLLM/CosyVoice` repository and HuggingFace model card `FunAudioLLM/CosyVoice2-0.5B`. No academic-only restriction on the 0.5B model card.

**Streaming.** First-class feature: bi-directional streaming (text-in + audio-out). First-packet synthesis latency: 150ms (claimed by FunAudioLLM; hardware not specified in the paper page). Chunk-aware causal flow matching specifically designed to support chunked generation. SDPA and KV cache for RTF optimization. vLLM 0.9.0+ supported (as of 2025/05). TensorRT-LLM integration provides 4× acceleration over HuggingFace transformers.

**Quantization / ONNX.** 7 quantized model variants available on HuggingFace. ONNX export: `Lourdle/CosyVoice2-0.5B_ONNX` exists on HuggingFace; `cosyvoice.cpp` (C++/GGML) supports f32, f16, q8_0, q5_0, q5_1, q4_0, q4_1 quantization with CUDA, Metal, and CPU backends. CosyVoice2-0.5B ONNX fp16 NaN bug was fixed November 2025.

**GPU latency (NVIDIA consumer).** No official benchmark on RTX 3090/4090 published. The 150ms first-chunk claim lacks a hardware citation in available docs. With TensorRT-LLM at 4× speedup, inferred first-chunk latency could approach 40ms on high-end GPU — speculative.

**CPU latency.** Not benchmarked. The C++ GGML port with q4_0 quantization makes CPU inference plausible but no numbers are published.

**Rust path.** No Rust/candle port. ONNX export and cosyvoice.cpp are the non-Python paths.

**PLC verdict.** Months-out, and arguably the strongest candidate. Apache 2.0 license removes the commercial blocker. Native streaming with 150ms first-chunk (GPU, unspecified) + chunk-aware causal design is architecturally aligned with PLC chunked generation. The 0.5B size is tractable for quantization. Main gaps: (1) 150ms first-chunk is still 7.5× the 20ms frame budget, requiring sub-sentence chunking research; (2) no published sub-30ms latency path yet; (3) Rust/embedded deployment path requires porting cosyvoice.cpp or building ONNX wrapper.

---

### 3. Sesame CSM (CSM-1B)

**Architecture.** Two autoregressive transformer stages. Backbone: ~1B-parameter Llama architecture (exact layer count not disclosed) processing interleaved text + audio tokens. Decoder: ~100M-parameter Mimi audio decoder producing split-RVQ codes at 12.5 Hz (1 semantic codebook + N−1 acoustic codebooks). Codec: Kyutai Mimi. Context window: 4096 audio tokens (~5.5 minutes). Voice conditioning: reference audio clips provided as prior conversation context — no separate speaker encoder, the backbone attends to prior audio.

**License.** Apache 2.0. (Model card at `sesame/csm-1b` on HuggingFace shows `apache-2.0`; license was changed from an initial more restrictive version.)

**Streaming.** No official streaming API. Community ports exist: `davidbrowne17/csm-streaming` and `interactivetech/csm-streaming-tts` on GitHub. The autoregressive backbone must generate tokens sequentially, making streaming possible in principle (yield audio tokens as produced) but with accumulated sampling latency.

**Quantization.** Two quantized derivative models on HuggingFace (methods not specified in model card). `cartesia-one/csm.rs` (Rust/candle) supports GGUF q8_0 and q4_k; AGPL-3.0 license. VRAM: 6–8GB FP16, 3–4GB INT8.

**GPU latency (NVIDIA consumer).** RTF ~0.28 on RTX 4090 (10s of audio in 2.8s). RTF ~0.8 on RTX 4070. TTFA: ~150ms (synthesis step only); full pipeline adds LLM TTFT 150–300ms + ASR 30–80ms. `torch.compile()` with static KV cache is the recommended optimization.

**CPU latency.** No published benchmarks. At RTF 0.28 on a 4090, CPU inference would likely be RTF 3–10×, far outside real-time.

**Rust path.** `cartesia-one/csm.rs` — active project, GGUF quantization, candle backend, CUDA + Metal + CPU. **This is the only model in the set with a production-grade Rust inference engine.** However, AGPL-3.0 complicates embedding in a proprietary Rust binary.

**PLC verdict.** Months-out, but harder than CosyVoice 2 for the PLC latency target. RTF 0.28 on RTX 4090 means 20ms of audio takes 5.6ms — acceptable per-frame if you only need a short fill segment, but the autoregressive model cannot produce those 5.6ms without first running the full sequential backbone over prior context. The 150ms synthesis TTFA is the floor for any output, making single-frame PLC impractical. Better fit: generating 200–500ms fill segments during a longer loss burst, with prosody that matches the speaker's conversational register. The csm.rs Rust port makes this the most embedded-ready candidate, modulo the AGPL license.

---

### 4. XTTS-v2 (Coqui)

**Architecture.** Two-stage autoregressive: (1) GPT-2-style decoder-only transformer with 443M parameters generates discrete audio tokens conditioned on mel-spectrogram speaker embeddings (32 fixed-length 1024-dim embeddings from a Perceiver Resampler over the reference mel). (2) HiFi-GAN vocoder (26M parameters) reconstructs waveform. Total: ~750M parameters. Voice cloning: reference WAV (≥6s) → Conditioning Encoder → speaker embeddings → injected into GPT decoder.

**License.** Coqui Public Model License 1.0.0. Non-commercial only. Commercial tier was discontinued when Coqui AI shut down. This is a hard blocker for any production use.

**Streaming.** Supported — first-audio latency ~150ms on consumer GPU (documented). This is XTTS-v2's strong suit: token-by-token autoregressive generation naturally streams audio.

**Quantization.** No official quantization. Community attempts exist but quality degradation is noted.

**PLC verdict.** License-blocked for commercial use. Even setting license aside, the autoregressive GPT decoder is slower than flow matching alternatives and the project is effectively abandoned (Coqui shut down 2024).

---

### 5. StyleTTS2

**Architecture.** Diffusion-based style modeling. Components: PL-BERT text encoder, text aligner (pre-trained), JDC pitch extractor, style diffusion model (latent variable over speaking style), HiFi-GAN decoder. Total parameters not published; estimated ~200–400M across all components. Voice cloning: compute style latent from reference audio via style encoder at inference time.

**License.** Code: MIT. Pre-trained models: custom terms requiring disclosure of synthesis or speaker permission. No explicit commercial restriction, but terms are ambiguous. A GPL-licensed dependency (phonemizer) contaminates the inference stack in some configurations.

**Streaming.** GPL fork has experimental streaming API; official repo does not.

**Quantization.** No documented ONNX or quantization path.

**GPU latency.** RTF computed on RTX 2080 Ti; 5–6s for 439-character input on LibriTTS (measured from a GitHub issue) — far slower than real-time for longer text. Diffusion steps can be reduced to 3 for speed, but quality degrades.

**PLC verdict.** Research-only for this use case. Diffusion-based style model adds latency, multi-component stack is hard to quantize end-to-end, no streaming, and the latency numbers are the worst of any model here.

---

### 6. OpenVoice v2

**Architecture.** Two-stage: (1) Base speaker TTS (modified VITS) with style control (emotion, accent, rhythm). (2) Tone Color Converter: 1D CNN encoder + invertible normalizing flow + HiFi-GAN decoder. Trained on 300K audio from 20K speakers. No published parameter counts. Inference: base TTS → audio → tone color converter applies reference speaker's timbre.

**License.** MIT (since April 2024). Verified at `myshell-ai/OpenVoice` GitHub. Free for commercial use.

**Streaming.** Not documented as a streaming model. Feed-forward architecture (normalizing flow) is fast: 12× real-time on A10G (85ms to generate 1s of audio). Upper bound ~40× with optimization.

**Quantization.** ONNX-exportable (mentioned in OpenVINO documentation). Community ONNX ports exist.

**GPU latency.** 85ms/s on A10G (12× real-time) = RTF ~0.083. This is competitive with F5-TTS fast mode. For a 100ms audio segment: ~8ms of compute, theoretically within the 20ms budget on high-end GPU.

**PLC verdict.** Deployable for offline quality, months-out for sub-frame PLC. The normalizing flow tone-color-converter stage is fast, but the base VITS TTS must generate the audio first (non-streaming), so first-audio latency is tied to VITS generation speed. The two-stage architecture means the reference speaker voice is applied as a post-process, which is an architecturally interesting fit for PLC (generate plausible speech first, then apply speaker tone color in a fast pass). Not designed for this use case but could be repurposed.

---

### 7. MetaVoice-1B

**Architecture.** Three-stage: (1) Causal GPT (AR transformer, ~1.2B total including this) predicts first 2 EnCodec hierarchies; (2) ~10M-parameter non-causal transformer predicts remaining 6 hierarchies in parallel; (3) EnCodec decoder. Voice cloning: speaker embedding from a separately trained speaker verification network, injected at token embedding layer. Requires 30s reference audio (American/British); 1min for Indian voices.

**License.** Apache 2.0. Verified at `metavoiceio/metavoice-src` GitHub.

**Streaming.** Not documented.

**Quantization.** Experimental int4 and int8 modes available (acknowledged quality degradation at int4). 30–90s startup due to `torch.compile`.

**GPU latency.** RTF < 1.0 on Ampere/Ada/Hopper after `torch.compile`. ~0.8–1.2 words/second processing. Not fast enough for sub-100ms segments.

**PLC verdict.** Research-only. Causal GPT backbone is slow, project appears abandoned (no updates since early 2024), and 30s reference audio requirement is too high for PLC scenarios where the speaker audio buffer may be short.

---

### 8. Parler-TTS Mini v1

**Architecture.** Seq2Seq: T5 text encoder → autoregressive decoder over Descript Audio Codec (DAC) tokens, with delayed codebook interleaving. 880M–0.9B parameters. Voice conditioning: natural language speaker descriptions (34 named speakers in the training set). No reference-audio voice cloning at inference time — only description-based style transfer.

**License.** Apache 2.0. Verified at `parler-tts/parler-tts-mini-v1` on HuggingFace.

**Streaming.** Streaming mode exists via external inference guide; not documented with latency numbers.

**Quantization.** OpenVINO optimization documented.

**PLC verdict.** Wrong problem. Parler-TTS cannot clone an arbitrary speaker's voice from reference audio — it only supports its 34 trained speaker personas. Completely unsuitable for the PLC use case, which requires matching the actual caller's voice.

---

## Top-3 Ranking for PLC Voice-Cloning Backend

**Rank 1: CosyVoice 2 (CosyVoice2-0.5B)**
- Apache 2.0 license (no commercial blocker)
- Only model with native chunk-aware streaming designed in from the start
- 150ms first-chunk latency claim (hardware unspecified but likely A100-class)
- C++/GGML port with q4_0–q8_0 quantization available
- ONNX export available
- 0.5B parameters — smallest tractable size in this set
- TensorRT-LLM 4× acceleration documented

**Rank 2: F5-TTS (v1, with Fast-F5-TTS 7-NFE variant)**
- MIT code license; CC-BY-NC model weights (commercial blocker, but retrain path exists on Apache-licensed data)
- 335M parameters — smallest in the set, easiest to quantize
- RTF 0.030 (7 NFE, RTX 3090) — fastest per-unit-audio compute of any model here
- ONNX community port exists; no streaming but sentence-chunking is the practical approach
- First-audio latency 300ms+ is the main gap; solvable with shorter chunk lengths
- Active community, MLX port, ongoing optimization work

**Rank 3: Sesame CSM-1B**
- Apache 2.0 license
- Best prosody quality for conversational audio (designed specifically for this register)
- RTF 0.28 on RTX 4090 — adequate for 200ms+ PLC fill segments
- Rust/candle port (csm.rs) with GGUF quantization — best Rust path of any model here
- AGPL-3.0 on csm.rs complicates commercial embedding; PyTorch path is Apache 2.0
- Autoregressive backbone means cannot stream below ~150ms TTFA floor

---

## Confidence

**Level**: medium

Multiple independent sources agree on architecture, parameter counts, and license for each model. RTF numbers for F5-TTS and Sesame CSM come from community benchmarks and GitHub issues rather than official papers, and CosyVoice 2's 150ms latency claim lacks a hardware citation. The PLC latency budget analysis (≤30ms per 20ms frame) is derived reasoning applied to these numbers — no source has benchmarked these models explicitly in a PLC scenario.

---

## Sources

- [1] **url**: https://github.com/SWivid/F5-TTS — Official F5-TTS repository; architecture, license (MIT code / CC-BY-NC weights), streaming issues, ONNX community pointer
- [2] **url**: https://arxiv.org/html/2410.06885v1 — F5-TTS paper (October 2024); 335.8M DiT parameters, 22 layers, 16 heads, RTF 0.15 at 16 NFE / 0.31 at 32 NFE on RTX 3090
- [3] **url**: https://fast-f5-tts.github.io/ — Fast-F5-TTS (EPSS, 7-step); RTF 0.030 on RTX 3090, 4× speedup over baseline
- [4] **url**: https://github.com/SWivid/F5-TTS/issues/1225 — F5-TTS streaming issue; first-packet latency 2s reported with nfe_step=64
- [5] **url**: https://github.com/DakeQQ/F5-TTS-ONNX — Community ONNX export for F5-TTS; fp16 and fp32, OpenVINO support
- [6] **url**: https://huggingface.co/FunAudioLLM/CosyVoice2-0.5B — CosyVoice2-0.5B model card; Apache 2.0, 0.5B parameters, 150ms streaming latency, 7 quantized variants, ONNX format
- [7] **url**: https://funaudiollm.github.io/cosyvoice2/ — CosyVoice 2.0 technical page; chunk-aware causal flow matching, 150ms first-packet, finite-scalar quantization
- [8] **url**: https://github.com/FunAudioLLM/CosyVoice — Official CosyVoice repo; Apache-2.0 license, vLLM/TensorRT-LLM integration
- [9] **url**: https://github.com/Lourdle/cosyvoice.cpp — cosyvoice.cpp C++/GGML port; q4_0 through q8_0 quantization, CUDA/Metal/CPU backends
- [10] **url**: https://huggingface.co/Lourdle/CosyVoice2-0.5B_ONNX — CosyVoice 2 ONNX export (fp16 NaN fix November 2025)
- [11] **url**: https://github.com/SesameAILabs/csm — Official CSM repo; Llama backbone + Mimi decoder, Apache 2.0
- [12] **url**: https://huggingface.co/sesame/csm-1b — CSM-1B model card; Apache 2.0, torch.compile optimization, 2 quantized derivatives
- [13] **url**: https://www.spheron.network/blog/speech-to-speech-gpu-cloud-moshi-sesame-csm-hertz-dev/ — CSM latency analysis; TTFA ~150ms synthesis-only, RTF ~0.28 on RTX 4090, VRAM 6–8GB FP16
- [14] **url**: https://github.com/cartesia-one/csm.rs — csm.rs Rust/candle implementation; GGUF q8_0 + q4_k, AGPL-3.0, MKL/CUDA/Metal backends, built-in RTF benchmark tool
- [15] **url**: https://github.com/yl4579/StyleTTS2 — StyleTTS2 official repo; MIT code, custom model terms, no quantization/ONNX, GPL fork has streaming
- [16] **url**: https://arxiv.org/html/2312.01479v6 — OpenVoice paper; architecture (VITS base + normalizing flow converter), 12× real-time on A10G (85ms/s), parameter counts not disclosed
- [17] **url**: https://github.com/myshell-ai/OpenVoice — OpenVoice repo; MIT license (April 2024+)
- [18] **url**: https://github.com/metavoiceio/metavoice-src — MetaVoice-1B repo; Apache 2.0, 1.2B params, ~10M non-causal transformer, RTF <1.0 after torch.compile, experimental int4/int8
- [19] **url**: https://huggingface.co/parler-tts/parler-tts-mini-v1 — Parler-TTS mini v1; Apache 2.0, 880M/0.9B, description-based speaker control only (no reference-audio cloning)
- [20] **url**: https://huggingface.co/coqui/XTTS-v2 — XTTS-v2; Coqui Public Model License (non-commercial), 443M GPT + 26M HiFi-GAN, streaming ~150ms first-audio on consumer GPU

---

## Open Questions

1. **CosyVoice 2 hardware citation for 150ms claim**: The 150ms first-packet latency is stated without specifying the GPU model. On a consumer RTX 3090/4090, is this achievable, or was it measured on an A100/H100? This significantly affects the months-out estimate.

2. **F5-TTS CC-BY-NC retraining path**: The fast path to a commercially deployable F5-TTS is retraining on Apache/CC-BY-4.0 speech data (e.g., LibriSpeech, VoxPopuli). Is the Emilia dataset the only blocker, or are there additional training data dependencies baked into the checkpoint?

3. **Sub-sentence chunking quality for flow-matching models**: F5-TTS and CosyVoice 2 require text as input. For PLC, the text of the concealed segment may not be known (the packet was lost). This is the fundamental mismatch — does the PLC use case require TTS-with-known-text (fill-word synthesis) or true audio extrapolation? If the latter, these models need an upstream LM to predict the text.

4. **csm.rs AGPL licensing**: Cartesia built csm.rs under AGPL-3.0. For embedding in a proprietary Rust binary (mjolnir-mesh), this is a hard blocker unless Cartesia offers a commercial license or the PyTorch-based CSM weights are wrapped in a custom inference engine.

5. **CosyVoice 2 vs CosyVoice 3**: The HuggingFace hub shows `Fun-CosyVoice3-0.5B-2512_RL` — a more recent RL-tuned variant with lower CER/WER. Whether CosyVoice 3 maintains the same streaming architecture and 150ms latency is unconfirmed and worth checking.

6. **Minimum reference audio duration for CSM PLC**: CSM's voice conditioning depends on prior conversation audio context. In a PLC scenario, how much prior audio of the same speaker is needed before the model can plausibly match their voice? The model has a 4096-token (~5.5min) context window, but usable conditioning might need only a few seconds.

7. **OpenVoice v2 two-stage PLC repurposing**: The normalizing flow tone-color-converter (fast, deterministic) could theoretically be run as a post-processor on top of any TTS output. Has anyone benchmarked the converter in isolation, without the VITS base TTS, as a voice-matching layer?

## Sub-Hypotheses

- **[cosyvoice2-plc-chunk-sizing]**: CosyVoice 2's chunk-aware causal model may be tunable to produce sub-50ms audio chunks, potentially closing the gap to the 20ms PLC frame budget — this requires profiling the model at minimum chunk sizes and measuring quality degradation, which cannot be resolved from documentation alone.

- **[f5-tts-commercial-retrain]**: F5-TTS with 7-NFE EPSS sampling reaches RTF 0.030 on RTX 3090, which is architecturally sufficient for the PLC latency target — the remaining blocker is the CC-BY-NC model weight license, resolvable if a permissive-data retrain preserves RTF; the feasibility of this retrain (dataset availability, training cost) cannot be confirmed from existing documentation.

- **[csm-rs-latency-floor]**: The csm.rs Rust/GGUF implementation may achieve a substantially lower RTF than the PyTorch baseline (0.28 on RTX 4090) through int4/int8 quantization — the built-in RTF benchmark tool exists but no published numbers are available, making the actual floor for the Rust path an open empirical question.
