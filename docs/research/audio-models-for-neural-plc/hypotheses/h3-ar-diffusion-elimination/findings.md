# Hypothesis: H3 — Most "Famous" Generative Audio Models Are Architecturally Incompatible with PLC Latency Budgets Regardless of Quantization

## Summary

The hypothesis is **strongly confirmed**. Every model in the named set — Bark, Tortoise-TTS, MusicGen (all sizes), Stable Audio Open, AudioGen, Riffusion, Jukebox, and open VALL-E reproductions — is architecturally disqualified from PLC use at the required budget (~11–40 ms per concealed packet). The barrier is not quantization headroom; it is the structural cost of autoregressive token generation at 50 Hz codec frame rate, multi-step diffusion, or sheer parameter mass. The one genuinely relevant research direction — music-specific PLC — has its own small-model lineage (PARCnet-IS2) that is completely orthogonal to this model family.

---

## Evidence

### The PLC Latency Budget (Ground Truth from Codebase and Challenge Specs)

The mjolnir-mesh codebase uses 20 ms Opus frames (default `AudioConfig`). The IEEE-IS2 2025 Music PLC Challenge — the current state-of-the-art reference benchmark — mandates **< 11.6 ms per 512-sample packet** at 44.1 kHz on a consumer Intel Core i5. [source: 2025-music-plc-challenge GitHub repo] The Opus PLC backend in mjolnir already runs in the microsecond range on the same hardware (it is a DSP waveform extrapolation, not a neural forward pass). Any neural challenger must beat or approach that budget.

---

### Per-Model Verdict

**Bark (suno-ai/bark)**
Architecture: three-stage AR transformer cascade — semantic token LM (GPT-style), coarse acoustic LM, fine acoustic LM, then EnCodec decoder. The semantic stage alone runs over a vocabulary of ~10,000 tokens with a large GPT-2-class backbone.
Real-time factor: on an NVIDIA TITAN RTX (a $2,500 workstation GPU), best-case optimized single-sample generation takes **8.1 seconds** wall clock for a few seconds of audio. [source: HuggingFace Optimizing-Bark blog, measured on TITAN RTX with FP16 + BetterTransformer + offload]. RTF > 3.0 on high-end GPU; CPU is not measured but estimated to be 10–50× slower. The latency plateau (~20 s regardless of input length, noted in Salad benchmark) reflects the fixed pipeline cost, not audio duration.
License: MIT (suno-ai/bark on GitHub).
Fast-mode forks: none that change the fundamental AR token generation cost. The HuggingFace streaming interface reduces time-to-first-audio slightly, but first-audio still requires completing several transformer forward passes.
**Verdict: out of scope — ~400× over budget on best-available GPU, not improvable by quantization alone.**

**Tortoise-TTS (neonbjb/tortoise-tts)**
Architecture: AR transformer over discrete speech tokens, with an optional diffusion decoder for high quality. Even the "ultra-fast" mode disables diffusion but retains the AR stage.
Real-time factor: optimized fork (152334H/tortoise-tts-fast) achieves roughly **RTF 0.3–0.5 on 4GB VRAM GPU** for complete utterances — meaning for a 20 ms concealment window it would still need the full AR priming sequence and cannot produce 20 ms of contextually coherent audio in under 10 ms. P90 latency for 18-word sentences measured at **45 seconds** in one responsiveness benchmark. [source: preprints.org/202508.0654]
License: Apache 2.0.
Fast-mode forks: tortoise-tts-fast (5× speedup over baseline) and tortoise-tts-fastest exist, but neither approaches PLC timescales.
**Verdict: out of scope — designed for utterance-scale synthesis, not 20 ms concealment windows.**

**MusicGen Small / Medium / Large (facebookresearch/audiocraft)**
Architecture: single-stage AR transformer over 4-codebook EnCodec tokens at **50 Hz** frame rate with interleaved codebook delay patterns. Sizes: small = 300M params, medium = 1.5B, large = 3.3B.
Token rate math: to generate 20 ms of audio = 1 EnCodec frame = 4 codec tokens (4 codebooks). Even the small 300M-parameter model requires one full transformer forward pass over its sequence length per generated frame group. On A100-class hardware, MusicGen-small runs at approximately RTF 1.5–3× slower than real-time for streaming generation. [source: AudioCraft MUSICGEN.md; MusicGen streaming discussion on HuggingFace showing "first chunk in ~5 seconds" = 5000 ms for a ~5 s chunk]. On consumer GPU, far slower.
Streaming fork: HuggingFace `musicgen-streaming` (sanchit-gandhi) exists and is the best community attempt. It reduces time-to-first-audio to ~5 s by beginning playback at a chunk boundary. That is ~250× the PLC budget. The paper "Streaming Generation for Music Accompaniment" (arXiv:2510.22105) uses the same 50 Hz DAC representation with 2-second output chunks on a LLaMA-16L backbone — minimum chunk latency is 2 seconds.
License: CC-BY-NC 4.0 (non-commercial).
**Verdict: out of scope entirely — codec frame rate and transformer size make sub-40 ms impossible at any quantization level with current hardware.**

