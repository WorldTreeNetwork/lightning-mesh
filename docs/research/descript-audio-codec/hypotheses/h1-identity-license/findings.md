# Hypothesis: DAC is the Kumar et al. 2023 "High-Fidelity Audio Compression with Improved RVQGAN" model, MIT-licensed

## Summary

**Confirmed on all material points.** DAC = "High-Fidelity Audio Compression with Improved RVQGAN" (arXiv:2306.06546, NeurIPS 2023 spotlight), authored by five Descript researchers, with official code at `github.com/descriptinc/descript-audio-codec`. **Both code AND pretrained weights are MIT-licensed with no separate weight restriction** — materially different from EnCodec (CC-BY-NC weights). Training data is entirely public (DAPS, DNS-4, Common Voice, VCTK, MUSDB, MTG-Jamendo, AudioSet).

## Evidence

### 1. Canonical Identity

- **Paper**: "High-Fidelity Audio Compression with Improved RVQGAN"
- **arXiv**: 2306.06546 (submitted June 11, 2023; revised Oct 26, 2023)
- **Venue**: NeurIPS 2023 **spotlight**
- **Authors**: Rithesh Kumar, Prem Seetharaman, Alejandro Luebs, Ishaan Kumar, Kundan Kumar — all at Descript at publication
- **Repo**: `github.com/descriptinc/descript-audio-codec`, default branch `main`, latest tag **1.0.0** (Jul 20, 2024), 6 releases total starting Jun 12, 2023
- **Maintenance**: Rithesh Kumar moved to Adobe Research Aug 2023. Last release Jul 2024. Active-maintenance status unknown.

### Lineage

| Model | Year | Relation to DAC |
|-------|------|-----------------|
| MelGAN | 2019 | Earlier Descript/Kumar work; vocoder paradigm |
| SoundStream | 2021 (Google) | First neural codec; DAC builds on its fully-conv + RVQ + quantizer-dropout |
| EnCodec | 2022 (Meta) | Direct predecessor; DAC replaces ELU with Snake, improves codebook, adds multi-band STFT discriminator |
| RVQGAN (internal) | ~2022 | Earlier Descript baseline that "Improved RVQGAN" supersedes |
| **DAC / Improved RVQGAN** | 2023 | The published model |

### 2. License (Critical)

- **Code**: MIT. (Copyright "2023-present, Descript")
- **Weights**: **MIT — same license, no carve-out.** README explicitly: "Weights are released as part of this repo under MIT license."
- **Training data**: All public; no Descript-proprietary data.
- **Patents**: No known patent disclosures in repo or paper. Snake activation borrowed from BigVGAN (Kakao, 2022), no patent filings apparent — but no exhaustive patent search conducted.
- **HF cards caveat**: The HuggingFace model cards (`descript/dac_*`) have empty license fields ("[More Information Needed]") — authoritative license is the GitHub README. Production consumers should rely on the GitHub statement, not the HF card.

### 3. Model Variants

| Variant | Sample Rate | Parameters | Initial weights tag |
|---------|-------------|-----------|---------------------|
| 44 kHz | 44 100 | **76.6 M** | 0.0.1 |
| 24 kHz | 24 000 | **74.7 M** | 0.0.4 |
| 16 kHz | 16 000 | **74.1 M** | 0.0.5 |

Paper-reported 44 kHz breakdown: 22 M encoder + 54 M decoder. Bitrate: 8 kbps @ ~90× compression (44 kHz). Variable bitrate via quantizer dropout.

### 4. Distinguishing Descript Releases

| Name | What | Separate from DAC? |
|------|------|-------------------|
| Lyrebird (acquired 2019) | Voice cloning → became OverDub product | Yes |
| MelGAN (2019, MIT) | Vocoder (mel → waveform) | Yes |
| Internal RVQGAN (~2022) | DAC's own prior baseline | Superseded |
| **DAC (2023, MIT)** | Improved RVQGAN | The model |

### Reported Metrics (qualitative — full table in NeurIPS PDF)

- SI-SDR: 9.12 dB final (vs 6.92 dB without snake activation)
- "Out-performs all competing codecs at all bitrates in objective and subjective metrics" vs EnCodec/Lyra/Opus
- Higher MUSHRA than EnCodec at all bitrates

## Confidence

**Level**: high. Multiple primary sources agree: arXiv, GitHub LICENSE/README, three HF cards, NeurIPS OpenReview, releases page.

## Sources

- [1] https://arxiv.org/abs/2306.06546
- [2] https://github.com/descriptinc/descript-audio-codec/blob/main/LICENSE
- [3] https://github.com/descriptinc/descript-audio-codec/blob/main/README.md
- [4] https://github.com/descriptinc/descript-audio-codec/releases
- [5] https://huggingface.co/descript/dac_44khz
- [6] https://huggingface.co/descript/dac_24khz
- [7] https://huggingface.co/descript/dac_16khz
- [8] https://github.com/descriptinc/melgan-neurips
- [9] https://openreview.net/pdf?id=qjnl1QUnFA

## Open Questions

1. Full per-domain ViSQOL/Mel-distance metrics table (NeurIPS PDF only).
2. HuggingFace cards lack explicit license — authoritative is GitHub README.
3. Patent risk requires formal IP search if commercially deployed.
4. Post-Descript active maintenance status unclear.
5. Stereo vs mono weight coverage for each released variant.