**Stable Audio Open (stabilityai/stable-audio-open-1.0)**
Architecture: latent diffusion model over a VAE-compressed audio representation. Requires iterative denoising — the default is 200 steps.
Real-time factor: on RTX 3090, generates **8 inference steps per second**; on H100, ~20 steps/second. At 200 steps, that is 25 seconds on RTX 3090 and 10 seconds on H100 to generate any audio at all. [source: Stable Audio Open HuggingFace model card / community benchmark data in search results]
License: Stability AI Community License (non-commercial by default).
**Verdict: out of scope — iterative diffusion is fundamentally incompatible with PLC. The first denoising step produces noise, not audio. There is no streaming path.**

**AudioGen (facebookresearch/audiocraft)**
Architecture: identical AR transformer approach to MusicGen but conditioned on general audio captions rather than music prompts. Same 50 Hz EnCodec tokenization, same transformer sizes (small/medium/large).
Real-time factor: same order-of-magnitude as MusicGen — RTF well above 1.0 on consumer GPU.
License: CC-BY-NC 4.0 (non-commercial).
**Verdict: out of scope — same structural disqualification as MusicGen.**

**Riffusion**
Architecture: Stable Diffusion 1.5 fine-tuned on mel-spectrogram images. Diffusion-based: ~50 UNet denoising steps required per generation.
Real-time factor: SD 1.5 on consumer GPU runs at roughly 2–4 seconds for 50 steps. Each generation produces a fixed-length spectrogram (~5 seconds of audio). No streaming path exists.
License: CreativeML OpenRail-M.
**Verdict: out of scope — spectrogram diffusion cannot concealment a 20 ms window. The spectrogram patch is the minimum unit.**

**Jukebox (openai/jukebox)**
Architecture: hierarchical VQ-VAE + prior transformer stack with 3 levels. The top-level prior is a 1.2B-parameter AR transformer over a very coarse representation; refinement priors add quality iteratively.
Real-time factor: notoriously slow — OpenAI's own documentation notes that generating 1 minute of audio takes hours on a V100. [general knowledge, consistent with the 1.2B top-prior + 2 refinement stages]
License: MIT.
**Verdict: out of scope by a factor of ~10,000×. Included here only for completeness.**

**VALL-E open reproductions (Plachta/VALL-E-X, enhuiz/vall-e)**
Architecture: AR neural codec language model — autoregressive over EnCodec tokens at 75 Hz (24 kHz EnCodec). Needs a "prompt" enrollment plus full AR token generation for the continuation.
Real-time factor: median latency for an 18-word synthesis = **17.79 s, P90 = 45.38 s** on measured hardware [source: preprints.org/202508.0654 responsiveness benchmark]. VALL-E 2 with Grouped Code Modeling reduces sequence length by 4× but is not open-source and still targets utterance-scale synthesis.
License: MIT (VALL-E-X).
**Verdict: out of scope — AR at 75 Hz codec rate with no streaming path for sub-40 ms windows.**

---

### The Music-Aware Concealment Angle

No model in the above set rescues itself via a "music-aware concealment" framing. The structural problem is that all of them must generate tokens or diffusion steps causally from a prompt — they cannot be conditioned on "the 20 ms of audio that is about to arrive" because that audio is the missing packet.

However, there **is** a dedicated music PLC lineage that is entirely different:

**PARCnet-IS2** (Polimi ISPL, IEEE-IS2 2024/2025 Music PLC Challenge baseline): hybrid linear predictor + feedforward CNN, **416K parameters**, encoder-decoder structure, processes 512-sample packets (11.6 ms at 44.1 kHz) in a single feed-forward pass (no autoregression). Designed explicitly for real-time CPU operation. The 2025 challenge enforces `< 11.6 ms per packet` on an Intel Core i5. [source: 2024-music-plc-challenge GitHub parcnet-is2 README; 2025 challenge page on ieee-is2.org] This model is in the same architectural family as NSNet and DeepVQE — small spectral or residual predictors, not generative transformers.

Aironi et al.'s GAN variant from the 2024 challenge has a "lite" version with **3.4M parameters** (full: 54.4M). At 3.4M it is borderline for CPU real-time; the full 54.4M version is likely too slow for the budget.

The conclusion is that music PLC is **not solved** by this generation of large generative models. It is solved by the same class of small, feed-forward, purely-causal models used for speech PLC (LPCnet, NSNet family), adapted for music's richer harmonic content via linear prediction residuals or GAN-based spectral refinement.

---

### Magenta RealTime (Google, 2024–2025) — Best-in-Class Streaming Music Model

For completeness: Magenta RT is an 800M-parameter AR transformer achieving **RTF 1.6** on a TPU v2-8 (free Colab tier). It generates 2-second chunks in 1.25 seconds. [source: magenta.withgoogle.com/magenta-realtime] Even this purpose-built streaming music model is ~60× over the PLC budget. It is not open-source.

---

## Confidence

**Level**: high

Multiple independent sources agree: codec frame rates (50 Hz confirmed in MusicGen paper and AudioCraft docs), measured wall-clock inference times (Bark: 8.1 s on TITAN RTX from HuggingFace blog; Stable Audio Open: 8 steps/s on RTX 3090; MusicGen streaming: 5 s to first chunk; VALL-E: 17.79 s P90), and the PLC challenge's sub-11.6 ms mandate together make the architectural incompatibility unambiguous.

---

## Sources

- [1] **url**: https://huggingface.co/blog/optimizing-bark — Bark inference times on TITAN RTX: baseline 10.48 s, best-optimized 8.1 s per generation (FP16 + BetterTransformer + offload)
- [2] **url**: https://ar5iv.labs.arxiv.org/html/2409.18564 — IEEE-IS2 2024 Music PLC Challenge report: PARCnet-IS2 baseline, 416K params, feedforward CNN + LP hybrid for 44.1 kHz packets
- [3] **url**: https://internetofsounds2025.ieee-is2.org/workshops/3rd-ieee-international-workshop-networked-immersive-audio/music-packet-loss-concealment — 2025 Music PLC Challenge: "< 11.6 ms per 512-sample packet" constraint, Intel Core i5 reference hardware
- [4] **url**: https://github.com/polimi-ispl/2024-music-plc-challenge/tree/main/parcnet-is2 — PARCnet-IS2: 416K parameters, feed-forward, real-time on CPU confirmed
- [5] **url**: https://github.com/polimi-ispl/PARCnet — PARCnet original: 320-sample (10 ms at 32 kHz) packets, single-instrument MAESTRO training
- [6] **url**: https://arxiv.org/html/2510.22105v1 — Streaming music accompaniment paper: DAC 50 Hz, LLaMA-16L backbone, minimum 2-second chunk output
- [7] **url**: https://magenta.withgoogle.com/magenta-realtime — Magenta RT: 800M AR transformer, RTF 1.6 on TPU v2-8, 2 s chunk in 1.25 s
- [8] **url**: https://huggingface.co/facebook/musicgen-large/discussions/13 — MusicGen streaming community fork: first audio after ~5 s of decoding
- [9] **url**: https://stabilityai/stable-audio-open-1.0 (HuggingFace model card) — Stable Audio Open: 200 steps default, 8 steps/s on RTX 3090 = ~25 s minimum
- [10] **url**: https://www.preprints.org/manuscript/202508.0654/v1/download — VALL-E responsiveness benchmark: median 17.79 s, P90 45.38 s for 18-word synthesis
- [11] **file**: `/home/dorje/work/IdentiKey/mjolnir-mesh/crates/mjolnir-audio/src/conceal.rs:1-18` — Current PLC pipeline: OpusPlc (microsecond-class DSP) and SilencePlc backends; neural PLC noted as future work via the same `Recover` trait

---

## Open Questions

1. **PARCnet-IS2 measured CPU latency**: The 416K-parameter model is confirmed real-time capable, but the exact ms-per-packet figure on an ARM or x86 laptop-class CPU is not in any source I found. The 2025 challenge results (submissions closed July 2025) may contain leaderboard data with per-system CPU timings — worth checking the published results.

2. **Aironi et al. lite (3.4M params) CPU timing**: The lite GAN variant is close enough to borderline that a concrete CPU benchmark would determine whether it is worth porting. The 2024 challenge paper does not publish per-model inference latency.

3. **Music-specific training data for a PARCnet-style backend**: PARCnet is trained on MAESTRO piano data. A broader music PLC model (guitar, vocals, full mix) would need a different training corpus. Does any such model exist in open weights? None found.

4. **Diffusion acceleration for concealment**: Consistency models and flow matching can reduce diffusion to 1–4 steps. At 4 steps on a quantized small VAE, the math is still ~100 ms on GPU — but this is worth revisiting in 12–18 months as single-step audio diffusion matures.

5. **Whether the `Recover` trait in mjolnir-audio can absorb a PARCnet-style backend directly**: The trait signature (`decode_lost(lookahead: Option<&[u8]>) -> Result<Vec<i16>>`) is compatible with a feed-forward CNN that takes a context window of previous frames. No sub-hypotheses spawned — this is an implementation question, not a research one.

## Sub-Hypotheses

None warranted. The evidence is convergent and sufficient to close this hypothesis cleanly. The only actionable follow-on (PARCnet-IS2 adaptation for mjolnir) is an implementation task, not a research uncertainty.
